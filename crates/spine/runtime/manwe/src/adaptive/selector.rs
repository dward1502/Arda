// sigil: REPAIR
// Deterministic route selection.
// No shared mutability; pure read-only filtering + scoring + stable sort.

use std::cmp::Ordering;

use crate::adaptive::candidate::RouteCandidate;
use crate::adaptive::error::{AdaptiveError, Result};
use crate::adaptive::policy::RoutePolicy;
use crate::adaptive::score::DeterministicScorer;
use crate::adaptive::types::RequestKind;

#[derive(Debug, Clone)]
pub struct SelectionOutcome {
    pub candidate: RouteCandidate,
    pub score: f64,
    pub rejected: Vec<RejectedCandidate>,
}

#[derive(Debug, Clone)]
pub struct RejectedCandidate {
    pub candidate: RouteCandidate,
    pub reason: String,
}

pub struct DeterministicSelector;

impl DeterministicSelector {
    pub fn select(
        &self,
        candidates: &[RouteCandidate],
        policy: &RoutePolicy,
        request_kind: RequestKind,
        scorer: &DeterministicScorer,
    ) -> Result<SelectionOutcome> {
        if candidates.is_empty() {
            return Err(AdaptiveError::NoEligibleRoute);
        }

        let mut scored: Vec<(RouteCandidate, f64)> = candidates
            .iter()
            .filter(|candidate| policy.validate_candidate(candidate).is_ok())
            .cloned()
            .map(|candidate| {
                let score = scorer
                    .score(&candidate, policy, request_kind, candidate.context_window)
                    .unwrap_or(0.0);
                (candidate, score)
            })
            .collect();

        if scored.is_empty() {
            return Err(AdaptiveError::NoEligibleRoute);
        }

        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.0.key().cmp(&b.0.key()))
        });

        let (chosen, score) = scored[0].clone();
        let rejected = scored
            .into_iter()
            .skip(1)
            .map(|(candidate, score)| RejectedCandidate {
                reason: format!("lower_score={:.2} vs {:.2}", score, score),
                candidate,
            })
            .collect();

        Ok(SelectionOutcome {
            candidate: chosen,
            score,
            rejected,
        })
    }
}
