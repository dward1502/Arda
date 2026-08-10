// sigil: REPAIR
//! JouleWork profiling shared across governance surfaces.
//!
//! This local diagnostic is inspired by, but does not implement, the upstream
//! JouleWork equation or wage/token model. See `../GOVERNANCE_PROVENANCE.md`
//! for the exact source, adaptation boundary, and terms review.

use arda_core::governance_gates::OperatorBurdenEstimate;
use arda_core::{JouleWorkMeasurementSource, Task};
use serde::{Deserialize, Serialize};

use crate::versions::{legacy_joulework_policy_version, JOULEWORK_POLICY_VERSION};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JouleWorkProfile {
    #[serde(default = "legacy_joulework_policy_version")]
    pub policy_version: String,
    pub estimated: f64,
    pub actual: f64,
    pub variance: f64,
    pub honesty_ratio: f64,
    pub measurement_source: JouleWorkMeasurementSource,
    pub measurement_confidence: f64,
    pub observed_measurement: bool,
    pub autonomy_truth_allowed: bool,
    pub efficient: bool,
    #[serde(default)]
    pub run_id: String,
    #[serde(default)]
    pub operator_burden: OperatorBurdenEstimate,
}

pub fn profile_joulework(task: &Task) -> JouleWorkProfile {
    profile_joulework_with_burden(task, OperatorBurdenEstimate::default())
}

pub fn profile_joulework_with_burden(
    task: &Task,
    mut operator_burden: OperatorBurdenEstimate,
) -> JouleWorkProfile {
    let estimated = task.joule_cost_estimated.max(0.0);
    let actual = task.joule_cost_actual.max(0.0);
    let variance = if estimated > 0.0 && actual > 0.0 {
        ((actual - estimated) / estimated).abs()
    } else {
        0.0
    };
    let honesty_ratio = if estimated > 0.0 && actual > 0.0 {
        (estimated / actual).min(actual / estimated)
    } else {
        1.0
    };

    operator_burden.confidence = if operator_burden.confidence.is_finite() {
        operator_burden.confidence.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let profile = JouleWorkProfile {
        policy_version: JOULEWORK_POLICY_VERSION.to_string(),
        estimated,
        actual,
        variance,
        honesty_ratio,
        measurement_source: task.joulework_measurement_source,
        measurement_confidence: task.joulework_measurement_confidence.clamp(0.0, 1.0),
        observed_measurement: task.joulework_measurement_source.is_observed(),
        autonomy_truth_allowed: task.joulework_measurement_source.is_autonomy_truth(),
        efficient: variance <= 0.25,
        run_id: task.id.to_string(),
        operator_burden,
    };
    crate::global_governance_metrics().observe_joule_honesty(&profile);
    profile
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joulework_profile_flags_large_mismatch() {
        let mut task = Task::new("route alert", "route");
        task.joule_cost_estimated = 4.0;
        task.joule_cost_actual = 9.0;

        let profile = profile_joulework(&task);
        assert!(profile.variance > 0.25);
        assert!(!profile.efficient);
    }

    #[test]
    fn joulework_profile_preserves_measurement_source_contract() {
        let mut task = Task::new("route observed usage", "route");
        task.joule_cost_estimated = 4.0;
        task.joule_cost_actual = 4.5;
        task.joulework_measurement_source = JouleWorkMeasurementSource::ProviderUsageReport;
        task.joulework_measurement_confidence = 1.4;

        let profile = profile_joulework(&task);
        assert_eq!(
            profile.measurement_source,
            JouleWorkMeasurementSource::ProviderUsageReport
        );
        assert_eq!(profile.measurement_confidence, 1.0);
        assert!(profile.observed_measurement);
        assert!(profile.autonomy_truth_allowed);
    }

    #[test]
    fn joulework_profile_marks_default_fallback_as_not_autonomy_truth() {
        let task = Task::new("route fallback usage", "route");

        let profile = profile_joulework(&task);
        assert_eq!(
            profile.measurement_source,
            JouleWorkMeasurementSource::DefaultFallback
        );
        assert_eq!(profile.measurement_confidence, 0.0);
        assert!(!profile.observed_measurement);
        assert!(!profile.autonomy_truth_allowed);
    }

    #[test]
    fn joulework_profile_keeps_operator_burden_explicitly_estimated() {
        let task = Task::new("prepare proactive reminder", "communicate");
        let profile = profile_joulework_with_burden(
            &task,
            OperatorBurdenEstimate {
                estimated_interruption_seconds: 20,
                estimated_recovery_seconds: 90,
                source: JouleWorkMeasurementSource::OperatorEstimate,
                confidence: 0.6,
            },
        );

        assert_eq!(profile.run_id, task.id.to_string());
        assert_eq!(profile.operator_burden.estimated_interruption_seconds, 20);
        assert_eq!(profile.operator_burden.estimated_recovery_seconds, 90);
        assert_eq!(
            profile.operator_burden.source,
            JouleWorkMeasurementSource::OperatorEstimate
        );
    }
}
