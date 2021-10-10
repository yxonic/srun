use std::str::Utf8Error;

use thiserror::Error;

/// All possible errors.
#[derive(Error, Debug)]
pub enum Error {
    #[error("Error in task specification: {0}.")]
    SpecError(String),

    #[error("Error while building image: {0}.")]
    BuildError(String),

    #[error("Error while accessing filesystem: {0:?}.")]
    IOError(std::io::Error),

    #[error("Script exited with code {0}.")]
    ErrorCode(u64),

    #[error("Error while connecting to docker service: {0:?}.")]
    ConnectionError(hyper::Error),

    #[error("Error while communicating with docker: {0:?}.")]
    DockerError(shiplift::Error),

    #[error("Decoding error with docker logs: {0:?}.")]
    EncodingError(Utf8Error),

    /// Unknown error occurred.
    #[error("Unknown error: {0}.")]
    UnknownError(String),
}
