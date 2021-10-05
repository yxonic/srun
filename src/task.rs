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
    pub fn run<T: Recorder>(&self, runner: &mut Runner<T>) -> Result<(), Error> {
        // TODO:
        // 1. prepare assets properly
        // 2. fill stage spec with default value, or die if not provided anyhow
        // 3. stage spec construction
        runner.prepare_assets()?;
        let name = String::from("main");
        let empty = vec![];
        let stages = self.stages.as_ref().unwrap_or(&empty);
        for stage in stages {
            let empty_vec = vec![];
            let empty_map = HashMap::new();
            runner.run_stage(
                stage.name.as_ref().unwrap_or(&name),
                StageSpec {
                    image: stage.image.as_ref().unwrap_or(&name),
                    extend: stage.extend.as_ref().unwrap_or(&empty_vec),
                    script: stage.script.as_ref().unwrap_or(&empty_vec),
                    envs: stage.envs.as_ref().unwrap_or(&empty_map),
                },
            )?;
        }
        Ok(())
    }
}
