//! Shared retention scoring for canonical Vairë memory records.
//!
//! Domain selection belongs to callers through the named configuration table.
//! The scoring functions contain no scope-specific policy branches.

use arda_core::contract::MemoryRecord;
use chrono::{DateTime, Utc};

const RETRIEVAL_SATURATION_COUNT: f64 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RetentionScore {
    pub recency: f64,
    pub importance: f64,
    pub retrieval_freq: f64,
    pub composite: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RetentionConfig {
    pub half_life_hours: f64,
    pub importance_weight: f64,
    pub recency_weight: f64,
    pub retrieval_weight: f64,
    pub floor: f64,
}

impl RetentionConfig {
    pub const fn new(
        half_life_hours: f64,
        importance_weight: f64,
        recency_weight: f64,
        retrieval_weight: f64,
        floor: f64,
    ) -> Self {
        Self {
            half_life_hours,
            importance_weight,
            recency_weight,
            retrieval_weight,
            floor,
        }
    }
}

/// Importance-dominated business retention (30-day half-life).
pub const BUSINESS_RETENTION: RetentionConfig = RetentionConfig::new(720.0, 0.60, 0.25, 0.15, 0.15);
/// Confirmed personal-memory retention (14-day half-life).
pub const PERSONAL_RETENTION: RetentionConfig = RetentionConfig::new(336.0, 0.50, 0.35, 0.15, 0.10);
/// Aggressive raw-system retention (one-day half-life).
pub const SYSTEM_RAW_RETENTION: RetentionConfig =
    RetentionConfig::new(24.0, 0.30, 0.50, 0.20, 0.25);
/// Promoted system fault signatures use the longer business half-life.
pub const SYSTEM_PROMOTED_RETENTION: RetentionConfig =
    RetentionConfig::new(720.0, 0.30, 0.50, 0.20, 0.25);

/// Shared exponential decay primitive used by retention and persona mood.
pub fn exponential_decay_weight(age_hours: f64, half_life_hours: f64) -> f64 {
    if !age_hours.is_finite() || !half_life_hours.is_finite() || half_life_hours <= 0.0 {
        return 0.0;
    }
    let age_hours = age_hours.max(0.0);
    (-std::f64::consts::LN_2 * age_hours / half_life_hours).exp()
}

/// Score a canonical record using caller-selected policy and retrieval evidence.
///
/// `recency_override` supports explicit policy decisions such as locking an
/// operator-authored, confirmed personal fact at full recency. Retrieval count
/// saturates at ten accesses and decays from the most recent retrieval time.
pub fn score_record(
    record: &MemoryRecord,
    config: RetentionConfig,
    as_of: DateTime<Utc>,
    retrieval_count: u64,
    last_retrieved_at: Option<DateTime<Utc>>,
    recency_override: Option<f64>,
) -> RetentionScore {
    let age_hours = hours_between(record.last_seen_at, as_of);
    let recency = recency_override
        .map(|value| value.clamp(0.0, 1.0))
        .unwrap_or_else(|| exponential_decay_weight(age_hours, config.half_life_hours));
    let importance = record.salience.clamp(0.0, 1.0);
    let retrieval_recency = last_retrieved_at
        .map(|retrieved_at| {
            exponential_decay_weight(hours_between(retrieved_at, as_of), config.half_life_hours)
        })
        .unwrap_or(0.0);
    let retrieval_freq =
        (retrieval_count as f64 / RETRIEVAL_SATURATION_COUNT).min(1.0) * retrieval_recency;
    let composite = config.importance_weight * importance
        + config.recency_weight * recency
        + config.retrieval_weight * retrieval_freq;

    RetentionScore {
        recency,
        importance,
        retrieval_freq,
        composite,
    }
}

pub fn is_forget_eligible(score: RetentionScore, config: RetentionConfig) -> bool {
    score.composite < config.floor
}

fn hours_between(earlier: DateTime<Utc>, later: DateTime<Utc>) -> f64 {
    (later - earlier).num_seconds().max(0) as f64 / 3600.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::contract::{MemoryKind, MemoryRecord};
    use chrono::{Duration, TimeZone};

    fn as_of() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 5, 0, 0, 0).unwrap()
    }

    fn record(age_hours: i64, salience: f64) -> MemoryRecord {
        let mut record = MemoryRecord::new("memory", MemoryKind::Episodic, "test", "memory");
        record.last_seen_at = as_of() - Duration::hours(age_hours);
        record.salience = salience;
        record
    }

    #[test]
    fn configurations_have_normalized_weights() {
        for config in [
            BUSINESS_RETENTION,
            PERSONAL_RETENTION,
            SYSTEM_RAW_RETENTION,
            SYSTEM_PROMOTED_RETENTION,
        ] {
            let sum = config.importance_weight + config.recency_weight + config.retrieval_weight;
            assert!((sum - 1.0).abs() < f64::EPSILON);
        }
    }

    #[test]
    fn operator_authored_personal_record_stays_above_floor_indefinitely() {
        let score = score_record(
            &record(24 * 365 * 100, 0.0),
            PERSONAL_RETENTION,
            as_of(),
            0,
            None,
            Some(1.0),
        );

        assert_eq!(score.recency, 1.0);
        assert!(!is_forget_eligible(score, PERSONAL_RETENTION));
    }

    #[test]
    fn low_importance_raw_system_record_is_forget_eligible_after_48_hours() {
        let score = score_record(
            &record(48, 0.1),
            SYSTEM_RAW_RETENTION,
            as_of(),
            0,
            None,
            None,
        );

        assert!(score.composite < SYSTEM_RAW_RETENTION.floor);
        assert!(is_forget_eligible(score, SYSTEM_RAW_RETENTION));
    }

    #[test]
    fn exponential_decay_reaches_half_at_the_configured_half_life() {
        assert!((exponential_decay_weight(24.0, 24.0) - 0.5).abs() < 1e-12);
    }
}
