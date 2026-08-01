#![cfg(feature = "full-cli")]
// sigil: REPAIR
//! Oracle governance gate — runs after `validator.validate()` and before
//! delegation. High-stakes plans (Critical priority or budget-heavy) get
//! a triad verdict from `OracleEngine`.

use super::decomposer::{Objective, PlannedTask, Priority};
use super::governance_policy::{TriadGateScore, TriadQuorumEvidence};
use arda_governance::{interpret_alignment, AlignmentSignals, LoveDynamicsTrend};
use arda_mandos::{GateResult, OracleEngine, OracleQuery, Verdict, VerdictOutcome};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GateDecision {
    /// No gate triggered — plan was below the high-stakes threshold.
    Skipped,
    /// Oracle returned Pass — plan may proceed.
    Approved { resonance: f64 },
    /// Oracle returned Conditional — plan may proceed, but conditions are noted.
    Conditional {
        resonance: f64,
        concerns: Vec<String>,
    },
    /// Oracle returned Fail — plan must escalate, do not delegate.
    Rejected {
        resonance: f64,
        concerns: Vec<String>,
    },
}

impl GateDecision {
    pub fn allows_delegation(&self) -> bool {
        matches!(
            self,
            Self::Skipped | Self::Approved { .. } | Self::Conditional { .. }
        )
    }
    pub fn requires_escalation(&self) -> bool {
        matches!(self, Self::Rejected { .. })
    }
}

#[derive(Debug, Clone)]
pub struct OracleGate {
    pub joule_threshold: f64,
}

impl Default for OracleGate {
    fn default() -> Self {
        Self {
            joule_threshold: 100.0,
        }
    }
}

impl OracleGate {
    pub fn evaluate_with_quorum_evidence(
        &self,
        objective: &Objective,
        plan: &[PlannedTask],
        required_quorum_ratio: f64,
        required_pass_rate: f64,
        force: bool,
    ) -> (GateDecision, Option<TriadQuorumEvidence>) {
        let total_joules: f64 = plan.iter().map(|t| t.joule_cost).sum();
        let has_critical = plan.iter().any(|t| t.priority == Priority::Critical);
        let triggered = force || has_critical || total_joules >= self.joule_threshold;
        if !triggered {
            return (GateDecision::Skipped, None);
        }

        let mut engine = OracleEngine::new();
        let task_summary = format!(
            "{} ({} tasks, {:.1} joules)",
            objective.statement,
            plan.len(),
            total_joules
        );
        let mut context: Vec<String> = objective.success_criteria.clone();
        context.extend(objective.constraints.iter().cloned());
        if let Some(d) = &objective.deadline {
            context.push(format!("deadline:{d}"));
        }
        for t in plan {
            context.push(format!(
                "step:{}/{:?}/{:.1}j",
                t.task_type, t.priority, t.joule_cost
            ));
        }

        let mut query = OracleQuery::new(
            format!("autopilot::{}", objective.id),
            task_summary,
            "ceo_autopilot",
        );
        query.context = context;
        let verdict = match engine.evaluate(query) {
            Ok(verdict) => verdict,
            Err(error) => {
                return (
                    GateDecision::Rejected {
                        resonance: 0.0,
                        concerns: vec![format!("Oracle query rejected: {error}")],
                    },
                    None,
                );
            }
        };
        let evidence =
            quorum_evidence_from_verdict(&verdict, required_quorum_ratio, required_pass_rate);
        (decision_from_verdict(verdict), Some(evidence))
    }

    pub fn evaluate(&self, objective: &Objective, plan: &[PlannedTask]) -> GateDecision {
        self.evaluate_with_quorum_evidence(objective, plan, 0.0, 0.0, false)
            .0
    }
}

fn quorum_evidence_from_verdict(
    verdict: &Verdict,
    required_quorum_ratio: f64,
    required_pass_rate: f64,
) -> TriadQuorumEvidence {
    let gate_scores = vec![
        gate_score("aurelius", &verdict.gates.aurelius),
        gate_score("bacon", &verdict.gates.bacon),
        gate_score("sun_tzu", &verdict.gates.sun_tzu),
    ];
    let total_gates = gate_scores.len();
    let passed_gates = gate_scores.iter().filter(|gate| gate.passed).count();
    let quorum_ratio = if total_gates == 0 {
        0.0
    } else {
        passed_gates as f64 / total_gates as f64
    };
    let mut concerns = Vec::new();
    for gate in [
        &verdict.gates.aurelius,
        &verdict.gates.bacon,
        &verdict.gates.sun_tzu,
    ] {
        concerns.extend(gate.concerns.iter().cloned());
    }

    TriadQuorumEvidence {
        source: "oracle_gate".into(),
        query_id: verdict.query_id.clone(),
        outcome: verdict.outcome.as_str().into(),
        resonance: verdict.resonance_score,
        passed_gates,
        total_gates,
        quorum_ratio,
        required_quorum_ratio,
        required_pass_rate,
        gate_scores,
        concerns,
        triad_philosopher: Some(interpret_alignment(AlignmentSignals {
            love_trend: love_trend_from_verdict(verdict),
            projected_empathy: verdict.resonance_score,
            empirical_grounding: quorum_ratio,
            independence: quorum_ratio,
            sycophancy_risk: (1.0 - verdict.gates.bacon.score).clamp(0.0, 1.0),
            joule_honesty: verdict.gates.aurelius.score.clamp(0.0, 1.0),
            joule_efficiency: verdict.gates.sun_tzu.score.clamp(0.0, 1.0),
            defection_pressure: (1.0 - verdict.resonance_score).clamp(0.0, 1.0),
        })),
    }
}

fn love_trend_from_verdict(verdict: &Verdict) -> LoveDynamicsTrend {
    match verdict.outcome {
        VerdictOutcome::Pass => LoveDynamicsTrend::Growing,
        VerdictOutcome::Conditional => LoveDynamicsTrend::Stable,
        VerdictOutcome::Fail | VerdictOutcome::Escalate => LoveDynamicsTrend::Decaying,
    }
}

fn gate_score(name: &str, result: &GateResult) -> TriadGateScore {
    TriadGateScore {
        gate: name.into(),
        passed: result.passed,
        score: result.score,
    }
}

fn decision_from_verdict(v: Verdict) -> GateDecision {
    let mut concerns = Vec::new();
    for g in [&v.gates.aurelius, &v.gates.bacon, &v.gates.sun_tzu] {
        for c in &g.concerns {
            concerns.push(c.clone());
        }
    }
    match v.outcome {
        VerdictOutcome::Pass => GateDecision::Approved {
            resonance: v.resonance_score,
        },
        VerdictOutcome::Conditional => GateDecision::Conditional {
            resonance: v.resonance_score,
            concerns,
        },
        VerdictOutcome::Fail | VerdictOutcome::Escalate => GateDecision::Rejected {
            resonance: v.resonance_score,
            concerns,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::super::decomposer::PlannedTask;
    use super::*;

    fn task(pri: Priority, j: f64) -> PlannedTask {
        PlannedTask {
            key: "k".into(),
            title: "t".into(),
            task_type: "ops".into(),
            depends_on: vec![],
            priority: pri,
            joule_cost: j,
            eta_seconds: 1,
            assigned_agent: Some("ceo".into()),
        }
    }

    #[test]
    fn low_stakes_plan_skips_gate() {
        let g = OracleGate {
            joule_threshold: 1000.0,
        };
        let obj = Objective {
            id: "o".into(),
            statement: "small".into(),
            constraints: vec![],
            deadline: None,
            success_criteria: vec![],
            tags: vec![],
        };
        let d = g.evaluate(&obj, &[task(Priority::Low, 1.0)]);
        assert!(matches!(d, GateDecision::Skipped));
        assert!(d.allows_delegation());
    }

    #[test]
    fn critical_plan_invokes_oracle() {
        let g = OracleGate::default();
        let obj = Objective {
            id: "o".into(),
            statement: "ship migration".into(),
            constraints: vec![],
            deadline: None,
            success_criteria: vec!["zero downtime".into()],
            tags: vec![],
        };
        let d = g.evaluate(&obj, &[task(Priority::Critical, 10.0)]);
        assert!(!matches!(d, GateDecision::Skipped));
    }

    #[test]
    fn quorum_evidence_surfaces_triad_philosopher_verdict() {
        let g = OracleGate::default();
        let obj = Objective {
            id: "o".into(),
            statement: "reroute provider traffic with independent evidence review".into(),
            constraints: vec!["verify evidence before delegation".into()],
            deadline: None,
            success_criteria: vec!["proof path documents source evidence".into()],
            tags: vec!["action_class:provider_reroute".into()],
        };

        let (_decision, evidence) = g.evaluate_with_quorum_evidence(
            &obj,
            &[task(Priority::Critical, 10.0)],
            0.66,
            0.45,
            true,
        );
        let philosopher = evidence
            .and_then(|quorum| quorum.triad_philosopher)
            .expect("oracle quorum evidence should carry Triad Philosopher verdict");

        assert!(matches!(
            philosopher.action,
            arda_governance::PhilosopherAction::Proceed
                | arda_governance::PhilosopherAction::Revise
                | arda_governance::PhilosopherAction::Hold
                | arda_governance::PhilosopherAction::Reject
        ));
        assert!(philosopher.alignment_score >= 0.0);
        assert!(philosopher.alignment_score <= 1.0);
        assert!(!philosopher.reason.is_empty());
    }
}
