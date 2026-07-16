// sigil: REPAIR
use arda_core::error::ArdaError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EconomicsError {
    #[error("economics error: {0}")]
    Message(String),
    #[error("IPC transport error: {0}")]
    Ipc(String),
    #[error("HTTP transport error: {0}")]
    Http(String),
    #[error("invalid payload: {0}")]
    InvalidPayload(String),
    #[error("unknown command: {0}")]
    UnknownCommand(String),
    #[error("missing required payload key `{0}`")]
    MissingPayloadKey(String),
    #[error("missing required numeric payload key `{0}`")]
    MissingNumericPayloadKey(String),
    #[error("daemon task failed: {0}")]
    DaemonTask(String),
    #[error("runtime error: {0}")]
    Runtime(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}

impl From<EconomicsError> for ArdaError {
    fn from(err: EconomicsError) -> Self {
        ArdaError::Agent {
            agent: "economics".to_owned(),
            message: err.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, EconomicsError>;
