//! High-level task runner and status management.

use std::collections::HashMap;

use crate::{
    asset::AssetManager,
    reporter::{Reporter, TextReporter},
    sandbox::Sandbox,
    Error,
};

#[derive(Debug)]
pub enum Status {
    Start,
    PrepareAssets,
    BuildStageScript(String),
    RunStage(String),
    FinishStage(String),
    Success,
    Error(String),
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
pub struct Runner<'docker, TReporter: Reporter> {
    assets: AssetManager,
    sandbox: Sandbox<'docker>,
    status: Status,
    reporter: TReporter,
}

impl Runner<'_, TextReporter> {
    pub fn new(docker: &shiplift::Docker) -> Runner<TextReporter> {
        Runner {
            sandbox: Sandbox::new(docker),
            assets: AssetManager {},
            reporter: TextReporter {},
            status: Status::Start,
        }
    }
}

impl<T: Reporter> Runner<'_, T> {
    fn set_status(&mut self, status: Status) -> Result<(), HandledError> {
        log::info!("changing status: {:?} -> {:?}", self.status, status);
        self.status = status;
        // do not report error again when reporting has failed
        self.reporter.emit_status(&self.status).ignore()?;
        Ok(())
    }
    pub fn prepare_assets(&mut self) -> Result<(), HandledError> {
        self.set_status(Status::PrepareAssets)?;
        self.assets.prepare().handle(self)?;
        Ok(())
    }
    pub async fn run_stage(&mut self, name: &str, stage: StageSpec) -> Result<(), HandledError> {
        log::info!("running stage: {}", name);

        log::info!("build stage script for `{}`", name);
        self.set_status(Status::BuildStageScript(name.into()))?;
        let image = self
            .sandbox
            .build(&stage.image, &stage.extend)
            .await
            .handle(self)?;

        log::info!("run stage `{}` with image: {}", name, image);
        self.set_status(Status::RunStage(name.into()))?;

        let dir = tempfile::tempdir().map_err(Error::IOError).handle(self)?;

        self.sandbox
            .run(
                &image,
                &stage.script,
                &stage.envs,
                dir.path(),
                &self.reporter,
            )
            .await
            .handle(self)?;
        Ok(())
    }
}

impl<T: Reporter> Drop for Runner<'_, T> {
    fn drop(&mut self) {
        if matches!(self.status, Status::Error(_)) {
            // runner is already dead, and the error has been reported
            return;
        }
        if std::thread::panicking() {
            // do not report when panicking
            return;
        }
        // indicates that all stages finished successfully
        self.set_status(Status::Success).unwrap();
    }
}

/// Represents an error that has been properly handled (reported) by runner.
#[derive(Debug)]
pub struct HandledError(pub Error);

trait ErrorHandler<T> {
    fn handle<TR: Reporter>(self, runner: &mut Runner<TR>) -> Result<T, HandledError>;
    fn ignore(self) -> Result<T, HandledError>;
}

impl<T> ErrorHandler<T> for Result<T, Error> {
    fn handle<TR: Reporter>(self, r: &mut Runner<TR>) -> Result<T, HandledError> {
        if let Err(e) = &self {
            r.set_status(Status::Error(format!("{:?}", e)))?;
        }
        // now error has been reported
        self.map_err(HandledError)
    }
    fn ignore(self) -> Result<T, HandledError> {
        self.map_err(HandledError)
    }
}
