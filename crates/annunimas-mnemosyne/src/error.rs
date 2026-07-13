// sigil: REPAIR
use annunimas_core::error::AnnunimasError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MnemosyneError {
    #[error("IPC transport error: {0}")]
    Ipc(String),

    #[error("HTTP transport error: {0}")]
    Http(String),

    #[error("invalid encode payload: {0}")]
    InvalidEncodePayload(String),

    #[error("unknown command: {0}")]
    UnknownCommand(String),

    #[error("plutus work-signal error: {0}")]
    PlutusWorkSignal(String),

    #[error("obsidian vault path not found: {0}")]
    ObsidianPathNotFound(String),

    #[error("daemon task failed: {0}")]
    DaemonTask(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}

impl From<MnemosyneError> for AnnunimasError {
    fn from(err: MnemosyneError) -> Self {
        AnnunimasError::Agent {
            agent: "mnemosyne".to_string(),
            message: err.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, MnemosyneError>;
