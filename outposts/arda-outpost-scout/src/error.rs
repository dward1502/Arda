use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScoutError {
    #[error("invalid path: {0}")]
    InvalidPath(String),
    #[error("observation protocol error: {0}")]
    Protocol(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, ScoutError>;
