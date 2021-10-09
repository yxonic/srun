use crate::{runner::Status, Error};

pub trait Recorder {
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

pub struct TextRecorder;

impl Recorder for TextRecorder {
    fn emit_status(&self, status: &Status) -> Result<(), Error> {
        log::info!("status: {:?}", status);
        Ok(())
    }
}
