use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    reporter::Reporter,
    runner::{Runner, StageSpec},
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
    workdir: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    mounts: Option<HashMap<String, String>>,
    #[serde(flatten)]
    defaults: Stage,
}

impl Task {
    pub fn from_yaml(s: &str) -> Result<Task, Error> {
        let task = serde_yaml::from_str(s).map_err(|e| Error::SpecError(e.to_string()))?;
        // TODO: validate
        Ok(task)
    }

    pub async fn run(self, runner: &mut Runner<'_, impl Reporter>) -> Result<(), Error> {
        // TODO: prepare assets properly
        runner.prepare_assets()?;

        let mounts = self.mounts.unwrap_or_default();
        let stages = self.stages.unwrap_or_else(|| vec![Stage::default()]);
        for (i, stage) in stages.into_iter().enumerate() {
            let name = stage.name.unwrap_or_else(|| format!("stage-{}", i));
            let defaults = &self.defaults;
            runner
                .run_stage(
                    &name,
                    StageSpec {
                        image: stage
                            .image
                            .or_else(|| defaults.image.clone())
                            .unwrap_or_default(),
                        extend: stage
                            .extend
                            .or_else(|| defaults.extend.clone())
                            .unwrap_or_default(),
                        workdir: stage
                            .workdir
                            .or_else(|| defaults.workdir.clone())
                            .unwrap_or_else(|| String::from("/workspace")),
                        script: stage
                            .script
                            .or_else(|| defaults.script.clone())
                            .unwrap_or_default(),
                        envs: stage
                            .envs
                            .or_else(|| defaults.envs.clone())
                            .unwrap_or_default(),
                        mounts: mounts.to_owned(),
                    },
                )
                .await?;
        }
        Ok(())
    }
}
