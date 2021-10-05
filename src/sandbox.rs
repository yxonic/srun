use crate::Error;

pub struct Sandbox;

impl Sandbox {
    pub fn build(&self) -> Result<(), Error> {
        Ok(())
    }
    pub fn run(&self) -> Result<(), Error> {
        Err(Error::UnknownError)
    }
}
