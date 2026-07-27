//! Canonical observation envelope and classification for outpost streams.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AuthorityClass, SCHEMA_VERSION};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ObservationScope {
    Crates,
    Apps,
    Memory,
    Health,
    Environmental,
    RuntimeTelemetry,
    Custom(String),
}

impl std::fmt::Display for ObservationScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObservationScope::Crates => write!(f, "crates"),
            ObservationScope::Apps => write!(f, "apps"),
            ObservationScope::Memory => write!(f, "memory"),
            ObservationScope::Health => write!(f, "health"),
            ObservationScope::Environmental => write!(f, "environmental"),
            ObservationScope::RuntimeTelemetry => write!(f, "runtime_telemetry"),
            ObservationScope::Custom(value) => write!(f, "{}", value),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ObservationClassification {
    RawMeasurement,
    DerivedEstimate,
    SelfReport,
    Default,
    Unavailable,
    ExperimentalDerived,
}

impl std::fmt::Display for ObservationClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObservationClassification::RawMeasurement => write!(f, "raw_measurement"),
            ObservationClassification::DerivedEstimate => write!(f, "derived_estimate"),
            ObservationClassification::SelfReport => write!(f, "self_report"),
            ObservationClassification::Default => write!(f, "default"),
            ObservationClassification::Unavailable => write!(f, "unavailable"),
            ObservationClassification::ExperimentalDerived => write!(f, "experimental_derived"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutpostObservation {
    pub id: String,
    pub schema_version: String,
    pub source: String,
    pub observed_at: DateTime<Utc>,
    pub collected_at: DateTime<Utc>,
    pub freshness_seconds: u64,
    pub confidence: f32,
    pub scope: ObservationScope,
    pub classification: ObservationClassification,
    pub authority: AuthorityClass,
    pub payload: serde_json::Value,
    pub provenance: Option<String>,
    pub local_only: bool,
}

impl OutpostObservation {
    pub fn new<S: Into<String>>(
        source: S,
        scope: ObservationScope,
        classification: ObservationClassification,
        authority: AuthorityClass,
        payload: serde_json::Value,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            schema_version: SCHEMA_VERSION.to_string(),
            source: source.into(),
            observed_at: now,
            collected_at: now,
            freshness_seconds: 0,
            confidence: 0.0,
            scope,
            classification,
            authority,
            payload,
            provenance: None,
            local_only: false,
        }
    }

    pub fn with_freshness(mut self, freshness_seconds: u64) -> Self {
        self.freshness_seconds = freshness_seconds;
        self
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn with_provenance<S: Into<String>>(mut self, provenance: S) -> Self {
        self.provenance = Some(provenance.into());
        self
    }

    pub fn local_only(mut self) -> Self {
        self.local_only = true;
        self
    }

    pub fn is_advisory(&self) -> bool {
        self.authority == AuthorityClass::Advisory
    }
}
