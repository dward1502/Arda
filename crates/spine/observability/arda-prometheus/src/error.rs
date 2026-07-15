use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PrometheusError {
    #[error("boot config not found: {path}")]
    BootConfigMissing { path: PathBuf },

    #[error("boot config parse error in {path}: {source}")]
    BootConfigInvalid {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<PrometheusError> for arda_core::error::ArdaError {
    fn from(e: PrometheusError) -> Self {
        match e {
            PrometheusError::BootConfigMissing { path } => {
                arda_core::error::ArdaError::Config(format!(
                    "boot config not found: {}",
                    path.display()
                ))
            }
            PrometheusError::BootConfigInvalid { path, source } => {
                arda_core::error::ArdaError::Config(format!(
                    "boot config invalid at {}: {source}",
                    path.display()
                ))
            }
            PrometheusError::Io(e) => arda_core::error::ArdaError::Ledger(e),
        }
    }
}
