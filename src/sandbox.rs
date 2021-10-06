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
        let options = BuildOptions::builder("/tmp/test").build();
        let mut stream = self.docker.images().build(&options);
        while let Some(build_result) = stream.next().await {
            match build_result {
                Ok(output) => println!("{:?}", output),
                Err(_) => return Err(Error::UnknownError),
            }
        }
        Ok("".into())
    }
    pub fn run(&self) -> Result<(), Error> {
        Err(Error::UnknownError)
    }
}
