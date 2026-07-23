use crate::adaptive::service::adaptive_routing::RouteExplanation;
use crate::adaptive::service::types::CharonService;
use crate::adaptive::service::types::{ManweRequestEnvelope, RouteDecision};
use chrono::Utc;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RouteHistoryEntry {
    pub ts_utc: String,
    pub route_id: String,
    pub agent_id: String,
    pub task_type: String,
    pub priority: String,
    pub provider_id: String,
    pub model_id: String,
    pub route_class: String,
    pub execution_lane: String,
    pub score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<RouteExplanation>,
}

#[derive(Debug, Clone)]
pub(super) struct StickyRouteSession {
    pub(super) provider_id: String,
    pub(super) model_id: String,
    pub(super) expires_at_utc: chrono::DateTime<Utc>,
}

pub(super) fn route_history_limit() -> usize {
    std::env::var("ARDA_MANWE_ROUTE_HISTORY_LIMIT")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(256)
}

fn request_wants_sticky_session(req: &ManweRequestEnvelope) -> bool {
    req.options
        .get("session_affinity")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| value.eq_ignore_ascii_case("sticky"))
}

fn sticky_session_key(req: &ManweRequestEnvelope) -> String {
    req.options
        .get("session_id")
        .or_else(|| req.options.get("conversation_id"))
        .or_else(|| req.options.get("thread_id"))
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!("{}:{value}", req.agent_id))
        .unwrap_or_else(|| format!("{}:{}:default", req.agent_id, req.task_type))
}

impl CharonService {
    pub async fn route_history(&self, limit: usize) -> Vec<RouteHistoryEntry> {
        let guard = self.route_history.read().await;
        let take = limit.min(guard.len());
        guard
            .iter()
            .rev()
            .take(take)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    pub(super) async fn record_route_history(&self, entry: RouteHistoryEntry) {
        let mut history = self.route_history.write().await;
        let limit = route_history_limit().max(1);
        while history.len() >= limit {
            history.pop_front();
        }
        history.push_back(entry);
    }

    pub(super) async fn sticky_route_override(
        &self,
        req: &ManweRequestEnvelope,
    ) -> Option<(String, String)> {
        if !request_wants_sticky_session(req) {
            return None;
        }
        let key = sticky_session_key(req);
        let now = Utc::now();
        let mut sessions = self.sticky_sessions.write().await;
        sessions.retain(|_, session| session.expires_at_utc > now);
        let session = sessions.get(&key)?;
        Some((session.provider_id.clone(), session.model_id.clone()))
    }

    pub(super) async fn update_sticky_route_session(
        &self,
        req: &ManweRequestEnvelope,
        decision: &RouteDecision,
    ) {
        if !request_wants_sticky_session(req) {
            return;
        }
        let ttl_minutes = req
            .options
            .get("session_affinity_ttl_minutes")
            .and_then(|value| value.as_i64())
            .filter(|value| *value > 0)
            .unwrap_or(15)
            .min(240);
        let session = StickyRouteSession {
            provider_id: decision.provider_id.clone(),
            model_id: decision.model_id.clone(),
            expires_at_utc: Utc::now() + chrono::Duration::minutes(ttl_minutes),
        };
        self.sticky_sessions
            .write()
            .await
            .insert(sticky_session_key(req), session);
    }
}
