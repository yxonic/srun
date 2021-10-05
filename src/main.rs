use std::fs;

use anyhow::{Context, Result};
use srun::{Runner, Task};

fn main() -> Result<()> {
    let task_str = fs::read_to_string("examples/task.yaml").context("task script not found")?;
    let task: Task = serde_yaml::from_str(&task_str)?;

    {
        let mut runner = Runner::new();
        task.run(&mut runner)?;
    }

    Ok(())
}
