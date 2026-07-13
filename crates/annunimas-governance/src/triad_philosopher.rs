// sigil: REPAIR
//! Triad Philosopher arbitration layer.
//!
//! This layer interprets conflicts between Love Dynamics, evidence discipline,
//! independence, sycophancy risk, defection pressure, and JouleWork. It is a
//! deterministic first pass, not a replacement for the existing Aurelius/Bacon/
//! Sun Tzu gates.

use annunimas_core::{Task, TaskStatus};
use serde::{Deserialize, Serialize};

use crate::{JouleWorkProfile, LoveDynamicsScore, LoveDynamicsTrend, ResonanceComponents};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AlignmentSignals {
    pub love_trend: LoveDynamicsTrend,
    pub projected_empathy: f64,
    pub empirical_grounding: f64,
    pub independence: f64,
    pub sycophancy_risk: f64,
    pub joule_honesty: f64,
    pub joule_efficiency: f64,
    pub defection_pressure: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhilosopherAction {
    Proceed,
    Revise,
    Hold,
    Reject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriadPhilosopherVerdict {
    pub action: PhilosopherAction,
    pub reason: String,
    pub alignment_score: f64,
}

pub fn interpret_alignment(signals: AlignmentSignals) -> TriadPhilosopherVerdict {
    let normalized = signals.normalized();
    let alignment_score = alignment_score(&normalized);

    if normalized.empirical_grounding < 0.50
        && (normalized.sycophancy_risk >= 0.70
            || normalized.love_trend == LoveDynamicsTrend::Decaying)
    {
        return TriadPhilosopherVerdict {
            action: PhilosopherAction::Hold,
            reason:
                "low evidence and high sycophancy risk: cooperation may be obedience, not alignment"
                    .to_string(),
            alignment_score,
        };
    }

    if normalized.defection_pressure >= 0.85 && normalized.empirical_grounding < 0.60 {
        return TriadPhilosopherVerdict {
            action: PhilosopherAction::Reject,
            reason: "high defection pressure without sufficient evidence grounding".to_string(),
            alignment_score,
        };
    }

    if normalized.love_trend == LoveDynamicsTrend::Decaying {
        return TriadPhilosopherVerdict {
            action: PhilosopherAction::Revise,
            reason: "love dynamics are decaying: cooperation is losing to defection pressure"
                .to_string(),
            alignment_score,
        };
    }

    if normalized.independence < 0.35 || normalized.sycophancy_risk > 0.65 {
        return TriadPhilosopherVerdict {
            action: PhilosopherAction::Revise,
            reason: "independence is weak or sycophancy risk is elevated".to_string(),
            alignment_score,
        };
    }

    if normalized.joule_efficiency < 0.35 {
        if normalized.empirical_grounding >= 0.75
            && normalized.love_trend == LoveDynamicsTrend::Growing
            && normalized.projected_empathy >= 0.70
        {
            return TriadPhilosopherVerdict {
                action: PhilosopherAction::Proceed,
                reason: "costly but justified: evidence is strong and love dynamics are growing"
                    .to_string(),
                alignment_score,
            };
        }

        return TriadPhilosopherVerdict {
            action: PhilosopherAction::Hold,
            reason:
                "JouleWork efficiency is low without enough alignment evidence to justify the cost"
                    .to_string(),
            alignment_score,
        };
    }

    if normalized.empirical_grounding < 0.50 {
        return TriadPhilosopherVerdict {
            action: PhilosopherAction::Hold,
            reason: "evidence grounding is insufficient for confident action".to_string(),
            alignment_score,
        };
    }

    TriadPhilosopherVerdict {
        action: PhilosopherAction::Proceed,
        reason: "alignment signals are coherent: evidence, independence, love dynamics, and JouleWork are acceptable".to_string(),
        alignment_score,
    }
}

/// Derive deterministic arbitration signals from the existing task, Love
/// Dynamics, JouleWork, and resonance metadata surfaces.
///
/// This is intentionally heuristic and side-effect free: it translates the
/// currently available governance facts into the normalized fields the Triad
/// Philosopher already understands without requiring new callers or LLM calls.
pub fn derive_alignment_signals(
    task: &Task,
    love: &LoveDynamicsScore,
    joule: &JouleWorkProfile,
    components: &ResonanceComponents,
) -> AlignmentSignals {
    let empirical_grounding = empirical_grounding(task);
    let independence = independence(task);
    let sycophancy_risk = sycophancy_risk(task);
    let joule_efficiency = if joule.efficient {
        1.0 - joule.variance.min(1.0)
    } else {
        (1.0 - joule.variance.min(1.0)) * 0.50
    };
    let status_defection = match task.status {
        TaskStatus::Failed { .. } => 0.85,
        TaskStatus::Retry { .. } => 0.55,
        TaskStatus::Pending => 0.35,
        TaskStatus::Running => 0.25,
        TaskStatus::Complete => 0.10,
    };
    let low_resonance_pressure = 1.0 - (components.phi_harmonic / 100.0);

    AlignmentSignals {
        love_trend: love.trend,
        projected_empathy: love.projected_empathy,
        empirical_grounding,
        independence,
        sycophancy_risk,
        joule_honesty: joule.honesty_ratio,
        joule_efficiency,
        defection_pressure: (love.defection * 0.50
            + status_defection * 0.30
            + low_resonance_pressure * 0.20)
            .clamp(0.0, 1.0),
    }
}

impl AlignmentSignals {
    fn normalized(self) -> Self {
        Self {
            love_trend: self.love_trend,
            projected_empathy: unit_interval(self.projected_empathy),
            empirical_grounding: unit_interval(self.empirical_grounding),
            independence: unit_interval(self.independence),
            sycophancy_risk: unit_interval(self.sycophancy_risk),
            joule_honesty: unit_interval(self.joule_honesty),
            joule_efficiency: unit_interval(self.joule_efficiency),
            defection_pressure: unit_interval(self.defection_pressure),
        }
    }
}

fn alignment_score(signals: &AlignmentSignals) -> f64 {
    let trend_score = match signals.love_trend {
        LoveDynamicsTrend::Growing => 1.0,
        LoveDynamicsTrend::Stable => 0.55,
        LoveDynamicsTrend::Decaying => 0.0,
    };

    let cooperative = signals.projected_empathy * 0.25
        + signals.empirical_grounding * 0.25
        + signals.independence * 0.20
        + signals.joule_honesty * 0.10
        + signals.joule_efficiency * 0.05
        + trend_score * 0.15;
    let risk = signals.sycophancy_risk * 0.50 + signals.defection_pressure * 0.50;

    (cooperative * (1.0 - (risk * 0.35))).clamp(0.0, 1.0)
}

fn unit_interval(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn empirical_grounding(task: &Task) -> f64 {
    let mut score: f64 = 0.20;

    if task.plan_id.is_some() {
        score += 0.10;
    }

    if let Some(result) = &task.result {
        if has_signal_key(result, &["evidence", "proof", "verification", "verified"]) {
            score += 0.30;
        }
        if has_signal_key(result, &["provenance", "source", "sources", "path", "url"]) {
            score += 0.25;
        }
        if has_signal_key(result, &["recommendation", "rationale", "reason"])
            || task.description.to_lowercase().contains("evidence")
        {
            score += 0.10;
        }
    }

    score.clamp(0.0, 1.0)
}

fn independence(task: &Task) -> f64 {
    let text = task.description.to_lowercase();
    let mut score: f64 = if task.assigned_agent.is_some() {
        0.70
    } else {
        0.55
    };

    if contains_any(
        &text,
        &[
            "independent",
            "audit",
            "review",
            "verify",
            "critique",
            "challenge",
        ],
    ) {
        score += 0.20;
    }
    if contains_any(&text, &["rubber stamp", "just approve", "without evidence"]) {
        score -= 0.35;
    }

    score.clamp(0.0, 1.0)
}

fn sycophancy_risk(task: &Task) -> f64 {
    let text = task.description.to_lowercase();
    let mut risk: f64 = 0.15;

    if contains_any(
        &text,
        &[
            "rubber stamp",
            "just agree",
            "just approve",
            "approve without evidence",
            "without evidence",
        ],
    ) {
        risk += 0.65;
    }
    if task.result.is_some() && !task_has_evidence(task) {
        risk += 0.15;
    }

    risk.clamp(0.0, 1.0)
}

fn task_has_evidence(task: &Task) -> bool {
    task.result
        .as_ref()
        .map(|result| has_signal_key(result, &["evidence", "proof", "verification", "provenance"]))
        .unwrap_or(false)
}

fn has_signal_key(value: &serde_json::Value, keys: &[&str]) -> bool {
    match value {
        serde_json::Value::Object(map) => map.iter().any(|(key, child)| {
            keys.iter().any(|needle| key.eq_ignore_ascii_case(needle))
                || has_signal_key(child, keys)
        }),
        serde_json::Value::Array(items) => items.iter().any(|item| has_signal_key(item, keys)),
        _ => false,
    }
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decaying_love_requests_revision() {
        let verdict = interpret_alignment(AlignmentSignals {
            love_trend: LoveDynamicsTrend::Decaying,
            projected_empathy: 0.4,
            empirical_grounding: 0.8,
            independence: 0.8,
            sycophancy_risk: 0.1,
            joule_honesty: 0.9,
            joule_efficiency: 0.9,
            defection_pressure: 0.7,
        });

        assert_eq!(verdict.action, PhilosopherAction::Revise);
    }
}
