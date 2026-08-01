//! Shared 0.0–1.0 normalization contract for policy internals.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UnitInterval(f64);

impl UnitInterval {
    pub fn new(value: f64) -> Self {
        Self(if value.is_finite() {
            value.clamp(0.0, 1.0)
        } else {
            0.0
        })
    }

    pub fn from_percent(value: f64) -> Self {
        Self::new(value / 100.0)
    }

    pub fn get(self) -> f64 {
        self.0
    }

    pub fn as_percent(self) -> f64 {
        self.0 * 100.0
    }
}

impl Default for UnitInterval {
    fn default() -> Self {
        Self(0.0)
    }
}

/// Accept unit-interval values and migrate legacy percentage values at the boundary.
pub fn normalize_legacy_unit_or_percent(value: f64) -> UnitInterval {
    if value > 1.0 {
        UnitInterval::from_percent(value)
    } else {
        UnitInterval::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_non_finite_and_out_of_range_values() {
        assert_eq!(UnitInterval::new(f64::NAN).get(), 0.0);
        assert_eq!(UnitInterval::new(-2.0).get(), 0.0);
        assert_eq!(UnitInterval::new(2.0).get(), 1.0);
    }

    #[test]
    fn migrates_legacy_percent_values() {
        assert_eq!(normalize_legacy_unit_or_percent(75.0).get(), 0.75);
        assert_eq!(normalize_legacy_unit_or_percent(0.75).get(), 0.75);
    }
}
