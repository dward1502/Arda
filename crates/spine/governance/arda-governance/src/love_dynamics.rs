// sigil: REPAIR
//! Dynamic Love Equation alignment scoring.
//!
//! Implements Brian Roemmele's differential framing:
//! `dE/dt = beta * (C - D) * E`, where empathy/cooperative alignment grows
//! when cooperative tendencies exceed defection pressure.
//! See `../GOVERNANCE_PROVENANCE.md` for the dated source, bounded Arda
//! adaptation, and copyright/permission boundary.

use serde::{Deserialize, Serialize};

use arda_core::governance_gates::{HumanFacingActionReview, HumanImpactReviewInput};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoveDynamicsTrend {
    Growing,
    Stable,
    Decaying,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LoveDynamicsInput {
    pub empathy: f64,
    pub cooperation: f64,
    pub defection: f64,
    pub beta: f64,
    pub delta_time: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LoveDynamicsScore {
    pub empathy: f64,
    pub cooperation: f64,
    pub defection: f64,
    pub beta: f64,
    pub delta_time: f64,
    pub delta_empathy: f64,
    pub projected_empathy: f64,
    pub trend: LoveDynamicsTrend,
}

pub fn evaluate_love_dynamics(input: LoveDynamicsInput) -> LoveDynamicsScore {
    let empathy = unit_interval(input.empathy);
    let cooperation = unit_interval(input.cooperation);
    let defection = unit_interval(input.defection);
    let beta = input.beta.max(0.0);
    let delta_time = input.delta_time.max(0.0);
    let delta_empathy = beta * (cooperation - defection) * empathy * delta_time;
    let projected_empathy = unit_interval(empathy + delta_empathy);
    let trend = classify_trend(delta_empathy);

    let score = LoveDynamicsScore {
        empathy,
        cooperation,
        defection,
        beta,
        delta_time,
        delta_empathy,
        projected_empathy,
        trend,
    };
    crate::global_governance_metrics().observe_love_dynamics(&score);
    score
}

/// Produce the canonical relational/human-impact contract shown to operators.
/// This remains a structured review rather than collapsing care, consent,
/// uncertainty, or coercion into the numeric Love Dynamics score.
pub fn evaluate_human_impact_review(input: HumanImpactReviewInput) -> HumanFacingActionReview {
    HumanFacingActionReview {
        schema_version: HumanFacingActionReview::SCHEMA_VERSION.to_string(),
        semantic: HumanFacingActionReview::SEMANTIC.to_string(),
        affected_parties: input.affected_parties,
        reversibility: input.reversibility,
        interruption_reason: input.interruption_reason,
        consent_authority: input.consent_authority,
        uncertainty: unit_interval(input.uncertainty),
        coercion_risk: input.coercion_risk,
    }
}

fn unit_interval(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn classify_trend(delta_empathy: f64) -> LoveDynamicsTrend {
    const EPSILON: f64 = 1.0e-9;
    if delta_empathy > EPSILON {
        LoveDynamicsTrend::Growing
    } else if delta_empathy < -EPSILON {
        LoveDynamicsTrend::Decaying
    } else {
        LoveDynamicsTrend::Stable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::governance_gates::{ActionReversibility, CoercionRisk, ConsentAuthority};

    #[test]
    fn non_finite_inputs_are_safely_clamped() {
        let score = evaluate_love_dynamics(LoveDynamicsInput {
            empathy: f64::NAN,
            cooperation: f64::INFINITY,
            defection: f64::NEG_INFINITY,
            beta: -1.0,
            delta_time: -3.0,
        });

        assert_eq!(score.empathy, 0.0);
        assert_eq!(score.cooperation, 0.0);
        assert_eq!(score.defection, 0.0);
        assert_eq!(score.beta, 0.0);
        assert_eq!(score.delta_time, 0.0);
        assert_eq!(score.trend, LoveDynamicsTrend::Stable);
    }

    #[test]
    fn human_impact_review_preserves_non_numeric_relational_fields() {
        let review = evaluate_human_impact_review(HumanImpactReviewInput {
            affected_parties: vec!["operator".to_string(), "child".to_string()],
            reversibility: ActionReversibility::Reversible,
            interruption_reason: Some("time-sensitive family appointment".to_string()),
            consent_authority: ConsentAuthority::OperatorAuthored,
            uncertainty: 1.4,
            coercion_risk: CoercionRisk::Low,
        });

        assert_eq!(review.semantic, HumanFacingActionReview::SEMANTIC);
        assert_eq!(review.affected_parties, ["operator", "child"]);
        assert_eq!(review.uncertainty, 1.0);
        assert_eq!(review.coercion_risk, CoercionRisk::Low);
    }
}
