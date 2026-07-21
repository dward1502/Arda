// sigil: REPAIR
// Bandit learning for adaptive route selection.
//
// This module owns the standalone epsilon-greedy bandit state for
// provider+model pairs. It is intentionally independent from the legacy
// `service::bandit` module so the adaptive subtree can evolve without
// pulling in service-layer I/O concerns.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BanditCandidateKey {
    pub provider_id: &'static str,
    pub model_id: &'static str,
}

impl BanditCandidateKey {
    pub const fn new(provider_id: &'static str, model_id: &'static str) -> Self {
        Self {
            provider_id,
            model_id,
        }
    }

    pub fn as_str(&self) -> String {
        format!("{}::{}", self.provider_id, self.model_id)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BanditArmStats {
    pub successes: u64,
    pub failures: u64,
    pub last_reward: f64,
}

impl BanditArmStats {
    pub fn observations(&self) -> u64 {
        self.successes + self.failures
    }

    pub fn success_rate(&self) -> f64 {
        let observations = self.observations();
        if observations == 0 {
            return 0.0;
        }
        self.successes as f64 / observations as f64
    }

    pub fn record_success(&mut self, reward: f64) {
        self.successes = self.successes.saturating_add(1);
        self.last_reward = reward;
    }

    pub fn record_failure(&mut self, reward: f64) {
        self.failures = self.failures.saturating_add(1);
        self.last_reward = reward;
    }
}

#[derive(Debug, Clone, Default)]
pub struct BanditState {
    pub arms: BTreeMap<String, BanditArmStats>,
    pub epsilon: f64,
    pub min_observations: u64,
}

impl BanditState {
    pub fn new(epsilon: f64, min_observations: u64) -> Self {
        Self {
            arms: BTreeMap::new(),
            epsilon: epsilon.clamp(0.0, 1.0),
            min_observations: min_observations.max(0),
        }
    }

    pub fn arm_mut(&mut self, key: &BanditCandidateKey) -> &mut BanditArmStats {
        let key_str = key.as_str();
        self.arms.entry(key_str).or_default()
    }

    pub fn arm(&self, key: &BanditCandidateKey) -> Option<&BanditArmStats> {
        self.arms.get(&key.as_str())
    }

    pub fn record_success(&mut self, key: &BanditCandidateKey, reward: f64) {
        self.arm_mut(key).record_success(reward);
    }

    pub fn record_failure(&mut self, key: &BanditCandidateKey, reward: f64) {
        self.arm_mut(key).record_failure(reward);
    }

    pub fn select_arm(
        &self,
        candidates: &[BanditCandidateKey],
        exploration_bonus: impl Fn(&BanditArmStats) -> f64,
    ) -> Option<BanditCandidateKey> {
        if candidates.is_empty() {
            return None;
        }

        let mut best: Option<(BanditCandidateKey, f64)> = None;
        for candidate in candidates {
            let key_str = candidate.as_str();
            let stats = self.arms.get(&key_str).copied().unwrap_or_default();
            let observations = stats.observations();

            let reward = if observations < self.min_observations {
                stats.success_rate() + exploration_bonus(&stats)
            } else if rand_float(key_str, "explore") < self.epsilon {
                stats.success_rate() + exploration_bonus(&stats)
            } else {
                stats.success_rate()
            };

            match best {
                Some((_, current_best)) if current_best >= reward => {}
                _ => {
                    best = Some((*candidate, reward));
                }
            }
        }

        best.map(|(key, _)| key)
    }
}

fn rand_float(seed: &str, suffix: &str) -> f64 {
    let combined = format!("{seed}::{suffix}");
    let mut hash: u64 = 0;
    for byte in combined.as_bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(*byte as u64);
    }
    (hash % 1_000_000) as f64 / 1_000_000.0
}
