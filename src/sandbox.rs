use futures::StreamExt;
use shiplift::{BuildOptions, Docker};

use crate::Error;

pub struct Sandbox<'docker> {
    docker: &'docker Docker,
}

impl Sandbox<'_> {
    pub fn new(docker: &Docker) -> Sandbox {
        Sandbox { docker }
    }
    // build docker image for the use of
    pub async fn build(&self, _image: &str, _extend: &[String]) -> Result<String, Error> {
        // TODO:
        // 1. use tempfile to build correct docker image
        // 2. proper error reporting
        let options = BuildOptions::builder("/tmp/test/").build();
        let mut stream = self.docker.images().build(&options);
        while let Some(build_result) = stream.next().await {
            match build_result {
                Ok(output) => {
                    if let Some(aux) = output.get("aux") {
                        if let Some(id) = aux.get("ID") {
                            // extract image sha256 and return
                            // id is given in the form of "sha256:<id>" (with quotes)
                            let id = id.to_string();
                            let id = id.trim_matches('"').split(':').nth(1);
                            return id.map(|e| e.to_string()).ok_or(Error::UnknownError);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{:?}", e);
                    return Err(Error::UnknownError);
                }
            }
        }
        Err(Error::UnknownError)
    }
    pub fn run(&self) -> Result<(), Error> {
        Err(Error::UnknownError)
    }
}
