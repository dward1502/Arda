// sigil: REPAIR
//! Dynamic Love Equation alignment scoring.
//!
//! Implements Brian Roemmele's differential framing:
//! `dE/dt = beta * (C - D) * E`, where empathy/cooperative alignment grows
//! when cooperative tendencies exceed defection pressure.

use serde::{Deserialize, Serialize};

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

    LoveDynamicsScore {
        empathy,
        cooperation,
        defection,
        beta,
        delta_time,
        delta_empathy,
        projected_empathy,
        trend,
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
}
