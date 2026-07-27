//! Canonical observation envelope and classification for outpost streams.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AuthorityClass, OutpostProtocolError, SCHEMA_VERSION};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentFeedback {
    pub source: String,
    pub scope: String,
    pub classification: ObservationClassification,
    pub authority: AuthorityClass,
    pub confidence: f32,
    pub schema_version: String,
    pub payload: serde_json::Value,
}

impl AgentFeedback {
    pub fn new<S: Into<String>>(
        source: S,
        scope: String,
        classification: ObservationClassification,
        authority: AuthorityClass,
        confidence: f32,
        schema_version: String,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            source: source.into(),
            scope,
            classification,
            authority,
            confidence: confidence.clamp(0.0, 1.0),
            schema_version,
            payload,
        }
    }

    pub fn into_outpost_observation(self, provenance: String) -> OutpostObservation {
        let now = Utc::now();
        OutpostObservation {
            id: Uuid::new_v4().to_string(),
            schema_version: self.schema_version,
            source: self.source,
            observed_at: now,
            collected_at: now,
            freshness_seconds: 0,
            confidence: self.confidence,
            scope: ObservationScope::Custom(self.scope),
            classification: self.classification,
            authority: self.authority,
            payload: self.payload,
            provenance: Some(provenance),
            local_only: false,
        }
    }
}

impl TryFrom<AgentFeedback> for OutpostObservation {
    type Error = OutpostProtocolError;
    fn try_from(value: AgentFeedback) -> std::result::Result<Self, Self::Error> {
        if value.schema_version != SCHEMA_VERSION {
            return Err(OutpostProtocolError::Conversion(format!(
                "schema_version mismatch: expected {}, got {}",
                SCHEMA_VERSION, value.schema_version
            )));
        }
        Ok(value.into_outpost_observation(
            "arda-outpost-scout://survey".to_string(),
        ))
    }
}

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
