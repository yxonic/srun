use std::collections::HashMap;

use crate::{asset::AssetManager, sandbox::Sandbox, Error};

#[derive(Debug)]
pub enum Status {
    Start,
    PrepareAssets,
    BuildStageScript(String),
    RunStage(String),
    FinishStage(String),
    Success,
    Error(Error),
}

#[derive(Debug)]
pub struct StageSpec<'a> {
    pub(crate) image: &'a str,
    pub(crate) extend: &'a Vec<String>,
    pub(crate) script: &'a Vec<String>,
    pub(crate) envs: &'a HashMap<String, String>,
}

pub struct Runner<TRec: Recorder> {
    sandbox: Sandbox,
    assets: AssetManager,
    recorder: TRec,
    status: Status,
}

impl Runner<TextRecorder> {
    pub fn new() -> Runner<TextRecorder> {
        Runner {
            sandbox: Sandbox {},
            assets: AssetManager {},
            recorder: TextRecorder {},
            status: Status::Start,
        }
    }
}

impl<TRec: Recorder> Runner<TRec> {
    fn set_status(&mut self, status: Status) -> Result<(), Error> {
        self.status = status;
        self.recorder.emit_status(&self.status)?;
        Ok(())
    }
    pub fn prepare_assets(&mut self) -> Result<(), Error> {
        self.set_status(Status::PrepareAssets).report_err(self)?;
        self.assets.prepare().report_err(self)?;
        Ok(())
    }
    pub fn run_stage(&mut self, name: &str, _stage: StageSpec) -> Result<(), Error> {
        self.set_status(Status::BuildStageScript(name.into()))
            .report_err(self)?;
        self.sandbox.build().report_err(self)?;
        self.set_status(Status::RunStage(name.into()))
            .report_err(self)?;
        self.sandbox.run().report_err(self)?;
        Ok(())
    }
}

impl<TRec: Recorder> Drop for Runner<TRec> {
    fn drop(&mut self) {
        if matches!(self.status, Status::Error(_)) {
            // runner is already dead, and the error has been reported
            return;
        }
        // indicates that all stages finished successfully
        self.recorder.emit_status(&Status::Success).unwrap();
    }
}

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
        println!("status: {:?}", status);
        Ok(())
    }
}

trait Reporter<TR: Recorder> {
    fn report_err(self, runner: &mut Runner<TR>) -> Self;
}

impl<T, TR: Recorder> Reporter<TR> for Result<T, Error> {
    fn report_err(self, r: &mut Runner<TR>) -> Self {
        if let Err(e) = &self {
            r.set_status(Status::Error(e.clone()))?;
        }
        self
    }
}
