use chrono::{DateTime, Utc};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use sha2::{Digest, Sha256};

pub const EVIDENCE_FRESHNESS_WINDOW_DAYS: i64 = 30;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Supplied,
    Retrieved,
    Inferred,
    Unavailable,
}

impl Serialize for EvidenceKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match self {
            Self::Supplied => "supplied",
            Self::Retrieved => "retrieved",
            Self::Inferred => "inferred",
            Self::Unavailable => "unavailable",
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStance {
    Supporting,
    Contradicting,
    Neutral,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceFreshness {
    Fresh,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceIndependence {
    Independent,
    Related,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct EvidenceRef {
    pub source_id: String,
    pub kind: EvidenceKind,
    pub locator: String,
    pub observed_at: DateTime<Utc>,
    pub digest: String,
    pub excerpt: Option<String>,
    pub claim: Option<String>,
    pub stance: EvidenceStance,
    pub freshness: EvidenceFreshness,
    pub independence: EvidenceIndependence,
    pub source_quality: f64,
    pub sensitive_excerpt: bool,
}

impl Serialize for EvidenceRef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("EvidenceRef", 12)?;
        state.serialize_field("source_id", &self.source_id)?;
        state.serialize_field("kind", &self.kind)?;
        state.serialize_field("locator", &self.locator)?;
        state.serialize_field("observed_at", &self.observed_at)?;
        state.serialize_field("digest", &self.digest)?;
        let excerpt = if self.sensitive_excerpt && self.excerpt.is_some() {
            Some("[REDACTED]")
        } else {
            self.excerpt.as_deref()
        };
        state.serialize_field("excerpt", &excerpt)?;
        state.serialize_field("claim", &self.claim)?;
        state.serialize_field("stance", &self.stance)?;
        state.serialize_field("freshness", &self.freshness)?;
        state.serialize_field("independence", &self.independence)?;
        state.serialize_field("source_quality", &self.source_quality)?;
        state.serialize_field("sensitive_excerpt", &self.sensitive_excerpt)?;
        state.end()
    }
}

impl EvidenceRef {
    pub fn supplied(
        source_id: impl Into<String>,
        locator: impl Into<String>,
        observed_at: DateTime<Utc>,
        content: impl Into<String>,
    ) -> Self {
        Self::new(
            source_id,
            EvidenceKind::Supplied,
            locator,
            observed_at,
            content,
            EvidenceStance::Neutral,
            EvidenceIndependence::Unknown,
            1.0,
            true,
        )
    }

    pub fn retrieved(
        source_id: impl Into<String>,
        locator: impl Into<String>,
        observed_at: DateTime<Utc>,
        content: impl Into<String>,
        source_quality: f64,
    ) -> Self {
        Self::new(
            source_id,
            EvidenceKind::Retrieved,
            locator,
            observed_at,
            content,
            EvidenceStance::Supporting,
            EvidenceIndependence::Unknown,
            source_quality,
            false,
        )
    }

    pub fn inferred(
        source_id: impl Into<String>,
        locator: impl Into<String>,
        observed_at: DateTime<Utc>,
        claim: impl Into<String>,
        stance: EvidenceStance,
    ) -> Self {
        let claim = claim.into();
        Self::new(
            source_id,
            EvidenceKind::Inferred,
            locator,
            observed_at,
            &claim,
            stance,
            EvidenceIndependence::Related,
            0.5,
            false,
        )
        .with_claim(claim, stance)
        .without_excerpt()
    }

    pub fn unavailable(
        source_id: impl Into<String>,
        locator: impl Into<String>,
        observed_at: DateTime<Utc>,
        reason: impl Into<String>,
    ) -> Self {
        let reason = reason.into();
        Self::new(
            source_id,
            EvidenceKind::Unavailable,
            locator,
            observed_at,
            &reason,
            EvidenceStance::Neutral,
            EvidenceIndependence::Unknown,
            0.0,
            false,
        )
        .with_claim(reason, EvidenceStance::Neutral)
        .without_excerpt()
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        source_id: impl Into<String>,
        kind: EvidenceKind,
        locator: impl Into<String>,
        observed_at: DateTime<Utc>,
        content: impl Into<String>,
        stance: EvidenceStance,
        independence: EvidenceIndependence,
        source_quality: f64,
        sensitive_excerpt: bool,
    ) -> Self {
        let source_id = source_id.into();
        let locator = locator.into();
        let content = content.into();
        let digest = evidence_digest(kind, &source_id, &locator, observed_at, &content);
        Self {
            source_id,
            kind,
            locator,
            observed_at,
            digest,
            excerpt: Some(content),
            claim: None,
            stance,
            freshness: EvidenceFreshness::Unknown,
            independence,
            source_quality: normalize_quality(source_quality),
            sensitive_excerpt,
        }
    }

    pub fn with_claim(mut self, claim: impl Into<String>, stance: EvidenceStance) -> Self {
        self.claim = Some(claim.into());
        self.stance = stance;
        self
    }

    pub fn with_independence(mut self, independence: EvidenceIndependence) -> Self {
        self.independence = independence;
        self
    }

    pub fn with_source_quality(mut self, source_quality: f64) -> Self {
        self.source_quality = normalize_quality(source_quality);
        self
    }

    pub fn with_sensitive_excerpt(mut self, sensitive: bool) -> Self {
        self.sensitive_excerpt = sensitive;
        self
    }

    pub fn without_excerpt(mut self) -> Self {
        self.excerpt = None;
        self
    }

    pub fn redacted_for_export(mut self) -> Self {
        if self.excerpt.is_some() {
            self.sensitive_excerpt = true;
        }
        self
    }

    pub fn classify_freshness(mut self, reference_time: DateTime<Utc>) -> Self {
        self.freshness = if !matches!(self.kind, EvidenceKind::Supplied | EvidenceKind::Retrieved)
            || self.observed_at > reference_time
        {
            EvidenceFreshness::Unknown
        } else if reference_time
            .signed_duration_since(self.observed_at)
            .num_days()
            > EVIDENCE_FRESHNESS_WINDOW_DAYS
        {
            EvidenceFreshness::Stale
        } else {
            EvidenceFreshness::Fresh
        };
        self
    }

    pub fn integrity(&self) -> EvidenceIntegrity {
        let content = match self.excerpt.as_deref() {
            Some("[REDACTED]") => return EvidenceIntegrity::Redacted,
            Some(excerpt) => excerpt,
            None => match self.claim.as_deref() {
                Some(claim) => claim,
                None => return EvidenceIntegrity::Unverifiable,
            },
        };
        if evidence_digest(
            self.kind,
            &self.source_id,
            &self.locator,
            self.observed_at,
            content,
        ) == self.digest
        {
            EvidenceIntegrity::Verified
        } else {
            EvidenceIntegrity::Invalid
        }
    }

    pub(crate) fn same_request_identity(&self, other: &Self) -> bool {
        self.source_id == other.source_id
            && self.kind == other.kind
            && self.locator == other.locator
            && self.observed_at == other.observed_at
            && self.digest == other.digest
            && self.claim == other.claim
            && self.stance == other.stance
            && self.freshness == other.freshness
            && self.independence == other.independence
            && self.source_quality == other.source_quality
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceDisposition {
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceIntegrity {
    Verified,
    Redacted,
    Unverifiable,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceAssessment {
    pub evidence: EvidenceRef,
    pub integrity: EvidenceIntegrity,
    pub disposition: EvidenceDisposition,
    pub affected_score: bool,
    pub score_effect: f64,
    pub rationale: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSignalKind {
    Missing,
    Stale,
    Conflicting,
    Corroborating,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceSignal {
    pub kind: EvidenceSignalKind,
    pub description: String,
    pub source_ids: Vec<String>,
}

fn evidence_digest(
    kind: EvidenceKind,
    source_id: &str,
    locator: &str,
    observed_at: DateTime<Utc>,
    content: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(match kind {
        EvidenceKind::Supplied => b"supplied".as_slice(),
        EvidenceKind::Retrieved => b"retrieved".as_slice(),
        EvidenceKind::Inferred => b"inferred".as_slice(),
        EvidenceKind::Unavailable => b"unavailable".as_slice(),
    });
    hasher.update([0]);
    hasher.update(source_id.as_bytes());
    hasher.update([0]);
    hasher.update(locator.as_bytes());
    hasher.update([0]);
    hasher.update(observed_at.to_rfc3339().as_bytes());
    hasher.update([0]);
    hasher.update(content.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn normalize_quality(source_quality: f64) -> f64 {
    if source_quality.is_finite() {
        source_quality.clamp(0.0, 1.0)
    } else {
        0.0
    }
}
