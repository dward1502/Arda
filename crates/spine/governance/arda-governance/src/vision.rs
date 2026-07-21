// sigil: ◈
//! Vision Governance Signal Source
//!
//! Derives a governance signal from the vision-LLM comparison pipeline.
//! Each iteration produces a `VisionSignal` capturing:
//! - match_score: how close the render is to the reference (0-1)
//! - convergence_rate: is the score improving across iterations?
//! - divergence_flags: elements that are getting worse
//!
//! These feed into resonance scoring as a "perceptual coherence" dimension.

use serde::{Deserialize, Serialize};

/// Vision-derived governance signal for a single iteration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionSignal {
    /// Iteration number (1-based)
    pub iteration: u32,
    /// Match score from vision-LLM comparison (0-1)
    pub match_score: f64,
    /// Score delta from previous iteration (positive = improving)
    pub score_delta: f64,
    /// Missing elements reported by vision-LLM
    pub missing: Vec<String>,
    /// Wrong elements reported by vision-LLM
    pub wrong: Vec<String>,
    /// Strengths reported by vision-LLM
    pub strengths: Vec<String>,
}

/// Aggregated vision governance across all iterations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionGovernance {
    /// All iteration signals
    pub signals: Vec<VisionSignal>,
    /// Overall convergence assessment
    pub convergence: VisionConvergence,
    /// Final perceptual coherence score (0-100)
    pub coherence_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VisionConvergence {
    /// Score is steadily improving
    Converging,
    /// Score is flat
    Stalled,
    /// Score is getting worse
    Diverging,
    /// Not enough data to assess
    InsufficientData,
}

impl VisionGovernance {
    /// Assess convergence from a series of signals
    pub fn assess(signals: Vec<VisionSignal>) -> Self {
        let convergence = Self::classify_convergence(&signals);
        let coherence_score = Self::compute_coherence(&signals, &convergence);
        Self {
            signals,
            convergence,
            coherence_score,
        }
    }

    fn classify_convergence(signals: &[VisionSignal]) -> VisionConvergence {
        if signals.len() < 2 {
            return VisionConvergence::InsufficientData;
        }
        let deltas: Vec<f64> = signals.iter().map(|s| s.score_delta).collect();
        let positive_deltas = deltas.iter().filter(|d| **d > 0.001).count();
        let negative_deltas = deltas.iter().filter(|d| **d < -0.001).count();

        if positive_deltas > negative_deltas * 2 {
            VisionConvergence::Converging
        } else if negative_deltas > positive_deltas {
            VisionConvergence::Diverging
        } else {
            VisionConvergence::Stalled
        }
    }

    fn compute_coherence(signals: &[VisionSignal], convergence: &VisionConvergence) -> f64 {
        let last_score = signals.last().map(|s| s.match_score).unwrap_or(0.5);
        let base = last_score * 100.0;
        let convergence_adj = match convergence {
            VisionConvergence::Converging => 10.0,
            VisionConvergence::Stalled => 0.0,
            VisionConvergence::Diverging => -15.0,
            VisionConvergence::InsufficientData => 0.0,
        };
        (base + convergence_adj).clamp(0.0, 100.0)
    }
}

impl Default for VisionGovernance {
    fn default() -> Self {
        Self {
            signals: vec![],
            convergence: VisionConvergence::InsufficientData,
            coherence_score: 50.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converging_signals_score_higher() {
        let signals = vec![
            VisionSignal {
                iteration: 1,
                match_score: 0.5,
                score_delta: 0.0,
                missing: vec!["handle".into()],
                wrong: vec![],
                strengths: vec![],
            },
            VisionSignal {
                iteration: 2,
                match_score: 0.7,
                score_delta: 0.2,
                missing: vec![],
                wrong: vec![],
                strengths: vec!["handle".into()],
            },
            VisionSignal {
                iteration: 3,
                match_score: 0.85,
                score_delta: 0.15,
                missing: vec![],
                wrong: vec![],
                strengths: vec!["handle".into(), "edges".into()],
            },
        ];
        let vg = VisionGovernance::assess(signals);
        assert_eq!(vg.convergence, VisionConvergence::Converging);
        assert!(vg.coherence_score > 90.0);
    }

    #[test]
    fn diverging_signals_penalized() {
        let signals = vec![
            VisionSignal {
                iteration: 1,
                match_score: 0.8,
                score_delta: 0.0,
                missing: vec![],
                wrong: vec![],
                strengths: vec![],
            },
            VisionSignal {
                iteration: 2,
                match_score: 0.6,
                score_delta: -0.2,
                missing: vec!["new artifact".into()],
                wrong: vec![],
                strengths: vec![],
            },
        ];
        let vg = VisionGovernance::assess(signals);
        assert_eq!(vg.convergence, VisionConvergence::Diverging);
        assert!(vg.coherence_score < 70.0);
    }

    #[test]
    fn insufficient_data_neutral() {
        let vg = VisionGovernance::assess(vec![]);
        assert_eq!(vg.convergence, VisionConvergence::InsufficientData);
        assert_eq!(vg.coherence_score, 50.0);
    }
}
