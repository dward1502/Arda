use crate::adaptive::service::types::CharonRequestEnvelope;
use crate::adaptive::service::types::CharonService;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Default)]
pub(super) struct BanditStore {
    path: PathBuf,
    state: std::sync::Mutex<BanditState>,
    pending_by_provider: std::sync::Mutex<BTreeMap<String, VecDeque<String>>>,
}

impl Clone for BanditStore {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            state: std::sync::Mutex::new(
                self.state
                    .lock()
                    .ok()
                    .map(|guard| guard.clone())
                    .unwrap_or_default(),
            ),
            pending_by_provider: std::sync::Mutex::new(
                self.pending_by_provider
                    .lock()
                    .ok()
                    .map(|guard| guard.clone())
                    .unwrap_or_default(),
            ),
        }
    }
}

impl BanditStore {
    pub(super) fn new(path: PathBuf) -> Self {
        let state = fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<BanditState>(&raw).ok())
            .unwrap_or_default();
        Self {
            path,
            state: Mutex::new(state),
            pending_by_provider: Mutex::new(BTreeMap::new()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct BanditState {
    #[serde(default)]
    arms: BTreeMap<String, BanditArm>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BanditArm {
    successes: u64,
    failures: u64,
}

impl CharonService {
    pub(super) fn record_bandit_route(
        &self,
        req: &CharonRequestEnvelope,
        provider_id: &str,
        model_id: &str,
    ) {
        let key = bandit_key(&request_learning_key(req), provider_id, model_id);
        let Ok(mut pending) = self.bandit.pending_by_provider.lock() else {
            return;
        };
        let queue = pending.entry(provider_id.to_string()).or_default();
        queue.push_back(key);
        while queue.len() > 256 {
            queue.pop_front();
        }
    }

    pub(super) fn observe_bandit_provider_result(&self, provider_id: &str, ok: bool) {
        let key = {
            let Ok(mut pending) = self.bandit.pending_by_provider.lock() else {
                return;
            };
            pending.get_mut(provider_id).and_then(VecDeque::pop_front)
        };
        let Some(key) = key else {
            return;
        };
        let Ok(mut state) = self.bandit.state.lock() else {
            return;
        };
        let arm = state.arms.entry(key).or_insert(BanditArm {
            successes: 0,
            failures: 0,
        });
        if ok {
            arm.successes = arm.successes.saturating_add(1);
        } else {
            arm.failures = arm.failures.saturating_add(1);
        }
        persist_bandit_state(&self.bandit.path, &state);
    }

    pub(super) fn bandit_score_bonus(
        &self,
        req: &CharonRequestEnvelope,
        provider_id: &str,
        model_id: &str,
    ) -> f64 {
        let weight = bandit_score_weight();
        if weight <= 0.0 {
            return 0.0;
        }
        let key = bandit_key(&request_learning_key(req), provider_id, model_id);
        let Ok(state) = self.bandit.state.lock() else {
            return 0.0;
        };
        let Some(arm) = state.arms.get(&key) else {
            return 0.0;
        };
        let observations = arm.successes.saturating_add(arm.failures);
        if observations < bandit_min_observations() {
            return 0.0;
        }
        let alpha = arm.successes as f64 + 1.0;
        let beta = arm.failures as f64 + 1.0;
        let mean = alpha / (alpha + beta);
        (mean - 0.5) * weight
    }
}

fn persist_bandit_state(path: &PathBuf, state: &BanditState) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(state) {
        let _ = fs::write(path, raw);
    }
}

fn bandit_key(task_type: &str, provider_id: &str, model_id: &str) -> String {
    format!("{task_type}\u{1f}{provider_id}\u{1f}{model_id}")
}

fn request_learning_key(req: &CharonRequestEnvelope) -> String {
    let has_tools = req.options.get("tools").is_some()
        || req.options.get("tool_choice").is_some()
        || req.messages.iter().any(|message| {
            message
                .get("tool_calls")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|items| !items.is_empty())
                || message.get("role").and_then(serde_json::Value::as_str) == Some("tool")
        });
    let structured = req.options.get("response_format").is_some();
    let streaming = req
        .options
        .get("stream")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    format!(
        "{}|tools={}|structured={}|stream={}",
        req.task_type, has_tools, structured, streaming
    )
}

fn bandit_score_weight() -> f64 {
    std::env::var("ARDA_CHARON_BANDIT_SCORE_WEIGHT")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| *value >= 0.0)
        .unwrap_or(8.0)
}

fn bandit_min_observations() -> u64 {
    std::env::var("ARDA_CHARON_BANDIT_MIN_OBSERVATIONS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(3)
}
