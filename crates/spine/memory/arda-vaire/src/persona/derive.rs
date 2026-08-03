//! Pure persona derivation over canonical memory records.

use super::types::{
    is_stale, meets_promotion_threshold, mood_decay_weight, trait_confidence, MoodSample,
    MoodSummary, PersonaProjection, PersonaTrait, PersonaTraitEvidence, ValueEvidence,
    MOOD_MAX_SAMPLES, MOOD_MAX_WINDOW_HOURS, PROMOTION_WINDOW_HOURS,
};
use arda_core::contract::{MemoryRecord, MemoryState};
use chrono::{DateTime, Duration, Utc};
use std::collections::{BTreeMap, BTreeSet};

pub const TRAIT_EVIDENCE_KEY: &str = "persona.trait_evidence";
pub const MOOD_KEY: &str = "persona.mood";
pub const VALUE_EVIDENCE_KEY: &str = "persona.value_evidence";

/// Derive an identity projection from explicit persona evidence.
///
/// Only active/promoted records owned by `actor` participate. Trait evidence
/// is independent by memory-record id, and arbitrary record tags/content are
/// intentionally ignored.
pub fn derive_projection(
    actor: &str,
    records: &[MemoryRecord],
    as_of: DateTime<Utc>,
) -> PersonaProjection {
    let eligible = records.iter().filter(|record| {
        record.agent.eq_ignore_ascii_case(actor)
            && matches!(record.state, MemoryState::Active | MemoryState::Promoted)
    });

    let mut traits_by_id: BTreeMap<String, BTreeMap<String, TraitObservation>> = BTreeMap::new();
    let mut mood_by_source: BTreeMap<String, MoodSample> = BTreeMap::new();
    let mut values_by_id: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for record in eligible {
        if let Some(value) = record.extensions.get(TRAIT_EVIDENCE_KEY) {
            if let Ok(evidence) = serde_json::from_value::<Vec<PersonaTraitEvidence>>(value.clone())
            {
                for candidate in evidence {
                    if candidate.id.trim().is_empty() || candidate.label.trim().is_empty() {
                        continue;
                    }
                    traits_by_id.entry(candidate.id).or_default().insert(
                        record.id.clone(),
                        TraitObservation {
                            label: candidate.label,
                            seen_at: record.last_seen_at,
                        },
                    );
                }
            }
        }

        if let Some(value) = record.extensions.get(MOOD_KEY) {
            if let Ok(samples) = serde_json::from_value::<Vec<MoodSample>>(value.clone()) {
                for mut sample in samples {
                    if !sample.valence.is_finite() || !(-1.0..=1.0).contains(&sample.valence) {
                        continue;
                    }
                    if sample.source_record.is_empty() {
                        sample.source_record = record.id.clone();
                    }
                    mood_by_source.insert(sample.source_record.clone(), sample);
                }
            }
        }

        if let Some(value) = record.extensions.get(VALUE_EVIDENCE_KEY) {
            if let Ok(values) = serde_json::from_value::<Vec<ValueEvidence>>(value.clone()) {
                for value in values {
                    if value.value_id.trim().is_empty() {
                        continue;
                    }
                    let sources = values_by_id.entry(value.value_id).or_default();
                    if value.source_records.is_empty() {
                        sources.insert(record.id.clone());
                    } else {
                        sources.extend(value.source_records);
                    }
                }
            }
        }
    }

    let mut traits = Vec::new();
    for (trait_id, observations) in traits_by_id {
        let mut evidence = observations.into_iter().collect::<Vec<_>>();
        evidence.sort_by_key(|(_, observation)| observation.seen_at);
        if !has_promotion_window(&evidence) {
            continue;
        }

        let evidence_count = evidence.len();
        let Some((_, first)) = evidence.first() else {
            continue;
        };
        let Some((last_id, last)) = evidence.last() else {
            continue;
        };
        traits.push(PersonaTrait {
            trait_id,
            label: last.label.clone(),
            evidence_count,
            confidence: trait_confidence(evidence_count),
            first_seen: first.seen_at,
            last_seen: last.seen_at,
            last_reinforced_by: Some(last_id.clone()),
            stale: is_stale(&last.seen_at, &as_of),
        });
    }
    traits.sort_by(|left, right| left.trait_id.cmp(&right.trait_id));

    let window_start = as_of - Duration::hours(MOOD_MAX_WINDOW_HOURS);
    let mut mood = mood_by_source
        .into_values()
        .filter(|sample| sample.timestamp >= window_start && sample.timestamp <= as_of)
        .collect::<Vec<_>>();
    mood.sort_by_key(|sample| sample.timestamp);
    if mood.len() > MOOD_MAX_SAMPLES {
        mood.drain(..mood.len() - MOOD_MAX_SAMPLES);
    }
    let mood_summary = summarize_mood(&mood, as_of);

    let value_evidence = values_by_id
        .into_iter()
        .map(|(value_id, sources)| ValueEvidence {
            value_id,
            evidence_count: sources.len(),
            source_records: sources.into_iter().collect(),
        })
        .collect();

    PersonaProjection {
        traits,
        mood,
        mood_summary,
        value_evidence,
        derived_at: as_of,
    }
}

#[derive(Debug)]
struct TraitObservation {
    label: String,
    seen_at: DateTime<Utc>,
}

fn has_promotion_window(evidence: &[(String, TraitObservation)]) -> bool {
    if !meets_promotion_threshold(evidence.len()) {
        return false;
    }
    let mut start = 0usize;
    for end in 0..evidence.len() {
        while evidence[end].1.seen_at - evidence[start].1.seen_at
            > Duration::hours(PROMOTION_WINDOW_HOURS)
        {
            start += 1;
        }
        if end - start + 1 >= 3 {
            return true;
        }
    }
    false
}

fn summarize_mood(samples: &[MoodSample], as_of: DateTime<Utc>) -> Option<MoodSummary> {
    if samples.is_empty() {
        return None;
    }
    let mut weighted_sum = 0.0;
    let mut weight_sum = 0.0;
    for sample in samples {
        let age_hours = (as_of - sample.timestamp).num_seconds().max(0) as f64 / 3600.0;
        let weight = mood_decay_weight(age_hours);
        weighted_sum += f64::from(sample.valence) * weight;
        weight_sum += weight;
    }
    if weight_sum == 0.0 {
        return None;
    }
    Some(MoodSummary {
        as_of,
        weighted_valence: weighted_sum / weight_sum,
        sample_count: samples.len(),
        window_hours: MOOD_MAX_WINDOW_HOURS,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::contract::MemoryKind;
    use chrono::TimeZone;

    fn at(days: i64, hours: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 3, 3, 0, 0, 0).unwrap()
            - Duration::days(days)
            - Duration::hours(hours)
    }

    fn trait_record(id: &str, days_ago: i64, state: MemoryState) -> MemoryRecord {
        let mut record = MemoryRecord::new(id, MemoryKind::Episodic, "arandur", "evidence");
        record.created_at = at(days_ago, 0);
        record.last_seen_at = at(days_ago, 0);
        record.state = state;
        record.extensions.insert(
            TRAIT_EVIDENCE_KEY.to_owned(),
            serde_json::to_value(vec![PersonaTraitEvidence {
                id: "decisive".to_owned(),
                label: "Decisive".to_owned(),
            }])
            .unwrap(),
        );
        record
    }

    #[test]
    fn single_evidence_does_not_promote_trait() {
        let projection = derive_projection(
            "arandur",
            &[trait_record("one", 1, MemoryState::Active)],
            at(0, 0),
        );
        assert!(projection.traits.is_empty());
    }

    #[test]
    fn three_independent_records_promote_at_point_three_confidence() {
        let records = vec![
            trait_record("one", 3, MemoryState::Active),
            trait_record("two", 2, MemoryState::Promoted),
            trait_record("three", 1, MemoryState::Active),
        ];
        let projection = derive_projection("arandur", &records, at(0, 0));
        assert_eq!(projection.traits.len(), 1);
        assert_eq!(projection.traits[0].evidence_count, 3);
        assert!((projection.traits[0].confidence - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn promoted_trait_remains_visible_and_becomes_stale() {
        let records = vec![
            trait_record("one", 63, MemoryState::Active),
            trait_record("two", 62, MemoryState::Active),
            trait_record("three", 61, MemoryState::Active),
        ];
        let projection = derive_projection("arandur", &records, at(0, 0));
        assert_eq!(projection.traits.len(), 1);
        assert!(projection.traits[0].stale);
    }

    #[test]
    fn revoked_evidence_is_excluded() {
        let records = vec![
            trait_record("one", 3, MemoryState::Active),
            trait_record("two", 2, MemoryState::Active),
            trait_record("revoked", 1, MemoryState::Revoked),
        ];
        assert!(derive_projection("arandur", &records, at(0, 0))
            .traits
            .is_empty());
    }

    #[test]
    fn mood_uses_exponential_recency_weighting() {
        let mut record = MemoryRecord::new("mood", MemoryKind::Episodic, "arandur", "mood");
        record.extensions.insert(
            MOOD_KEY.to_owned(),
            serde_json::to_value(vec![
                MoodSample {
                    timestamp: at(0, 5),
                    valence: 1.0,
                    source_record: "recent".to_owned(),
                    outcome_class: "success".to_owned(),
                },
                MoodSample {
                    timestamp: at(5, 0),
                    valence: -1.0,
                    source_record: "old".to_owned(),
                    outcome_class: "error".to_owned(),
                },
            ])
            .unwrap(),
        );
        let projection = derive_projection("arandur", &[record], at(0, 0));
        let summary = projection.mood_summary.unwrap();
        let recent_weight = mood_decay_weight(5.0);
        let old_weight = mood_decay_weight(120.0);
        let expected = (recent_weight - old_weight) / (recent_weight + old_weight);
        assert!((summary.weighted_valence - expected).abs() < 1e-12);
        assert!(recent_weight / old_weight > 20.0);
    }

    #[test]
    fn empty_mood_window_is_none_and_derivation_is_idempotent() {
        let records = vec![
            trait_record("one", 3, MemoryState::Active),
            trait_record("two", 2, MemoryState::Active),
            trait_record("three", 1, MemoryState::Active),
        ];
        let first = derive_projection("arandur", &records, at(0, 0));
        let second = derive_projection("arandur", &records, at(0, 0));
        assert_eq!(first, second);
        assert!(first.mood_summary.is_none());
    }
}
