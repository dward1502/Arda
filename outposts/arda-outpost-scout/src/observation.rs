use arda_outpost_protocol::{
    AgentFeedback, AuthorityClass, ObservationClassification, ObservationScope, OutpostObservation,
    SCHEMA_VERSION,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid path: {0}")]
    InvalidPath(String),
    #[error("manifest error: {0}")]
    Manifest(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("toml error: {0}")]
    Toml(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CratePackage {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrateObservation {
    pub path: String,
    pub name: String,
    pub purpose: Option<String>,
    pub status: CrateStatus,
    pub key_entrypoints: Vec<String>,
    pub test_surface: Vec<String>,
    pub dependencies: Vec<String>,
    pub dev_patterns: Vec<String>,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CrateStatus {
    Active,
    Deprecated,
    Stubbed,
    Shell,
    Unknown,
}

impl std::fmt::Display for CrateStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CrateStatus::Active => write!(f, "active"),
            CrateStatus::Deprecated => write!(f, "deprecated"),
            CrateStatus::Stubbed => write!(f, "stubbed"),
            CrateStatus::Shell => write!(f, "shell"),
            CrateStatus::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SurveyReport {
    pub source: String,
    pub generated_at: DateTime<Utc>,
    pub schema_version: String,
    pub observations: Vec<CrateObservation>,
}

impl SurveyReport {
    pub fn new<S: Into<String>>(source: S, observations: Vec<CrateObservation>) -> Self {
        Self {
            source: source.into(),
            generated_at: Utc::now(),
            schema_version: SCHEMA_VERSION.to_string(),
            observations,
        }
    }
}

impl From<CrateObservation> for AgentFeedback {
    fn from(value: CrateObservation) -> Self {
        let status = value.status.clone();
        Self::new(
            "arda-outpost-scout",
            "crates".to_string(),
            classification_for_status(status),
            AuthorityClass::Advisory,
            0.8,
            SCHEMA_VERSION.to_string(),
            serde_json::to_value(value).expect("serialize crate observation"),
        )
    }
}

pub fn classification_for_status(status: CrateStatus) -> ObservationClassification {
    match status {
        CrateStatus::Active => ObservationClassification::DerivedEstimate,
        CrateStatus::Shell | CrateStatus::Unknown => ObservationClassification::SelfReport,
        CrateStatus::Stubbed | CrateStatus::Deprecated => ObservationClassification::Unavailable,
    }
}

pub fn build_observation(source: impl Into<String>, payload: impl Serialize) -> OutpostObservation {
    OutpostObservation::new(
        source,
        ObservationScope::Custom("outpost_scout".to_string()),
        ObservationClassification::SelfReport,
        AuthorityClass::Advisory,
        serde_json::to_value(payload).expect("serialize observation payload"),
    )
    .local_only()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_status_maps_to_derived_estimate() {
        assert!(matches!(
            classification_for_status(CrateStatus::Active),
            ObservationClassification::DerivedEstimate
        ));
    }

    #[test]
    fn shell_and_unknown_statuses_map_to_self_report() {
        assert!(matches!(
            classification_for_status(CrateStatus::Shell),
            ObservationClassification::SelfReport
        ));
        assert!(matches!(
            classification_for_status(CrateStatus::Unknown),
            ObservationClassification::SelfReport
        ));
    }

    #[test]
    fn stubbed_and_deprecated_statuses_map_to_unavailable() {
        assert!(matches!(
            classification_for_status(CrateStatus::Stubbed),
            ObservationClassification::Unavailable
        ));
        assert!(matches!(
            classification_for_status(CrateStatus::Deprecated),
            ObservationClassification::Unavailable
        ));
    }
}
