// sigil: REPAIR
use arda_core::error::Result;
use arda_mnemosyne::MnemosyneService;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

mod api;
mod governance;
mod human_lifecycle;
mod lifecycle_audit;
mod lifecycle_policy;
mod organization;
mod path_policy;
mod runtime_state;
mod sigils;
mod support;
mod sweep;

use path_policy::{
    default_destructive_policy_path, default_root, default_watch_paths, default_world_state_path,
    is_low_value_warden_repair_target, is_permission_error, low_value_warden_repair_class,
    scheduler_snapshot, should_skip_watch_file, sweep_interval_hours,
};
use sigils::{
    action_record_matches_rule, hades_event_sigil, json_value_matches_rule, read_sigil, sigil_label,
};
use support::{
    append_jsonl, append_jsonl_batch, background_signal_limit, count_malformed_jsonl,
    hades_removal_step_limit, hades_sweep_limit, read_recent_jsonl, touch,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HadesStatus {
    pub last_sweep_utc: Option<String>,
    pub next_sweep_utc: String,
    pub pending_actions: usize,
    pub orphans_active: usize,
    pub condemned_pending: usize,
    pub quarantined: usize,
    pub warden_connected: bool,
    pub malformed_queue_records: usize,
    pub malformed_log_records: usize,
    pub malformed_joulework_records: usize,
    pub malformed_warden_queue_records: usize,
    pub malformed_athena_handoff_records: usize,
    pub scheduler: serde_json::Value,
}

#[derive(Clone)]
pub struct HadesService {
    root: PathBuf,
    log_path: PathBuf,
    joulework_path: PathBuf,
    queue_path: PathBuf,
    warden_queue_path: PathBuf,
    athena_handoff_queue_path: PathBuf,
    archive_root: PathBuf,
    state_path: PathBuf,
    world_state_path: PathBuf,
    destructive_policy_path: PathBuf,
    mnemosyne: Option<MnemosyneService>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct HadesState {
    last_sweep_utc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DestructiveQuorumPolicy {
    enabled: bool,
    required_approvers: usize,
    triad_approvers: Vec<String>,
    require_evidence: bool,
}

impl Default for DestructiveQuorumPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            required_approvers: 2,
            triad_approvers: vec![
                "aurelius".to_owned(),
                "bacon".to_owned(),
                "sun_tzu".to_owned(),
            ],
            require_evidence: true,
        }
    }
}

#[derive(Debug, Clone)]
struct QuorumEvaluation {
    allowed: bool,
    required_approvers: usize,
    triad_approvers: Vec<String>,
    approved_count: usize,
    approved_by: Vec<String>,
    has_evidence: bool,
    reason: String,
    love_equation_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JouleWorkRecord {
    ts_utc: String,
    component: String,
    operation: String,
    files_scanned: usize,
    actions_taken: usize,
    orphans_found: usize,
    held_for_review: usize,
    estimated_joules: f64,
    baseline_joules: f64,
    outside_historical_scope: bool,
    inference_provider: String,
    inference_model: String,
    inference_origin: String,
    notes: String,
}

impl HadesService {
    pub fn from_default_or_fallback() -> Result<Self> {
        let primary = default_root();
        match Self::new(&primary) {
            Ok(v) => Ok(v),
            Err(err) => {
                if !is_permission_error(&err) {
                    return Err(err);
                }
                Self::new(PathBuf::from("data").join("hades"))
            }
        }
    }

    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        let log_path = root.join("hades_log.jsonl");
        let joulework_path = root.join("joulework.jsonl");
        let queue_path = root.join("action_queue.jsonl");
        let warden_queue_path = root.join("warden_queue.jsonl");
        let athena_handoff_queue_path = root.join("athena_handoff_queue.jsonl");
        let archive_root = root.join("archive");
        let state_path = root.join("state.json");
        let destructive_policy_path = default_destructive_policy_path();
        fs::create_dir_all(&archive_root)?;
        touch(&log_path)?;
        touch(&joulework_path)?;
        touch(&queue_path)?;
        touch(&warden_queue_path)?;
        touch(&athena_handoff_queue_path)?;
        if !state_path.exists() {
            std::fs::write(&state_path, "{}")?;
        }

        Ok(Self {
            root,
            log_path,
            joulework_path,
            queue_path,
            warden_queue_path,
            athena_handoff_queue_path,
            archive_root,
            state_path,
            world_state_path: default_world_state_path(),
            destructive_policy_path,
            mnemosyne: MnemosyneService::from_default_or_fallback().ok(),
        })
    }
}

#[cfg(test)]
mod tests;
