use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const NEXT_ACTION_SCHEMA_VERSION: &str = "arda.next-action.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NextActionSourceKind {
    Queue,
    PersonalOperations,
    Workbench,
    Research,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NextActionFreshness {
    Fresh,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NextActionAuthorityState {
    Ready,
    ReviewRequired,
    Blocked,
    Advisory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NextActionStatus {
    Ready,
    Blocked,
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NextActionCandidate {
    pub id: String,
    pub title: String,
    pub source_kind: NextActionSourceKind,
    pub source_ref: String,
    pub reason: String,
    pub freshness: NextActionFreshness,
    pub authority_state: NextActionAuthorityState,
    pub next_operator_action: String,
    pub priority: u8,
    pub operator_authored: bool,
    pub terminal: bool,
    pub future_gated: bool,
    pub inferred_without_review: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NextActionExclusionCounts {
    pub stale: usize,
    pub terminal: usize,
    pub future_gated: usize,
    pub inferred_without_review: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NextActionProjection {
    pub schema_version: String,
    pub generated_at: DateTime<Utc>,
    pub status: NextActionStatus,
    pub selected: Option<NextActionCandidate>,
    pub reason: String,
    pub excluded: NextActionExclusionCounts,
}

pub fn select_next_action(
    candidates: Vec<NextActionCandidate>,
    generated_at: DateTime<Utc>,
) -> NextActionProjection {
    let mut excluded = NextActionExclusionCounts::default();
    let mut eligible = Vec::new();
    for candidate in candidates {
        if candidate.freshness == NextActionFreshness::Stale {
            excluded.stale += 1;
        } else if candidate.terminal {
            excluded.terminal += 1;
        } else if candidate.future_gated {
            excluded.future_gated += 1;
        } else if candidate.inferred_without_review {
            excluded.inferred_without_review += 1;
        } else {
            eligible.push(candidate);
        }
    }
    eligible.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| left.id.cmp(&right.id))
    });
    let selected = eligible.into_iter().next();
    let status = match selected.as_ref().map(|candidate| candidate.authority_state) {
        Some(NextActionAuthorityState::Blocked) => NextActionStatus::Blocked,
        Some(_) => NextActionStatus::Ready,
        None => NextActionStatus::Empty,
    };
    let reason = selected
        .as_ref()
        .map(|candidate| candidate.reason.clone())
        .unwrap_or_else(|| "No current trustworthy action is available.".to_string());
    NextActionProjection {
        schema_version: NEXT_ACTION_SCHEMA_VERSION.to_string(),
        generated_at,
        status,
        selected,
        reason,
        excluded,
    }
}
