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
pub struct StageSpec {
    pub(crate) image: String,
    pub(crate) extend: Vec<String>,
    pub(crate) script: Vec<String>,
    pub(crate) envs: HashMap<String, String>,
}

/// Task runner that prepares for the task, runs the task, tracks running state,
/// and report the process. 
///
/// You should always initiate a new runner for each task.
pub struct Runner<'docker, TRecorder: Recorder> {
    assets: AssetManager,
    sandbox: Sandbox<'docker>,
    status: Status,
    recorder: TRecorder,
}

impl<'a> Runner<'a, TextRecorder> {
    pub fn new(docker: &shiplift::Docker) -> Runner<TextRecorder> {
        Runner {
            sandbox: Sandbox::new(docker),
            assets: AssetManager {},
            recorder: TextRecorder {},
            status: Status::Start,
        }
    }
}

impl<'a, T: Recorder> Runner<'a, T> {
    fn set_status(&mut self, status: Status) -> Result<(), HandledError> {
        self.status = status;
        // do not report error again when reporting has failed
        self.recorder.emit_status(&self.status).ignore()?;
        Ok(())
    }
    pub fn prepare_assets(&mut self) -> Result<(), HandledError> {
        self.set_status(Status::PrepareAssets)?;
        self.assets.prepare().handle(self)?;
        Ok(())
    }
    pub async fn run_stage(&mut self, name: &str, stage: StageSpec) -> Result<(), HandledError> {
        self.set_status(Status::BuildStageScript(name.into()))?;
        self.sandbox.build(&stage.image, &stage.extend).await.handle(self)?;
        self.set_status(Status::RunStage(name.into()))?;
        self.sandbox.run().handle(self)?;
        Ok(())
    }
}

impl<'a, T: Recorder> Drop for Runner<'a, T> {
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

/// Represents an error that has been properly handled (reported to the
/// recorder) by runner.
pub struct HandledError(Error);

impl HandledError {
    pub fn unwrap(self) -> Error {
        self.0
    }
}

trait ErrorHandler<T> {
    fn handle<TR: Recorder>(self, runner: &mut Runner<TR>) -> Result<T, HandledError>;
    fn ignore(self) -> Result<T, HandledError>;
}

impl<T> ErrorHandler<T> for Result<T, Error> {
    fn handle<TR: Recorder>(self, r: &mut Runner<TR>) -> Result<T, HandledError> {
        if let Err(e) = &self {
            r.set_status(Status::Error(e.clone()))?;
        }
        // now error has been reported
        self.map_err(HandledError)
    }
    fn ignore(self) -> Result<T, HandledError> {
        self.map_err(HandledError)
    }
}
