//! High-level task runner and status management.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::{
    asset::AssetManager,
    permission::Permissions,
    reporter::{Reporter, TextReporter},
    sandbox::{RunOptions, Sandbox},
    Error,
};

pub use crate::sandbox::RunOptions as StageSpec;

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

/// Task runner that prepares for the task, runs the task, tracks running state,
/// and report the process.
///
/// You should always initiate a new runner for each task.
pub struct Runner<'docker, TReporter: RunnerReporter> {
    sandbox: Sandbox<'docker>,
    status: Status,
    assets: AssetManager,
    permisssions: Permissions,
    reporter: TReporter,
}

impl Runner<'_, TextReporter> {
    pub fn new(docker: &shiplift::Docker) -> Result<Runner<TextReporter>, Error> {
        Ok(Runner {
            sandbox: Sandbox::new(docker),
            assets: AssetManager::new()?,
            reporter: TextReporter {},
            permisssions: Permissions::default(),
            status: Status::Start,
        })
    }
}

impl<T: RunnerReporter> Runner<'_, T> {
    fn set_status(&mut self, status: Status) -> Result<(), HandledError> {
        log::info!("changing status: {:?} -> {:?}", self.status, status);
        self.status = status;
        // do not report error again when reporting has failed
        self.reporter.emit_status(&self.status).ignore()?;
        Ok(())
    }
    pub async fn prepare_assets(
        &mut self,
        assets: HashMap<String, String>,
    ) -> Result<(), HandledError> {
        self.set_status(Status::PrepareAssets)?;
        self.assets.prepare(assets).await.handle(self)?;
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

        self.sandbox
            .run(
                &RunOptions { image, ..stage },
                &self.assets,
                &self.permisssions,
                &self.reporter,
            )
            .await
            .handle(self)?;
        Ok(())
    }
}

impl<T: RunnerReporter> Drop for Runner<'_, T> {
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
    fn handle(self, runner: &mut Runner<impl RunnerReporter>) -> Result<T, HandledError>;
    fn ignore(self) -> Result<T, HandledError>;
}

impl<T, E> ErrorHandler<T> for Result<T, E>
where
    E: Into<Error> + std::fmt::Debug,
{
    fn handle(self, r: &mut Runner<impl RunnerReporter>) -> Result<T, HandledError> {
        match self {
            Err(e) => {
                r.set_status(Status::Error(format!("{:?}", e)))?;
                Err(HandledError(e.into()))
            }
            Ok(r) => Ok(r),
        }
    }
    fn ignore(self) -> Result<T, HandledError> {
        self.map_err(|e| HandledError(e.into()))
    }
}

impl From<HandledError> for Error {
    fn from(e: HandledError) -> Self {
        e.0
    }
}

/// Report status
pub trait RunnerReporter: Reporter {
    fn emit_status(&self, status: &Status) -> Result<(), Error> {
        self.report_status(status, Utc::now())
    }
    fn report_status(&self, status: &Status, timestamp: DateTime<Utc>) -> Result<(), Error>;
}

impl RunnerReporter for TextReporter {
    fn report_status(&self, status: &Status, _: DateTime<Utc>) -> Result<(), Error> {
        if let Status::Error(e) = status {
            log::warn!("error: {:?}", e);
        }
        Ok(())
    }
}
