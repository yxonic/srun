use std::fs;

use anyhow::{Context, Result};
use srun::{Runner, Task};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let task_str = fs::read_to_string("examples/task.yaml").context("task script not found")?;
    let task = Task::from_yaml(&task_str)?;

    let docker = shiplift::Docker::new();

    {
        let mut runner = Runner::new(&docker);
        task.run(&mut runner).await.context("failed to run task")?;
        // drop to ensure runner finalize gracefully
    }

    Ok(())
}
