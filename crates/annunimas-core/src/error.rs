// sigil: REPAIR
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AnnunimasError {
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Task error: {0}")]
    Task(String),
    #[error("Agent error: {agent} — {message}")]
    Agent { agent: String, message: String },
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    #[error("Routing error: no agent available for task type '{0}'")]
    NoRoute(String),
    #[error("Ledger write error: {0}")]
    Ledger(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, AnnunimasError>;
