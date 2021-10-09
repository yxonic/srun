use crate::{runner::Status, Error};

pub trait Reporter {
    fn emit_status(&self, _status: &Status) -> Result<(), Error> {
        Ok(())
    }
    fn emit_stdout(&self, _line: &str) -> Result<(), Error> {
        Ok(())
    }
    fn emit_stderr(&self, _line: &str) -> Result<(), Error> {
        Ok(())
    }
}

pub struct TextReporter;

impl Reporter for TextReporter {
    fn emit_status(&self, status: &Status) -> Result<(), Error> {
        log::info!("status: {:?}", status);
        Ok(())
    }
}
