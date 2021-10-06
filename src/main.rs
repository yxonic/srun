use std::fs;

use anyhow::{Context, Result};
use srun::{Runner, Task};

#[tokio::main]
async fn main() -> Result<()> {
    let task_str = fs::read_to_string("examples/task.yaml").context("task script not found")?;
    let task: Task = serde_yaml::from_str(&task_str)?;

    let docker = shiplift::Docker::new();

    {
        let mut runner = Runner::new(&docker);
        task.run(&mut runner).await?;
        // drop to ensure runner finalize gracefully
    }

    Ok(())
}
