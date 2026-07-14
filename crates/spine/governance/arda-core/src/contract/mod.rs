// sigil: ∇ ◈ ↝
//
// Agent state contract — see spec/agent-state-contract.md.
//
// v0.1 is intentionally minimal. Every persisted record carries
// `contract_version`; readers must reject mismatched majors.

pub mod decision;
pub mod goal;
pub mod ledger_entry;
pub mod memory;
pub mod plan;
pub mod reflection;

pub use decision::{Decision, DecisionClass, PhilosopherVerdict, TriadOutcome, TriadVerdict};
pub use goal::{Goal, GoalPriority, GoalStatus};
pub use ledger_entry::{LedgerEntry, LedgerKind};
pub use memory::{MemoryKind, MemoryRecord, MemoryState};
pub use plan::{Plan, PlanStatus, PlanStep};
pub use reflection::{Reflection, ReflectionOutcome};

pub use crate::task::{Task, TaskId, TaskStatus};

pub const CONTRACT_VERSION: &str = "0.1.0";

pub fn contract_version() -> String {
    CONTRACT_VERSION.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goal_roundtrips_with_version_stamped() {
        let g = Goal::new("goal_x", "X", "do x", "prometheus", GoalPriority::High);
        let json = serde_json::to_string(&g).unwrap();
        assert!(json.contains("\"contract_version\":\"0.1.0\""));
        let back: Goal = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "goal_x");
        assert_eq!(back.status, GoalStatus::Active);
    }

    #[test]
    fn decision_carries_triad_outcome() {
        let pass = PhilosopherVerdict {
            verdict: TriadVerdict::Pass,
            reason: None,
        };
        let triad = TriadOutcome {
            verdict: TriadVerdict::Pass,
            aurelius: pass.clone(),
            bacon: pass.clone(),
            sun_tzu: pass,
        };
        let d = Decision::new(
            "dec_1",
            DecisionClass::Dispatch,
            "tsk_1",
            "edge_core",
            "lowest joule estimate",
            triad,
        );
        let json = serde_json::to_value(&d).unwrap();
        assert_eq!(json["triad"]["verdict"], "pass");
        assert_eq!(json["decision_class"], "dispatch");
    }

    #[test]
    fn ledger_entry_envelopes_payload() {
        let payload = serde_json::json!({"hello": "world"});
        let e = LedgerEntry::new("led_1", "supervisor", LedgerKind::Other, payload);
        let s = serde_json::to_string(&e).unwrap();
        let back: LedgerEntry = serde_json::from_str(&s).unwrap();
        assert_eq!(back.kind, LedgerKind::Other);
        assert_eq!(back.payload["hello"], "world");
    }

    #[test]
    fn reflection_honesty_delta_is_actual_minus_estimated() {
        let mut r = Reflection::new("ref_1", "tsk_1", "plan_1", ReflectionOutcome::Success, 0.9);
        r.joule_estimated = 1.0;
        r.joule_actual = 0.7;
        assert!((r.honesty_delta() + 0.3).abs() < 1e-9);
    }

    #[test]
    fn extensions_roundtrip_via_serde_flatten() {
        let json = serde_json::json!({
            "contract_version": "0.1.0",
            "id": "goal_x",
            "title": "X",
            "intent": "do x",
            "owner_agent": "prometheus",
            "status": "active",
            "priority": "high",
            "created_at": "2026-05-06T00:00:00Z",
            "updated_at": "2026-05-06T00:00:00Z",
            "x_experimental_field": "value-from-future-reader"
        });
        let g: Goal = serde_json::from_value(json).unwrap();
        assert_eq!(
            g.extensions
                .get("x_experimental_field")
                .and_then(|v| v.as_str()),
            Some("value-from-future-reader")
        );
        let back = serde_json::to_value(&g).unwrap();
        assert_eq!(back["x_experimental_field"], "value-from-future-reader");
    }
}
