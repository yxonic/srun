use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    runner::{Recorder, Runner, StageSpec},
    Error,
};

/// Stage specification.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Stage {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extend: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    script: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    envs: Option<HashMap<String, String>>,
}

/// Task specification.
#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    #[serde(skip_serializing_if = "Option::is_none")]
    stages: Option<Vec<Stage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    assets: Option<HashMap<String, String>>,
    #[serde(flatten)]
    defaults: Stage,
}

impl Task {
    pub async fn run<T: Recorder>(self, runner: &mut Runner<'_, T>) -> Result<(), Error> {
        // TODO:
        // 1. prepare assets properly
        // 2. fill stage spec with default value, or die if not provided anyhow
        // 3. stage spec construction
        runner.prepare_assets().map_err(|e| e.0)?;

        let stages = self.stages.unwrap_or_else(|| vec![Stage::default()]);
        for (i, stage) in stages.into_iter().enumerate() {
            let defaults = &self.defaults;
            runner
                .run_stage(
                    &stage.name.unwrap_or_else(|| format!("stage-{}", i)),
                    StageSpec {
                        image: stage
                            .image
                            .or_else(|| defaults.image.clone())
                            .ok_or(Error::UnknownError)?,
                        extend: stage
                            .extend
                            .or_else(|| defaults.extend.clone())
                            .unwrap_or_default(),
                        script: stage
                            .script
                            .or_else(|| defaults.script.clone())
                            .unwrap_or_default(),
                        envs: stage
                            .envs
                            .or_else(|| defaults.envs.clone())
                            .unwrap_or_default(),
                    },
                )
                .await
                .map_err(|e| e.0)?;
        }
        Ok(())
    }
}
