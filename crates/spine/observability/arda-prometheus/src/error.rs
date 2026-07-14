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

impl From<PrometheusError> for annunimas_core::error::AnnunimasError {
    fn from(e: PrometheusError) -> Self {
        match e {
            PrometheusError::BootConfigMissing { path } => {
                annunimas_core::error::AnnunimasError::Config(format!(
                    "boot config not found: {}",
                    path.display()
                ))
            }
            PrometheusError::BootConfigInvalid { path, source } => {
                annunimas_core::error::AnnunimasError::Config(format!(
                    "boot config invalid at {}: {source}",
                    path.display()
                ))
            }
            PrometheusError::Io(e) => annunimas_core::error::AnnunimasError::Ledger(e),
        }
    }
}
