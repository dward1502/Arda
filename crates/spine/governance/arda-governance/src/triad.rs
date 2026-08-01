// sigil: REPAIR
//! Triad Gate Implementation
//!
//! Three-gate validation: Aurelius (logic), Bacon (empirics), Sun Tzu (strategy)

use std::{fs, path::Path};

use arda_core::task::Task;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::evidence::{
    assess_governance_evidence, GovernanceEvidenceAssessment, GovernanceEvidenceContext,
    GovernanceEvidenceGrade,
};
use crate::versions::{
    legacy_triad_policy_version, GOVERNANCE_CHAIN_POLICY_VERSION, TRIAD_POLICY_VERSION,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceReviewMode {
    #[default]
    HeuristicLocal,
    IndependentAgent,
    HumanReviewed,
    ConsensusReceipted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriadConfig {
    pub strict: bool,
    pub required_passes: Option<u32>,
}

impl Default for TriadConfig {
    fn default() -> Self {
        Self {
            strict: false,
            required_passes: Some(2),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateOutcome {
    Pass,
    Fail,
    Conditional,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceGateName {
    Aurelius,
    Bacon,
    SunTzu,
    Named(String),
}

impl GovernanceGateName {
    pub fn from_lens_id(lens_id: &str) -> Self {
        match lens_id {
            "aurelius" => Self::Aurelius,
            "bacon" => Self::Bacon,
            "sun_tzu" => Self::SunTzu,
            other => Self::Named(other.to_string()),
        }
    }

    pub fn as_lens_id(&self) -> &str {
        match self {
            Self::Aurelius => "aurelius",
            Self::Bacon => "bacon",
            Self::SunTzu => "sun_tzu",
            Self::Named(name) => name,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceVetoCode {
    GateFailed,
    InsufficientPassCount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceVetoReason {
    pub code: GovernanceVetoCode,
    #[serde(default)]
    pub failed_gates: Vec<GovernanceGateName>,
    pub required_passes: u32,
    pub observed_passes: u32,
}

impl GovernanceVetoReason {
    pub fn gate_failed(lens_id: &str, required_passes: u32, observed_passes: u32) -> Self {
        Self {
            code: GovernanceVetoCode::GateFailed,
            failed_gates: vec![GovernanceGateName::from_lens_id(lens_id)],
            required_passes,
            observed_passes,
        }
    }

    pub fn render_compatibility(&self) -> String {
        if self.failed_gates.is_empty() {
            "INSUFFICIENT_PASS_COUNT".to_string()
        } else {
            let mut gates = self
                .failed_gates
                .iter()
                .map(|gate| format!("{}_FAIL", gate.as_lens_id().to_ascii_uppercase()))
                .collect::<Vec<_>>();
            gates.sort();
            gates.join("|")
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriadResult {
    #[serde(default = "legacy_triad_policy_version")]
    pub policy_version: String,
    pub chain_id: String,
    pub chain_version: String,
    pub profile_source: String,
    pub review_mode: GovernanceReviewMode,
    pub profile_maturity: String,
    pub aurelius: GateOutcome,
    pub bacon: GateOutcome,
    pub sun_tzu: GateOutcome,
    pub aurelius_score: f64,
    pub bacon_score: f64,
    pub sun_tzu_score: f64,
    pub passed: bool,
    pub veto_reason: Option<String>,
    #[serde(default)]
    pub veto: Option<GovernanceVetoReason>,
    #[serde(default)]
    pub evidence: GovernanceEvidenceAssessment,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GovernanceLensConfig {
    pub id: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    pub pass_threshold: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GovernanceChainConfig {
    pub schema_version: String,
    pub chain_id: String,
    pub chain_version: String,
    pub profile_source: String,
    #[serde(default)]
    pub review_mode: GovernanceReviewMode,
    pub profile_maturity: String,
    pub strict: bool,
    pub required_passes: Option<u32>,
    pub autonomous_blocking_enabled: bool,
    #[serde(default)]
    pub lenses: Vec<GovernanceLensConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GovernanceLensVerdict {
    pub lens_id: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    pub outcome: GateOutcome,
    pub score: f64,
    pub pass_threshold: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GovernanceChainResult {
    #[serde(default = "legacy_triad_policy_version")]
    pub policy_version: String,
    pub chain_id: String,
    pub chain_version: String,
    pub profile_source: String,
    pub review_mode: GovernanceReviewMode,
    pub profile_maturity: String,
    pub required_passes: u32,
    pub autonomous_blocking_enabled: bool,
    pub passed: bool,
    pub veto_reason: Option<String>,
    #[serde(default)]
    pub veto: Option<GovernanceVetoReason>,
    pub lenses: Vec<GovernanceLensVerdict>,
    #[serde(default)]
    pub evidence: GovernanceEvidenceAssessment,
}

impl GovernanceChainConfig {
    pub const SCHEMA_VERSION: &'static str = "arda.governance.chains.v1";

    pub fn default_triad() -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION.to_string(),
            chain_id: "default_triad".to_string(),
            chain_version: GOVERNANCE_CHAIN_POLICY_VERSION.to_string(),
            profile_source: "config/governance/philosophers.toml".to_string(),
            review_mode: GovernanceReviewMode::HeuristicLocal,
            profile_maturity: "draft_human_authored".to_string(),
            strict: false,
            required_passes: Some(2),
            autonomous_blocking_enabled: false,
            lenses: vec![
                GovernanceLensConfig {
                    id: "aurelius".to_string(),
                    display_name: "Marcus Aurelius".to_string(),
                    profile_id: Some("aurelius".to_string()),
                    pass_threshold: 0.60,
                },
                GovernanceLensConfig {
                    id: "bacon".to_string(),
                    display_name: "Francis Bacon".to_string(),
                    profile_id: Some("bacon".to_string()),
                    pass_threshold: 0.50,
                },
                GovernanceLensConfig {
                    id: "sun_tzu".to_string(),
                    display_name: "Sun Tzu".to_string(),
                    profile_id: Some("sun_tzu".to_string()),
                    pass_threshold: 0.50,
                },
            ],
        }
    }

    pub fn validate_g3_config(&self) -> Result<(), GovernanceChainError> {
        if self.schema_version != Self::SCHEMA_VERSION {
            return Err(GovernanceChainError::InvalidSchemaVersion {
                actual: self.schema_version.clone(),
            });
        }
        require_chain_field(&self.chain_id, "chain_id")?;
        require_chain_field(&self.chain_version, "chain_version")?;
        if self.chain_version != "heuristic_local_v1"
            && self.chain_version != GOVERNANCE_CHAIN_POLICY_VERSION
        {
            return Err(GovernanceChainError::UnsupportedPolicyVersion {
                actual: self.chain_version.clone(),
            });
        }
        require_chain_field(&self.profile_source, "profile_source")?;
        require_chain_field(&self.profile_maturity, "profile_maturity")?;
        if self.lenses.is_empty() {
            return Err(GovernanceChainError::EmptyLenses);
        }
        for lens in &self.lenses {
            require_chain_field(&lens.id, "lens.id")?;
            require_chain_field(&lens.display_name, "lens.display_name")?;
            if !lens.pass_threshold.is_finite()
                || lens.pass_threshold < 0.0
                || lens.pass_threshold > 1.0
            {
                return Err(GovernanceChainError::InvalidPassThreshold {
                    lens_id: lens.id.clone(),
                    threshold: lens.pass_threshold,
                });
            }
        }
        Ok(())
    }

    pub fn to_toml_string(&self) -> Result<String, GovernanceChainError> {
        toml::to_string_pretty(self).map_err(GovernanceChainError::Serialize)
    }
}

#[derive(Debug, Error)]
pub enum GovernanceChainError {
    #[error("failed to read governance chain config from {path}: {source}")]
    Read {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to parse governance chain TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("failed to serialize governance chain TOML: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("unsupported governance chain schema_version: {actual}")]
    InvalidSchemaVersion { actual: String },
    #[error("unsupported governance chain chain_version/policy version: {actual}")]
    UnsupportedPolicyVersion { actual: String },
    #[error("governance chain {field} must not be empty")]
    EmptyField { field: &'static str },
    #[error("governance chain must include at least one lens")]
    EmptyLenses,
    #[error("governance chain lens {lens_id} has invalid pass_threshold {threshold}")]
    InvalidPassThreshold { lens_id: String, threshold: f64 },
    #[error("{0}")]
    UnsafeAutonomyFlag(String),
}

pub fn load_governance_chain(
    path: impl AsRef<Path>,
) -> Result<GovernanceChainConfig, GovernanceChainError> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path).map_err(|source| GovernanceChainError::Read {
        path: path.display().to_string(),
        source,
    })?;
    load_governance_chain_from_str(&raw)
}

pub fn load_governance_chain_from_str(
    raw: &str,
) -> Result<GovernanceChainConfig, GovernanceChainError> {
    let config: GovernanceChainConfig = toml::from_str(raw)?;
    config.validate_g3_config()?;
    Ok(config)
}

/// Evaluate a task against an explicit governance-chain configuration.
///
/// ```
/// use arda_core::Task;
/// use arda_governance::{evaluate_governance_chain, GovernanceChainConfig};
///
/// let task = Task::new("verify the deployment with source evidence", "governance");
/// let result = evaluate_governance_chain(&task, &GovernanceChainConfig::default_triad());
/// assert_eq!(result.chain_id, "default_triad");
/// assert!(!result.lenses.is_empty());
/// ```
pub fn evaluate_governance_chain(
    task: &Task,
    config: &GovernanceChainConfig,
) -> GovernanceChainResult {
    let evidence = assess_governance_evidence(task);
    let lenses = config
        .lenses
        .iter()
        .map(|lens| {
            let score = score_governance_lens(task, &lens.id, &evidence);
            GovernanceLensVerdict {
                lens_id: lens.id.clone(),
                display_name: lens.display_name.clone(),
                profile_id: lens.profile_id.clone(),
                outcome: gate_from_score(score, lens.pass_threshold),
                score,
                pass_threshold: lens.pass_threshold,
            }
        })
        .collect::<Vec<_>>();

    let required = config.required_passes.unwrap_or(lenses.len() as u32);
    let passes = lenses
        .iter()
        .filter(|lens| lens.outcome == GateOutcome::Pass)
        .count() as u32;
    let hard_fail = lenses.iter().any(|lens| lens.outcome == GateOutcome::Fail);
    let passed = if config.strict {
        !hard_fail && passes >= required
    } else {
        passes >= required
    };
    let veto_reason = if passed {
        None
    } else {
        Some(chain_veto_reason(&lenses))
    };
    let failed_gates = lenses
        .iter()
        .filter(|lens| lens.outcome == GateOutcome::Fail)
        .map(|lens| GovernanceGateName::from_lens_id(&lens.lens_id))
        .collect::<Vec<_>>();
    let veto = (!passed).then_some(GovernanceVetoReason {
        code: if failed_gates.is_empty() {
            GovernanceVetoCode::InsufficientPassCount
        } else {
            GovernanceVetoCode::GateFailed
        },
        failed_gates,
        required_passes: required,
        observed_passes: passes,
    });

    GovernanceChainResult {
        policy_version: GOVERNANCE_CHAIN_POLICY_VERSION.to_string(),
        chain_id: config.chain_id.clone(),
        chain_version: config.chain_version.clone(),
        profile_source: config.profile_source.clone(),
        review_mode: config.review_mode,
        profile_maturity: config.profile_maturity.clone(),
        required_passes: required,
        autonomous_blocking_enabled: false,
        passed,
        veto_reason,
        veto,
        lenses,
        evidence: evidence.assessment,
    }
}

/// Validate task through the Triad Gate
pub fn triad_validate(task: &Task, config: Option<&TriadConfig>) -> TriadResult {
    let default_config = TriadConfig::default();
    let config = config.unwrap_or(&default_config);
    let chain_config = GovernanceChainConfig {
        required_passes: config.required_passes,
        strict: config.strict,
        ..GovernanceChainConfig::default_triad()
    };
    let chain = evaluate_governance_chain(task, &chain_config);

    let aurelius = lens_outcome(&chain.lenses, "aurelius");
    let bacon = lens_outcome(&chain.lenses, "bacon");
    let sun_tzu = lens_outcome(&chain.lenses, "sun_tzu");

    let result = TriadResult {
        policy_version: TRIAD_POLICY_VERSION.to_string(),
        chain_id: chain.chain_id,
        chain_version: chain.chain_version,
        profile_source: chain.profile_source,
        review_mode: chain.review_mode,
        profile_maturity: chain.profile_maturity,
        aurelius,
        bacon,
        sun_tzu,
        aurelius_score: lens_score(&chain.lenses, "aurelius"),
        bacon_score: lens_score(&chain.lenses, "bacon"),
        sun_tzu_score: lens_score(&chain.lenses, "sun_tzu"),
        passed: chain.passed,
        veto_reason: chain.veto_reason,
        veto: chain.veto,
        evidence: chain.evidence,
    };
    crate::global_governance_metrics().observe_triad(&result);
    result
}

fn gate_from_score(score: f64, pass_threshold: f64) -> GateOutcome {
    if score >= pass_threshold {
        GateOutcome::Pass
    } else if score >= 0.35 {
        GateOutcome::Conditional
    } else {
        GateOutcome::Fail
    }
}

fn require_chain_field(value: &str, field: &'static str) -> Result<(), GovernanceChainError> {
    if value.trim().is_empty() {
        return Err(GovernanceChainError::EmptyField { field });
    }
    Ok(())
}

fn score_governance_lens(task: &Task, lens_id: &str, evidence: &GovernanceEvidenceContext) -> f64 {
    match lens_id {
        "aurelius" => score_aurelius(task, evidence),
        "bacon" => score_bacon(task, evidence),
        "sun_tzu" => score_sun_tzu(task, evidence),
        _ => score_named_lens(task, lens_id, evidence),
    }
}

pub(crate) fn score_governance_lens_for_scorer(task: &Task, lens_id: &str) -> Option<f64> {
    match lens_id {
        "aurelius" | "bacon" | "sun_tzu" => {
            let evidence = assess_governance_evidence(task);
            Some(score_governance_lens(task, lens_id, &evidence))
        }
        _ => None,
    }
}

fn score_named_lens(task: &Task, lens_id: &str, evidence: &GovernanceEvidenceContext) -> f64 {
    let desc = task.description.to_lowercase();
    let lens = lens_id.to_lowercase();
    let mut score: f64 = 0.40;

    if !desc.trim().is_empty() {
        score += 0.10;
    }
    if desc.contains(&lens) {
        score += 0.15;
    }
    if has_action_verb(&desc) {
        score += 0.10;
    }
    if desc.contains("because") || desc.contains("evidence") || desc.contains("source") {
        score += 0.15;
    }
    if !has_contradiction(&desc) {
        score += 0.10;
    }

    apply_evidence_grade(score, evidence, false)
}

fn lens_outcome(lenses: &[GovernanceLensVerdict], lens_id: &str) -> GateOutcome {
    lenses
        .iter()
        .find(|lens| lens.lens_id == lens_id)
        .map(|lens| lens.outcome)
        .unwrap_or(GateOutcome::Fail)
}

fn lens_score(lenses: &[GovernanceLensVerdict], lens_id: &str) -> f64 {
    lenses
        .iter()
        .find(|lens| lens.lens_id == lens_id)
        .map(|lens| lens.score)
        .unwrap_or(0.0)
}

fn chain_veto_reason(lenses: &[GovernanceLensVerdict]) -> String {
    let mut vetoes = lenses
        .iter()
        .filter(|lens| lens.outcome == GateOutcome::Fail)
        .map(|lens| format!("{}_FAIL", lens.lens_id.to_uppercase()))
        .collect::<Vec<_>>();

    if vetoes.is_empty() {
        "INSUFFICIENT_PASS_COUNT".to_string()
    } else {
        vetoes.sort();
        vetoes.join("|")
    }
}

fn score_aurelius(task: &Task, context: &GovernanceEvidenceContext) -> f64 {
    if let Some(evidence) = context.evidence.as_ref() {
        let mut score: f64 = 0.35;
        if !evidence.action_intent.trim().is_empty() {
            score += 0.25;
        }
        if !evidence.disconfirming_evidence.is_empty() {
            score += 0.15;
        }
        if evidence.risk_boundary.is_some() {
            score += 0.10;
        }
        if evidence.cooperation.unwrap_or(0.0) >= evidence.defection.unwrap_or(1.0) {
            score += 0.10;
        }
        if has_contradiction(&evidence.action_intent) {
            score -= 0.40;
        }
        return apply_evidence_grade(score, context, false);
    }

    let desc = task.description.trim();
    if desc.is_empty() {
        return 0.0;
    }

    let mut score: f64 = 0.50;
    if desc.len() >= 12 {
        score += 0.20;
    }
    if !has_contradiction(desc) {
        score += 0.20;
    } else {
        score -= 0.35;
    }
    if has_action_verb(desc) {
        score += 0.10;
    }
    apply_evidence_grade(score, context, false)
}

fn score_bacon(task: &Task, context: &GovernanceEvidenceContext) -> f64 {
    if let Some(evidence) = context.evidence.as_ref() {
        let valid_anchors = evidence
            .evidence_anchors
            .iter()
            .filter(|anchor| !anchor.kind.trim().is_empty() && !anchor.uri.trim().is_empty())
            .count();
        let mut score: f64 = 0.30;
        if valid_anchors > 0 {
            score += 0.40;
        }
        if !evidence.disconfirming_evidence.is_empty() {
            score += 0.15;
        }
        if task.clarifications_resolved > 0 {
            score += 0.05;
        }
        return apply_evidence_grade(score, context, true);
    }

    let desc = task.description.to_lowercase();
    let mut score: f64 = 0.30;

    if desc.contains("http://") || desc.contains("https://") {
        score += 0.30;
    }
    if desc.chars().any(|c| c.is_ascii_digit()) {
        score += 0.20;
    }
    if desc.contains("because") || desc.contains("evidence") || desc.contains("source") {
        score += 0.20;
    }
    if task.clarifications_resolved > 0 {
        score += 0.10;
    }

    apply_evidence_grade(score, context, true)
}

fn score_sun_tzu(task: &Task, context: &GovernanceEvidenceContext) -> f64 {
    if let Some(evidence) = context.evidence.as_ref() {
        let mut score: f64 = 0.35;
        if evidence.risk_boundary.is_some() {
            score += 0.20;
        }
        if evidence.fallback_path.is_some() {
            score += 0.20;
        }
        if !evidence.action_intent.trim().is_empty() {
            score += 0.10;
        }
        if evidence.justified_urgency.is_some() {
            score += 0.05;
        }
        score -= evidence.defection.unwrap_or(0.0) * 0.20;
        return apply_evidence_grade(score, context, false);
    }

    let mut score: f64 = 0.55;
    let task_type = task.task_type.to_lowercase();
    let desc = task.description.to_lowercase();

    // Known task classes are easier to route and execute strategically.
    if ["ingest", "research", "query", "monitor", "dispatch"].contains(&task_type.as_str()) {
        score += 0.20;
    } else {
        score -= 0.10;
    }

    // Penalize urgency-without-context patterns.
    let urgent = ["urgent", "asap", "immediately", "emergency"]
        .iter()
        .any(|k| desc.contains(k));
    if urgent && !desc.contains("because") {
        score -= 0.25;
    }

    // Penalize large cost overruns when costs are provided.
    if task.joule_cost_actual > 0.0 && task.joule_cost_estimated > 0.0 {
        let ratio = task.joule_cost_actual / task.joule_cost_estimated;
        if ratio > 1.5 {
            score -= 0.20;
        }
    }

    apply_evidence_grade(score, context, false)
}

fn apply_evidence_grade(
    score: f64,
    context: &GovernanceEvidenceContext,
    evidence_required: bool,
) -> f64 {
    let score = match context.assessment.grade {
        GovernanceEvidenceGrade::StructuredValidated => score,
        GovernanceEvidenceGrade::StructuredPartial => {
            score - (context.assessment.missing_fields.len() as f64 * 0.06) - 0.10
        }
        GovernanceEvidenceGrade::HeuristicOnly | GovernanceEvidenceGrade::NoEvidence => {
            score * 0.75
        }
    };
    if evidence_required && !context.uses_validated_structured_evidence() {
        score.clamp(0.0, 0.49)
    } else {
        score.clamp(0.0, 1.0)
    }
}

fn has_contradiction(desc: &str) -> bool {
    let lower = desc.to_lowercase();
    let contradictions = [
        ("always", "never"),
        ("must", "must not"),
        ("increase", "decrease"),
        ("allow", "deny"),
    ];

    contradictions
        .iter()
        .any(|(a, b)| lower.contains(a) && lower.contains(b))
}

fn has_action_verb(desc: &str) -> bool {
    let lower = desc.to_lowercase();
    [
        "run",
        "ingest",
        "analyze",
        "query",
        "deploy",
        "route",
        "summarize",
    ]
    .iter()
    .any(|verb| lower.contains(verb))
}

// ---------------------------------------------------------------
// loop_engine integration: a TriadConsultant impl that calls
// triad_validate() and maps TriadResult into the contract-shaped
// TriadOutcome. Used by `arda-cli loop tick`.
// ---------------------------------------------------------------

use arda_core::contract::{PhilosopherVerdict, TriadOutcome as ContractTriadOutcome, TriadVerdict};
use arda_core::loop_engine::TriadConsultant;

/// Live triad consultant. Wraps `triad_validate` and reshapes the
/// result into the contract `TriadOutcome` ledgered on every Decision.
#[derive(Debug, Default, Clone)]
pub struct LiveTriad {
    pub config: TriadConfig,
}

impl LiveTriad {
    pub fn new() -> Self {
        Self::default()
    }
}

fn gate_to_verdict(g: GateOutcome) -> TriadVerdict {
    match g {
        GateOutcome::Pass => TriadVerdict::Pass,
        GateOutcome::Conditional => TriadVerdict::Conditional,
        GateOutcome::Fail => TriadVerdict::Fail,
    }
}

fn philosopher(g: GateOutcome, score: f64, name: &str) -> PhilosopherVerdict {
    PhilosopherVerdict {
        verdict: gate_to_verdict(g),
        reason: Some(format!("{name} score={score:.2}")),
    }
}

impl TriadConsultant for LiveTriad {
    fn consult(&self, task: &Task) -> ContractTriadOutcome {
        let r = triad_validate(task, Some(&self.config));
        let overall = if r.passed {
            // If any gate is Conditional, surface that as the
            // overall verdict to keep the signal visible.
            if [r.aurelius, r.bacon, r.sun_tzu].contains(&GateOutcome::Conditional) {
                TriadVerdict::Conditional
            } else {
                TriadVerdict::Pass
            }
        } else {
            TriadVerdict::Fail
        };
        ContractTriadOutcome {
            verdict: overall,
            aurelius: philosopher(r.aurelius, r.aurelius_score, "aurelius"),
            bacon: philosopher(r.bacon, r.bacon_score, "bacon"),
            sun_tzu: philosopher(r.sun_tzu, r.sun_tzu_score, "sun_tzu"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_for_same_task() {
        let task = Task::new(
            "ingest https://example.com because source is official",
            "ingest",
        );
        let a = triad_validate(&task, None);
        let b = triad_validate(&task, None);
        assert_eq!(a.aurelius, b.aurelius);
        assert_eq!(a.bacon, b.bacon);
        assert_eq!(a.sun_tzu, b.sun_tzu);
        assert_eq!(a.passed, b.passed);
    }

    #[test]
    fn contradiction_fails_logic() {
        let task = Task::new("always allow and never allow this route", "query");
        let result = triad_validate(&task, None);
        assert!(matches!(
            result.aurelius,
            GateOutcome::Fail | GateOutcome::Conditional
        ));
    }

    #[test]
    fn default_configurable_chain_matches_legacy_triad_result() {
        let task = Task::new(
            "ingest https://example.com because source is official 2026",
            "ingest",
        );
        let legacy = triad_validate(&task, None);
        let chain = GovernanceChainConfig::default_triad();
        let generalized = evaluate_governance_chain(&task, &chain);

        assert_eq!(chain.chain_id, "default_triad");
        assert_eq!(chain.required_passes, Some(2));
        assert_eq!(generalized.required_passes, 2);
        assert_eq!(generalized.chain_id, legacy.chain_id);
        assert_eq!(generalized.chain_version, legacy.chain_version);
        assert_eq!(generalized.profile_source, legacy.profile_source);
        assert_eq!(generalized.review_mode, legacy.review_mode);
        assert_eq!(generalized.profile_maturity, legacy.profile_maturity);
        assert_eq!(generalized.passed, legacy.passed);
        assert_eq!(generalized.veto_reason, legacy.veto_reason);
        assert_eq!(generalized.lenses.len(), 3);
        assert_eq!(generalized.lenses[0].lens_id, "aurelius");
        assert_eq!(generalized.lenses[0].outcome, legacy.aurelius);
        assert_eq!(generalized.lenses[1].lens_id, "bacon");
        assert_eq!(generalized.lenses[1].outcome, legacy.bacon);
        assert_eq!(generalized.lenses[2].lens_id, "sun_tzu");
        assert_eq!(generalized.lenses[2].outcome, legacy.sun_tzu);
        assert!(!generalized.autonomous_blocking_enabled);
    }

    #[test]
    fn configurable_chain_can_project_non_execution_metadata() {
        let raw = r#"
schema_version = "arda.governance.chains.v1"
chain_id = "default_triad"
chain_version = "heuristic_local_v1"
profile_source = "config/governance/philosophers.toml"
review_mode = "heuristic_local"
profile_maturity = "draft_human_authored"
strict = false
required_passes = 2
autonomous_blocking_enabled = false

[[lenses]]
id = "aurelius"
display_name = "Marcus Aurelius"
profile_id = "aurelius"
pass_threshold = 0.60

[[lenses]]
id = "bacon"
display_name = "Francis Bacon"
profile_id = "bacon"
pass_threshold = 0.50
"#;
        let chain = load_governance_chain_from_str(raw).expect("valid safe chain config");
        let task = Task::new("query source evidence 2026 because path exists", "query");
        let result = evaluate_governance_chain(&task, &chain);

        assert_eq!(result.chain_id, "default_triad");
        assert_eq!(result.required_passes, 2);
        assert_eq!(result.review_mode, GovernanceReviewMode::HeuristicLocal);
        assert_eq!(result.profile_maturity, "draft_human_authored");
        assert_eq!(result.lenses.len(), 2);
        assert!(result.lenses.iter().all(|lens| lens.score.is_finite()));
        assert!(!result.autonomous_blocking_enabled);
    }

    #[test]
    fn legacy_chain_flag_cannot_enable_runtime_blocking() {
        let raw = GovernanceChainConfig::default_triad()
            .to_toml_string()
            .expect("default chain should serialize")
            .replace(
                "autonomous_blocking_enabled = false",
                "autonomous_blocking_enabled = true",
            );
        let chain = load_governance_chain_from_str(&raw)
            .expect("legacy flag remains parseable during Phase 8 migration");
        assert!(chain.autonomous_blocking_enabled);

        let result = evaluate_governance_chain(&Task::new("deploy safely", "deployment"), &chain);
        assert!(!result.autonomous_blocking_enabled);
    }

    #[test]
    fn repository_default_chain_config_matches_g3_contract() {
        let chain = load_governance_chain_from_str(include_str!(
            "../../../../../config/governance/chains.toml"
        ))
        .expect("repository default governance chain should parse and validate");
        let default_chain = GovernanceChainConfig::default_triad();

        assert_eq!(chain.schema_version, GovernanceChainConfig::SCHEMA_VERSION);
        assert_eq!(chain.chain_id, default_chain.chain_id);
        assert_eq!(chain.chain_version, default_chain.chain_version);
        assert_eq!(chain.profile_source, default_chain.profile_source);
        assert_eq!(chain.review_mode, GovernanceReviewMode::HeuristicLocal);
        assert_eq!(chain.profile_maturity, "draft_human_authored");
        assert_eq!(chain.required_passes, Some(2));
        assert_eq!(chain.lenses.len(), 3);
        assert!(chain
            .lenses
            .iter()
            .all(|lens| (0.0..=1.0).contains(&lens.pass_threshold)));
        assert!(!chain.autonomous_blocking_enabled);
    }
}
