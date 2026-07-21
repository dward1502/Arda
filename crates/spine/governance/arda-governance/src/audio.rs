// sigil: ◈
//! Audio Governance Signal Source
//!
//! Captures ambient audio features and derives a governance signal:
//! - noise_floor: background noise level (dB)
//! - speech_ratio: fraction of time with human speech detected
//! - tone_valence: aggregate emotional valence of detected speech (-1 to 1)
//!
//! These feed into resonance scoring as an "environmental coherence" dimension:
//! quiet, focused environments score higher; chaotic/noisy environments score lower.

use serde::{Deserialize, Serialize};

/// Audio environment snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioGovernance {
    /// Timestamp of the measurement
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Background noise floor in dB (lower = quieter)
    pub noise_floor_db: f64,
    /// Fraction of window with speech (0-1)
    pub speech_ratio: f64,
    /// Aggregate tone valence (-1 negative, 0 neutral, 1 positive)
    pub tone_valence: f64,
    /// Classification
    pub environment: AudioEnvironment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AudioEnvironment {
    Quiet,
    Focused,
    Conversational,
    Noisy,
    Chaotic,
}

impl AudioGovernance {
    /// Classify the audio environment from raw measurements
    pub fn classify(noise_floor_db: f64, speech_ratio: f64) -> AudioEnvironment {
        if noise_floor_db > 70.0 || speech_ratio > 0.8 {
            AudioEnvironment::Chaotic
        } else if noise_floor_db > 55.0 || speech_ratio > 0.5 {
            AudioEnvironment::Noisy
        } else if speech_ratio > 0.15 {
            AudioEnvironment::Conversational
        } else if noise_floor_db > 35.0 {
            AudioEnvironment::Focused
        } else {
            AudioEnvironment::Quiet
        }
    }

    /// Compute an environmental coherence score (0-100).
    /// Quiet/focused environments are better for governance decisions.
    pub fn coherence_score(&self) -> f64 {
        let base = match self.environment {
            AudioEnvironment::Quiet => 95.0,
            AudioEnvironment::Focused => 85.0,
            AudioEnvironment::Conversational => 65.0,
            AudioEnvironment::Noisy => 40.0,
            AudioEnvironment::Chaotic => 15.0,
        };
        // Tone valence bonus/penalty: positive valence helps, negative hurts
        let valence_adj = self.tone_valence * 10.0;
        (base + valence_adj).clamp(0.0, 100.0)
    }
}

impl Default for AudioGovernance {
    fn default() -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            noise_floor_db: 30.0,
            speech_ratio: 0.0,
            tone_valence: 0.0,
            environment: AudioEnvironment::Quiet,
        }
    }
}

/// Build an AudioGovernance from raw measurements
pub fn capture_audio_governance(
    noise_floor_db: f64,
    speech_ratio: f64,
    tone_valence: f64,
) -> AudioGovernance {
    let environment = AudioGovernance::classify(noise_floor_db, speech_ratio);
    AudioGovernance {
        timestamp: chrono::Utc::now(),
        noise_floor_db,
        speech_ratio,
        tone_valence,
        environment,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_environment_scores_high() {
        let audio = capture_audio_governance(25.0, 0.0, 0.3);
        assert_eq!(audio.environment, AudioEnvironment::Quiet);
        assert!(audio.coherence_score() > 90.0);
    }

    #[test]
    fn chaotic_environment_scores_low() {
        let audio = capture_audio_governance(80.0, 0.9, -0.5);
        assert_eq!(audio.environment, AudioEnvironment::Chaotic);
        assert!(audio.coherence_score() < 25.0);
    }

    #[test]
    fn negative_valence_penalizes() {
        let positive = capture_audio_governance(30.0, 0.1, 0.8);
        let negative = capture_audio_governance(30.0, 0.1, -0.8);
        assert!(positive.coherence_score() > negative.coherence_score());
    }

    #[test]
    fn conversational_mid_range() {
        let audio = capture_audio_governance(40.0, 0.3, 0.0);
        assert_eq!(audio.environment, AudioEnvironment::Conversational);
        assert!(audio.coherence_score() > 50.0 && audio.coherence_score() < 80.0);
    }
}
