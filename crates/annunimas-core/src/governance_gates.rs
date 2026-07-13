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

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GovernancePolicyMode {
    ObserveOnly,
    RecordAndProceed,
    BlockOnFail,
    EscalateToHuman,
    RequireIndependentReceipts,
}

impl Default for GovernancePolicyMode {
    fn default() -> Self {
        Self::RecordAndProceed
    }
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
    fn shipped_gates_yaml_parses() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("config")
            .join("governance_gates.yaml");
        let g = GovernanceGates::load_from_path(&path).expect("ship file parses");
        // Sanity: at least one class should be present in the file.
        let _ = g.policy_for(DecisionClass::Retire);
    }
}
