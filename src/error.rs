use thiserror::Error;

/// All possible errors.
#[derive(Error, Debug, Clone)]
pub enum Error {
    /// Unknown error occurred.
    #[error("Unknown error")]
    UnknownError,
}
