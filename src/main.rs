use std::fs;
use std::{convert::TryInto, path::PathBuf};

use anyhow::{Context, Result};
use clap::{App, Arg};
use srun::{Permissions, PermissionsOptions, Runner, Task};

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
        .arg(
            Arg::new("allow-net")
                .about("Allow network access")
                .long("--allow-net"),
        )
        .arg(
            Arg::new("allow-read")
                .about("Allow read access")
                .long("--allow-read")
                .takes_value(true)
                .multiple_values(true),
        )
        .arg(
            Arg::new("allow-write")
                .about("Allow write access")
                .long("--allow-write")
                .takes_value(true)
                .multiple_values(true),
        )
        .get_matches();

    let file = matches
        .value_of("INPUT")
        .context("task script not provided")?;

    let task_str = fs::read_to_string(file).context("task script not found")?;
    let task = Task::from_yaml(&task_str)?;

    let docker = bollard::Docker::connect_with_socket_defaults()?;

    let permissions = Permissions::from_options(&PermissionsOptions {
        allow_read: matches.values_of("allow-read").map(|e| {
            e.map(|v| {
                PathBuf::from(v)
                    .canonicalize()
                    .context(format!("path {} not exist", v))
                    .unwrap()
            })
            .collect()
        }),
        allow_write: matches.values_of("allow-write").map(|e| {
            e.map(|v| {
                PathBuf::from(v)
                    .canonicalize()
                    .context(format!("path {} not exist", v))
                    .unwrap()
            })
            .collect()
        }),
        allow_net: matches.is_present("allow-net"),
    });

    log::info!("run with permission: {:?}", permissions);

    {
        let mut runner = Runner::new(&docker, Some(permissions))?;
        let r = task.run(&mut runner).await;
        if let Err(srun::Error::ErrorCode(code)) = r {
            std::process::exit(code.try_into().unwrap());
        }
        r.context("failed to run task")?;
        // drop to ensure runner finalize gracefully
    }

    Ok(())
}
