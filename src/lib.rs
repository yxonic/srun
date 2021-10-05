//! `srun` is a library and a command-line tool for running specific tasks in a
//! sandbox environment. Tasks are designed to be specified by structural input
//! like a YAML script. Therefore, this library is also capable of building a
//! remote runner service.

pub mod asset;
mod error;
pub mod runner;
pub mod sandbox;
mod task;

pub use error::Error;
pub use runner::Runner;
pub use task::Task;
