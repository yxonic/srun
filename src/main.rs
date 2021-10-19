use std::convert::TryInto;
use std::fs;

use anyhow::{Context, Result};
use clap::{App, Arg};
use srun::{Runner, Task};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let matches = App::new("srun")
        .arg(
            Arg::new("INPUT")
                .about("Input yaml file describing the task")
                .required(true)
                .index(1),
        )
        .get_matches();

    let file = matches
        .value_of("INPUT")
        .context("task script not provided")?;

    let task_str = fs::read_to_string(file).context("task script not found")?;
    let task = Task::from_yaml(&task_str)?;

    let docker = shiplift::Docker::new();

    {
        let mut runner = Runner::new(&docker);
        let r = task.run(&mut runner).await;
        if let Err(srun::Error::ErrorCode(code)) = r {
            std::process::exit(code.try_into().unwrap());
        }
        r.context("failed to run task")?;
        // drop to ensure runner finalize gracefully
    }

    Ok(())
}
