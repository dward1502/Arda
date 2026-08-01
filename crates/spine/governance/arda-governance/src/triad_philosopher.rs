// sigil: REPAIR
//! Triad Philosopher arbitration layer.
//!
//! This layer interprets conflicts between Love Dynamics, evidence discipline,
//! independence, sycophancy risk, defection pressure, and JouleWork. It is a
//! deterministic first pass, not a replacement for the existing Aurelius/Bacon/
//! Sun Tzu gates.

use arda_core::{Task, TaskStatus};
use serde::{Deserialize, Serialize};

use crate::{
    assess_empirical_distrust, assess_nonconformist_bee, JouleWorkProfile, LoveDynamicsScore,
    LoveDynamicsTrend, PhilosopherLifecycleReceipt, ResonanceComponents,
};

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
    #[serde(default)]
    pub lifecycle: PhilosopherLifecycleReceipt,
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
            lifecycle: PhilosopherLifecycleReceipt::default(),
        };
    }

    if normalized.defection_pressure >= 0.85 && normalized.empirical_grounding < 0.60 {
        return TriadPhilosopherVerdict {
            action: PhilosopherAction::Reject,
            reason: "high defection pressure without sufficient evidence grounding".to_string(),
            alignment_score,
            lifecycle: PhilosopherLifecycleReceipt::default(),
        };
    }

    if normalized.love_trend == LoveDynamicsTrend::Decaying {
        return TriadPhilosopherVerdict {
            action: PhilosopherAction::Revise,
            reason: "love dynamics are decaying: cooperation is losing to defection pressure"
                .to_string(),
            alignment_score,
            lifecycle: PhilosopherLifecycleReceipt::default(),
        };
    }

    if normalized.independence < 0.35 || normalized.sycophancy_risk > 0.65 {
        return TriadPhilosopherVerdict {
            action: PhilosopherAction::Revise,
            reason: "independence is weak or sycophancy risk is elevated".to_string(),
            alignment_score,
            lifecycle: PhilosopherLifecycleReceipt::default(),
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
                lifecycle: PhilosopherLifecycleReceipt::default(),
            };
        }

        return TriadPhilosopherVerdict {
            action: PhilosopherAction::Hold,
            reason:
                "JouleWork efficiency is low without enough alignment evidence to justify the cost"
                    .to_string(),
            alignment_score,
            lifecycle: PhilosopherLifecycleReceipt::default(),
        };
    }

    if normalized.empirical_grounding < 0.50 {
        return TriadPhilosopherVerdict {
            action: PhilosopherAction::Hold,
            reason: "evidence grounding is insufficient for confident action".to_string(),
            alignment_score,
            lifecycle: PhilosopherLifecycleReceipt::default(),
        };
    }

    TriadPhilosopherVerdict {
        action: PhilosopherAction::Proceed,
        reason: "alignment signals are coherent: evidence, independence, love dynamics, and JouleWork are acceptable".to_string(),
        alignment_score,
        lifecycle: PhilosopherLifecycleReceipt::default(),
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
    let empirical = assess_empirical_distrust(task);
    let bee = assess_nonconformist_bee(task);
    let empirical_grounding = empirical.empirical_grounding;
    let independence = bee.independence;
    let sycophancy_risk = bee.sycophancy_risk;
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
