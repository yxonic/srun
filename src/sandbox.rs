use futures::StreamExt;
use shiplift::{Docker, BuildOptions};

use crate::Error;

pub struct Sandbox<'docker> {
    docker: &'docker Docker
}

impl<'docker> Sandbox<'docker> {
    pub fn new(docker: &Docker) -> Sandbox {
        Sandbox { docker }
    }
    pub async fn build(&self, image: &str, extend: &Vec<String>) -> Result<(), Error> {
        let options = BuildOptions::builder("/tmp/test").build();
        let mut stream = self.docker.images().build(&options);
        while let Some(build_result) = stream.next().await {
            match build_result {
                Ok(output) => println!("{:?}", output),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Ok(())
    }
    pub fn run(&self) -> Result<(), Error> {
        Err(Error::UnknownError)
    }
}
