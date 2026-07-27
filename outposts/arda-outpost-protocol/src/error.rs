use thiserror::Error;

#[derive(Debug, Error)]
pub enum OutpostProtocolError {
    #[error("invalid observation id: {0}")]
    InvalidObservationId(String),
    #[error("invalid classification: {0}")]
    InvalidClassification(String),
    #[error("invalid authority class: {0}")]
    InvalidAuthority(String),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("chrono error: {0}")]
    Chrono(#[from] chrono::ParseError),
}

pub type Result<T> = std::result::Result<T, OutpostProtocolError>;
