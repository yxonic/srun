use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    runner::{Recorder, Runner, StageSpec},
    Error,
};

/// Stage specification.
#[derive(Debug, Serialize, Deserialize)]
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
    pub async fn run<'docker, T: Recorder>(self, runner: &mut Runner<'docker, T>) -> Result<(), Error> {
        // TODO:
        // 1. prepare assets properly
        // 2. fill stage spec with default value, or die if not provided anyhow
        // 3. stage spec construction
        runner.prepare_assets().map_err(|e| e.into())?;
        let name = String::from("main");
        let stages = self.stages.unwrap_or(vec![]);
        for stage in stages.into_iter() {
            runner
                .run_stage(
                    stage.name.as_ref().unwrap_or(&name),
                    StageSpec {
                        image: stage.image.unwrap_or_default(),
                        extend: stage.extend.unwrap_or_default(),
                        script: stage.script.unwrap_or_default(),
                        envs: stage.envs.unwrap_or_default(),
                    },
                )
                .await
                .map_err(|e| e.into())?;
        }
        Ok(())
    }
}
