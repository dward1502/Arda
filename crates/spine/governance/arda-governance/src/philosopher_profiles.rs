// sigil: REPAIR
//! Data contract for Triad Philosopher bootstrap profiles.
//!
//! G2 is intentionally data-only: profiles may be loaded and validated, but
//! this module does not enable autonomous blocking, generated corpus promotion,
//! or consensus claims.

use std::{collections::HashSet, fs, path::Path};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::GovernanceReviewMode;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhilosopherProfileMaturity {
    DraftHumanAuthored,
    IndependentReviewReceipted,
    AutonomousConsensusReceipted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhilosopherProfileSourceKind {
    HumanAuthored,
    GeneratedArtifact,
}

fn default_source_kind() -> PhilosopherProfileSourceKind {
    PhilosopherProfileSourceKind::HumanAuthored
}

fn default_source_revision() -> String {
    "legacy_human_authored_bootstrap".to_string()
}

fn default_review_authority() -> String {
    "human_governance_maintainers".to_string()
}

fn default_promotion_criteria() -> Vec<String> {
    vec!["independent human review receipt required before promotion".to_string()]
}

/// Immutable-by-value provenance snapshot for a philosopher-derived action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhilosopherLifecycleReceipt {
    pub schema_version: String,
    pub profile_id: String,
    pub profile_source: String,
    pub source_kind: PhilosopherProfileSourceKind,
    pub source_revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_artifact: Option<String>,
    pub maturity: PhilosopherProfileMaturity,
    pub authority: String,
    pub review_authority: String,
    pub review_mode: GovernanceReviewMode,
    #[serde(default)]
    pub promotion_criteria: Vec<String>,
}

impl Default for PhilosopherLifecycleReceipt {
    fn default() -> Self {
        Self {
            schema_version: "arda.governance.philosopher_lifecycle.v1".to_string(),
            profile_id: "triad_philosopher".to_string(),
            profile_source: "built_in:triad_philosopher".to_string(),
            source_kind: PhilosopherProfileSourceKind::HumanAuthored,
            source_revision: "arda-governance-phase7-v1".to_string(),
            generated_artifact: None,
            maturity: PhilosopherProfileMaturity::DraftHumanAuthored,
            authority: "human_authored_heuristic".to_string(),
            review_authority: "human_governance_maintainers".to_string(),
            review_mode: GovernanceReviewMode::HeuristicLocal,
            promotion_criteria: default_promotion_criteria(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhilosopherProfile {
    pub id: String,
    pub display_name: String,
    pub lens: String,
    #[serde(default)]
    pub canonical_sources: Vec<String>,
    #[serde(default)]
    pub decision_questions: Vec<String>,
    #[serde(default)]
    pub failure_modes: Vec<String>,
    pub maturity: PhilosopherProfileMaturity,
    pub implementation_status: PhilosopherProfileMaturity,
    pub authority: String,
    pub veto_scope: String,
    pub confidence_floor: f64,
    #[serde(default)]
    pub primary_questions: Vec<String>,
    #[serde(default)]
    pub required_evidence: Vec<String>,
    #[serde(default)]
    pub forbidden_claims: Vec<String>,
    #[serde(default = "default_source_kind")]
    pub source_kind: PhilosopherProfileSourceKind,
    #[serde(default = "default_source_revision")]
    pub source_revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_artifact: Option<String>,
    #[serde(default = "default_review_authority")]
    pub review_authority: String,
    #[serde(default = "default_promotion_criteria")]
    pub promotion_criteria: Vec<String>,
}

impl PhilosopherProfile {
    pub fn lifecycle_receipt(
        &self,
        profile_source: impl Into<String>,
        review_mode: GovernanceReviewMode,
    ) -> PhilosopherLifecycleReceipt {
        PhilosopherLifecycleReceipt {
            schema_version: "arda.governance.philosopher_lifecycle.v1".to_string(),
            profile_id: self.id.clone(),
            profile_source: profile_source.into(),
            source_kind: self.source_kind,
            source_revision: self.source_revision.clone(),
            generated_artifact: self.generated_artifact.clone(),
            maturity: self.maturity.clone(),
            authority: self.authority.clone(),
            review_authority: self.review_authority.clone(),
            review_mode,
            promotion_criteria: self.promotion_criteria.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhilosopherProfileSet {
    pub schema_version: String,
    pub authority: String,
    pub default_maturity: PhilosopherProfileMaturity,
    pub autonomous_blocking_enabled: bool,
    pub generated_corpus_promotion_enabled: bool,
    #[serde(default)]
    pub profiles: Vec<PhilosopherProfile>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhilosopherProfileStatus {
    pub id: String,
    pub display_name: String,
    pub lens: String,
    pub maturity: PhilosopherProfileMaturity,
    pub implementation_status: PhilosopherProfileMaturity,
    pub authority: String,
    pub confidence_floor: f64,
    pub autonomous_blocking_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhilosopherProfileStatusProjection {
    pub schema_version: String,
    pub authority: String,
    pub chain_id: String,
    pub chain_version: String,
    pub profile_source: String,
    pub review_mode: String,
    pub profile_maturity: String,
    pub autonomous_blocking_enabled: bool,
    pub generated_corpus_promotion_enabled: bool,
    pub profile_count: usize,
    pub profiles: Vec<PhilosopherProfileStatus>,
}

impl PhilosopherProfileSet {
    pub const SCHEMA_VERSION: &'static str = "arda.governance.philosopher_profiles.v1";

    pub fn profile(&self, id: &str) -> Option<&PhilosopherProfile> {
        self.profiles.iter().find(|profile| profile.id == id)
    }

    pub fn status_projection(
        &self,
        profile_source: impl Into<String>,
    ) -> PhilosopherProfileStatusProjection {
        let profiles = self
            .profiles
            .iter()
            .map(|profile| PhilosopherProfileStatus {
                id: profile.id.clone(),
                display_name: profile.display_name.clone(),
                lens: profile.lens.clone(),
                maturity: profile.maturity.clone(),
                implementation_status: profile.implementation_status.clone(),
                authority: profile.authority.clone(),
                confidence_floor: profile.confidence_floor,
                autonomous_blocking_enabled: false,
            })
            .collect::<Vec<_>>();

        PhilosopherProfileStatusProjection {
            schema_version: self.schema_version.clone(),
            authority: self.authority.clone(),
            chain_id: "default_triad".to_string(),
            chain_version: "heuristic_local_v1".to_string(),
            profile_source: profile_source.into(),
            review_mode: "heuristic_local".to_string(),
            profile_maturity: maturity_review_mode(&self.default_maturity).to_string(),
            // Legacy profile flags never directly enable runtime blocking. Phase 8
            // routes that decision through RuntimeBlockingAuthority exclusively.
            autonomous_blocking_enabled: false,
            generated_corpus_promotion_enabled: self.generated_corpus_promotion_enabled,
            profile_count: profiles.len(),
            profiles,
        }
    }

    pub fn validate_g2_bootstrap(&self) -> Result<(), PhilosopherProfileError> {
        if self.schema_version != Self::SCHEMA_VERSION {
            return Err(PhilosopherProfileError::InvalidSchemaVersion {
                actual: self.schema_version.clone(),
            });
        }

        if self.generated_corpus_promotion_enabled {
            return Err(PhilosopherProfileError::UnsafeAutonomyFlag(
                "generated_corpus_promotion_enabled must remain false in G2".to_string(),
            ));
        }

        if self.default_maturity != PhilosopherProfileMaturity::DraftHumanAuthored {
            return Err(PhilosopherProfileError::NonDraftMaturity {
                id: "default_maturity".to_string(),
            });
        }

        if self.profiles.is_empty() {
            return Err(PhilosopherProfileError::EmptyProfiles);
        }

        let mut ids = HashSet::new();
        for profile in &self.profiles {
            validate_profile(profile)?;
            if !ids.insert(profile.id.as_str()) {
                return Err(PhilosopherProfileError::DuplicateId {
                    id: profile.id.clone(),
                });
            }
        }

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum PhilosopherProfileError {
    #[error("failed to read philosopher profiles from {path}: {source}")]
    Read {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to parse philosopher profiles TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("unsupported philosopher profile schema_version: {actual}")]
    InvalidSchemaVersion { actual: String },
    #[error("{0}")]
    UnsafeAutonomyFlag(String),
    #[error("only draft_human_authored profiles are allowed in G2: {id}")]
    NonDraftMaturity { id: String },
    #[error(
        "philosopher profile {id} implementation_status must match draft_human_authored in G2"
    )]
    NonDraftImplementationStatus { id: String },
    #[error("philosopher profile {id} confidence_floor must be finite and between 0.0 and 1.0")]
    InvalidConfidenceFloor { id: String },
    #[error("philosopher profile set must include at least one profile")]
    EmptyProfiles,
    #[error("duplicate philosopher profile id: {id}")]
    DuplicateId { id: String },
    #[error("philosopher profile {id} has an empty required field: {field}")]
    EmptyField { id: String, field: &'static str },
    #[error("philosopher profile {id} must include at least one {field}")]
    EmptyList { id: String, field: &'static str },
    #[error("generated philosopher profile {id} must identify generated_artifact")]
    MissingGeneratedArtifact { id: String },
}

pub fn load_philosopher_profiles(
    path: impl AsRef<Path>,
) -> Result<PhilosopherProfileSet, PhilosopherProfileError> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path).map_err(|source| PhilosopherProfileError::Read {
        path: path.display().to_string(),
        source,
    })?;
    load_philosopher_profiles_from_str(&raw)
}

pub fn load_philosopher_profiles_from_str(
    raw: &str,
) -> Result<PhilosopherProfileSet, PhilosopherProfileError> {
    let profiles: PhilosopherProfileSet = toml::from_str(raw)?;
    profiles.validate_g2_bootstrap()?;
    Ok(profiles)
}

fn maturity_review_mode(maturity: &PhilosopherProfileMaturity) -> &'static str {
    match maturity {
        PhilosopherProfileMaturity::DraftHumanAuthored => "draft_human_authored",
        PhilosopherProfileMaturity::IndependentReviewReceipted => "independent_review_receipted",
        PhilosopherProfileMaturity::AutonomousConsensusReceipted => {
            "autonomous_consensus_receipted"
        }
    }
}

fn validate_profile(profile: &PhilosopherProfile) -> Result<(), PhilosopherProfileError> {
    require_non_empty(&profile.id, &profile.id, "id")?;
    require_non_empty(&profile.id, &profile.display_name, "display_name")?;
    require_non_empty(&profile.id, &profile.lens, "lens")?;
    require_non_empty(&profile.id, &profile.authority, "authority")?;
    require_non_empty(&profile.id, &profile.veto_scope, "veto_scope")?;
    require_non_empty(&profile.id, &profile.source_revision, "source_revision")?;
    require_non_empty(&profile.id, &profile.review_authority, "review_authority")?;

    if profile.source_kind == PhilosopherProfileSourceKind::GeneratedArtifact
        && profile
            .generated_artifact
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .is_empty()
    {
        return Err(PhilosopherProfileError::MissingGeneratedArtifact {
            id: profile.id.clone(),
        });
    }

    if profile.maturity != PhilosopherProfileMaturity::DraftHumanAuthored {
        return Err(PhilosopherProfileError::NonDraftMaturity {
            id: profile.id.clone(),
        });
    }

    if profile.implementation_status != PhilosopherProfileMaturity::DraftHumanAuthored {
        return Err(PhilosopherProfileError::NonDraftImplementationStatus {
            id: profile.id.clone(),
        });
    }

    if !profile.confidence_floor.is_finite()
        || profile.confidence_floor < 0.0
        || profile.confidence_floor > 1.0
    {
        return Err(PhilosopherProfileError::InvalidConfidenceFloor {
            id: profile.id.clone(),
        });
    }

    require_non_empty_list(&profile.id, &profile.canonical_sources, "canonical_sources")?;
    require_non_empty_list(
        &profile.id,
        &profile.promotion_criteria,
        "promotion_criteria",
    )?;
    require_non_empty_list(
        &profile.id,
        &profile.decision_questions,
        "decision_questions",
    )?;
    require_non_empty_list(&profile.id, &profile.failure_modes, "failure_modes")?;
    require_non_empty_list(&profile.id, &profile.primary_questions, "primary_questions")?;
    require_non_empty_list(&profile.id, &profile.required_evidence, "required_evidence")?;
    require_non_empty_list(&profile.id, &profile.forbidden_claims, "forbidden_claims")?;

    Ok(())
}

fn require_non_empty(
    id: &str,
    value: &str,
    field: &'static str,
) -> Result<(), PhilosopherProfileError> {
    if value.trim().is_empty() {
        return Err(PhilosopherProfileError::EmptyField {
            id: id.to_string(),
            field,
        });
    }
    Ok(())
}

fn require_non_empty_list(
    id: &str,
    values: &[String],
    field: &'static str,
) -> Result<(), PhilosopherProfileError> {
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        return Err(PhilosopherProfileError::EmptyList {
            id: id.to_string(),
            field,
        });
    }
    Ok(())
}
