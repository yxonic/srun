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

    #[error("Permission denied: {0}.")]
    PermissionDeniedError(String),

    #[error("Error in cache system: {0:?}.")]
    CacheError(cached_path::Error),

    #[error("Script exited with code {0}.")]
    ErrorCode(u64),

    #[error("Error while connecting to docker service: {0:?}.")]
    ConnectionError(hyper::Error),

    #[error("Error while communicating with docker: {0:?}.")]
    DockerError(shiplift::Error),

    #[error("Decoding error with docker logs: {0:?}.")]
    EncodingError(std::str::Utf8Error),

    /// Unknown error occurred.
    #[error("Unknown error: {0}.")]
    UnknownError(String),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IOError(e)
    }
}

impl From<cached_path::Error> for Error {
    fn from(e: cached_path::Error) -> Self {
        Error::CacheError(e)
    }
}

impl From<hyper::Error> for Error {
    fn from(e: hyper::Error) -> Self {
        Error::ConnectionError(e)
    }
}

impl From<shiplift::Error> for Error {
    fn from(e: shiplift::Error) -> Self {
        Error::DockerError(e)
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(e: std::str::Utf8Error) -> Self {
        Error::EncodingError(e)
    }
}
