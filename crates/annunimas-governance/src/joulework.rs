// sigil: REPAIR
//! JouleWork profiling shared across governance surfaces.

use annunimas_core::{JouleWorkMeasurementSource, Task};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JouleWorkProfile {
    pub estimated: f64,
    pub actual: f64,
    pub variance: f64,
    pub honesty_ratio: f64,
    pub measurement_source: JouleWorkMeasurementSource,
    pub measurement_confidence: f64,
    pub observed_measurement: bool,
    pub autonomy_truth_allowed: bool,
    pub efficient: bool,
}

pub fn profile_joulework(task: &Task) -> JouleWorkProfile {
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

    JouleWorkProfile {
        estimated,
        actual,
        variance,
        honesty_ratio,
        measurement_source: task.joulework_measurement_source,
        measurement_confidence: task.joulework_measurement_confidence.clamp(0.0, 1.0),
        observed_measurement: task.joulework_measurement_source.is_observed(),
        autonomy_truth_allowed: task.joulework_measurement_source.is_autonomy_truth(),
        efficient: variance <= 0.25,
    }
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
}
