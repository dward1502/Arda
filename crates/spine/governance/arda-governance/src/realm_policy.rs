//! Per-realm governance policy, safe blocking authority, and atomic reload receipts.

use std::{
    collections::{BTreeMap, HashSet},
    fs,
    ops::Deref,
    path::Path,
    sync::RwLock,
    time::Duration,
};

use arda_core::Task;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    score_governance_with_timeout, GovernanceReadinessLevel, GovernanceReadinessReport,
    GovernanceScoreReceipt, GovernanceScorer, GovernanceScorerState,
};

const KNOWN_LENSES: [&str; 3] = ["aurelius", "bacon", "sun_tzu"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RealmPolicyRule {
    #[serde(default = "default_required_lenses")]
    pub required_lenses: Vec<String>,
    #[serde(default = "default_weights")]
    pub weights: BTreeMap<String, f64>,
    #[serde(default = "default_thresholds")]
    pub thresholds: BTreeMap<String, f64>,
    #[serde(default = "default_minimum_weighted_score")]
    pub minimum_weighted_score: f64,
    #[serde(default)]
    pub strict: bool,
    #[serde(default)]
    pub review_requirements: Vec<String>,
    #[serde(default)]
    pub autonomous_blocking_enabled: bool,
}

impl Default for RealmPolicyRule {
    fn default() -> Self {
        Self {
            required_lenses: default_required_lenses(),
            weights: default_weights(),
            thresholds: default_thresholds(),
            minimum_weighted_score: default_minimum_weighted_score(),
            strict: false,
            review_requirements: vec!["operator_review".to_string()],
            autonomous_blocking_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealmBlockingControls {
    #[serde(default)]
    pub rollback_enabled: bool,
    #[serde(default)]
    pub operator_disable_enabled: bool,
    #[serde(default)]
    pub independent_review_receipt_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RealmPolicyScope {
    pub scope_id: String,
    pub realm: String,
    pub action_class: String,
    #[serde(flatten)]
    pub rule: RealmPolicyRule,
    pub readiness_subsystem: String,
    #[serde(default)]
    pub blocking_controls: RealmBlockingControls,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RealmPolicyConfig {
    pub schema_version: String,
    pub policy_version: String,
    #[serde(default)]
    pub global_default: RealmPolicyRule,
    #[serde(default)]
    pub scopes: Vec<RealmPolicyScope>,
}

pub struct ResolvedRealmPolicy<'a> {
    pub scope_id: Option<&'a str>,
    pub rule: &'a RealmPolicyRule,
}

impl Deref for ResolvedRealmPolicy<'_> {
    type Target = RealmPolicyRule;

    fn deref(&self) -> &Self::Target {
        self.rule
    }
}

impl RealmPolicyConfig {
    pub const SCHEMA_VERSION: &'static str = "arda.governance.realm_policy.v1";

    pub fn safe_default() -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION.to_string(),
            policy_version: "safe-default-v1".to_string(),
            global_default: RealmPolicyRule::default(),
            scopes: Vec::new(),
        }
    }

    pub fn resolve(&self, realm: &str, action_class: &str) -> Option<ResolvedRealmPolicy<'_>> {
        if realm.trim().is_empty() || action_class.trim().is_empty() {
            return None;
        }
        let scope = self
            .scopes
            .iter()
            .find(|scope| scope.realm == realm && scope.action_class == action_class);
        Some(match scope {
            Some(scope) => ResolvedRealmPolicy {
                scope_id: Some(&scope.scope_id),
                rule: &scope.rule,
            },
            None => ResolvedRealmPolicy {
                scope_id: None,
                rule: &self.global_default,
            },
        })
    }

    pub fn validate(&self) -> Result<(), RealmPolicyError> {
        if self.schema_version != Self::SCHEMA_VERSION {
            return Err(RealmPolicyError::InvalidSchemaVersion(
                self.schema_version.clone(),
            ));
        }
        require_non_empty(&self.policy_version, "policy_version")?;
        if self.global_default.autonomous_blocking_enabled {
            return Err(RealmPolicyError::GlobalAutonomousBlocking);
        }
        validate_rule(&self.global_default, "global_default")?;

        let mut scope_ids = HashSet::new();
        let mut scope_keys = HashSet::new();
        for scope in &self.scopes {
            require_non_empty(&scope.scope_id, "scope_id")?;
            require_named_scope(&scope.realm, "realm")?;
            require_named_scope(&scope.action_class, "action_class")?;
            require_non_empty(&scope.readiness_subsystem, "readiness_subsystem")?;
            if !scope_ids.insert(scope.scope_id.clone()) {
                return Err(RealmPolicyError::DuplicateScope(scope.scope_id.clone()));
            }
            if !scope_keys.insert((scope.realm.clone(), scope.action_class.clone())) {
                return Err(RealmPolicyError::DuplicateScope(format!(
                    "{}:{}",
                    scope.realm, scope.action_class
                )));
            }
            validate_rule(&scope.rule, &scope.scope_id)?;
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum RealmPolicyError {
    #[error("failed to read realm policy from {path}: {source}")]
    Read {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to parse realm policy TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("unsupported realm policy schema version: {0}")]
    InvalidSchemaVersion(String),
    #[error("realm policy {0} must not be empty")]
    EmptyField(&'static str),
    #[error("realm policy {field} must name an explicit scope and cannot use a wildcard")]
    UnnamedScope { field: &'static str },
    #[error("duplicate realm policy scope: {0}")]
    DuplicateScope(String),
    #[error("realm policy {scope} contains unknown lens: {lens}")]
    UnknownLens { scope: String, lens: String },
    #[error("realm policy {scope} weight for {lens} must be finite and greater than zero")]
    InvalidWeight { scope: String, lens: String },
    #[error("realm policy {scope} threshold for {lens} must be in the unit interval")]
    InvalidThreshold { scope: String, lens: String },
    #[error("realm policy {scope} minimum weighted score must be in the unit interval")]
    InvalidMinimumWeightedScore { scope: String },
    #[error(
        "global autonomous blocking is forbidden; blocking requires an explicitly named scope"
    )]
    GlobalAutonomousBlocking,
    #[error("realm policy scope is missing: {realm}:{action_class}")]
    MissingScope { realm: String, action_class: String },
}

pub fn load_realm_policy(path: impl AsRef<Path>) -> Result<RealmPolicyConfig, RealmPolicyError> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path).map_err(|source| RealmPolicyError::Read {
        path: path.display().to_string(),
        source,
    })?;
    load_realm_policy_from_str(&raw)
}

pub fn load_realm_policy_from_str(raw: &str) -> Result<RealmPolicyConfig, RealmPolicyError> {
    let config: RealmPolicyConfig = toml::from_str(raw)?;
    config.validate()?;
    Ok(config)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RealmGovernanceVerdict {
    pub policy_version: String,
    pub realm: String,
    pub action_class: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_id: Option<String>,
    pub weighted_score: f64,
    pub minimum_weighted_score: f64,
    pub degraded: bool,
    pub passed: bool,
    pub review_requirements: Vec<String>,
    pub scorer_receipts: Vec<GovernanceScoreReceipt>,
}

pub async fn evaluate_realm_governance(
    task: &Task,
    policy: &RealmPolicyConfig,
    realm: &str,
    action_class: &str,
    scorer: &dyn GovernanceScorer,
    timeout: Duration,
) -> Result<RealmGovernanceVerdict, RealmPolicyError> {
    policy.validate()?;
    let resolved =
        policy
            .resolve(realm, action_class)
            .ok_or_else(|| RealmPolicyError::MissingScope {
                realm: realm.to_string(),
                action_class: action_class.to_string(),
            })?;

    let mut receipts = Vec::with_capacity(resolved.required_lenses.len());
    for lens in &resolved.required_lenses {
        let request = crate::GovernanceScoreRequest::new(task.clone(), lens.clone());
        receipts.push(score_governance_with_timeout(scorer, request, timeout).await);
    }

    let degraded = receipts
        .iter()
        .any(|receipt| receipt.state != GovernanceScorerState::Complete);
    let total_weight = resolved
        .required_lenses
        .iter()
        .map(|lens| resolved.weights[lens])
        .sum::<f64>();
    let weighted_score = receipts
        .iter()
        .map(|receipt| receipt.score * resolved.weights[&receipt.lens_id])
        .sum::<f64>()
        / total_weight;
    let every_required_lens_passes = receipts.iter().all(|receipt| {
        receipt.state == GovernanceScorerState::Complete
            && receipt.score >= resolved.thresholds[&receipt.lens_id]
    });
    let score_passes = weighted_score >= resolved.minimum_weighted_score;
    let passed = !degraded && score_passes && (!resolved.strict || every_required_lens_passes);

    Ok(RealmGovernanceVerdict {
        policy_version: policy.policy_version.clone(),
        realm: realm.to_string(),
        action_class: action_class.to_string(),
        scope_id: resolved.scope_id.map(str::to_string),
        weighted_score,
        minimum_weighted_score: resolved.minimum_weighted_score,
        degraded,
        passed,
        review_requirements: resolved.review_requirements.clone(),
        scorer_receipts: receipts,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeBlockingDecision {
    pub policy_version: String,
    pub realm: String,
    pub action_class: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_id: Option<String>,
    pub blocking_requested: bool,
    pub blocking_enabled: bool,
    pub readiness_level: GovernanceReadinessLevel,
    pub reason: String,
}

pub struct RuntimeBlockingAuthority;

impl RuntimeBlockingAuthority {
    pub fn evaluate(
        policy: &RealmPolicyConfig,
        realm: &str,
        action_class: &str,
        readiness: &GovernanceReadinessReport,
        operator_blocking_enabled: bool,
    ) -> RuntimeBlockingDecision {
        if let Err(error) = policy.validate() {
            return blocking_decision(
                policy,
                (realm, action_class),
                None,
                false,
                GovernanceReadinessLevel::DocumentedOnly,
                &format!("invalid realm policy; blocking denied: {error}"),
            );
        }
        let Some(scope) = policy
            .scopes
            .iter()
            .find(|scope| scope.realm == realm && scope.action_class == action_class)
        else {
            return blocking_decision(
                policy,
                (realm, action_class),
                None,
                false,
                GovernanceReadinessLevel::DocumentedOnly,
                "no explicitly named blocking scope; global default is non-blocking",
            );
        };

        let readiness_scope = readiness
            .subsystems
            .iter()
            .find(|candidate| candidate.subsystem == scope.readiness_subsystem);
        let readiness_level = readiness_scope
            .map(|candidate| candidate.current_level)
            .unwrap_or(GovernanceReadinessLevel::DocumentedOnly);
        let requested = scope.rule.autonomous_blocking_enabled;

        let (enabled, reason) = if !requested {
            (false, "scope policy does not request blocking")
        } else if !operator_blocking_enabled || !scope.blocking_controls.operator_disable_enabled {
            (
                false,
                "operator blocking control is disabled or unavailable",
            )
        } else if !scope.blocking_controls.rollback_enabled {
            (false, "scope has no verified rollback control")
        } else if scope
            .blocking_controls
            .independent_review_receipt_ids
            .is_empty()
        {
            (false, "scope has no independent-review receipt requirement")
        } else if readiness_level != GovernanceReadinessLevel::AutonomyReadyForScope {
            (
                false,
                "scope readiness has not reached autonomy_ready_for_scope",
            )
        } else if !scope
            .blocking_controls
            .independent_review_receipt_ids
            .iter()
            .all(|required| {
                readiness_scope.is_some_and(|candidate| candidate.receipts.contains(required))
            })
        {
            (false, "required independent-review receipts are missing")
        } else {
            (
                true,
                "explicit scope passed runtime blocking authority checks",
            )
        };

        blocking_decision(
            policy,
            (realm, action_class),
            Some((&scope.scope_id, requested)),
            enabled,
            readiness_level,
            reason,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RealmPolicyReloadStatus {
    Applied,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealmPolicyReloadReceipt {
    pub schema_version: String,
    pub source: String,
    pub generation: u64,
    pub previous_policy_version: String,
    pub previous_policy_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposed_policy_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposed_policy_hash: Option<String>,
    pub status: RealmPolicyReloadStatus,
    pub reason: String,
    pub changed_at_utc: String,
}

struct RealmPolicyStoreState {
    config: RealmPolicyConfig,
    generation: u64,
}

pub struct RealmPolicyStore {
    state: RwLock<RealmPolicyStoreState>,
}

impl RealmPolicyStore {
    pub fn new(config: RealmPolicyConfig) -> Result<Self, RealmPolicyError> {
        config.validate()?;
        Ok(Self {
            state: RwLock::new(RealmPolicyStoreState {
                config,
                generation: 1,
            }),
        })
    }

    pub fn snapshot(&self) -> RealmPolicyConfig {
        self.state.read().unwrap().config.clone()
    }

    pub fn reload_from_str(
        &self,
        source: impl Into<String>,
        raw: &str,
    ) -> RealmPolicyReloadReceipt {
        let source = source.into();
        let proposed = load_realm_policy_from_str(raw);
        let mut state = self.state.write().unwrap();
        let previous_policy_version = state.config.policy_version.clone();
        let previous_policy_hash = policy_hash(&state.config);

        match proposed {
            Ok(config) => {
                state.generation += 1;
                let receipt = RealmPolicyReloadReceipt {
                    schema_version: "arda.governance.realm_policy_reload.v1".to_string(),
                    source,
                    generation: state.generation,
                    previous_policy_version,
                    previous_policy_hash,
                    proposed_policy_version: Some(config.policy_version.clone()),
                    proposed_policy_hash: Some(policy_hash(&config)),
                    status: RealmPolicyReloadStatus::Applied,
                    reason: "validated policy applied atomically".to_string(),
                    changed_at_utc: Utc::now().to_rfc3339(),
                };
                state.config = config;
                receipt
            }
            Err(error) => RealmPolicyReloadReceipt {
                schema_version: "arda.governance.realm_policy_reload.v1".to_string(),
                source,
                generation: state.generation,
                previous_policy_version,
                previous_policy_hash,
                proposed_policy_version: None,
                proposed_policy_hash: None,
                status: RealmPolicyReloadStatus::Rejected,
                reason: error.to_string(),
                changed_at_utc: Utc::now().to_rfc3339(),
            },
        }
    }
}

fn validate_rule(rule: &RealmPolicyRule, scope: &str) -> Result<(), RealmPolicyError> {
    if rule.required_lenses.is_empty() {
        return Err(RealmPolicyError::EmptyField("required_lenses"));
    }
    for lens in rule
        .required_lenses
        .iter()
        .chain(rule.weights.keys())
        .chain(rule.thresholds.keys())
    {
        if !KNOWN_LENSES.contains(&lens.as_str()) {
            return Err(RealmPolicyError::UnknownLens {
                scope: scope.to_string(),
                lens: lens.clone(),
            });
        }
    }
    for lens in &rule.required_lenses {
        let weight = rule.weights.get(lens).copied().unwrap_or_default();
        if !weight.is_finite() || weight <= 0.0 {
            return Err(RealmPolicyError::InvalidWeight {
                scope: scope.to_string(),
                lens: lens.clone(),
            });
        }
        let threshold = rule.thresholds.get(lens).copied().unwrap_or(f64::NAN);
        if !unit_interval(threshold) {
            return Err(RealmPolicyError::InvalidThreshold {
                scope: scope.to_string(),
                lens: lens.clone(),
            });
        }
    }
    for (lens, weight) in &rule.weights {
        if !weight.is_finite() || *weight <= 0.0 {
            return Err(RealmPolicyError::InvalidWeight {
                scope: scope.to_string(),
                lens: lens.clone(),
            });
        }
    }
    for (lens, threshold) in &rule.thresholds {
        if !unit_interval(*threshold) {
            return Err(RealmPolicyError::InvalidThreshold {
                scope: scope.to_string(),
                lens: lens.clone(),
            });
        }
    }
    if !unit_interval(rule.minimum_weighted_score) {
        return Err(RealmPolicyError::InvalidMinimumWeightedScore {
            scope: scope.to_string(),
        });
    }
    Ok(())
}

fn blocking_decision(
    policy: &RealmPolicyConfig,
    coordinates: (&str, &str),
    scope: Option<(&str, bool)>,
    enabled: bool,
    readiness_level: GovernanceReadinessLevel,
    reason: &str,
) -> RuntimeBlockingDecision {
    RuntimeBlockingDecision {
        policy_version: policy.policy_version.clone(),
        realm: coordinates.0.to_string(),
        action_class: coordinates.1.to_string(),
        scope_id: scope.map(|(scope_id, _)| scope_id.to_string()),
        blocking_requested: scope.is_some_and(|(_, requested)| requested),
        blocking_enabled: enabled,
        readiness_level,
        reason: reason.to_string(),
    }
}

fn require_non_empty(value: &str, field: &'static str) -> Result<(), RealmPolicyError> {
    if value.trim().is_empty() {
        Err(RealmPolicyError::EmptyField(field))
    } else {
        Ok(())
    }
}

fn require_named_scope(value: &str, field: &'static str) -> Result<(), RealmPolicyError> {
    if value.trim().is_empty() || value == "*" || value.eq_ignore_ascii_case("global") {
        Err(RealmPolicyError::UnnamedScope { field })
    } else {
        Ok(())
    }
}

fn unit_interval(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn default_required_lenses() -> Vec<String> {
    KNOWN_LENSES
        .iter()
        .map(|lens| (*lens).to_string())
        .collect()
}

fn default_weights() -> BTreeMap<String, f64> {
    KNOWN_LENSES
        .iter()
        .map(|lens| ((*lens).to_string(), 1.0))
        .collect()
}

fn default_thresholds() -> BTreeMap<String, f64> {
    [("aurelius", 0.60), ("bacon", 0.50), ("sun_tzu", 0.50)]
        .into_iter()
        .map(|(lens, threshold)| (lens.to_string(), threshold))
        .collect()
}

fn default_minimum_weighted_score() -> f64 {
    0.60
}

fn policy_hash(config: &RealmPolicyConfig) -> String {
    let encoded = serde_json::to_vec(config).unwrap_or_default();
    crate::scorer::sha256_hex(&encoded)
}
