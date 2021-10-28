use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::str::from_utf8;

use bollard::container::{Config, LogOutput, LogsOptions};
use bollard::image::BuildImageOptions;
use bollard::models::HostConfig;
use bollard::Docker;
use futures::future::join;
use futures::StreamExt;

use crate::{permission::Permissions, AssetManager, Error, Reporter};

/// Represents a sandboxed environment for task building and running.
pub struct Sandbox<'docker> {
    docker: &'docker Docker,
}

impl Sandbox<'_> {
    /// Create a new sandbox environment.
    pub fn new(docker: &Docker) -> Sandbox {
        Sandbox { docker }
    }

    /// Build docker image and return image ID.
    pub async fn build(&self, image: &str, extend: &[String]) -> Result<String, Error> {
        let dir = tempfile::tempdir()?;
        let dir_path = dir.path().to_str().expect("tempdir should always be valid");

        {
            let file_path = dir.path().join("Dockerfile");
            log::debug!("writing Dockerfile at: {:?}", file_path);
            let mut file = File::create(file_path)?;
            writeln!(file, "FROM {}", image)?;
            if !extend.is_empty() {
                writeln!(file, "RUN {}", extend.join(" && ").replace('\n', ""))?;
            }
        }

        let options = BuildImageOptions::<String>::default();
        let mut bytes = vec![];
        tarball::dir(&mut bytes, dir_path)?;
        let mut stream = self
            .docker
            .build_image(options, None, Some(hyper::Body::from(bytes)));

        log::info!(
            "building image for task from `{}` with {} lines of extend script",
            image,
            extend.len()
        );

        while let Some(build_result) = stream.next().await {
            match build_result {
                Ok(output) => {
                    log::debug!("builder output: {:?}", output);
                    if let Some(aux) = output.aux {
                        if let Some(id) = aux.id {
                            // extract image sha256 and return
                            // id is given in the form of "sha256:<id>" (with quotes)
                            let id = id;
                            let id = id
                                .trim_matches('"')
                                .split(':')
                                .nth(1)
                                .expect("id should be given in form of \"sha256:<id>\"");
                            log::info!("successfully built: {}", id);
                            return Ok(id.into());
                        }
                    }
                    if let Some(error) = output.error {
                        return Err(Error::BuildError(error));
                    }
                }
                Err(bollard::errors::Error::HyperResponseError { err: e }) => {
                    return Err(Error::ConnectionError(e));
                }
                Err(e) => {
                    return Err(Error::BuildError(format!("{:?}", e)));
                }
            }
        }
        Err(Error::UnknownError("image not successfully built".into()))
    }

    /// Run scripts with envs.
    pub async fn run(
        &self,
        options: RunOptions,
        asset: &AssetManager,
        permissions: &Permissions,
        reporter: &impl Reporter,
    ) -> Result<(), Error> {
        log::info!(
            "create container using {} with envs {:?}",
            options.image,
            options.envs
        );

        let asset_path = asset.path();
        let file_path = asset_path.join(".run.sh");
        log::debug!("writing stage script at: {:?}", file_path);
        let mut file = File::create(file_path)?;
        for line in options.script.iter() {
            writeln!(file, "{}", line)?;
        }
        file.flush()?;

        let mut binds: Vec<String> = vec![];
        binds.push(format!(
            "{}:/assets",
            asset_path
                .to_str()
                .expect("path should always be valid utf-8 string")
        ));
        for (k, v) in options.mounts.iter() {
            let path = Path::new(v).canonicalize()?;
            binds.push(format!(
                "{}:{}{}",
                path.to_str()
                    .expect("path should always be valid utf-8 string"),
                k,
                if permissions.write.check(&path).is_ok() {
                    ""
                } else if let Err(e) = permissions.read.check(&path) {
                    return Err(e);
                } else {
                    ":ro"
                }
            ));
        }

        let config = Config {
            image: Some(options.image),
            tty: Some(true),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            env: Some(
                options
                    .envs
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<String>>(),
            ),
            network_disabled: if permissions.net.check().is_ok() {
                None
            } else {
                Some(true)
            },
            stop_timeout: Some(3 * 60),
            working_dir: Some(options.workdir),
            cmd: Some(
                vec!["sh", "-e", "/assets/.run.sh"]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
            ),
            host_config: Some(HostConfig {
                nano_cpus: Some(1_000_000_000),
                memory: Some(1 << 30),
                binds: Some(binds),
                auto_remove: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };

        let container = self
            .docker
            .create_container::<String, String>(None, config)
            .await?;

        log::info!("created container with id: {}", container.id);

        self.docker
            .start_container::<String>(&container.id, None)
            .await?;

        log::info!("container started");

        log::debug!("processing logs and wait for container to finish");

        let log_op = self.process_logs(&container.id, reporter);
        let mut stream = self.docker.wait_container::<String>(&container.id, None);
        let wait_op = stream.next();
        let (log, exit) = join(log_op, wait_op).await;

        let _ = log?;
        let e =
            exit.ok_or_else(|| Error::UnknownError("failed to fetch wait response".into()))??;

        log::info!("container exited with code {}", e.status_code);
        if e.status_code > 0 {
            // report exit code if failed
            reporter.report_stderr(
                &format!("[program exited with code {}]", e.status_code),
                chrono::Utc::now(),
            )?;
            return Err(Error::ErrorCode(e.status_code as u64));
        }

        Ok(())
    }

    async fn process_logs(
        &self,
        container_id: &str,
        reporter: &impl Reporter,
    ) -> Result<(), Error> {
        let mut stream = self.docker.logs::<String>(
            container_id,
            Some(LogsOptions {
                stdout: true,
                stderr: true,
                follow: true,
                ..Default::default()
            }),
        );

        // TODO: get limit from configuration
        let mut limit = 500;
        while let Some(exec_result) = stream.next().await {
            limit -= 1;
            if limit < 0 {
                break;
            }
            let chunk = exec_result?;
            match chunk {
                LogOutput::StdOut { message: bytes } => {
                    let line = from_utf8(&bytes)?;
                    log::debug!("stdout | {}", line.trim_end());
                    reporter.emit_stdout(line)?;
                }
                LogOutput::StdErr { message: bytes } => {
                    let line = from_utf8(&bytes)?;
                    log::debug!("stderr | {}", line.trim_end());
                    reporter.emit_stderr(line)?;
                }
                LogOutput::Console { message: bytes } => {
                    let line = from_utf8(&bytes)?;
                    log::debug!("console | {}", line.trim_end());
                    reporter.emit_console(line)?;
                }
                _ => unreachable!(),
            };
        }
        Ok(())
    }
}

/// Defines a stage to be run by runner.
#[derive(Debug)]
pub struct RunOptions {
    pub(crate) image: String,
    pub(crate) extend: Vec<String>,
    pub(crate) workdir: String,
    pub(crate) script: Vec<String>,
    pub(crate) envs: HashMap<String, String>,
    pub(crate) mounts: HashMap<String, String>,
}

mod tarball {
    // copied from shiplift
    use crate::Error;
    use flate2::{write::GzEncoder, Compression};
    use std::{
        fs::{self, File},
        io::{self, Write},
        path::{Path, MAIN_SEPARATOR},
    };
    use tar::Builder;

    // todo: this is pretty involved. (re)factor this into its own crate
    pub fn dir<W>(buf: W, path: &str) -> Result<(), Error>
    where
        W: Write,
    {
        let mut archive = Builder::new(GzEncoder::new(buf, Compression::best()));
        fn bundle<F>(dir: &Path, f: &mut F, bundle_dir: bool) -> io::Result<()>
        where
            F: FnMut(&Path) -> io::Result<()>,
        {
            if fs::metadata(dir)?.is_dir() {
                if bundle_dir {
                    f(dir)?;
                }
                for entry in fs::read_dir(dir)? {
                    let entry = entry?;
                    if fs::metadata(entry.path())?.is_dir() {
                        bundle(&entry.path(), f, true)?;
                    } else {
                        f(entry.path().as_path())?;
                    }
                }
            }
            Ok(())
        }

        {
            let base_path = Path::new(path).canonicalize()?;
            // todo: don't unwrap
            let mut base_path_str = base_path.to_str().unwrap().to_owned();
            if let Some(last) = base_path_str.chars().last() {
                if last != MAIN_SEPARATOR {
                    base_path_str.push(MAIN_SEPARATOR)
                }
            }

            let mut append = |path: &Path| {
                let canonical = path.canonicalize()?;
                // todo: don't unwrap
                let relativized = canonical
                    .to_str()
                    .expect("canonical path must be valid")
                    .trim_start_matches(&base_path_str[..]);
                if path.is_dir() {
                    archive.append_dir(Path::new(relativized), &canonical)?
                } else {
                    archive.append_file(Path::new(relativized), &mut File::open(&canonical)?)?
                }
                Ok(())
            };
            bundle(Path::new(path), &mut append, false)?;
        }
        archive.finish()?;

        Ok(())
    }
}
