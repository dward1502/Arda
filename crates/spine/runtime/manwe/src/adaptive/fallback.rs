// sigil: REPAIR
// Fallback behavior for adaptive route execution.
//
// This module only owns the fallback classification/retry contract and
// bounded-attempt logic. It does not perform I/O itself; callers inspect the
// returned `FallbackDecision` and decide whether to retry the next candidate
// or fail the request.

use crate::adaptive::candidate::RouteCandidate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackOutcomeClass {
    TransientUnavailable,
    ProviderAuthError,
    HardStop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FallbackAttemptLimit {
    pub max_attempts: u32,
    pub attempts_used: u32,
}

impl FallbackAttemptLimit {
    pub fn new(max_attempts: u32) -> Self {
        Self {
            max_attempts,
            attempts_used: 0,
        }
    }

    pub fn exhausted(&self) -> bool {
        self.attempts_used >= self.max_attempts
    }

    pub fn bump(&mut self) {
        self.attempts_used = self.attempts_used.saturating_add(1);
    }

    pub fn remaining(&self) -> u32 {
        self.max_attempts.saturating_sub(self.attempts_used)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackDecision {
    RetryNextCandidate,
    FailImmediate,
}

#[derive(Debug, Clone)]
pub struct FallbackContext {
    pub route_class: &'static str,
    pub provider_id: Option<String>,
    pub outcome_class: FallbackOutcomeClass,
}

pub fn classify_fallback_outcome(
    candidate: &RouteCandidate,
    error_kind: &str,
) -> FallbackOutcomeClass {
    let lowered = error_kind.to_ascii_lowercase();
    if lowered.contains("auth")
        || lowered.contains("401")
        || lowered.contains("403")
        || lowered.contains("provider")
    {
        return FallbackOutcomeClass::ProviderAuthError;
    }

    if lowered.contains("timeout")
        || lowered.contains("unavailable")
        || lowered.contains("502")
        || lowered.contains("503")
        || lowered.contains("429")
        || lowered.contains("retryable")
    {
        return FallbackOutcomeClass::TransientUnavailable;
    }

    FallbackOutcomeClass::HardStop
}

pub fn decide_fallback(
    context: &FallbackContext,
    remaining: u32,
) -> FallbackDecision {
    match context.outcome_class {
        FallbackOutcomeClass::ProviderAuthError | FallbackOutcomeClass::HardStop => {
            FallbackDecision::FailImmediate
        }
        FallbackOutcomeClass::TransientUnavailable => {
            if remaining == 0 {
                FallbackDecision::FailImmediate
            } else {
                FallbackDecision::RetryNextCandidate
            }
        }
    }
}

pub fn should_allow_fallback_for_candidate(
    candidate: &RouteCandidate,
) -> bool {
    matches!(
        candidate.route_class,
        crate::adaptive::candidate::RouteClass::Fallback
            | crate::adaptive::candidate::RouteClass::Stability
    )
}
