use std::error::Error;
use std::{fmt, io};

#[derive(Debug)]
pub enum FasterError<'a> {
    IOError(io::Error),
    InvalidType,
    RecoveryError,
    CheckpointError,
    BuilderError(&'a str),
}

impl<'a> fmt::Display for FasterError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FasterError::IOError(err) => write!(f, "{}", err.description()),
            FasterError::InvalidType => write!(f, "Cannot call method with in-memory FasterKv"),
            FasterError::RecoveryError => write!(f, "Failed to recover"),
            FasterError::CheckpointError => write!(f, "Checkpoint failed"),
            FasterError::BuilderError(err) => write!(f, "Builder error: {}", err),
        }
    }
}

impl<'a> From<io::Error> for FasterError<'a> {
    fn from(e: io::Error) -> Self {
        FasterError::IOError(e)
    }
}

impl<'a> Error for FasterError<'a> {}
