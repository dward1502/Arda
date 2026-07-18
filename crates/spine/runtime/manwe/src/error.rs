// sigil: REPAIR
#[cfg(feature = "adaptive")]
use arda_core::error::ArdaError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CharonError {
    #[error("IPC transport error: {0}")]
    Ipc(String),

    #[error("HTTP transport error: {0}")]
    Http(String),

    #[error("invalid payload: {0}")]
    InvalidPayload(String),

    #[error("unknown command: {0}")]
    UnknownCommand(String),

    #[error("provider config error: {0}")]
    Config(String),

    #[error("unknown provider: {0}")]
    UnknownProvider(String),

    #[error("unknown model `{model}` for provider `{provider}`")]
    UnknownModel { provider: String, model: String },

    #[error("ambiguous model: {0}")]
    AmbiguousModel(String),

    #[error("missing provider_id")]
    MissingProviderId,

    #[error("no provider available for task type `{0}`")]
    NoProviderAvailable(String),

    #[error("daemon task failed: {0}")]
    DaemonTask(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Serde(#[from] serde_json::Error),

    #[error(transparent)]
    Toml(#[from] toml::de::Error),
}

#[cfg(feature = "adaptive")]
impl From<CharonError> for ArdaError {
    fn from(err: CharonError) -> Self {
        ArdaError::Agent {
            agent: "charon".to_string(),
            message: err.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, CharonError>;
