//! Typed, advisory environmental governance signals.
//!
//! Environmental context is evidence for operator awareness. It is never a
//! blocking governance gate and cannot independently approve or reject work.

use crate::solar::{solar_multiplier, SolarGeomagData};
use crate::{AudioGovernance, VisionGovernance};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::future::Future;

pub const ENVIRONMENTAL_POLICY_VERSION: &str = "environmental-coherence-v1";
pub const DEFAULT_FRESHNESS_WINDOW_SECS: i64 = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceSignalSource {
    Audio,
    Vision,
    Solar,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementQuality {
    Measured,
    Derived,
    Defaulted,
    #[default]
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalFreshness {
    Fresh,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SignalHealth {
    Healthy,
    Degraded { reason: String },
    Unavailable { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum GovernanceSignal {
    Audio(AudioGovernance),
    Vision(VisionGovernance),
    Solar(SolarGeomagData),
}

impl GovernanceSignal {
    pub fn source(&self) -> GovernanceSignalSource {
        match self {
            Self::Audio(_) => GovernanceSignalSource::Audio,
            Self::Vision(_) => GovernanceSignalSource::Vision,
            Self::Solar(_) => GovernanceSignalSource::Solar,
        }
    }

    pub fn coherence_score(&self) -> f64 {
        match self {
            Self::Audio(audio) => audio.coherence_score(),
            Self::Vision(vision) => vision.coherence_score,
            Self::Solar(solar) => solar_multiplier(solar) * 100.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceSignalEnvelope {
    pub source: GovernanceSignalSource,
    pub source_timestamp: Option<DateTime<Utc>>,
    pub collected_at: DateTime<Utc>,
    pub freshness: SignalFreshness,
    pub confidence: f64,
    pub measurement_quality: MeasurementQuality,
    pub health: SignalHealth,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal: Option<GovernanceSignal>,
}

impl GovernanceSignalEnvelope {
    pub fn measured(
        signal: GovernanceSignal,
        source_timestamp: Option<DateTime<Utc>>,
        collected_at: DateTime<Utc>,
        confidence: f64,
    ) -> Self {
        let freshness = classify_freshness(source_timestamp, collected_at);
        Self {
            source: signal.source(),
            source_timestamp,
            collected_at,
            freshness,
            confidence: normalize_confidence(confidence),
            measurement_quality: MeasurementQuality::Measured,
            health: SignalHealth::Healthy,
            signal: Some(signal),
        }
    }

    pub fn degraded(
        source: GovernanceSignalSource,
        reason: impl Into<String>,
        signal: Option<GovernanceSignal>,
        source_timestamp: Option<DateTime<Utc>>,
        collected_at: DateTime<Utc>,
        quality: MeasurementQuality,
        confidence: f64,
    ) -> Self {
        Self {
            source,
            source_timestamp,
            collected_at,
            freshness: classify_freshness(source_timestamp, collected_at),
            confidence: normalize_confidence(confidence),
            measurement_quality: quality,
            health: SignalHealth::Degraded {
                reason: reason.into(),
            },
            signal,
        }
    }

    pub fn unavailable(
        source: GovernanceSignalSource,
        reason: impl Into<String>,
        collected_at: DateTime<Utc>,
    ) -> Self {
        Self {
            source,
            source_timestamp: None,
            collected_at,
            freshness: SignalFreshness::Unknown,
            confidence: 0.0,
            measurement_quality: MeasurementQuality::Unavailable,
            health: SignalHealth::Unavailable {
                reason: reason.into(),
            },
            signal: None,
        }
    }

    pub fn coherence_score(&self) -> Option<f64> {
        if matches!(
            self.measurement_quality,
            MeasurementQuality::Defaulted | MeasurementQuality::Unavailable
        ) {
            return None;
        }
        self.signal.as_ref().map(GovernanceSignal::coherence_score)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentalAdvisory {
    Supportive,
    Neutral,
    Caution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentalCoherence {
    pub policy_version: String,
    pub advisory_only: bool,
    pub advisory: EnvironmentalAdvisory,
    pub score: f64,
    pub available_sources: usize,
    pub degraded_sources: usize,
    pub unavailable_sources: usize,
    pub rationale: String,
    pub signals: Vec<GovernanceSignalEnvelope>,
}

pub fn environmental_coherence(signals: Vec<GovernanceSignalEnvelope>) -> EnvironmentalCoherence {
    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;
    let mut available_sources = 0usize;
    let mut degraded_sources = 0usize;
    let mut unavailable_sources = 0usize;

    for envelope in &signals {
        match envelope.health {
            SignalHealth::Healthy => {}
            SignalHealth::Degraded { .. } => degraded_sources += 1,
            SignalHealth::Unavailable { .. } => unavailable_sources += 1,
        }
        if let Some(score) = envelope.coherence_score() {
            let freshness_weight = match envelope.freshness {
                SignalFreshness::Fresh => 1.0,
                SignalFreshness::Stale => 0.5,
                SignalFreshness::Unknown => 0.75,
            };
            let weight = envelope.confidence * freshness_weight;
            if weight > 0.0 {
                weighted_sum += score * weight;
                total_weight += weight;
                available_sources += 1;
            }
        }
    }

    let score = if total_weight > 0.0 {
        weighted_sum / total_weight
    } else {
        50.0
    };
    let every_usable_signal_is_fresh = signals
        .iter()
        .filter(|signal| signal.coherence_score().is_some())
        .all(|signal| signal.freshness == SignalFreshness::Fresh);
    let advisory = if available_sources == 0 {
        EnvironmentalAdvisory::Neutral
    } else if score <= 50.0 {
        EnvironmentalAdvisory::Caution
    } else if score >= 75.0
        && available_sources >= 2
        && degraded_sources == 0
        && unavailable_sources == 0
        && every_usable_signal_is_fresh
    {
        EnvironmentalAdvisory::Supportive
    } else {
        EnvironmentalAdvisory::Neutral
    };
    let rationale = format!(
        "advisory environmental context: {available_sources} usable, {degraded_sources} degraded, {unavailable_sources} unavailable; score {score:.1}; never used as an approval or rejection gate"
    );
    let result = EnvironmentalCoherence {
        policy_version: ENVIRONMENTAL_POLICY_VERSION.to_string(),
        advisory_only: true,
        advisory,
        score,
        available_sources,
        degraded_sources,
        unavailable_sources,
        rationale,
        signals,
    };
    crate::global_governance_metrics().observe_environmental(&result);
    result
}

/// Collect independent HUD signal producers concurrently under the shared
/// background-pressure gate, then evaluate their advisory composite.
pub async fn collect_environmental_signals<AF, VF>(
    solar_client: &crate::SolarClient,
    audio_fetch: AF,
    vision_fetch: VF,
) -> EnvironmentalCoherence
where
    AF: Future<Output = GovernanceSignalEnvelope>,
    VF: Future<Output = GovernanceSignalEnvelope>,
{
    let audio =
        arda_core::background::try_run_bounded_async("governance_environmental_hud", 3, || {
            audio_fetch
        });
    let vision =
        arda_core::background::try_run_bounded_async("governance_environmental_hud", 3, || {
            vision_fetch
        });
    let solar =
        arda_core::background::try_run_bounded_async("governance_environmental_hud", 3, || {
            solar_client.fetch_signal()
        });
    let (audio, vision, solar) = tokio::join!(audio, vision, solar);
    let now = Utc::now();
    environmental_coherence(vec![
        audio.unwrap_or_else(|| {
            GovernanceSignalEnvelope::unavailable(
                GovernanceSignalSource::Audio,
                "audio HUD fetch rejected by bounded async gate",
                now,
            )
        }),
        vision.unwrap_or_else(|| {
            GovernanceSignalEnvelope::unavailable(
                GovernanceSignalSource::Vision,
                "vision HUD fetch rejected by bounded async gate",
                now,
            )
        }),
        solar.unwrap_or_else(|| {
            GovernanceSignalEnvelope::unavailable(
                GovernanceSignalSource::Solar,
                "solar HUD fetch rejected by bounded async gate",
                now,
            )
        }),
    ])
}

pub fn audio_signal(
    audio: AudioGovernance,
    collected_at: DateTime<Utc>,
) -> GovernanceSignalEnvelope {
    let timestamp = audio.timestamp;
    GovernanceSignalEnvelope::measured(
        GovernanceSignal::Audio(audio),
        Some(timestamp),
        collected_at,
        0.9,
    )
}

pub fn vision_signal(
    vision: VisionGovernance,
    source_timestamp: Option<DateTime<Utc>>,
    collected_at: DateTime<Utc>,
) -> GovernanceSignalEnvelope {
    let confidence = if vision.signals.len() >= 2 { 0.85 } else { 0.5 };
    let quality = if vision.signals.is_empty() {
        MeasurementQuality::Defaulted
    } else {
        MeasurementQuality::Derived
    };
    if quality == MeasurementQuality::Defaulted {
        GovernanceSignalEnvelope::degraded(
            GovernanceSignalSource::Vision,
            "no vision iterations; neutral default excluded from composite score",
            Some(GovernanceSignal::Vision(vision)),
            source_timestamp,
            collected_at,
            quality,
            confidence,
        )
    } else {
        let mut envelope = GovernanceSignalEnvelope::measured(
            GovernanceSignal::Vision(vision),
            source_timestamp,
            collected_at,
            confidence,
        );
        envelope.measurement_quality = quality;
        envelope
    }
}

fn classify_freshness(
    source_timestamp: Option<DateTime<Utc>>,
    collected_at: DateTime<Utc>,
) -> SignalFreshness {
    match source_timestamp {
        Some(timestamp)
            if collected_at.signed_duration_since(timestamp)
                <= Duration::seconds(DEFAULT_FRESHNESS_WINDOW_SECS)
                && timestamp <= collected_at + Duration::seconds(5) =>
        {
            SignalFreshness::Fresh
        }
        Some(_) => SignalFreshness::Stale,
        None => SignalFreshness::Unknown,
    }
}

fn normalize_confidence(confidence: f64) -> f64 {
    if confidence.is_finite() {
        confidence.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture_audio_governance;

    #[test]
    fn unavailable_inputs_are_neutral_and_not_counted_as_measurements() {
        let result = environmental_coherence(vec![GovernanceSignalEnvelope::unavailable(
            GovernanceSignalSource::Solar,
            "upstream timeout",
            Utc::now(),
        )]);
        assert_eq!(result.advisory, EnvironmentalAdvisory::Neutral);
        assert_eq!(result.score, 50.0);
        assert_eq!(result.available_sources, 0);
        assert!(result.advisory_only);
    }

    #[test]
    fn low_quality_environment_is_advisory_caution_not_a_gate() {
        let now = Utc::now();
        let audio = capture_audio_governance(80.0, 0.9, -0.5);
        let result = environmental_coherence(vec![audio_signal(audio, now)]);
        assert_eq!(result.advisory, EnvironmentalAdvisory::Caution);
        assert!(result.advisory_only);
        assert!(result
            .rationale
            .contains("never used as an approval or rejection gate"));
    }

    #[test]
    fn supportive_requires_multiple_available_sources() {
        let now = Utc::now();
        let audio = capture_audio_governance(25.0, 0.0, 0.5);
        let single = environmental_coherence(vec![audio_signal(audio.clone(), now)]);
        assert_eq!(single.advisory, EnvironmentalAdvisory::Neutral);

        let vision = VisionGovernance::assess(vec![crate::VisionSignal {
            iteration: 1,
            match_score: 0.9,
            score_delta: 0.1,
            missing: vec![],
            wrong: vec![],
            strengths: vec!["aligned".to_string()],
        }]);
        let combined = environmental_coherence(vec![
            audio_signal(audio, now),
            vision_signal(vision, Some(now), now),
        ]);
        assert_eq!(combined.advisory, EnvironmentalAdvisory::Supportive);
    }

    fn solar_envelope(
        kp_index: f64,
        dst_index: f64,
        timestamp: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> GovernanceSignalEnvelope {
        GovernanceSignalEnvelope::measured(
            GovernanceSignal::Solar(SolarGeomagData {
                timestamp,
                kp_index,
                dst_index,
                bz: 0.0,
                solar_flux: 0.0,
                bz_quality: MeasurementQuality::Unavailable,
                solar_flux_quality: MeasurementQuality::Unavailable,
                activity_level: if kp_index >= 5.0 {
                    "storm".to_string()
                } else {
                    "quiet".to_string()
                },
            }),
            Some(timestamp),
            now,
            0.9,
        )
    }

    #[test]
    fn quiet_storm_stale_and_unavailable_have_bounded_advisory_semantics() {
        let now = Utc::now();
        let audio = capture_audio_governance(25.0, 0.0, 0.2);
        let quiet = environmental_coherence(vec![
            audio_signal(audio.clone(), now),
            solar_envelope(2.0, -10.0, now, now),
        ]);
        assert_eq!(quiet.advisory, EnvironmentalAdvisory::Supportive);

        let storm = environmental_coherence(vec![solar_envelope(7.0, -80.0, now, now)]);
        assert_eq!(storm.advisory, EnvironmentalAdvisory::Caution);

        let stale = environmental_coherence(vec![
            audio_signal(audio, now),
            solar_envelope(2.0, -10.0, now - Duration::minutes(30), now),
        ]);
        assert_eq!(stale.advisory, EnvironmentalAdvisory::Neutral);

        let unavailable = environmental_coherence(vec![GovernanceSignalEnvelope::unavailable(
            GovernanceSignalSource::Solar,
            "fixture unavailable",
            now,
        )]);
        assert_eq!(unavailable.advisory, EnvironmentalAdvisory::Neutral);
    }

    #[test]
    fn assessment_is_visible_in_live_in_process_telemetry() {
        let now = Utc::now();
        let _ = environmental_coherence(vec![GovernanceSignalEnvelope::unavailable(
            GovernanceSignalSource::Solar,
            "telemetry fixture",
            now,
        )]);
        let snapshot = crate::global_governance_metrics().snapshot();
        assert!(snapshot
            .histogram("arda_governance_environmental_coherence")
            .is_some());
        assert!(snapshot.counters.iter().any(|counter| {
            counter.name == "arda_governance_environmental_signals_total"
                && counter.labels.get("source").map(String::as_str) == Some("solar")
                && counter.labels.get("health").map(String::as_str) == Some("unavailable")
        }));
    }
}
