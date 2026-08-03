//! Persona identity types for arda-vaire.
//!
//! These types form the personality identity layer. Traits are grown from
//! evidence — they are never written from single events. Mood is derived
//! from weighted valence samples and decays over time.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Explicit, record-local evidence that may contribute to a trait.
///
/// Producers write a list of these under `persona.trait_evidence`. Derivation
/// never turns arbitrary tags or prose into identity claims.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersonaTraitEvidence {
    pub id: String,
    pub label: String,
}

/// A named trait that has been promoted from evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersonaTrait {
    pub trait_id: String,
    pub label: String,
    pub evidence_count: usize,
    pub confidence: f64,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub last_reinforced_by: Option<String>,
    pub stale: bool,
}

/// A single mood observation with valence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MoodSample {
    pub timestamp: DateTime<Utc>,
    /// Valence in the range [-1.0, 1.0].
    pub valence: f32,
    pub source_record: String,
    pub outcome_class: String,
}

/// A cached summary of mood over a sliding window.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MoodSummary {
    pub as_of: DateTime<Utc>,
    pub weighted_valence: f64,
    pub sample_count: usize,
    pub window_hours: i64,
}

/// Evidence for a single value/trait.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValueEvidence {
    pub value_id: String,
    pub evidence_count: usize,
    pub source_records: Vec<String>,
}

/// The persona projection emitted to downstream consumers (HUD, visual bridge).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PersonaProjection {
    pub traits: Vec<PersonaTrait>,
    pub mood: Vec<MoodSample>,
    pub mood_summary: Option<MoodSummary>,
    pub value_evidence: Vec<ValueEvidence>,
    pub derived_at: DateTime<Utc>,
}

/// The result of a promotion check — either the trait is promoted or not.
#[derive(Debug, Clone, PartialEq)]
pub enum PromotionResult {
    Promoted(PersonaTrait),
    InsufficientEvidence {
        trait_id: String,
        evidence_count: usize,
        required: usize,
    },
}

/// Promotion rule: a trait requires at least 3 independent evidence records
/// inside a 30-day window before it is written at all.
///
/// confidence = min(1.0, evidence_count / 10.0)
/// 60 days without reinforcement → stale: true
pub const PROMOTION_EVIDENCE_THRESHOLD: usize = 3;
pub const PROMOTION_WINDOW_HOURS: i64 = 30 * 24; // 30 days
pub const STALE_THRESHOLD_DAYS: i64 = 60;

/// Mood decay half-life in hours (24h).
pub const MOOD_DECAY_LAMBDA: f64 = std::f64::consts::LN_2 / 24.0;

/// Mood window: last 200 samples or 14 days (whichever smaller).
pub const MOOD_MAX_SAMPLES: usize = 200;
pub const MOOD_MAX_WINDOW_HOURS: i64 = 14 * 24;

/// Check whether a trait meets the promotion threshold.
pub fn meets_promotion_threshold(evidence_count: usize) -> bool {
    evidence_count >= PROMOTION_EVIDENCE_THRESHOLD
}

/// Compute confidence for a trait given its evidence count.
pub fn trait_confidence(evidence_count: usize) -> f64 {
    (evidence_count as f64 / 10.0).min(1.0)
}

/// Check whether a trait is stale (60 days without reinforcement).
pub fn is_stale(last_reinforced: &DateTime<Utc>, as_of: &DateTime<Utc>) -> bool {
    let days_since = (as_of.timestamp() - last_reinforced.timestamp()) / 86400;
    days_since >= STALE_THRESHOLD_DAYS
}

/// Compute the exponential decay weight for a sample given its age in hours.
pub fn mood_decay_weight(age_hours: f64) -> f64 {
    (-MOOD_DECAY_LAMBDA * age_hours).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(dt: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(dt)
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn promotion_threshold_requires_three_evidence() {
        assert!(!meets_promotion_threshold(0));
        assert!(!meets_promotion_threshold(1));
        assert!(!meets_promotion_threshold(2));
        assert!(meets_promotion_threshold(3));
        assert!(meets_promotion_threshold(5));
    }

    #[test]
    fn trait_confidence_caps_at_one() {
        assert_eq!(trait_confidence(0), 0.0);
        assert_eq!(trait_confidence(3), 0.3);
        assert_eq!(trait_confidence(5), 0.5);
        assert_eq!(trait_confidence(10), 1.0);
        assert_eq!(trait_confidence(20), 1.0);
    }

    #[test]
    fn stale_after_sixty_days() {
        let last = utc("2026-01-01T00:00:00Z");
        let as_of = utc("2026-03-02T00:00:00Z"); // 61 days
        assert!(is_stale(&last, &as_of));

        let as_of_59 = utc("2026-03-01T00:00:00Z"); // 59 days
        assert!(!is_stale(&last, &as_of_59));
    }

    #[test]
    fn mood_decay_weight_decreases_with_age() {
        let w0 = mood_decay_weight(0.0);
        let w1 = mood_decay_weight(1.0);
        let w24 = mood_decay_weight(24.0);
        assert!(w0 > w1);
        assert!(w1 > w24);
        // 24h half-life: weight at 24h should be ~0.5
        assert!((w24 - 0.5).abs() < 0.01);
    }

    #[test]
    fn mood_decay_weight_positive_for_recent() {
        let w = mood_decay_weight(1.0);
        assert!(w > 0.0 && w <= 1.0);
    }
}
