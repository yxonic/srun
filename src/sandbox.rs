//! Low-level sandboxing and running facilities.
use std::io::Write;
use std::{collections::HashMap, fs::File};

use futures::StreamExt;
use shiplift::{BuildOptions, Docker};

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

    /// Build docker image for the use of later stages.
    pub async fn build(&self, image: &str, extend: &[String]) -> Result<String, Error> {
        let dir = tempfile::tempdir().map_err(Error::IOError)?;
        let dir_path = dir.path().to_str().expect("tempdir should always be valid");

        {
            let file_path = dir.path().join("Dockerfile");
            log::debug!("writing Dockerfile at: {:?}", file_path);
            let mut file = File::create(file_path).map_err(Error::IOError)?;
            writeln!(file, "FROM {}", image).map_err(Error::IOError)?;
            if !extend.is_empty() {
                writeln!(file, "RUN {}", extend.join(" && ")).map_err(Error::IOError)?;
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
    pub fn run(&self, _script: &[String], _envs: &HashMap<String, String>) -> Result<(), Error> {
        todo!("stage run not implemented")
    }
}
