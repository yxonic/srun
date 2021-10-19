//! Low-level sandboxing and running facilities.
use std::io::Write;
use std::path::Path;
use std::str::from_utf8;
use std::time::Duration;
use std::{collections::HashMap, fs::File};

use futures::future::join;
use futures::StreamExt;
use shiplift::tty::TtyChunk;
use shiplift::{BuildOptions, Container, ContainerOptions, Docker, LogsOptions};

use crate::reporter::Reporter;
use crate::Error;

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

        let options = BuildOptions::builder(dir_path).build();
        let mut stream = self.docker.images().build(&options);

        log::info!(
            "building image for task from `{}` with {} lines of extend script",
            image,
            extend.len()
        );

        while let Some(build_result) = stream.next().await {
            match build_result {
                Ok(output) => {
                    log::debug!("builder output: {}", output);
                    if let Some(aux) = output.get("aux") {
                        if let Some(id) = aux.get("ID") {
                            // extract image sha256 and return
                            // id is given in the form of "sha256:<id>" (with quotes)
                            let id = id.to_string();
                            let id = id
                                .trim_matches('"')
                                .split(':')
                                .nth(1)
                                .expect("id should be given in form of \"sha256:<id>\"");
                            log::info!("successfully built: {}", id);
                            return Ok(id.into());
                        }
                    }
                    if let Some(error) = output.get("error") {
                        return Err(Error::BuildError(error.to_string()));
                    }
                }
                Err(shiplift::Error::Hyper(e)) => {
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
        image: &str,
        workdir: &str,
        script: &[String],
        envs: &HashMap<String, String>,
        mounts: &HashMap<String, String>,
        asset_path: &Path,
        reporter: &impl Reporter,
    ) -> Result<(), Error> {
        log::info!("create container using {} with envs {:?}", image, envs);

        let file_path = asset_path.join(".run.sh");
        log::debug!("writing stage script at: {:?}", file_path);
        let mut file = File::create(file_path)?;
        for line in script {
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
        for (k, v) in mounts {
            // TODO: check for permission
            binds.push(format!(
                "{}:{}:ro",
                Path::new(v)
                    .canonicalize()?
                    .to_str()
                    .expect("path should always be valid utf-8 string"),
                k,
            ));
        }

        let options = ContainerOptions::builder(image)
            .volumes(binds.iter().map(|s| s as &str).collect())
            .working_dir(workdir)
            .cmd(vec!["sh", "-e", "/assets/.run.sh"])
            .env(
                // TODO: probably better solution needed
                envs.iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<String>>()
                    .iter()
                    .map(|s| s as &str)
                    .collect::<Vec<&str>>(),
            )
            .attach_stdout(true)
            .attach_stderr(true)
            // TODO: make resource restrictions configurable
            .stop_timeout(Duration::from_secs(3 * 60))
            .cpus(1.0)
            .memory(1 << 30)
            .network_mode("none")
            .auto_remove(true)
            .build();

        let container = self.docker.containers().create(&options).await?;

        log::info!("created container with id: {}", container.id);

        let container = self.docker.containers().get(&container.id);
        container.start().await?;

        log::info!("container started");

        log::debug!("processing logs and wait for container to finish");
        let log_op = self.process_logs(&container, reporter);
        let wait_op = container.wait();
        let (log, exit) = join(log_op, wait_op).await;

        let _ = log?;
        let e = exit?;

        log::info!("container exited with code {}", e.status_code);
        if e.status_code > 0 {
            // report exit code if failed
            reporter.report_stderr(
                &format!("[program exited with code {}]", e.status_code),
                chrono::Utc::now(),
            )?;
            return Err(Error::ErrorCode(e.status_code));
        }

        Ok(())
    }

    async fn process_logs(
        &self,
        container: &Container<'_>,
        reporter: &impl Reporter,
    ) -> Result<(), Error> {
        let mut stream = container.logs(
            &LogsOptions::builder()
                .follow(true)
                .timestamps(true)
                .stdout(true)
                .stderr(true)
                .build(),
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
                TtyChunk::StdOut(bytes) => {
                    let line = from_utf8(&bytes)?;
                    log::debug!("stdout | {}", line.trim_end());
                    reporter.emit_stdout(line)?
                }
                TtyChunk::StdErr(bytes) => {
                    let line = from_utf8(&bytes)?;
                    log::debug!("stderr | {}", line.trim_end());
                    reporter.emit_stderr(line)?
                }
                _ => unreachable!(),
            };
        }
        Ok(())
    }
}
