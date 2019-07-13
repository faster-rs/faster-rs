use std::error::Error;
use std::{fmt, io};

#[derive(Debug)]
pub enum FasterError {
    IOError(io::Error),
    InvalidType,
    RecoveryError,
    CheckpointError,
}

impl fmt::Display for FasterError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FasterError::IOError(err) => write!(f, "{}", err.description()),
            FasterError::InvalidType => write!(f, "Cannot call method with in-memory FasterKv"),
            FasterError::RecoveryError => write!(f, "Failed to recover"),
            FasterError::CheckpointError => write!(f, "Checkpoint failed"),
        }
    }
}

impl From<io::Error> for FasterError {
    fn from(e: io::Error) -> Self {
        FasterError::IOError(e)
    }
}

impl Error for FasterError {}
