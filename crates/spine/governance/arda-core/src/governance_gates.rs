//! Council/Oracle governance gates (Phase 2 step 9).
//!
//! Per-DecisionClass policy: should this class route through Council
//! deliberation before execution, what does that deliberation cost in
//! joules, and should Triad Fail outcomes block the action? Loaded
//! from `config/governance_gates.yaml`; the default is a no-op gate
//! (no council, no cost, record-and-proceed Triad outcomes) so opting
//! into stricter behavior is a config edit, not a code change.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::contract::DecisionClass;
use crate::task::JouleWorkMeasurementSource;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionReversibility {
    Reversible,
    Compensatable,
    Irreversible,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsentAuthority {
    OperatorAuthored,
    ScopedApproval,
    PolicyAllowed,
    Inferred,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoercionRisk {
    None,
    Low,
    Elevated,
    High,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JouleWorkBudgetClass {
    Quiet,
    Routine,
    Elevated,
    Consequential,
}

impl JouleWorkBudgetClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Quiet => "quiet",
            Self::Routine => "routine",
            Self::Elevated => "elevated",
            Self::Consequential => "consequential",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorBurdenEstimate {
    pub estimated_interruption_seconds: u64,
    pub estimated_recovery_seconds: u64,
    pub source: JouleWorkMeasurementSource,
    pub confidence: f64,
}

impl Default for OperatorBurdenEstimate {
    fn default() -> Self {
        Self {
            estimated_interruption_seconds: 0,
            estimated_recovery_seconds: 0,
            source: JouleWorkMeasurementSource::DefaultFallback,
            confidence: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HumanImpactReviewInput {
    pub affected_parties: Vec<String>,
    pub reversibility: ActionReversibility,
    pub interruption_reason: Option<String>,
    pub consent_authority: ConsentAuthority,
    pub uncertainty: f64,
    pub coercion_risk: CoercionRisk,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HumanFacingActionReview {
    pub schema_version: String,
    pub semantic: String,
    pub affected_parties: Vec<String>,
    pub reversibility: ActionReversibility,
    pub interruption_reason: Option<String>,
    pub consent_authority: ConsentAuthority,
    pub uncertainty: f64,
    pub coercion_risk: CoercionRisk,
}

impl HumanFacingActionReview {
    pub const SCHEMA_VERSION: &'static str = "arda.human-impact-review.v1";
    pub const SEMANTIC: &'static str = "canonical_relational_human_impact_review";

    pub fn proactive_message_explanation(&self, budget_class: JouleWorkBudgetClass) -> String {
        format!(
            "Interruption basis: {}. JouleWork budget class: {}.",
            self.interruption_reason
                .as_deref()
                .unwrap_or("no interruption basis supplied"),
            budget_class.as_str()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AutonomyBasisDecision {
    pub contract: &'static str,
    pub allowed: bool,
    pub reason: &'static str,
    pub measurement_source: JouleWorkMeasurementSource,
    pub measurement_confidence: f64,
}

/// Runtime budget contract consumed by governance without depending on a
/// concrete economics implementation.
pub trait AffordabilityPolicy: Send + Sync {
    fn policy_name(&self) -> &'static str;
    fn can_afford(&self, estimated_cost: f64) -> bool;
}

/// Default used by compatibility dispatch entrypoints that have no economics
/// provider wired yet.
pub struct AllowAllAffordability;

impl AffordabilityPolicy for AllowAllAffordability {
    fn policy_name(&self) -> &'static str {
        "allow_all_compatibility"
    }

    fn can_afford(&self, _estimated_cost: f64) -> bool {
        true
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AffordabilityDecision {
    pub contract: &'static str,
    pub policy: &'static str,
    pub policy_mode: GovernancePolicyMode,
    pub estimated_cost: f64,
    pub allowed: bool,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GovernancePolicyMode {
    ObserveOnly,
    #[default]
    RecordAndProceed,
    BlockOnFail,
    EscalateToHuman,
    RequireIndependentReceipts,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct GovernancePolicy {
    #[serde(default)]
    pub require_council: bool,
    #[serde(default)]
    pub council_joule_cost: f64,
    #[serde(default)]
    pub block_on_triad_fail: bool,
    #[serde(default, rename = "policy_mode")]
    mode: GovernancePolicyMode,
}

impl GovernancePolicy {
    pub fn policy_mode(&self) -> GovernancePolicyMode {
        if self.block_on_triad_fail {
            GovernancePolicyMode::BlockOnFail
        } else {
            self.mode
        }
    }

    pub fn blocks_on_triad_fail(&self) -> bool {
        matches!(self.policy_mode(), GovernancePolicyMode::BlockOnFail)
    }
}

impl Default for GovernancePolicy {
    fn default() -> Self {
        Self {
            require_council: false,
            council_joule_cost: 0.0,
            block_on_triad_fail: false,
            mode: GovernancePolicyMode::RecordAndProceed,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct GovernanceGates {
    default: GovernancePolicy,
    by_class: HashMap<String, GovernancePolicy>,
    by_action_class: HashMap<String, GovernancePolicy>,
}

#[derive(Debug, thiserror::Error)]
pub enum GovernanceGatesError {
    #[error("governance gates io: {0}")]
    Io(#[from] std::io::Error),
    #[error("governance gates parse: {0}")]
    Parse(String),
}

#[derive(Debug, Deserialize)]
struct GatesFile {
    #[serde(default)]
    default: GovernancePolicy,
    #[serde(default)]
    classes: HashMap<String, GovernancePolicy>,
    #[serde(default)]
    action_classes: HashMap<String, GovernancePolicy>,
}

impl GovernanceGates {
    /// Permissive default: no class requires council, no joule cost,
    /// and Triad Fail remains record-and-proceed evidence.
    /// The dispatcher uses this when no config file is present.
    pub fn permissive() -> Self {
        Self::default()
    }

    pub fn load_from_path(path: &Path) -> Result<Self, GovernanceGatesError> {
        let raw = std::fs::read_to_string(path)?;
        Self::load_from_str(&raw)
    }

    pub fn load_from_str(raw: &str) -> Result<Self, GovernanceGatesError> {
        let file: GatesFile =
            serde_yaml::from_str(raw).map_err(|e| GovernanceGatesError::Parse(e.to_string()))?;
        Ok(Self {
            default: file.default,
            by_class: file.classes,
            by_action_class: file.action_classes,
        })
    }

    pub fn policy_for(&self, class: DecisionClass) -> GovernancePolicy {
        let key = match class {
            DecisionClass::Dispatch => "dispatch",
            DecisionClass::Governance => "governance",
            DecisionClass::Budget => "budget",
            DecisionClass::Retire => "retire",
            DecisionClass::Bid => "bid",
        };
        self.by_class.get(key).copied().unwrap_or(self.default)
    }

    pub fn policy_for_action_class(&self, action_class: &str) -> GovernancePolicy {
        self.by_action_class
            .get(action_class)
            .copied()
            .unwrap_or(self.default)
    }

    pub fn evaluate_affordability(
        &self,
        affordability: &dyn AffordabilityPolicy,
        estimated_cost: f64,
    ) -> AffordabilityDecision {
        let policy_mode = self.policy_for(DecisionClass::Budget).policy_mode();
        let finite_nonnegative = estimated_cost.is_finite() && estimated_cost >= 0.0;
        let affordable = finite_nonnegative && affordability.can_afford(estimated_cost);
        AffordabilityDecision {
            contract: "arda.governance.affordability.v1",
            policy: affordability.policy_name(),
            policy_mode,
            estimated_cost,
            allowed: affordable,
            reason: if !finite_nonnegative {
                "invalid_estimated_cost"
            } else if affordable {
                "within_budget"
            } else {
                "budget_exceeded"
            },
        }
    }

    pub fn evaluate_autonomy_basis(
        &self,
        measurement_source: JouleWorkMeasurementSource,
        measurement_confidence: f64,
    ) -> AutonomyBasisDecision {
        let confidence_is_valid =
            measurement_confidence.is_finite() && (0.0..=1.0).contains(&measurement_confidence);
        let confidence_is_sufficient = confidence_is_valid && measurement_confidence > 0.0;
        let allowed = confidence_is_sufficient && measurement_source.is_autonomy_truth();
        AutonomyBasisDecision {
            contract: "arda.governance.autonomy-basis.v1",
            allowed,
            reason: if !confidence_is_valid {
                "invalid_measurement_confidence"
            } else if !measurement_source.is_autonomy_truth() {
                "fallback_or_synthetic_measurement"
            } else if !confidence_is_sufficient {
                "insufficient_measurement_confidence"
            } else {
                "independent_measurement_basis_present"
            },
            measurement_source,
            measurement_confidence,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissive_lets_everything_through() {
        let g = GovernanceGates::permissive();
        for c in [
            DecisionClass::Dispatch,
            DecisionClass::Governance,
            DecisionClass::Budget,
            DecisionClass::Retire,
            DecisionClass::Bid,
        ] {
            let p = g.policy_for(c);
            assert!(!p.require_council);
            assert_eq!(p.council_joule_cost, 0.0);
            assert!(!p.block_on_triad_fail);
        }
    }

    #[test]
    fn loads_yaml_with_per_class_overrides() {
        let raw = r#"
default:
  require_council: false
  council_joule_cost: 0.0
classes:
  retire:
    require_council: true
    council_joule_cost: 5.0
    block_on_triad_fail: true
  governance:
    require_council: true
    council_joule_cost: 2.0
"#;
        let g = GovernanceGates::load_from_str(raw).unwrap();
        let r = g.policy_for(DecisionClass::Retire);
        assert!(r.require_council);
        assert_eq!(r.council_joule_cost, 5.0);
        assert!(r.blocks_on_triad_fail());
        let d = g.policy_for(DecisionClass::Dispatch);
        assert!(!d.require_council);
        assert!(!d.blocks_on_triad_fail());
    }

    #[test]
    fn loads_policy_modes_and_action_class_overrides() {
        let raw = r#"
default:
  policy_mode: record_and_proceed
classes:
  dispatch:
    policy_mode: block_on_fail
action_classes:
  read_only_audit:
    policy_mode: observe_only
  provider_reroute:
    policy_mode: escalate_to_human
  destructive_delete:
    policy_mode: require_independent_receipts
"#;
        let g = GovernanceGates::load_from_str(raw).unwrap();

        assert_eq!(
            g.policy_for(DecisionClass::Dispatch).policy_mode(),
            GovernancePolicyMode::BlockOnFail
        );
        assert!(g.policy_for(DecisionClass::Dispatch).blocks_on_triad_fail());
        assert_eq!(
            g.policy_for_action_class("read_only_audit").policy_mode(),
            GovernancePolicyMode::ObserveOnly
        );
        assert_eq!(
            g.policy_for_action_class("provider_reroute").policy_mode(),
            GovernancePolicyMode::EscalateToHuman
        );
        assert_eq!(
            g.policy_for_action_class("destructive_delete")
                .policy_mode(),
            GovernancePolicyMode::RequireIndependentReceipts
        );
        assert_eq!(
            g.policy_for_action_class("unknown_action").policy_mode(),
            GovernancePolicyMode::RecordAndProceed
        );
    }

    #[test]
    fn affordability_hook_rejects_over_budget_and_invalid_costs() {
        struct TenJouleBudget;
        impl AffordabilityPolicy for TenJouleBudget {
            fn policy_name(&self) -> &'static str {
                "test_ten_joule_budget"
            }

            fn can_afford(&self, estimated_cost: f64) -> bool {
                estimated_cost <= 10.0
            }
        }

        let gates = GovernanceGates::permissive();
        assert!(gates.evaluate_affordability(&TenJouleBudget, 10.0).allowed);
        let exceeded = gates.evaluate_affordability(&TenJouleBudget, 10.1);
        assert!(!exceeded.allowed);
        assert_eq!(exceeded.reason, "budget_exceeded");
        assert_eq!(
            gates
                .evaluate_affordability(&TenJouleBudget, f64::NAN)
                .reason,
            "invalid_estimated_cost"
        );
    }

    #[test]
    fn fallback_and_synthetic_totals_cannot_authorize_autonomy() {
        let gates = GovernanceGates::permissive();
        for source in [
            JouleWorkMeasurementSource::DefaultFallback,
            JouleWorkMeasurementSource::SyntheticRestoration,
        ] {
            let decision = gates.evaluate_autonomy_basis(source, 1.0);
            assert!(!decision.allowed);
            assert_eq!(decision.reason, "fallback_or_synthetic_measurement");
        }
        assert!(
            gates
                .evaluate_autonomy_basis(JouleWorkMeasurementSource::RuntimeTimer, 0.9)
                .allowed
        );
        assert_eq!(
            gates
                .evaluate_autonomy_basis(JouleWorkMeasurementSource::OperatorEstimate, 0.0)
                .reason,
            "insufficient_measurement_confidence"
        );
    }

    #[test]
    fn proactive_explanation_names_interruption_basis_and_budget_class() {
        let review = HumanFacingActionReview {
            schema_version: HumanFacingActionReview::SCHEMA_VERSION.to_string(),
            semantic: HumanFacingActionReview::SEMANTIC.to_string(),
            affected_parties: vec!["operator".to_string()],
            reversibility: ActionReversibility::Reversible,
            interruption_reason: Some("appointment starts in ten minutes".to_string()),
            consent_authority: ConsentAuthority::OperatorAuthored,
            uncertainty: 0.1,
            coercion_risk: CoercionRisk::Low,
        };
        let explanation = review.proactive_message_explanation(JouleWorkBudgetClass::Routine);
        assert!(explanation.contains("appointment starts in ten minutes"));
        assert!(explanation.contains("budget class: routine"));
    }

    #[test]
    fn empty_yaml_yields_permissive_defaults() {
        let g = GovernanceGates::load_from_str("").unwrap();
        for c in [
            DecisionClass::Dispatch,
            DecisionClass::Governance,
            DecisionClass::Budget,
            DecisionClass::Retire,
            DecisionClass::Bid,
        ] {
            let p = g.policy_for(c);
            assert!(!p.require_council);
            assert_eq!(p.council_joule_cost, 0.0);
            assert!(!p.block_on_triad_fail);
        }
    }

    #[test]
    fn exact_joule_boundary_is_within_budget() {
        struct ExactTenJouleBudget;
        impl AffordabilityPolicy for ExactTenJouleBudget {
            fn policy_name(&self) -> &'static str {
                "exact_ten_joule"
            }

            fn can_afford(&self, estimated_cost: f64) -> bool {
                estimated_cost <= 10.0
            }
        }

        let gates = GovernanceGates::permissive();
        let bounded = gates.evaluate_affordability(&ExactTenJouleBudget, 10.0);
        assert!(bounded.allowed);
        assert_eq!(bounded.reason, "within_budget");
    }

    #[test]
    fn block_on_fail_policy_mode_surfaces_for_budget_class() {
        let raw = r#"
default:
  policy_mode: record_and_proceed
classes:
  budget:
    policy_mode: block_on_fail
"#;
        let gates = GovernanceGates::load_from_str(raw).unwrap();
        let budget_policy = gates.policy_for(DecisionClass::Budget);
        assert!(budget_policy.blocks_on_triad_fail());
        assert_eq!(
            budget_policy.policy_mode(),
            GovernancePolicyMode::BlockOnFail
        );
    }

    #[test]
    fn action_class_override_overrides_class_default() {
        let raw = r#"
default:
  require_council: false
  council_joule_cost: 0.0
classes:
  dispatch:
    require_council: true
    council_joule_cost: 1.0
action_classes:
  read_only_audit:
    require_council: false
"#;
        let gates = GovernanceGates::load_from_str(raw).unwrap();
        let class_default = gates.policy_for(DecisionClass::Dispatch);
        let action_override = gates.policy_for_action_class("read_only_audit");
        assert!(class_default.require_council);
        assert_eq!(class_default.council_joule_cost, 1.0);
        assert!(!action_override.require_council);
        assert_eq!(action_override.council_joule_cost, 0.0);
    }

    #[test]
    fn non_json_payload_returns_parse_error() {
        let err = GovernanceGates::load_from_str("- not: [valid").unwrap_err();
        assert!(
            matches!(err, GovernanceGatesError::Parse(_)),
            "expected Parse error, got {err:?}"
        );
    }
}
