// sigil: REPAIR
use annunimas_core::error::AnnunimasError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HadesError {
    #[error("IPC transport error: {0}")]
    Ipc(String),

    #[error("HTTP transport error: {0}")]
    Http(String),

    #[error("ATHENA IPC error: {0}")]
    AthenaIpc(String),

    #[error("plutus work-signal error: {0}")]
    PlutusWorkSignal(String),

    #[error("invalid sigil regex: {0}")]
    InvalidSigilRegex(String),

    #[error("storage init failed: {0}")]
    StorageInit(String),

    #[error("destructive quorum denied for {action}: {reason}")]
    QuorumDenied { action: String, reason: String },

    #[error("{label} concurrency gate saturated")]
    ConcurrencyGateSaturated { label: String },

    #[error("daemon task failed: {0}")]
    DaemonTask(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}

impl From<HadesError> for AnnunimasError {
    fn from(err: HadesError) -> Self {
        AnnunimasError::Agent {
            agent: "hades".to_string(),
            message: err.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, HadesError>;
