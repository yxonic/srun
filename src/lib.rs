//! `srun` is a library and a command-line tool for running specific tasks in a
//! sandbox environment. Tasks are designed to be specified by structural input
//! like a YAML script. Therefore, this library is also capable of building a
//! remote runner service.

mod asset;
mod error;
mod permission;
mod reporter;
pub mod runner;
pub mod sandbox;
mod task;

pub use asset::AssetManager;
pub use error::Error;
pub use reporter::Reporter;
pub use runner::Runner;
pub use sandbox::Sandbox;
pub use task::Task;
