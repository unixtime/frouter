use std::error::Error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum FileRouterError {
    IoError(io::Error),
    ConfigError(String),
}

impl fmt::Display for FileRouterError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FileRouterError::IoError(e) => write!(f, "IO Error: {}", e),
            FileRouterError::ConfigError(s) => write!(f, "Config Error: {}", s),
        }
    }
}

impl Error for FileRouterError {}

impl From<io::Error> for FileRouterError {
    fn from(error: io::Error) -> Self {
        FileRouterError::IoError(error)
    }
}
