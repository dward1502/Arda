// sigil: REPAIR
//! Triad Gate Implementation
//!
//! Three-gate validation: Aurelius (logic), Bacon (empirics), Sun Tzu (strategy)

use std::{fs, path::Path};

use arda_core::task::Task;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceReviewMode {
    HeuristicLocal,
    IndependentAgent,
    HumanReviewed,
    ConsensusReceipted,
}

impl Default for GovernanceReviewMode {
    fn default() -> Self {
        Self::HeuristicLocal
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriadResult {
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
    pub chain_id: String,
    pub chain_version: String,
    pub profile_source: String,
    pub review_mode: GovernanceReviewMode,
    pub profile_maturity: String,
    pub required_passes: u32,
    pub autonomous_blocking_enabled: bool,
    pub passed: bool,
    pub veto_reason: Option<String>,
    pub lenses: Vec<GovernanceLensVerdict>,
}

impl GovernanceChainConfig {
    pub const SCHEMA_VERSION: &'static str = "arda.governance.chains.v1";

    pub fn default_triad() -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION.to_string(),
            chain_id: "default_triad".to_string(),
            chain_version: "heuristic_local_v1".to_string(),
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
        require_chain_field(&self.profile_source, "profile_source")?;
        require_chain_field(&self.profile_maturity, "profile_maturity")?;
        if self.autonomous_blocking_enabled {
            return Err(GovernanceChainError::UnsafeAutonomyFlag(
                "autonomous_blocking_enabled must remain false until Phase G7".to_string(),
            ));
        }
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

pub fn evaluate_governance_chain(
    task: &Task,
    config: &GovernanceChainConfig,
) -> GovernanceChainResult {
    let lenses = config
        .lenses
        .iter()
        .map(|lens| {
            let score = score_governance_lens(task, &lens.id);
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

    GovernanceChainResult {
        chain_id: config.chain_id.clone(),
        chain_version: config.chain_version.clone(),
        profile_source: config.profile_source.clone(),
        review_mode: config.review_mode,
        profile_maturity: config.profile_maturity.clone(),
        required_passes: required,
        autonomous_blocking_enabled: false,
        passed,
        veto_reason,
        lenses,
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

    TriadResult {
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
    }
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

fn score_governance_lens(task: &Task, lens_id: &str) -> f64 {
    match lens_id {
        "aurelius" => score_aurelius(task),
        "bacon" => score_bacon(task),
        "sun_tzu" => score_sun_tzu(task),
        _ => score_named_lens(task, lens_id),
    }
}

fn score_named_lens(task: &Task, lens_id: &str) -> f64 {
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

    score.clamp(0.0, 1.0)
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

fn score_aurelius(task: &Task) -> f64 {
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
    score.min(1.0)
}

fn score_bacon(task: &Task) -> f64 {
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

    score.min(1.0)
}

fn score_sun_tzu(task: &Task) -> f64 {
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

    score.clamp(0.0, 1.0)
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

use arda_core::contract::{
    PhilosopherVerdict, TriadOutcome as ContractTriadOutcome, TriadVerdict,
};
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
    fn configurable_chain_rejects_autonomous_blocking_before_g7() {
        let raw = GovernanceChainConfig::default_triad()
            .to_toml_string()
            .expect("default chain should serialize")
            .replace(
                "autonomous_blocking_enabled = false",
                "autonomous_blocking_enabled = true",
            );
        let err = load_governance_chain_from_str(&raw)
            .expect_err("G3 chains must not enable autonomous blocking");
        assert!(err
            .to_string()
            .contains("autonomous_blocking_enabled must remain false"));
    }

    #[test]
    fn repository_default_chain_config_matches_g3_contract() {
        let chain =
            load_governance_chain_from_str(include_str!("../../../config/governance/chains.toml"))
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
