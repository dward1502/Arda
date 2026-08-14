#![cfg(feature = "full-cli")]
// sigil: REPAIR
//! Main autonomous CEO loop with Oracle gate + Apollo execution + A2H escalation.

use super::a2h::{
    append_pending_authorization, authorize_for_escalation_with_id, process_h2a_responses,
    write_message, H2AProcessReport, HumanApprovedObjective,
};
use super::bootstrap::{load_defaults, load_registry_from_world, LoadedDefaults};
use super::core_executor_bridge::{
    dispatch_with_conditions as executor_dispatch, CoreExecutorClient, Dispatch, ExecutionStatus,
};
use super::dashboard::{build_snapshot, DashboardSnapshot};
use super::decomposer::{Objective, ObjectiveDecomposer, PlannedTask, Priority};
use super::delegation::{delegate_plan, AgentRegistry, DelegationReport};
use super::evidence_registry::EvidenceRegistry;
use super::governance_policy::{GovernanceDecision, GovernanceGate, GovernancePolicy};
use super::learning::LearningStore;
use super::oracle_gate::{GateDecision, OracleGate};
use super::outcomes::OutcomeObserver;
use super::pipeline_bridge::submit_plan as submit_plan_to_pipeline;
use super::planner::{
    acceptance_criteria_from_report, source_contract_and_type_for_path, ObjectivePacket,
    ObjectivePacketInput, ObjectivePacketReport,
};
use super::queue_operation::{append_approved_packet_plan, QueueOperation, QueueOperationStatus};
use super::queue_writer::append_apollo_dispatch_attempt_to_queue;
use super::reporting::{write_daily_report, write_weekly_report};
use super::service_health::{ServiceHealthMonitor, ServiceHealthReport, UserSystemd};
use super::source_registry::SourceRegistry;
use super::task_queue::{QueueRecord, TaskQueueAnalyzer, TaskQueueMetrics};
use super::taxonomy::is_apollo_dispatchable;
use super::validator::{PlanValidator, ValidationResult};
use crate::prometheus::orders::{OrderStatus, OrderStore};
use crate::prometheus::queue_authority::canonical_project_task_queue;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct AutopilotConfig {
    pub root: PathBuf,
    pub queue_path: PathBuf,
    pub objectives_path: PathBuf,
    pub world_path: PathBuf,
    pub state_path: PathBuf,
    pub learning_path: PathBuf,
    pub heartbeat_path: PathBuf,
    pub report_dir: PathBuf,
    pub outcome_cursor_path: PathBuf,
    pub a2h_path: PathBuf,
    pub h2a_path: PathBuf,
    pub a2h_pending_path: PathBuf,
    pub arandur_recommendations_path: PathBuf,
    pub interval: Duration,
    pub joule_budget: f64,
    pub max_per_agent: usize,
    pub heartbeat_max_bytes: u64,
    pub oracle_joule_threshold: f64,
    pub read_only: bool,
    pub systemd_pattern: String,
    pub apollo_socket_path: PathBuf,
    pub apollo_max_attempts: u32,
    pub joule_cycle_limit: f64,
    pub joule_hourly_limit: f64,
    pub pipeline_submit_enabled: bool,
    pub pause_poll_interval: Duration,
    pub circuit_breaker_path: PathBuf,
    pub consecutive_failure_limit: usize,
}

impl AutopilotConfig {
    pub fn from_root(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        let core_root = root.join("core");
        let LoadedDefaults {
            joule_budget,
            heartbeat_ms: _,
            base_costs: _,
        } = load_defaults(&core_root);
        Self {
            queue_path: canonical_project_task_queue(&root),
            objectives_path: root.join("core/projects/objectives/inbox.jsonl"),
            world_path: root.join("core/state/world.json"),
            state_path: root.join("data/ceo/autopilot.state.json"),
            learning_path: root.join("data/ceo/learning.json"),
            heartbeat_path: root.join("data/ceo/heartbeats.jsonl"),
            report_dir: root.join("data/ceo/reports"),
            outcome_cursor_path: root.join("data/ceo/outcomes.cursor.json"),
            a2h_path: root.join("data/comm/a2h.jsonl"),
            h2a_path: root.join("data/comm/h2a.jsonl"),
            a2h_pending_path: root.join("data/ceo/a2h.pending.jsonl"),
            arandur_recommendations_path: root.join("data/arandur/recommendations.jsonl"),
            interval: Duration::from_secs(30),
            joule_budget,
            max_per_agent: 16,
            heartbeat_max_bytes: 5 * 1024 * 1024,
            oracle_joule_threshold: 100.0,
            read_only: false,
            systemd_pattern: "arda-*".into(),
            apollo_socket_path: apollo_socket_default(&root),
            apollo_max_attempts: 2,
            joule_cycle_limit: joule_budget,
            joule_hourly_limit: joule_budget * 4.0,
            pipeline_submit_enabled: true,
            pause_poll_interval: Duration::from_secs(60),
            circuit_breaker_path: root.join("tmp/ceo/circuit_breaker.flag"),
            consecutive_failure_limit: 5,
            root,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CycleReport {
    pub timestamp: String,
    pub queue: TaskQueueMetrics,
    pub services: ServiceHealthReport,
    pub dashboard: DashboardSnapshot,
    pub objective_selection: ObjectiveSelectionReport,
    pub objectives_processed: usize,
    pub plans: Vec<PlanCycle>,
    pub outcomes_ingested: usize,
    pub h2a: H2AProcessReport,
    pub hades_introspection: HadesIntrospectionProjection,
    pub sovereign_adapters: SovereignAdapterProjection,
    pub council_runtime: CouncilRuntimeProjection,
    pub autonomy_readiness: AutonomyReadinessGateProjection,
    pub report_path: Option<String>,
    pub weekly_report_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HadesIntrospectionProjection {
    pub contract: String,
    pub source_available: bool,
    pub policy_report_path: String,
    pub review_queue_path: String,
    pub source_contract: Option<String>,
    pub generated_at_utc: Option<String>,
    pub source_findings_total: usize,
    pub consistency_holds_total: usize,
    pub review_queue_records: usize,
    pub review_queue_projection_recommended: bool,
    pub cleanup_authorized: bool,
    pub requires_operator_approval_for_mutation: bool,
    pub no_file_moves_or_deletes_performed: bool,
    pub error: Option<String>,
}

impl Default for HadesIntrospectionProjection {
    fn default() -> Self {
        Self {
            contract: HADES_INTROSPECTION_CONTRACT.to_owned(),
            source_available: false,
            policy_report_path: String::new(),
            review_queue_path: String::new(),
            source_contract: None,
            generated_at_utc: None,
            source_findings_total: 0,
            consistency_holds_total: 0,
            review_queue_records: 0,
            review_queue_projection_recommended: false,
            cleanup_authorized: false,
            requires_operator_approval_for_mutation: true,
            no_file_moves_or_deletes_performed: true,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SovereignAdapterProjection {
    pub contract: String,
    pub config_path: String,
    pub source_available: bool,
    pub adapter_count: usize,
    pub active_runtime_adapter_count: usize,
    pub evidence_only_adapter_count: usize,
    pub missing_required_adapter_count: usize,
    pub adapters: Vec<SovereignAdapterReceipt>,
    pub error: Option<String>,
}

impl Default for SovereignAdapterProjection {
    fn default() -> Self {
        Self {
            contract: SOVEREIGN_ADAPTERS_CONTRACT.to_owned(),
            config_path: CANONICAL_AUTONOMY_CONFIG.to_owned(),
            source_available: false,
            adapter_count: 0,
            active_runtime_adapter_count: 0,
            evidence_only_adapter_count: 0,
            missing_required_adapter_count: 0,
            adapters: Vec::new(),
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SovereignAdapterReceipt {
    pub id: String,
    pub crate_name: String,
    pub status: String,
    pub loop_stages: Vec<String>,
    pub gate: String,
    pub runtime_adapter: bool,
    pub source_path: String,
    pub source_records: usize,
    pub cycle_receipts: usize,
    pub gate_effect: String,
    pub evidence_summary: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CouncilRuntimeProjection {
    pub contract: String,
    pub ledger_path: String,
    pub source_available: bool,
    pub existing_record_count: usize,
    pub appended_record_count: usize,
    pub latest_conversation_id: Option<String>,
    pub task_promotion_allowed: bool,
    pub evidence_only: bool,
    pub error: Option<String>,
}

impl Default for CouncilRuntimeProjection {
    fn default() -> Self {
        Self {
            contract: COUNCIL_RUNTIME_CONTRACT.to_owned(),
            ledger_path: "data/council/agent_conversations.jsonl".to_owned(),
            source_available: false,
            existing_record_count: 0,
            appended_record_count: 0,
            latest_conversation_id: None,
            task_promotion_allowed: false,
            evidence_only: true,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AutonomyReadinessGateProjection {
    pub contract: String,
    pub decision: String,
    pub task_promotion_allowed: bool,
    pub human_required: bool,
    pub preflight_path: String,
    pub preflight_source_available: bool,
    pub cleanup_approval_packets_path: String,
    pub cleanup_approval_packets_available: bool,
    pub external_source_lane_ledger_path: String,
    pub external_source_lane_ledger_available: bool,
    pub lane_count: usize,
    pub lane_incomplete_count: usize,
    pub cleanup_packet_count: usize,
    pub external_source_lane_count: usize,
    pub external_source_blocked_count: usize,
    pub sovereign_missing_required_adapter_count: usize,
    pub council_unresolved_escalation_count: usize,
    pub reasons: Vec<String>,
    pub evidence_paths: Vec<String>,
}

impl Default for AutonomyReadinessGateProjection {
    fn default() -> Self {
        Self {
            contract: AUTONOMY_READINESS_GATE_CONTRACT.to_owned(),
            decision: "hold".into(),
            task_promotion_allowed: false,
            human_required: false,
            preflight_path: "data/prometheus/autonomy_operating_loop_preflight.json".into(),
            preflight_source_available: false,
            cleanup_approval_packets_path: "data/hades/autonomy_cleanup_approval_packets.json"
                .into(),
            cleanup_approval_packets_available: false,
            external_source_lane_ledger_path: "data/athena/external_source_lane_ledger.jsonl"
                .into(),
            external_source_lane_ledger_available: false,
            lane_count: 0,
            lane_incomplete_count: 0,
            cleanup_packet_count: 0,
            external_source_lane_count: 0,
            external_source_blocked_count: 0,
            sovereign_missing_required_adapter_count: 0,
            council_unresolved_escalation_count: 0,
            reasons: Vec::new(),
            evidence_paths: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct AutonomyLoopConfig {
    sovereign_crates: Option<Vec<SovereignCrateConfig>>,
    lanes: Option<Vec<AutonomyLaneConfig>>,
    #[serde(rename = "loop")]
    loop_config: Option<AutonomyLoopStageConfig>,
}

#[derive(Debug, Deserialize)]
struct AutonomyLaneConfig {
    id: String,
    agent: Option<String>,
    engine_interface: Option<String>,
    default_policy: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AutonomyLoopStageConfig {
    stages: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AutonomyPreflightSummary {
    pub lane_count: usize,
    pub lane_configured_count: usize,
    pub lane_incomplete_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct AutonomyPreflightLoop {
    pub configured_stages: Vec<String>,
    pub missing_required_stages: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AutonomyPreflightReport {
    pub schema_version: String,
    pub generated_at_utc: String,
    pub config_path: String,
    pub task_promotion_allowed: bool,
    #[serde(rename = "loop")]
    pub loop_: AutonomyPreflightLoop,
    pub summary: AutonomyPreflightSummary,
}

pub fn inspect_autonomy_preflight(
    root: impl AsRef<Path>,
) -> std::io::Result<AutonomyPreflightReport> {
    let root = root.as_ref();
    let config_path = root.join(CANONICAL_AUTONOMY_CONFIG);
    let legacy_path = root.join(LEGACY_AUTONOMY_CONFIG);
    if config_path.exists() && legacy_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "ambiguous autonomy config paths: canonical={} legacy={}",
                config_path.display(),
                legacy_path.display()
            ),
        ));
    }
    let content = std::fs::read_to_string(&config_path)?;
    let config = toml::from_str::<AutonomyLoopConfig>(&content)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    let lanes = config.lanes.unwrap_or_default();
    let lane_incomplete_count = lanes
        .iter()
        .filter(|lane| {
            lane.id.trim().is_empty()
                || lane.agent.as_deref().is_none_or(str::is_empty)
                || lane.engine_interface.as_deref().is_none_or(str::is_empty)
                || lane.default_policy.as_deref().is_none_or(str::is_empty)
        })
        .count();
    let configured_stages = config
        .loop_config
        .and_then(|loop_config| loop_config.stages)
        .unwrap_or_default();
    let required_stages = ["info", "ingest", "audit"];
    let missing_required_stages = required_stages
        .into_iter()
        .filter(|required| !configured_stages.iter().any(|stage| stage == required))
        .map(str::to_owned)
        .collect();

    Ok(AutonomyPreflightReport {
        schema_version: "arda.autonomy_operating_loop_preflight.v1".to_owned(),
        generated_at_utc: Utc::now().to_rfc3339(),
        config_path: config_path.display().to_string(),
        task_promotion_allowed: false,
        loop_: AutonomyPreflightLoop {
            configured_stages,
            missing_required_stages,
        },
        summary: AutonomyPreflightSummary {
            lane_count: lanes.len(),
            lane_configured_count: lanes.len().saturating_sub(lane_incomplete_count),
            lane_incomplete_count,
        },
    })
}

pub fn write_autonomy_preflight(root: impl AsRef<Path>) -> std::io::Result<PathBuf> {
    let root = root.as_ref();
    let report = inspect_autonomy_preflight(root)?;
    let output = root.join("data/prometheus/autonomy_operating_loop_preflight.json");
    let parent = output.parent().expect("preflight output has parent");
    std::fs::create_dir_all(parent)?;
    let temporary = output.with_extension(format!("tmp-{}", std::process::id()));
    let body = serde_json::to_vec_pretty(&report).map_err(std::io::Error::other)?;
    std::fs::write(&temporary, body)?;
    if let Err(error) = std::fs::rename(&temporary, &output) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(output)
}

#[derive(Debug, Deserialize)]
struct SovereignCrateConfig {
    id: String,
    #[serde(rename = "crate")]
    crate_name: Option<String>,
    loop_stages: Option<Vec<String>>,
    gate: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ObjectiveSelectionReport {
    pub contract: String,
    pub mutation_policy: String,
    pub objectives_considered: usize,
    pub objectives_selected: usize,
    pub objectives_blocked_by_gate: usize,
    pub effective_queue_open_count: usize,
    pub stale_raw_queue_record_count: usize,
    pub status: String,
    pub no_selection_reason: Option<String>,
    pub selected_objective_id: Option<String>,
    pub next_recommended_action: String,
    pub blocked_candidate_groups: Vec<BlockedCandidateGroup>,
    pub next_automation_gate_packet: Option<NextAutomationGatePacket>,
    pub objective_packet_report: ObjectivePacketReport,
    pub candidates: Vec<ObjectiveCandidateReport>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BlockedCandidateGroup {
    pub reason_code: String,
    pub governance_class: String,
    pub review_gate: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct NextAutomationGatePacket {
    pub packet_type: String,
    pub recommendation_id: String,
    pub candidate_id: String,
    pub title: String,
    pub owner: Option<String>,
    pub priority: Option<String>,
    pub requires_operator_approval: bool,
    pub canonical_queue_mutation_allowed: bool,
    pub approval_packet_required: bool,
    pub approval_packet_schema: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ObjectiveCandidateReport {
    pub source_path: String,
    pub source_record_id: String,
    pub candidate_id: String,
    pub title: String,
    pub effective_status: String,
    pub owner: Option<String>,
    pub priority: Option<String>,
    pub governance_class: String,
    pub review_gate: GovernanceGate,
    pub blocked_reason_code: Option<String>,
    pub approval_packet_id: Option<String>,
    pub completion_receipt_path: Option<String>,
    pub selected_reason: Option<String>,
    pub rejection_reason: Option<String>,
}

impl Default for ObjectiveSelectionReport {
    fn default() -> Self {
        Self {
            contract: OBJECTIVE_SELECTION_CONTRACT.into(),
            mutation_policy: "report_only".into(),
            objectives_considered: 0,
            objectives_selected: 0,
            objectives_blocked_by_gate: 0,
            effective_queue_open_count: 0,
            stale_raw_queue_record_count: 0,
            status: "no_action".into(),
            no_selection_reason: Some("no_candidates_available".into()),
            selected_objective_id: None,
            next_recommended_action:
                "add a bounded objective to the objective inbox or canonical queue".into(),
            blocked_candidate_groups: Vec::new(),
            next_automation_gate_packet: None,
            objective_packet_report: ObjectivePacketReport::read_only(Vec::new()),
            candidates: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanCycle {
    pub objective_id: String,
    pub plan: Vec<PlannedTask>,
    pub validation: ValidationResult,
    pub governance: GovernanceDecision,
    pub gate: GateDecision,
    pub autonomy_readiness_decision: String,
    pub autonomy_readiness_reasons: Vec<String>,
    pub delegation: Option<DelegationReport>,
    pub queued_task_ids: Vec<String>,
    pub queue_operation: Option<QueueOperation>,
    pub apollo_dispatches: Vec<Dispatch>,
    pub a2h_emitted: bool,
    pub joule_limited: bool,
    pub pipeline_submitted: bool,
}

pub struct CeoAutopilot {
    cfg: AutopilotConfig,
    decomposer: ObjectiveDecomposer,
    validator: PlanValidator,
    registry: AgentRegistry,
    learning: arda_core::learning::LearningState,
    oracle: OracleGate,
    governance_policy: GovernancePolicy,
    apollo: CoreExecutorClient,
    consecutive_failures: usize,
}

const HADES_INTROSPECTION_CONTRACT: &str = "arda.prometheus.hades_introspection_projection.v1";
const SOVEREIGN_ADAPTERS_CONTRACT: &str = "arda.prometheus.sovereign_adapter_projection.v1";
const COUNCIL_RUNTIME_CONTRACT: &str = "arda.prometheus.council_runtime_projection.v1";
const AUTONOMY_READINESS_GATE_CONTRACT: &str = "arda.prometheus.autonomy_readiness_gate.v1";
const CANONICAL_AUTONOMY_CONFIG: &str = "config/governance/autonomy_operating_loop.toml";
const LEGACY_AUTONOMY_CONFIG: &str = "config/autonomy_operating_loop.toml";
const AUTONOMY_PREFLIGHT_MAX_AGE_HOURS: i64 = 24;

fn load_hades_introspection(root: &Path) -> HadesIntrospectionProjection {
    let policy_report_path = root.join("data/hades/lifecycle_policy_automation_report.json");
    let review_queue_path = root.join("data/hades/lifecycle_review_queue.jsonl");
    let review_queue_records = count_jsonl_records(&review_queue_path);
    let base = |source_available: bool, error: Option<String>| HadesIntrospectionProjection {
        contract: HADES_INTROSPECTION_CONTRACT.to_owned(),
        source_available,
        policy_report_path: policy_report_path.display().to_string(),
        review_queue_path: review_queue_path.display().to_string(),
        source_contract: None,
        generated_at_utc: None,
        source_findings_total: 0,
        consistency_holds_total: 0,
        review_queue_records,
        review_queue_projection_recommended: false,
        cleanup_authorized: false,
        requires_operator_approval_for_mutation: true,
        no_file_moves_or_deletes_performed: true,
        error,
    };

    let content = match std::fs::read_to_string(&policy_report_path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return base(false, None),
        Err(err) => return base(false, Some(err.to_string())),
    };
    let value = match serde_json::from_str::<Value>(&content) {
        Ok(value) => value,
        Err(err) => return base(false, Some(err.to_string())),
    };
    let summary = value.get("policy_summary").unwrap_or(&Value::Null);

    HadesIntrospectionProjection {
        contract: HADES_INTROSPECTION_CONTRACT.to_owned(),
        source_available: true,
        policy_report_path: policy_report_path.display().to_string(),
        review_queue_path: review_queue_path.display().to_string(),
        source_contract: value
            .get("contract")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        generated_at_utc: value
            .get("generated_at_utc")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        source_findings_total: value
            .get("source_findings_total")
            .and_then(Value::as_u64)
            .or_else(|| summary.get("findings_total").and_then(Value::as_u64))
            .map(|total| total as usize)
            .unwrap_or(0),
        consistency_holds_total: summary
            .get("consistency_holds_total")
            .and_then(Value::as_u64)
            .map(|total| total as usize)
            .unwrap_or(0),
        review_queue_records,
        review_queue_projection_recommended: value
            .get("review_queue_projection_recommended")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        cleanup_authorized: value
            .get("cleanup_authorized")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        requires_operator_approval_for_mutation: value
            .get("requires_operator_approval_for_mutation")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        no_file_moves_or_deletes_performed: value
            .get("no_file_moves_or_deletes_performed")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        error: None,
    }
}

fn count_jsonl_records(path: &Path) -> usize {
    std::fs::read_to_string(path)
        .map(|content| {
            content
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count()
        })
        .unwrap_or(0)
}

fn load_sovereign_adapters(
    root: &Path,
    cfg: &AutopilotConfig,
    plans: &[PlanCycle],
    h2a: &H2AProcessReport,
) -> SovereignAdapterProjection {
    let config_path = root.join(CANONICAL_AUTONOMY_CONFIG);
    let legacy_path = root.join(LEGACY_AUTONOMY_CONFIG);
    let display_config_path = config_path.display().to_string();
    if config_path.exists() && legacy_path.exists() {
        return SovereignAdapterProjection {
            config_path: display_config_path,
            error: Some(format!(
                "ambiguous_autonomy_config_paths: canonical={} legacy={}",
                config_path.display(),
                legacy_path.display()
            )),
            ..Default::default()
        };
    }
    let content = match std::fs::read_to_string(&config_path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return SovereignAdapterProjection {
                config_path: display_config_path,
                error: None,
                ..Default::default()
            };
        }
        Err(err) => {
            return SovereignAdapterProjection {
                config_path: display_config_path,
                error: Some(err.to_string()),
                ..Default::default()
            };
        }
    };
    let parsed = match toml::from_str::<AutonomyLoopConfig>(&content) {
        Ok(parsed) => parsed,
        Err(err) => {
            return SovereignAdapterProjection {
                config_path: display_config_path,
                source_available: true,
                error: Some(err.to_string()),
                ..Default::default()
            };
        }
    };

    let adapters = parsed
        .sovereign_crates
        .unwrap_or_default()
        .into_iter()
        .map(|item| adapter_receipt(root, cfg, plans, h2a, item))
        .collect::<Vec<_>>();
    let active_runtime_adapter_count = adapters
        .iter()
        .filter(|adapter| adapter.runtime_adapter && adapter.gate_effect != "evidence_only")
        .count();
    let evidence_only_adapter_count = adapters
        .iter()
        .filter(|adapter| adapter.runtime_adapter && adapter.gate_effect == "evidence_only")
        .count();
    let missing_required_adapter_count = adapters
        .iter()
        .filter(|adapter| adapter.status == "contract_required" && !adapter.runtime_adapter)
        .count();

    SovereignAdapterProjection {
        contract: SOVEREIGN_ADAPTERS_CONTRACT.to_owned(),
        config_path: display_config_path,
        source_available: true,
        adapter_count: adapters.len(),
        active_runtime_adapter_count,
        evidence_only_adapter_count,
        missing_required_adapter_count,
        adapters,
        error: None,
    }
}

fn adapter_receipt(
    root: &Path,
    cfg: &AutopilotConfig,
    plans: &[PlanCycle],
    h2a: &H2AProcessReport,
    item: SovereignCrateConfig,
) -> SovereignAdapterReceipt {
    let status = item.status.unwrap_or_else(|| "unknown".into());
    let loop_stages = item.loop_stages.unwrap_or_default();
    let gate = item.gate.unwrap_or_default();
    let base = |runtime_adapter: bool,
                source_path: String,
                source_records: usize,
                cycle_receipts: usize,
                gate_effect: String,
                evidence_summary: Vec<String>| {
        SovereignAdapterReceipt {
            id: item.id.clone(),
            crate_name: item.crate_name.clone().unwrap_or_default(),
            status: status.clone(),
            loop_stages: loop_stages.clone(),
            gate: gate.clone(),
            runtime_adapter,
            source_path,
            source_records,
            cycle_receipts,
            gate_effect,
            evidence_summary,
        }
    };

    match item.id.as_str() {
        "governance" => {
            let blocked = plans
                .iter()
                .filter(|plan| plan.governance.blocks_delegation())
                .count();
            let escalated = plans
                .iter()
                .filter(|plan| plan.governance.requires_escalation())
                .count();
            let human_required = plans
                .iter()
                .filter(|plan| plan.governance.requires_human)
                .count();
            base(
                true,
                "crates/spine/observability/arda-aule/src/autopilot/governance_policy.rs".into(),
                plans.len(),
                plans.len(),
                "task_promotion_gate".into(),
                vec![
                    format!("plans_classified={}", plans.len()),
                    format!("blocked={blocked}"),
                    format!("escalated={escalated}"),
                    format!("human_required={human_required}"),
                ],
            )
        }
        "oracle" => {
            let invoked = plans
                .iter()
                .filter(|plan| !matches!(plan.gate, GateDecision::Skipped))
                .count();
            let rejected = plans
                .iter()
                .filter(|plan| matches!(plan.gate, GateDecision::Rejected { .. }))
                .count();
            base(
                true,
                "crates/spine/observability/arda-aule/src/autopilot/oracle_gate.rs".into(),
                plans.len(),
                invoked,
                "validation_gate".into(),
                vec![
                    format!("oracle_gate_invoked={invoked}"),
                    format!("oracle_rejected={rejected}"),
                ],
            )
        }
        "plutus" => {
            let delegated_joules = clean_zero(
                plans
                    .iter()
                    .filter(|plan| !plan.joule_limited && !plan.queued_task_ids.is_empty())
                    .flat_map(|plan| plan.plan.iter())
                    .map(|task| task.joule_cost)
                    .sum::<f64>(),
            );
            let joule_limited = plans.iter().filter(|plan| plan.joule_limited).count();
            base(
                true,
                "crates/spine/observability/arda-aule/src/autopilot/runner.rs".into(),
                plans.len(),
                plans.len(),
                "budget_gate".into(),
                vec![
                    format!("cycle_limit={:.1}", cfg.joule_cycle_limit),
                    format!("hourly_limit={:.1}", cfg.joule_hourly_limit),
                    format!("delegated_joules={delegated_joules:.1}"),
                    format!("joule_limited={joule_limited}"),
                ],
            )
        }
        "human" => {
            let a2h_emitted = plans.iter().filter(|plan| plan.a2h_emitted).count();
            base(
                true,
                "data/comm/h2a.jsonl".into(),
                h2a.responses_processed + h2a.objectives_resumed + h2a.denials_recorded,
                h2a.responses_processed + a2h_emitted,
                "human_approval_gate".into(),
                vec![
                    format!("h2a_responses_processed={}", h2a.responses_processed),
                    format!("h2a_objectives_resumed={}", h2a.objectives_resumed),
                    format!("h2a_denials_recorded={}", h2a.denials_recorded),
                    format!("a2h_emitted={a2h_emitted}"),
                ],
            )
        }
        "council" => {
            let ledger = root.join("data/council/agent_conversations.jsonl");
            let records = count_jsonl_records(&ledger);
            base(
                true,
                ledger.display().to_string(),
                records,
                records,
                "evidence_only".into(),
                vec![
                    format!("conversation_records={records}"),
                    "council_output_does_not_approve_execution_by_itself".into(),
                ],
            )
        }
        "ceo" => base(
            true,
            "crates/spine/observability/arda-aule/src/autopilot/runner.rs".into(),
            plans.len(),
            plans.len(),
            "canonical_engine_guard".into(),
            vec!["prometheus_ceo_autopilot_is_active_runtime_engine".into()],
        ),
        _ => base(
            false,
            String::new(),
            0,
            0,
            "missing_adapter".into(),
            vec!["no runtime adapter registered for sovereign crate id".into()],
        ),
    }
}

fn load_council_runtime(
    root: &Path,
    read_only: bool,
    objective_selection: &ObjectiveSelectionReport,
    plans: &[PlanCycle],
) -> CouncilRuntimeProjection {
    let ledger_path = root.join("data/council/agent_conversations.jsonl");
    let existing_record_count = count_jsonl_records(&ledger_path);
    let latest_conversation_id = latest_conversation_id(&ledger_path);
    let mut projection = CouncilRuntimeProjection {
        contract: COUNCIL_RUNTIME_CONTRACT.to_owned(),
        ledger_path: ledger_path.display().to_string(),
        source_available: ledger_path.exists(),
        existing_record_count,
        latest_conversation_id,
        ..Default::default()
    };

    if read_only || !should_emit_council_cycle_record(objective_selection, plans) {
        return projection;
    }

    let record = council_cycle_record(objective_selection, plans);
    if let Some(parent) = ledger_path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            projection.error = Some(err.to_string());
            return projection;
        }
    }
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&ledger_path)
    {
        Ok(mut handle) => {
            use std::io::Write;
            match serde_json::to_string(&record) {
                Ok(line) => {
                    if let Err(err) = writeln!(handle, "{line}") {
                        projection.error = Some(err.to_string());
                    } else {
                        projection.source_available = true;
                        projection.appended_record_count = 1;
                        projection.existing_record_count = existing_record_count + 1;
                        projection.latest_conversation_id = record
                            .get("conversation_id")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned);
                    }
                }
                Err(err) => projection.error = Some(err.to_string()),
            }
        }
        Err(err) => projection.error = Some(err.to_string()),
    }
    projection
}

fn should_emit_council_cycle_record(
    objective_selection: &ObjectiveSelectionReport,
    plans: &[PlanCycle],
) -> bool {
    objective_selection.objectives_considered > 0
        || objective_selection.objectives_blocked_by_gate > 0
        || !plans.is_empty()
}

fn council_cycle_record(
    objective_selection: &ObjectiveSelectionReport,
    plans: &[PlanCycle],
) -> Value {
    let ts = Utc::now().to_rfc3339();
    let selected = objective_selection
        .selected_objective_id
        .as_deref()
        .unwrap_or("none");
    let conversation_id = format!("ceo_autopilot_cycle_{}", ts.replace([':', '.', '+'], "_"));
    let held = plans
        .iter()
        .filter(|plan| {
            plan.governance.blocks_delegation()
                || !plan.gate.allows_delegation()
                || plan.joule_limited
        })
        .count();
    serde_json::json!({
        "schema_version": "arda.council.agent_conversation.v1",
        "conversation_id": conversation_id,
        "ts_utc": ts,
        "topic": "ceo-autopilot cycle assessment",
        "speaker_agent": "prometheus",
        "seat": "orchestrator",
        "message_class": "receipt",
        "actionability": "completed_evidence",
        "risk_lane": "read_only",
        "summary": format!(
            "CEO autopilot assessed {} objective candidate(s), selected {}, blocked {}, produced {} plan(s), and held {} plan(s). Council record is evidence only and does not approve execution.",
            objective_selection.objectives_considered,
            selected,
            objective_selection.objectives_blocked_by_gate,
            plans.len(),
            held
        ),
        "related_task": selected,
        "confidence": "medium",
        "source_links": [
            CANONICAL_AUTONOMY_CONFIG,
            "docs/contracts/autonomous-operating-loop-contract.md"
        ],
        "receipt_links": [
            "data/ceo/autopilot.state.json",
            "data/ceo/heartbeats.jsonl"
        ]
    })
}

fn latest_conversation_id(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    content
        .lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find_map(|value| {
            value
                .get("conversation_id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
}

fn load_autonomy_readiness_gate(
    root: &Path,
    hades_introspection: &HadesIntrospectionProjection,
    sovereign_adapters: &SovereignAdapterProjection,
    council_runtime: &CouncilRuntimeProjection,
) -> AutonomyReadinessGateProjection {
    let preflight_path = root.join("data/prometheus/autonomy_operating_loop_preflight.json");
    let cleanup_path = root.join("data/hades/autonomy_cleanup_approval_packets.json");
    let external_source_path = root.join("data/athena/external_source_lane_ledger.jsonl");
    let mut gate = AutonomyReadinessGateProjection {
        preflight_path: preflight_path.display().to_string(),
        cleanup_approval_packets_path: cleanup_path.display().to_string(),
        external_source_lane_ledger_path: external_source_path.display().to_string(),
        evidence_paths: vec![
            preflight_path.display().to_string(),
            cleanup_path.display().to_string(),
            external_source_path.display().to_string(),
            council_runtime.ledger_path.clone(),
            sovereign_adapters.config_path.clone(),
        ],
        ..Default::default()
    };

    let mut hold_reasons = Vec::new();
    let mut human_reasons = Vec::new();

    match read_json_value(&preflight_path) {
        Ok(Some(value)) => {
            gate.preflight_source_available = true;
            match value
                .get("generated_at_utc")
                .and_then(Value::as_str)
                .map(str::parse::<DateTime<Utc>>)
            {
                Some(Ok(generated_at))
                    if generated_at
                        < Utc::now() - ChronoDuration::hours(AUTONOMY_PREFLIGHT_MAX_AGE_HOURS) =>
                {
                    hold_reasons.push("autonomy_preflight_stale".to_string());
                }
                Some(Ok(_)) => {}
                Some(Err(_)) => {
                    hold_reasons.push("autonomy_preflight_generated_at_invalid".to_string())
                }
                None => hold_reasons.push("autonomy_preflight_generated_at_missing".to_string()),
            }
            let summary = value.get("summary").unwrap_or(&Value::Null);
            gate.lane_count = summary
                .get("lane_count")
                .and_then(Value::as_u64)
                .map(|value| value as usize)
                .unwrap_or(0);
            gate.lane_incomplete_count = summary
                .get("lane_incomplete_count")
                .and_then(Value::as_u64)
                .map(|value| value as usize)
                .unwrap_or(0);
            if gate.lane_count == 0 {
                hold_reasons.push("preflight_has_no_configured_lanes".to_string());
            }
            if gate.lane_incomplete_count > 0 {
                hold_reasons.push(format!(
                    "lane_health_incomplete:{}",
                    gate.lane_incomplete_count
                ));
            }
            let missing_stage_count = value
                .pointer("/loop/missing_required_stages")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0);
            if missing_stage_count > 0 {
                hold_reasons.push(format!("loop_required_stage_gaps:{missing_stage_count}"));
            }
        }
        Ok(None) => hold_reasons.push("autonomy_preflight_missing".to_string()),
        Err(err) => hold_reasons.push(format!("autonomy_preflight_unreadable:{err}")),
    }

    match read_json_value(&cleanup_path) {
        Ok(Some(value)) => {
            gate.cleanup_approval_packets_available = true;
            gate.cleanup_packet_count = value
                .get("candidate_count")
                .and_then(Value::as_u64)
                .map(|value| value as usize)
                .unwrap_or_else(|| {
                    value
                        .get("packets")
                        .and_then(Value::as_array)
                        .map(Vec::len)
                        .unwrap_or(0)
                });
            let approval_required = value
                .get("requires_operator_approval_for_mutation")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let cleanup_authorized = value
                .get("cleanup_authorized")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if gate.cleanup_packet_count > 0 && approval_required && !cleanup_authorized {
                human_reasons.push(format!(
                    "hades_cleanup_packets_need_operator_approval:{}",
                    gate.cleanup_packet_count
                ));
            }
            if value
                .get("no_file_moves_or_deletes_performed")
                .and_then(Value::as_bool)
                != Some(true)
            {
                human_reasons.push("hades_cleanup_mutation_boundary_unclear".to_string());
            }
        }
        Ok(None) => hold_reasons.push("hades_cleanup_approval_packets_missing".to_string()),
        Err(err) => hold_reasons.push(format!("hades_cleanup_approval_packets_unreadable:{err}")),
    }

    match read_jsonl_values(&external_source_path) {
        Ok(Some(records)) => {
            gate.external_source_lane_ledger_available = true;
            gate.external_source_lane_count = records.len();
            gate.external_source_blocked_count = records
                .iter()
                .filter(|record| {
                    record
                        .get("task_promotion_allowed")
                        .and_then(Value::as_bool)
                        != Some(true)
                })
                .count();
            if gate.external_source_blocked_count > 0 {
                hold_reasons.push(format!(
                    "external_source_lanes_without_canonical_receipts:{}",
                    gate.external_source_blocked_count
                ));
            }
        }
        Ok(None) => hold_reasons.push("athena_external_source_lane_ledger_missing".to_string()),
        Err(err) => hold_reasons.push(format!(
            "athena_external_source_lane_ledger_unreadable:{err}"
        )),
    }

    if !hades_introspection.no_file_moves_or_deletes_performed
        || hades_introspection.cleanup_authorized
    {
        human_reasons.push("hades_introspection_cleanup_boundary_requires_review".to_string());
    }
    if !hades_introspection.source_available && hades_introspection.error.is_some() {
        hold_reasons.push("hades_introspection_unavailable".to_string());
    }
    if !sovereign_adapters.source_available {
        hold_reasons.push("sovereign_adapter_projection_missing_config".to_string());
    }
    if let Some(err) = &sovereign_adapters.error {
        hold_reasons.push(format!("sovereign_adapter_projection_error:{err}"));
    }
    gate.sovereign_missing_required_adapter_count =
        sovereign_adapters.missing_required_adapter_count;
    if gate.sovereign_missing_required_adapter_count > 0 {
        hold_reasons.push(format!(
            "sovereign_required_adapters_missing:{}",
            gate.sovereign_missing_required_adapter_count
        ));
    }

    gate.council_unresolved_escalation_count =
        count_council_unresolved_escalations(Path::new(&council_runtime.ledger_path));
    if gate.council_unresolved_escalation_count > 0 {
        human_reasons.push(format!(
            "council_unresolved_escalation:{}",
            gate.council_unresolved_escalation_count
        ));
    }
    if let Some(err) = &council_runtime.error {
        hold_reasons.push(format!("council_runtime_error:{err}"));
    }

    gate.human_required = !human_reasons.is_empty();
    if gate.human_required {
        gate.decision = "human_required".to_string();
        gate.task_promotion_allowed = false;
        gate.reasons = human_reasons
            .into_iter()
            .chain(hold_reasons)
            .collect::<Vec<_>>();
    } else if !hold_reasons.is_empty() {
        gate.decision = "hold".to_string();
        gate.task_promotion_allowed = false;
        gate.reasons = hold_reasons;
    } else {
        gate.decision = "allow".to_string();
        gate.task_promotion_allowed = true;
        gate.reasons = vec!["all_required_autonomy_readiness_evidence_present".to_string()];
    }
    gate
}

fn read_json_value(path: &Path) -> Result<Option<Value>, String> {
    match std::fs::read_to_string(path) {
        Ok(content) => serde_json::from_str::<Value>(&content)
            .map(Some)
            .map_err(|err| err.to_string()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err.to_string()),
    }
}

fn read_jsonl_values(path: &Path) -> Result<Option<Vec<Value>>, String> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.to_string()),
    };
    let mut values = Vec::new();
    for (index, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value = serde_json::from_str::<Value>(line)
            .map_err(|err| format!("line {}: {err}", index + 1))?;
        values.push(value);
    }
    Ok(Some(values))
}

fn count_council_unresolved_escalations(path: &Path) -> usize {
    read_jsonl_values(path)
        .ok()
        .flatten()
        .unwrap_or_default()
        .into_iter()
        .filter(|record| {
            matches!(
                record.get("risk_lane").and_then(Value::as_str),
                Some("human_gated" | "external")
            ) || matches!(
                record.get("actionability").and_then(Value::as_str),
                Some("gated_action")
            ) || record
                .get("escalation_required")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || record
                    .get("unresolved_tension")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
        })
        .count()
}

impl CeoAutopilot {
    pub fn new(cfg: AutopilotConfig, registry: AgentRegistry) -> Self {
        let learning = LearningStore::new(&cfg.learning_path).load();
        let validator = PlanValidator {
            joule_budget: cfg.joule_budget,
            max_per_agent: cfg.max_per_agent,
        };
        let oracle = OracleGate {
            joule_threshold: cfg.oracle_joule_threshold,
        };
        let governance_policy = GovernancePolicy::load_from_root(&cfg.root);
        let defaults = load_defaults(cfg.root.join("core"));
        let mut costs = std::collections::BTreeMap::new();
        for (k, v) in defaults.base_costs {
            costs.insert(k, v);
        }
        let decomposer = ObjectiveDecomposer::default().with_base_costs(costs);
        let apollo = CoreExecutorClient::auto(cfg.apollo_socket_path.clone());
        tracing::info!(
            transport = apollo.transport_label(),
            socket = %cfg.apollo_socket_path.display(),
            "ceo autopilot apollo dispatch transport selected",
        );
        Self {
            cfg,
            decomposer,
            validator,
            registry,
            learning,
            oracle,
            governance_policy,
            apollo,
            consecutive_failures: 0,
        }
    }

    pub fn from_world(cfg: AutopilotConfig) -> Self {
        let registry = load_registry_from_world(&cfg.world_path, 600);
        Self::new(cfg, registry)
    }

    pub fn config(&self) -> &AutopilotConfig {
        &self.cfg
    }
    pub fn registry_mut(&mut self) -> &mut AgentRegistry {
        &mut self.registry
    }

    pub fn read_objectives(&self) -> Vec<Objective> {
        let Ok(content) = std::fs::read_to_string(&self.cfg.objectives_path) else {
            return Vec::new();
        };
        content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str::<Objective>(l).ok())
            .collect()
    }

    pub async fn run_cycle(&mut self) -> CycleReport {
        let prior_transport = self.apollo.transport_label();
        if self
            .apollo
            .refresh_transport(self.cfg.apollo_socket_path.clone())
        {
            tracing::info!(
                previous_transport = prior_transport,
                transport = self.apollo.transport_label(),
                socket = %self.cfg.apollo_socket_path.display(),
                "ceo autopilot apollo dispatch transport refreshed",
            );
        }
        let observer = OutcomeObserver::new(&self.cfg.outcome_cursor_path);
        let mut outcomes_ingested = if self.cfg.read_only {
            0
        } else {
            observer.ingest(&self.cfg.queue_path, &mut self.registry, &mut self.learning)
        };

        if !self.cfg.read_only {
            let plan_step_dispatches = self.execute_pending_plan_steps().await;
            if !plan_step_dispatches.is_empty() {
                outcomes_ingested +=
                    observer.ingest(&self.cfg.queue_path, &mut self.registry, &mut self.learning);
            }
        }

        let queue = TaskQueueAnalyzer::new(&self.cfg.queue_path).analyze();
        let services = ServiceHealthMonitor {
            systemd: UserSystemd,
            pattern: self.cfg.systemd_pattern.clone(),
        }
        .collect();
        let dashboard = build_snapshot(&queue, &services);

        let (approved_objectives, h2a) = if self.cfg.read_only {
            (Vec::new(), H2AProcessReport::default())
        } else {
            process_h2a_responses(&self.cfg.a2h_pending_path, &self.cfg.h2a_path)
                .unwrap_or_default()
        };
        let inbox_objectives = self.read_objectives();
        let inbox_had_objectives = !inbox_objectives.is_empty();
        let mut cycle_delegated_joules = 0.0;
        let hourly_delegated_joules = hourly_delegated_joules(&self.cfg.heartbeat_path);
        let (cycle_objectives, objective_selection) = select_cycle_objectives(
            &self.cfg,
            &self.decomposer,
            &self.governance_policy,
            approved_objectives,
            inbox_objectives,
        );
        let hades_introspection = load_hades_introspection(&self.cfg.root);
        let preliminary_sovereign_adapters =
            load_sovereign_adapters(&self.cfg.root, &self.cfg, &[], &h2a);
        let preliminary_council_runtime =
            load_council_runtime(&self.cfg.root, true, &objective_selection, &[]);
        let autonomy_readiness = load_autonomy_readiness_gate(
            &self.cfg.root,
            &hades_introspection,
            &preliminary_sovereign_adapters,
            &preliminary_council_runtime,
        );
        let mut plans = Vec::new();
        for cycle_obj in &cycle_objectives {
            let obj = &cycle_obj.objective;
            let plan = self.decomposer.decompose(obj);
            let validation = self.validator.validate(&plan);
            let mut governance = self.governance_policy.classify_objective(obj, &plan);

            // Oracle governance gate runs after structural validation. Triad-quorum
            // action classes force an ORACLE verdict so quorum evidence can be
            // propagated back into the final GovernanceDecision.
            let mut gate = if cycle_obj.human_approved {
                GateDecision::Approved { resonance: 1.0 }
            } else if validation.ok
                && (governance.requires_triad || !governance.blocks_delegation())
            {
                let (gate, triad_evidence) = self.oracle.evaluate_with_quorum_evidence(
                    obj,
                    &plan,
                    self.governance_policy.triad_quorum_ratio,
                    self.governance_policy.triad_required_pass_rate,
                    governance.requires_triad,
                );
                if governance.requires_triad {
                    governance = self
                        .governance_policy
                        .classify_objective_with_triad_evidence(obj, &plan, triad_evidence);
                }
                gate
            } else if governance.blocks_delegation() {
                GateDecision::Rejected {
                    resonance: 0.0,
                    concerns: governance.reasons.clone(),
                }
            } else {
                GateDecision::Skipped
            };

            if governance.blocks_delegation() && !cycle_obj.human_approved {
                gate = GateDecision::Rejected {
                    resonance: 0.0,
                    concerns: governance.reasons.clone(),
                };
            }

            let mut delegation = None;
            let mut queued_ids = Vec::new();
            let mut queue_operation = None;
            let mut apollo_dispatches = Vec::new();
            let mut a2h_emitted = false;
            let mut joule_limited = false;
            let mut pipeline_submitted = false;

            if (gate.requires_escalation() || governance.requires_escalation())
                && !self.cfg.read_only
            {
                let request_id = uuid::Uuid::new_v4();
                let msg = authorize_for_escalation_with_id(request_id, obj, &gate);
                let _ = write_message(&self.cfg.a2h_path, &msg);
                let _ = append_pending_authorization(
                    &self.cfg.a2h_pending_path,
                    request_id,
                    obj,
                    &gate,
                );
                a2h_emitted = true;
            }

            let plan_joules = plan.iter().map(|task| task.joule_cost).sum::<f64>();
            if validation.ok
                && gate.allows_delegation()
                && (governance.allowed_to_delegate || cycle_obj.human_approved)
                && !self.cfg.read_only
            {
                let operator_approved = cycle_obj.objective_packet.approval_packet_id.is_some();
                if !autonomy_readiness.task_promotion_allowed && !operator_approved {
                    let mut queue_packet = cycle_obj.objective_packet.clone();
                    queue_packet.canonical_queue_mutation_allowed =
                        queue_packet.approval_packet_id.is_some();
                    queue_operation = Some(QueueOperation::blocked(
                        format!("queue_operation:{}", queue_packet.packet_id),
                        &queue_packet,
                        &self.cfg.queue_path,
                        self.cfg.read_only,
                        QueueOperationStatus::BlockedAutonomyReadiness,
                        format!(
                            "autonomy_readiness_gate_{}:{}",
                            autonomy_readiness.decision,
                            autonomy_readiness.reasons.join(";")
                        ),
                    ));
                } else if cycle_delegated_joules + plan_joules > self.cfg.joule_cycle_limit
                    || hourly_delegated_joules + cycle_delegated_joules + plan_joules
                        > self.cfg.joule_hourly_limit
                {
                    joule_limited = true;
                } else {
                    if self.cfg.pipeline_submit_enabled {
                        pipeline_submitted =
                            submit_plan_to_pipeline(&self.cfg.root, obj, &plan, &gate)
                                .await
                                .is_ok();
                    }
                    let d = delegate_plan(&mut self.registry, &self.learning, &plan);
                    let oracle_conditions = match &gate {
                        GateDecision::Conditional { concerns, .. } => concerns.as_slice(),
                        _ if cycle_obj.human_approved => cycle_obj.human_conditions.as_slice(),
                        _ => &[],
                    };
                    let mut queue_packet = cycle_obj.objective_packet.clone();
                    queue_packet.canonical_queue_mutation_allowed =
                        queue_packet.approval_packet_id.is_some();
                    let operation = append_approved_packet_plan(
                        &self.cfg.queue_path,
                        &queue_packet,
                        &obj.id,
                        &plan,
                        Some(&d),
                        oracle_conditions,
                        &autonomy_readiness.decision,
                        &autonomy_readiness.reasons,
                        self.cfg.read_only,
                    );
                    let ids = if operation.result_status == QueueOperationStatus::Appended {
                        operation.appended_task_ids.clone()
                    } else {
                        Vec::new()
                    };
                    queue_operation = Some(operation);

                    // Dispatch operational tasks through Apollo.
                    for (pt, qid) in plan.iter().zip(ids.iter()) {
                        if !is_apollo_dispatchable(&pt.task_type) {
                            continue;
                        }
                        let max_attempts = self.cfg.apollo_max_attempts.max(1);
                        for attempt in 1..=max_attempts {
                            let attempt_qid = qid.clone();
                            let dr = executor_dispatch(
                                &self.apollo,
                                &attempt_qid,
                                pt,
                                oracle_conditions,
                            )
                            .await;
                            let _ = append_apollo_dispatch_attempt_to_queue(
                                &self.cfg.queue_path,
                                &obj.id,
                                pt,
                                &dr,
                                attempt,
                                max_attempts,
                            );
                            let should_retry = dispatch_retryable(&dr) && attempt < max_attempts;
                            let final_failure = dispatch_retryable(&dr) && attempt == max_attempts;
                            if final_failure {
                                self.escalate_failed_apollo_dispatch(&attempt_qid, pt, &dr);
                            }
                            apollo_dispatches.push(dr);
                            if !should_retry {
                                break;
                            }
                        }
                    }
                    outcomes_ingested += observer.ingest(
                        &self.cfg.queue_path,
                        &mut self.registry,
                        &mut self.learning,
                    );

                    delegation = Some(d);
                    queued_ids = ids;
                    cycle_delegated_joules += plan_joules;
                }
            }

            plans.push(PlanCycle {
                objective_id: obj.id.clone(),
                plan,
                validation,
                governance,
                gate,
                autonomy_readiness_decision: autonomy_readiness.decision.clone(),
                autonomy_readiness_reasons: autonomy_readiness.reasons.clone(),
                delegation,
                queued_task_ids: queued_ids,
                queue_operation,
                apollo_dispatches,
                a2h_emitted,
                joule_limited,
                pipeline_submitted,
            });
        }

        let sovereign_adapters = load_sovereign_adapters(&self.cfg.root, &self.cfg, &plans, &h2a);
        let council_runtime = load_council_runtime(
            &self.cfg.root,
            self.cfg.read_only,
            &objective_selection,
            &plans,
        );

        let mut report = CycleReport {
            timestamp: Utc::now().to_rfc3339(),
            queue,
            services,
            dashboard,
            objective_selection,
            objectives_processed: cycle_objectives.len(),
            plans,
            outcomes_ingested,
            h2a,
            hades_introspection,
            sovereign_adapters,
            council_runtime,
            autonomy_readiness,
            report_path: None,
            weekly_report_path: None,
        };
        if !self.cfg.read_only {
            if let Ok(path) = write_daily_report(
                &self.cfg.report_dir,
                &self.cfg.heartbeat_path,
                &self.cfg.learning_path,
                &report,
            ) {
                report.report_path = Some(path.to_string_lossy().to_string());
            }
            if let Ok(path) = write_weekly_report(
                &self.cfg.report_dir,
                &self.cfg.heartbeat_path,
                &self.cfg.learning_path,
                &report,
            ) {
                report.weekly_report_path = Some(path.to_string_lossy().to_string());
            }
        }
        if !self.cfg.read_only {
            self.persist(&report);
            if inbox_had_objectives {
                let _ = std::fs::write(&self.cfg.objectives_path, "");
            }
        }

        report
    }

    fn persist(&self, report: &CycleReport) {
        self.persist_state_snapshot(report);
        let _ = LearningStore::new(&self.cfg.learning_path).save(&self.learning);
        rotate_heartbeat(&self.cfg.heartbeat_path, self.cfg.heartbeat_max_bytes);
        if let Some(p) = self.cfg.heartbeat_path.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.cfg.heartbeat_path)
        {
            self.write_heartbeat_summary(report, &mut f);
        }
    }

    fn persist_state_snapshot(&self, report: &CycleReport) {
        if let Some(p) = self.cfg.state_path.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        if let Ok(json) = serde_json::to_vec_pretty(report) {
            let _ = std::fs::write(&self.cfg.state_path, json);
        }
    }

    fn write_heartbeat_summary(&self, report: &CycleReport, f: &mut std::fs::File) {
        use std::io::Write;
        let delegated_joules = clean_zero(
            report
                .plans
                .iter()
                .filter(|p| !p.joule_limited && !p.queued_task_ids.is_empty())
                .flat_map(|p| p.plan.iter())
                .map(|task| task.joule_cost)
                .sum::<f64>(),
        );
        let governance_held = report
            .plans
            .iter()
            .filter(|p| {
                p.governance.blocks_delegation() || !p.gate.allows_delegation() || p.joule_limited
            })
            .count();
        let governance_escalated = report
            .plans
            .iter()
            .filter(|p| {
                p.a2h_emitted || p.governance.requires_escalation() || p.gate.requires_escalation()
            })
            .count();
        let governance_human_required = report
            .plans
            .iter()
            .filter(|p| p.governance.requires_human)
            .count();
        let governance_triad_required = report
            .plans
            .iter()
            .filter(|p| {
                matches!(
                    p.governance.gate,
                    super::governance_policy::GovernanceGate::TriadQuorumRequired
                )
            })
            .count();
        let governance_triad_approved = report
            .plans
            .iter()
            .filter(|p| {
                matches!(
                    p.governance.gate,
                    super::governance_policy::GovernanceGate::TriadQuorumApproved
                )
            })
            .count();
        let governance_hades_review_required = report
            .plans
            .iter()
            .filter(|p| {
                matches!(
                    p.governance.gate,
                    super::governance_policy::GovernanceGate::HadesReviewRequired
                )
            })
            .count();
        let governance_read_only_benchmark_required = report
            .plans
            .iter()
            .filter(|p| {
                matches!(
                    p.governance.gate,
                    super::governance_policy::GovernanceGate::ReadOnlyBenchmarkRequired
                )
            })
            .count();
        let governance_classes =
            report
                .plans
                .iter()
                .fold(serde_json::Map::new(), |mut classes, plan| {
                    let entry = classes
                        .entry(plan.governance.action_class.clone())
                        .or_insert_with(|| serde_json::json!(0));
                    let next = entry.as_u64().unwrap_or(0) + 1;
                    *entry = serde_json::json!(next);
                    classes
                });
        let summary = serde_json::json!({
            "ts": report.timestamp,
            "queue_pending": report.queue.pending,
            "queue_in_progress": report.queue.in_progress,
            "completion_rate_24h": report.queue.completion_rate_24h,
            "services_failed": report.services.failed,
            "service_score": report.services.overall_score,
            "alerts": report.dashboard.alerts.len(),
            "objectives": report.objectives_processed,
            "objectives_considered": report.objective_selection.objectives_considered,
            "objectives_selected": report.objective_selection.objectives_selected,
            "objectives_blocked_by_gate": report.objective_selection.objectives_blocked_by_gate,
            "effective_queue_open_count": report.objective_selection.effective_queue_open_count,
            "stale_raw_queue_record_count": report.objective_selection.stale_raw_queue_record_count,
            "objective_selection_status": report.objective_selection.status,
            "selected_objective_id": report.objective_selection.selected_objective_id,
            "next_recommended_action": report.objective_selection.next_recommended_action,
            "outcomes_ingested": report.outcomes_ingested,
            "h2a_responses_processed": report.h2a.responses_processed,
            "h2a_objectives_resumed": report.h2a.objectives_resumed,
            "h2a_denials_recorded": report.h2a.denials_recorded,
            "report_path": report.report_path,
            "weekly_report_path": report.weekly_report_path,
            "delegated_joules": delegated_joules,
            "plans_queued": report.plans.iter().map(|p| p.queued_task_ids.len()).sum::<usize>(),
            "pipeline_submissions": report.plans.iter().filter(|p| p.pipeline_submitted).count(),
            "apollo_dispatches": report.plans.iter().map(|p| p.apollo_dispatches.len()).sum::<usize>(),
            "a2h_escalations": report.plans.iter().filter(|p| p.a2h_emitted).count(),
            "governance_held": governance_held,
            "governance_escalated": governance_escalated,
            "governance_human_required": governance_human_required,
            "governance_triad_required": governance_triad_required,
            "governance_triad_approved": governance_triad_approved,
            "governance_hades_review_required": governance_hades_review_required,
            "governance_read_only_benchmark_required": governance_read_only_benchmark_required,
            "governance_classes": governance_classes,
            "sovereign_adapter_count": report.sovereign_adapters.adapter_count,
            "sovereign_active_runtime_adapter_count": report.sovereign_adapters.active_runtime_adapter_count,
            "sovereign_evidence_only_adapter_count": report.sovereign_adapters.evidence_only_adapter_count,
            "sovereign_missing_required_adapter_count": report.sovereign_adapters.missing_required_adapter_count,
            "council_existing_record_count": report.council_runtime.existing_record_count,
            "council_appended_record_count": report.council_runtime.appended_record_count,
            "council_task_promotion_allowed": report.council_runtime.task_promotion_allowed,
            "autonomy_readiness_decision": report.autonomy_readiness.decision,
            "autonomy_readiness_task_promotion_allowed": report.autonomy_readiness.task_promotion_allowed,
            "autonomy_readiness_reasons": report.autonomy_readiness.reasons,
        });
        let _ = writeln!(f, "{}", summary);
    }

    async fn execute_pending_plan_steps(&self) -> Vec<Dispatch> {
        let steps = load_pending_plan_steps(&self.cfg.queue_path);
        let mut dispatches = Vec::new();
        for step in steps {
            let max_attempts = self.cfg.apollo_max_attempts.max(1);
            for attempt in 1..=max_attempts {
                let attempt_qid = if attempt == 1 {
                    step.queue_id.clone()
                } else {
                    format!("{}__retry{}", step.queue_id, attempt)
                };
                let dispatch = executor_dispatch(&self.apollo, &attempt_qid, &step.plan, &[]).await;
                let appended = append_apollo_dispatch_attempt_to_queue(
                    &self.cfg.queue_path,
                    &step.objective_id,
                    &step.plan,
                    &dispatch,
                    attempt,
                    max_attempts,
                )
                .unwrap_or(false);
                if !appended {
                    let _ = append_skipped_plan_step_to_queue(
                        &self.cfg.queue_path,
                        &step,
                        &dispatch,
                        attempt,
                        max_attempts,
                    );
                }
                let should_retry = dispatch_retryable(&dispatch) && attempt < max_attempts;
                let final_failure = dispatch_retryable(&dispatch) && attempt == max_attempts;
                if final_failure || !appended {
                    self.escalate_failed_apollo_dispatch(&attempt_qid, &step.plan, &dispatch);
                }
                dispatches.push(dispatch);
                if !should_retry {
                    break;
                }
            }
        }
        dispatches
    }

    pub fn observe_outcome(
        &mut self,
        agent: &str,
        task_type: &str,
        success: bool,
        duration_secs: f64,
        joules: f64,
    ) {
        self.learning
            .observe(agent, task_type, success, duration_secs, joules);
        self.registry.record_completed(agent, success);
    }

    fn escalate_failed_apollo_dispatch(
        &self,
        task_id: &str,
        plan: &PlannedTask,
        dispatch: &Dispatch,
    ) {
        let Ok(store) = OrderStore::new(self.cfg.root.join("data/prometheus")) else {
            return;
        };
        let order_id = uuid::Uuid::new_v4();
        let reason = match dispatch {
            Dispatch::Submitted {
                status, transport, ..
            } => {
                format!(
                    "Apollo dispatch exhausted retries for queue task {task_id}: status={status:?} transport={transport}"
                )
            }
            Dispatch::Skipped { reason } => {
                format!("Apollo dispatch skipped queue task {task_id}: {reason}")
            }
        };
        let _ = store.append_order(
            order_id,
            &plan.task_type,
            OrderStatus::Escalated,
            plan.assigned_agent.as_deref(),
            &reason,
        );
        let _ = store.append_escalation(order_id, &reason, 0.0);
    }
}

fn clean_zero(value: f64) -> f64 {
    if value == 0.0 {
        0.0
    } else {
        value
    }
}

struct CycleObjective {
    objective: Objective,
    objective_packet: ObjectivePacket,
    human_approved: bool,
    human_conditions: Vec<String>,
}

struct ObjectiveCandidate {
    objective: Objective,
    report: ObjectiveCandidateReport,
    human_approved: bool,
    human_conditions: Vec<String>,
    requires_review: bool,
}

const OBJECTIVE_SELECTION_CONTRACT: &str = "arda.arandur.objective_selection.v1";

fn select_cycle_objectives(
    cfg: &AutopilotConfig,
    decomposer: &ObjectiveDecomposer,
    governance_policy: &GovernancePolicy,
    approved_objectives: Vec<HumanApprovedObjective>,
    inbox_objectives: Vec<Objective>,
) -> (Vec<CycleObjective>, ObjectiveSelectionReport) {
    let source_registry = SourceRegistry::arandur_with_queue(&cfg.root, &cfg.queue_path);
    let h2a_source_path = source_registry
        .by_contract("arda.h2a.approvals.v1")
        .map(|source| source.path.to_string_lossy().to_string())
        .unwrap_or_else(|| cfg.h2a_path.to_string_lossy().to_string());
    let objective_inbox_source_path = source_registry
        .by_contract("arda.prometheus.objective_inbox.v1")
        .map(|source| source.path.to_string_lossy().to_string())
        .unwrap_or_else(|| cfg.objectives_path.to_string_lossy().to_string());
    let canonical_queue_source_path = source_registry
        .by_contract("arda.canonical_task_queue.v1")
        .map(|source| source.path.to_string_lossy().to_string())
        .unwrap_or_else(|| cfg.queue_path.to_string_lossy().to_string());
    let (effective_queue_records, stale_raw_queue_record_count) =
        effective_open_queue_records(&cfg.queue_path);
    let mut candidates = Vec::new();

    for approved in approved_objectives {
        let source_record_id = approved.objective.id.clone();
        let candidate_id = approved.objective.id.clone();
        let title = approved.objective.statement.clone();
        candidates.push(ObjectiveCandidate {
            objective: approved.objective,
            report: ObjectiveCandidateReport {
                source_path: h2a_source_path.clone(),
                source_record_id,
                candidate_id,
                title,
                effective_status: "approved".into(),
                owner: Some("human".into()),
                priority: Some("critical".into()),
                governance_class: "human_approved".into(),
                review_gate: GovernanceGate::SafeAutonomous,
                blocked_reason_code: None,
                approval_packet_id: Some(format!("h2a:{}", approved.request_id)),
                completion_receipt_path: None,
                selected_reason: None,
                rejection_reason: None,
            },
            human_approved: true,
            human_conditions: approved.conditions,
            requires_review: false,
        });
    }

    for objective in inbox_objectives {
        let source_record_id = objective.id.clone();
        let candidate_id = objective.id.clone();
        let title = objective.statement.clone();
        candidates.push(ObjectiveCandidate {
            objective,
            report: ObjectiveCandidateReport {
                source_path: objective_inbox_source_path.clone(),
                source_record_id,
                candidate_id,
                title,
                effective_status: "inbox".into(),
                owner: Some("prometheus".into()),
                priority: Some("medium".into()),
                governance_class: "unclassified".into(),
                review_gate: GovernanceGate::ReviewRequired,
                blocked_reason_code: None,
                approval_packet_id: None,
                completion_receipt_path: None,
                selected_reason: None,
                rejection_reason: None,
            },
            human_approved: false,
            human_conditions: Vec::new(),
            requires_review: false,
        });
    }

    for candidate in arandur_recommendation_candidates(&cfg.arandur_recommendations_path) {
        candidates.push(candidate);
    }

    for record in effective_queue_records {
        if let Some(objective) = objective_from_queue_record(&record) {
            let candidate_id = objective.id.clone();
            let title = objective.statement.clone();
            candidates.push(ObjectiveCandidate {
                objective,
                report: ObjectiveCandidateReport {
                    source_path: canonical_queue_source_path.clone(),
                    source_record_id: TaskQueueAnalyzer::effective_record_key(&record),
                    candidate_id,
                    title,
                    effective_status: record.status.unwrap_or_else(|| "unknown".into()),
                    owner: record.owner,
                    priority: record.priority,
                    governance_class: "unclassified".into(),
                    review_gate: GovernanceGate::ReviewRequired,
                    blocked_reason_code: None,
                    approval_packet_id: None,
                    completion_receipt_path: None,
                    selected_reason: None,
                    rejection_reason: None,
                },
                human_approved: false,
                human_conditions: Vec::new(),
                requires_review: false,
            });
        }
    }

    let evidence_registry = EvidenceRegistry::from_audit_root(&cfg.root);
    let executed_arandur_candidates = evidence_registry.operator_approved_candidate_receipts();
    candidates.retain(|candidate| {
        !is_superseded_open_arandur_recommendation(candidate, &executed_arandur_candidates)
    });
    let objectives_considered = candidates.len();
    let mut selected = None;
    let mut reports = Vec::new();
    let mut objectives_blocked_by_gate = 0;

    for mut candidate in candidates {
        if candidate.human_approved {
            candidate.report.governance_class = "human_approved".into();
            candidate.report.review_gate = GovernanceGate::SafeAutonomous;
            if selected.is_none() {
                candidate.report.selected_reason =
                    Some("explicit operator approval selected first".into());
                selected = Some((
                    candidate.objective.clone(),
                    true,
                    candidate.human_conditions.clone(),
                ));
            } else {
                candidate.report.rejection_reason =
                    Some("another higher-priority candidate was selected".into());
            }
            reports.push(candidate.report);
            continue;
        }

        if candidate.report.governance_class == "arandur_recommendation" {
            if let Some(receipt_path) =
                candidate
                    .report
                    .approval_packet_id
                    .as_ref()
                    .and_then(|approval_packet_id| {
                        executed_arandur_candidates.get(&(
                            candidate.report.candidate_id.clone(),
                            approval_packet_id.clone(),
                        ))
                    })
            {
                candidate.report.effective_status = "executed_verified".into();
                candidate.report.governance_class = "operator_approved_executed".into();
                candidate.report.review_gate = GovernanceGate::SafeAutonomous;
                candidate.report.blocked_reason_code = None;
                candidate.report.completion_receipt_path = Some(receipt_path.clone());
                candidate.report.rejection_reason = Some(
                    "recognized_completed: execution receipt confirms operator-approved candidate packet already executed; not selecting again"
                        .into(),
                );
                reports.push(candidate.report);
                continue;
            }
        }

        if candidate.requires_review {
            candidate.report.governance_class = "arandur_recommendation".into();
            candidate.report.review_gate = GovernanceGate::ReviewRequired;
            candidate.report.blocked_reason_code =
                Some("review_gated_recommendation_requires_operator_review".into());
            if selected.is_some() {
                candidate.report.rejection_reason =
                    Some("another higher-priority candidate was selected".into());
            } else {
                objectives_blocked_by_gate += 1;
                candidate.report.rejection_reason = Some(
                    "blocked_by_gate:ReviewRequired:Arandur recommendation requires operator review before canonical selection"
                        .into(),
                );
            }
            reports.push(candidate.report);
            continue;
        }

        if candidate.report.governance_class == "arandur_recommendation"
            && candidate.report.approval_packet_id.is_none()
        {
            candidate.report.review_gate = GovernanceGate::ReviewRequired;
            candidate.report.blocked_reason_code = Some("operator_approval_packet_missing".into());
            if selected.is_some() {
                candidate.report.rejection_reason =
                    Some("another higher-priority candidate was selected".into());
            } else {
                objectives_blocked_by_gate += 1;
                candidate.report.rejection_reason = Some(
                    "blocked_by_gate:ReviewRequired:Arandur recommendation requires an explicit operator approval packet before canonical selection"
                        .into(),
                );
            }
            reports.push(candidate.report);
            continue;
        }

        if candidate.report.governance_class == "arandur_recommendation"
            && candidate.report.approval_packet_id.is_some()
        {
            candidate.report.governance_class = "operator_approved".into();
            candidate.report.review_gate = GovernanceGate::SafeAutonomous;
            if selected.is_none() {
                candidate.report.selected_reason = Some(
                    "explicit Arandur operator approval packet selected for supervised delegation"
                        .into(),
                );
                selected = Some((
                    candidate.objective.clone(),
                    true,
                    candidate.human_conditions.clone(),
                ));
            } else {
                candidate.report.rejection_reason =
                    Some("another higher-priority candidate was selected".into());
            }
            reports.push(candidate.report);
            continue;
        }

        let plan = decomposer.decompose(&candidate.objective);
        let governance = governance_policy.classify_objective(&candidate.objective, &plan);
        candidate.report.governance_class = governance.action_class.clone();
        candidate.report.review_gate = governance.gate.clone();

        if selected.is_some() {
            candidate.report.rejection_reason =
                Some("another higher-priority candidate was selected".into());
        } else if objective_gate_allows_supervised_selection(&governance) {
            candidate.report.selected_reason = Some(format!(
                "governance gate {:?} permits supervised delegation",
                governance.gate
            ));
            selected = Some((candidate.objective.clone(), false, Vec::new()));
        } else {
            objectives_blocked_by_gate += 1;
            let reason_code = blocked_reason_code(&governance);
            candidate.report.blocked_reason_code = Some(reason_code.to_string());
            candidate.report.rejection_reason = Some(format!(
                "blocked_by_gate:{:?}:{}",
                governance.gate,
                governance.reasons.join("; ")
            ));
        }
        reports.push(candidate.report);
    }

    let selected_objective_id = selected
        .as_ref()
        .map(|(objective, _, _)| objective.id.clone());
    let blocked_candidate_groups = blocked_candidate_groups(&reports, stale_raw_queue_record_count);
    let next_automation_gate_packet = next_automation_gate_packet(&reports);
    let queue_path_display = cfg.queue_path.to_string_lossy().to_string();
    let objective_packets = objective_packets_from_reports(&source_registry, &reports);
    let objective_packet_report = ObjectivePacketReport::read_only(objective_packets.clone());
    let selected_packet = selected_objective_id.as_ref().and_then(|selected_id| {
        objective_packets
            .iter()
            .find(|packet| &packet.candidate_id == selected_id && packet.selected)
            .cloned()
    });
    let cycle_objectives = match (selected, selected_packet) {
        (Some((objective, human_approved, human_conditions)), Some(objective_packet)) => {
            vec![CycleObjective {
                objective,
                objective_packet,
                human_approved,
                human_conditions,
            }]
        }
        _ => Vec::new(),
    };
    let completed_only = objectives_considered > 0
        && selected_objective_id.is_none()
        && objectives_blocked_by_gate == 0
        && reports
            .iter()
            .all(|report| report.governance_class == "operator_approved_executed");
    let status = if selected_objective_id.is_some() {
        "selected"
    } else if objectives_considered == 0 || completed_only {
        "no_action"
    } else {
        "blocked"
    }
    .to_string();
    let no_selection_reason = if selected_objective_id.is_some() {
        None
    } else if objectives_considered == 0 {
        Some("no_candidates_available".to_string())
    } else if completed_only {
        Some("all_operator_approved_candidates_already_executed".to_string())
    } else {
        Some("all_candidates_blocked_by_governance_gate".to_string())
    };
    let next_recommended_action = match status.as_str() {
        "selected" => "execute selected supervised objective".to_string(),
        "blocked" => {
            "resolve blocked governance gates or provide explicit operator approval".to_string()
        }
        _ if completed_only => "operator-approved Arandur candidates already executed; continue with a fresh approval packet or direct operator instruction".to_string(),
        _ => "add a bounded objective to the objective inbox or canonical queue".to_string(),
    };

    (
        cycle_objectives,
        ObjectiveSelectionReport {
            contract: OBJECTIVE_SELECTION_CONTRACT.into(),
            mutation_policy: if cfg.read_only {
                "read_only_report_only".into()
            } else {
                "supervised_single_objective".into()
            },
            objectives_considered,
            objectives_selected: usize::from(selected_objective_id.is_some()),
            objectives_blocked_by_gate,
            effective_queue_open_count: reports
                .iter()
                .filter(|report| report.source_path == queue_path_display)
                .count(),
            stale_raw_queue_record_count,
            status,
            no_selection_reason,
            selected_objective_id,
            next_recommended_action,
            blocked_candidate_groups,
            next_automation_gate_packet,
            objective_packet_report,
            candidates: reports,
        },
    )
}

fn objective_packets_from_reports(
    source_registry: &SourceRegistry,
    reports: &[ObjectiveCandidateReport],
) -> Vec<ObjectivePacket> {
    reports
        .iter()
        .map(|report| {
            let (source_contract, source_type) =
                source_contract_and_type_for_path(source_registry, &report.source_path);
            ObjectivePacket::from_report(
                source_contract,
                source_type,
                ObjectivePacketInput {
                    source_path: report.source_path.clone(),
                    source_record_id: report.source_record_id.clone(),
                    candidate_id: report.candidate_id.clone(),
                    title: report.title.clone(),
                    owner: report.owner.clone(),
                    priority: report.priority.clone(),
                    governance_class: report.governance_class.clone(),
                    review_gate: report.review_gate.clone(),
                    acceptance_criteria: acceptance_criteria_from_report(
                        report.blocked_reason_code.as_deref(),
                        report.rejection_reason.as_deref(),
                        report.selected_reason.as_deref(),
                        report.completion_receipt_path.as_deref(),
                    ),
                    approval_packet_id: report.approval_packet_id.clone(),
                    completion_receipt_path: report.completion_receipt_path.clone(),
                    blocked_reason_code: report.blocked_reason_code.clone(),
                    selected: report.selected_reason.is_some(),
                },
            )
        })
        .collect()
}

fn objective_gate_allows_supervised_selection(governance: &GovernanceDecision) -> bool {
    matches!(governance.gate, GovernanceGate::SafeAutonomous) && governance.allowed_to_delegate
}

fn effective_open_queue_records(queue_path: &Path) -> (Vec<QueueRecord>, usize) {
    let records = TaskQueueAnalyzer::new(queue_path)
        .load()
        .unwrap_or_default();
    let stale_raw_queue_record_count = stale_open_queue_record_count(&records);
    let open = TaskQueueAnalyzer::effective_records(records)
        .into_iter()
        .filter(|record| is_open_status(record.status.as_deref()))
        .collect();
    (open, stale_raw_queue_record_count)
}

fn stale_open_queue_record_count(records: &[QueueRecord]) -> usize {
    let mut latest_by_source_record_id: BTreeMap<String, (usize, &QueueRecord)> = BTreeMap::new();
    for (index, record) in records.iter().enumerate() {
        latest_by_source_record_id.insert(
            TaskQueueAnalyzer::effective_record_key(record),
            (index, record),
        );
    }
    records
        .iter()
        .enumerate()
        .filter(|(index, record)| {
            let record_key = TaskQueueAnalyzer::effective_record_key(record);
            is_open_status(record.status.as_deref())
                && latest_by_source_record_id
                    .get(&record_key)
                    .map(|(latest_index, latest)| {
                        *latest_index != *index || !is_open_status(latest.status.as_deref())
                    })
                    .unwrap_or(false)
        })
        .count()
}

fn is_open_status(status: Option<&str>) -> bool {
    matches!(
        status,
        Some("pending" | "queued" | "in_progress" | "running" | "active")
    )
}

fn blocked_reason_code(governance: &GovernanceDecision) -> &'static str {
    match governance.gate {
        GovernanceGate::HumanRequired => "unsafe_human_required",
        GovernanceGate::HadesReviewRequired => "hades_lifecycle_review_required",
        GovernanceGate::ReviewRequired => "unknown_action_class",
        GovernanceGate::TriadQuorumRequired => "triad_quorum_required",
        GovernanceGate::ReadOnlyBenchmarkRequired => "read_only_benchmark_required",
        GovernanceGate::SafeAutonomous | GovernanceGate::TriadQuorumApproved => "not_blocked",
    }
}

fn blocked_candidate_groups(
    reports: &[ObjectiveCandidateReport],
    stale_raw_queue_record_count: usize,
) -> Vec<BlockedCandidateGroup> {
    let mut counts: BTreeMap<(String, String, String), usize> = BTreeMap::new();
    if stale_raw_queue_record_count > 0 {
        counts.insert(
            (
                "stale_or_superseded_raw_queue_record".into(),
                "raw_queue_record".into(),
                "superseded".into(),
            ),
            stale_raw_queue_record_count,
        );
    }
    for report in reports {
        let Some(reason_code) = report.blocked_reason_code.as_ref() else {
            continue;
        };
        let key = (
            reason_code.clone(),
            report.governance_class.clone(),
            format!("{:?}", report.review_gate),
        );
        *counts.entry(key).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .map(
            |((reason_code, governance_class, review_gate), count)| BlockedCandidateGroup {
                reason_code,
                governance_class,
                review_gate,
                count,
            },
        )
        .collect()
}

fn is_superseded_open_arandur_recommendation(
    candidate: &ObjectiveCandidate,
    executed_arandur_candidates: &BTreeMap<(String, String), String>,
) -> bool {
    if candidate.report.governance_class != "arandur_recommendation"
        || candidate.report.effective_status != "review_required"
        || candidate.report.approval_packet_id.is_some()
    {
        return false;
    }

    executed_arandur_candidates
        .keys()
        .any(|(candidate_id, _)| candidate_id == &candidate.report.candidate_id)
}

fn next_automation_gate_packet(
    reports: &[ObjectiveCandidateReport],
) -> Option<NextAutomationGatePacket> {
    reports
        .iter()
        .find(|report| {
            report.governance_class == "arandur_recommendation"
                && report.blocked_reason_code.as_deref()
                    == Some("review_gated_recommendation_requires_operator_review")
        })
        .map(|report| NextAutomationGatePacket {
            packet_type: "arandur.next_automation_gate_selection.v1".into(),
            recommendation_id: report.source_record_id.clone(),
            candidate_id: report.candidate_id.clone(),
            title: report.title.clone(),
            owner: report.owner.clone(),
            priority: report.priority.clone(),
            requires_operator_approval: true,
            canonical_queue_mutation_allowed: false,
            approval_packet_required: true,
            approval_packet_schema:
                "approval_packet:{approval_id,status=approved,approved_by,approved_at}".into(),
        })
}

fn objective_from_queue_record(record: &QueueRecord) -> Option<Objective> {
    let statement = record
        .title
        .clone()
        .or_else(|| record.result.clone())
        .unwrap_or_else(|| record.id.clone());
    if statement.trim().is_empty() {
        return None;
    }
    let mut tags = Vec::new();
    if let Some(scope) = record.extra.get("scope").and_then(Value::as_str) {
        tags.push(format!("scope:{scope}"));
    }
    if let Some(action_class) = record.extra.get("action_class").and_then(Value::as_str) {
        tags.push(format!("action_class:{action_class}"));
    }
    Some(Objective {
        id: record.id.clone(),
        statement,
        constraints: Vec::new(),
        deadline: None,
        success_criteria: Vec::new(),
        tags,
    })
}

fn arandur_recommendation_candidates(path: &Path) -> Vec<ObjectiveCandidate> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            let value = serde_json::from_str::<Value>(trimmed).ok()?;
            if value.get("review_status").and_then(Value::as_str) == Some("rejected") {
                return None;
            }
            let recommendation_id = value
                .get("recommendation_id")
                .and_then(Value::as_str)
                .or_else(|| {
                    value
                        .get("recommended_candidate_id")
                        .and_then(Value::as_str)
                })?
                .to_string();
            let candidate = value.get("candidate");
            let candidate_id = candidate
                .and_then(|candidate| candidate.get("id"))
                .and_then(Value::as_str)
                .unwrap_or(recommendation_id.as_str())
                .to_string();
            let title = candidate
                .and_then(|candidate| candidate.get("title"))
                .and_then(Value::as_str)
                .or_else(|| value.get("recommended_action").and_then(Value::as_str))
                .unwrap_or(candidate_id.as_str())
                .to_string();
            if title.trim().is_empty() {
                return None;
            }
            let owner = candidate
                .and_then(|candidate| candidate.get("owner"))
                .and_then(Value::as_str)
                .map(str::to_string);
            let priority = candidate
                .and_then(|candidate| candidate.get("priority"))
                .and_then(Value::as_str)
                .map(str::to_string);
            let review_required = value
                .get("review_required")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let effective_status = if review_required {
                "review_required"
            } else {
                candidate
                    .and_then(|candidate| candidate.get("status"))
                    .and_then(Value::as_str)
                    .unwrap_or("recommended")
            }
            .to_string();
            Some(ObjectiveCandidate {
                objective: Objective {
                    id: candidate_id.clone(),
                    statement: title.clone(),
                    constraints: Vec::new(),
                    deadline: None,
                    success_criteria: Vec::new(),
                    tags: vec!["source:arandur_recommendation".into()],
                },
                report: ObjectiveCandidateReport {
                    source_path: path.to_string_lossy().to_string(),
                    source_record_id: recommendation_id,
                    candidate_id,
                    title,
                    effective_status,
                    owner,
                    priority,
                    governance_class: "arandur_recommendation".into(),
                    review_gate: if review_required {
                        GovernanceGate::ReviewRequired
                    } else {
                        GovernanceGate::SafeAutonomous
                    },
                    blocked_reason_code: None,
                    approval_packet_id: approved_arandur_packet_id(&value),
                    completion_receipt_path: None,
                    selected_reason: None,
                    rejection_reason: None,
                },
                human_approved: false,
                human_conditions: Vec::new(),
                requires_review: review_required,
            })
        })
        .collect()
}

fn approved_arandur_packet_id(value: &Value) -> Option<String> {
    let packet = value.get("approval_packet")?;
    let status = packet.get("status").and_then(Value::as_str)?;
    if status != "approved" {
        return None;
    }
    let approved_by = packet.get("approved_by").and_then(Value::as_str)?;
    if approved_by.trim().is_empty() {
        return None;
    }
    let approved_at = packet.get("approved_at").and_then(Value::as_str)?;
    if approved_at.trim().is_empty() {
        return None;
    }
    packet
        .get("id")
        .or_else(|| packet.get("approval_id"))
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .map(str::to_string)
}

#[derive(Debug, Clone)]
struct PendingPlanStep {
    queue_id: String,
    objective_id: String,
    plan: PlannedTask,
}

fn load_pending_plan_steps(queue_path: &Path) -> Vec<PendingPlanStep> {
    let Ok(content) = std::fs::read_to_string(queue_path) else {
        return Vec::new();
    };
    pending_plan_steps_from_lines(content.lines())
}

fn pending_plan_steps_from_lines<'a>(lines: impl Iterator<Item = &'a str>) -> Vec<PendingPlanStep> {
    let mut latest_status = std::collections::BTreeMap::<String, String>::new();
    let mut latest_records = std::collections::BTreeMap::<String, Value>::new();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        let Some(id) = value.get("id").and_then(Value::as_str) else {
            continue;
        };
        if let Some(status) = value.get("status").and_then(Value::as_str) {
            latest_status.insert(id.to_string(), status.to_string());
            latest_records.insert(id.to_string(), value);
        }
    }

    latest_records
        .into_iter()
        .filter_map(|(id, value)| {
            let status = latest_status
                .get(&id)
                .map(String::as_str)
                .unwrap_or("unknown");
            if status != "pending" && status != "queued" {
                return None;
            }
            let step = pending_plan_step_from_value(&id, &value)?;
            if is_apollo_dispatchable(&step.plan.task_type) {
                Some(step)
            } else {
                None
            }
        })
        .collect()
}

fn pending_plan_step_from_value(id: &str, value: &Value) -> Option<PendingPlanStep> {
    let plan_id = value.get("plan_id").and_then(Value::as_str).or_else(|| {
        value
            .get("meta")
            .and_then(|meta| meta.get("objective_id"))
            .and_then(Value::as_str)
    })?;
    let step_index = value
        .get("plan_step_index")
        .and_then(Value::as_u64)
        .or_else(|| {
            value
                .get("meta")
                .and_then(|meta| meta.get("plan_step_index"))
                .and_then(Value::as_u64)
        });
    let plan_key = value
        .get("meta")
        .and_then(|meta| meta.get("plan_key"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| step_index.map(|idx| format!("step_{idx}")))?;
    let task_type = value.get("task_type").and_then(Value::as_str)?.to_string();
    let title = value
        .get("title")
        .and_then(Value::as_str)
        .or_else(|| value.get("description").and_then(Value::as_str))
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("{plan_id}#{plan_key}: {task_type}"));
    let assigned_agent = value
        .get("assigned_agent")
        .and_then(Value::as_str)
        .or_else(|| value.get("owner").and_then(Value::as_str))
        .map(ToOwned::to_owned);
    let priority = value
        .get("priority")
        .and_then(Value::as_str)
        .map(parse_priority)
        .unwrap_or(Priority::Medium);
    let depends_on = value
        .get("depends_on")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let joule_cost = value
        .get("joule_cost_estimate")
        .and_then(Value::as_f64)
        .or_else(|| value.get("joule_cost_estimated").and_then(Value::as_f64))
        .unwrap_or(0.0);
    let eta_seconds = value
        .get("eta_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(30);

    Some(PendingPlanStep {
        queue_id: id.to_string(),
        objective_id: plan_id.to_string(),
        plan: PlannedTask {
            key: plan_key,
            title,
            task_type,
            depends_on,
            priority,
            joule_cost,
            eta_seconds,
            assigned_agent,
        },
    })
}

fn parse_priority(raw: &str) -> Priority {
    match raw.to_ascii_lowercase().as_str() {
        "low" => Priority::Low,
        "high" => Priority::High,
        "critical" => Priority::Critical,
        _ => Priority::Medium,
    }
}

fn append_skipped_plan_step_to_queue(
    queue_path: &Path,
    step: &PendingPlanStep,
    dispatch: &Dispatch,
    attempt: u32,
    max_attempts: u32,
) -> std::io::Result<()> {
    if let Some(parent) = queue_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(queue_path)?;
    let now = Utc::now();
    let reason = match dispatch {
        Dispatch::Skipped { reason } => reason.clone(),
        Dispatch::Submitted { status, .. } => format!("apollo status {status:?} was not appended"),
    };
    let record = serde_json::json!({
        "id": step.queue_id,
        "title": step.plan.title,
        "owner": step.plan.assigned_agent.clone().unwrap_or_else(|| "ceo".to_string()),
        "priority": format!("{:?}", step.plan.priority).to_lowercase(),
        "status": "failed",
        "result": reason,
        "task_type": step.plan.task_type,
        "depends_on": step.plan.depends_on,
        "joule_cost_estimate": step.plan.joule_cost,
        "queued_at_utc": now.to_rfc3339(),
        "started_at_utc": now.to_rfc3339(),
        "completed_at_utc": now.to_rfc3339(),
        "meta": {
            "origin": "ceo_autopilot_plan_step_worker",
            "objective_id": step.objective_id,
            "plan_key": step.plan.key,
            "apollo": true,
            "retry_attempt": attempt,
            "retry_max_attempts": max_attempts,
        },
        "glyphs": ["∇", "⚡"],
    });
    use std::io::Write;
    writeln!(f, "{}", record)
}

fn dispatch_retryable(dispatch: &Dispatch) -> bool {
    matches!(
        dispatch,
        Dispatch::Submitted {
            status: ExecutionStatus::Failed | ExecutionStatus::Cancelled | ExecutionStatus::Timeout,
            ..
        }
    )
}

/// Resolve the Apollo IPC socket path used for autopilot dispatch.
///
/// Order: explicit `ARDA_APOLLO_SOCKET` env var > `<root>/data/apollo/apollo.sock`.
fn apollo_socket_default(root: &Path) -> PathBuf {
    if let Ok(raw) = std::env::var("ARDA_APOLLO_SOCKET") {
        if !raw.is_empty() {
            return PathBuf::from(raw);
        }
    }
    root.join("data/apollo/apollo.sock")
}

fn hourly_delegated_joules(heartbeat_path: &Path) -> f64 {
    let Ok(content) = std::fs::read_to_string(heartbeat_path) else {
        return 0.0;
    };
    let now = Utc::now();
    content
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|row| {
            row.get("ts")
                .and_then(|value| value.as_str())
                .and_then(|raw| chrono::DateTime::parse_from_rfc3339(raw).ok())
                .map(|ts| now - ts.with_timezone(&Utc) <= chrono::Duration::hours(1))
                .unwrap_or(false)
        })
        .filter_map(|row| row.get("delegated_joules").and_then(|value| value.as_f64()))
        .sum()
}

fn rotate_heartbeat(path: &Path, max_bytes: u64) {
    let Ok(meta) = std::fs::metadata(path) else {
        return;
    };
    if meta.len() < max_bytes {
        return;
    }
    let stamp = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let rotated = path.with_extension(format!("jsonl.{stamp}"));
    let _ = std::fs::rename(path, rotated);
}

pub async fn ceo_loop(mut autopilot: CeoAutopilot, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::SeqCst) {
        let pause_flag = autopilot.cfg.root.join("tmp/ceo/pause.flag");
        let breaker_flag = autopilot.cfg.circuit_breaker_path.clone();

        if breaker_flag.exists() {
            tracing::warn!(
                consecutive_failures = autopilot.consecutive_failures,
                limit = autopilot.cfg.consecutive_failure_limit,
                "ceo_loop circuit_breaker active"
            );
            tokio::time::sleep(autopilot.cfg.pause_poll_interval).await;
            continue;
        }
        if pause_flag.exists() {
            tokio::time::sleep(autopilot.cfg.pause_poll_interval).await;
            continue;
        }

        let report = autopilot.run_cycle().await;

        let cycle_successful = report.objectives_processed > 0
            || report
                .plans
                .iter()
                .any(|p| p.pipeline_submitted || !p.queued_task_ids.is_empty());

        if cycle_successful {
            autopilot.consecutive_failures = 0;
        } else {
            autopilot.consecutive_failures += 1;
            if autopilot.consecutive_failures >= autopilot.cfg.consecutive_failure_limit {
                let _ = std::fs::write(
                    &breaker_flag,
                    format!(
                        "circuit_breaker_tripped_at_{}_consecutive_failures",
                        autopilot.consecutive_failures
                    ),
                );
            }
        }

        tokio::time::sleep(autopilot.cfg.interval).await;
    }
}

#[cfg(test)]
mod tests {
    use super::super::delegation::AgentCapabilities;
    use super::*;

    fn write_allow_readiness_artifacts(root: &Path) {
        std::fs::create_dir_all(
            root.join(CANONICAL_AUTONOMY_CONFIG)
                .parent()
                .expect("canonical config parent"),
        )
        .expect("config dir");
        std::fs::write(
            root.join(CANONICAL_AUTONOMY_CONFIG),
            r#"
[[sovereign_crates]]
id = "governance"
crate = "arda-governance"
status = "contract_required"

[[sovereign_crates]]
id = "oracle"
crate = "arda-oracle"
status = "active_prototype"

[[sovereign_crates]]
id = "plutus"
crate = "arda-economics"
status = "contract_required"

[[sovereign_crates]]
id = "human"
crate = "arda-human"
status = "contract_required"

[[sovereign_crates]]
id = "council"
crate = "arda-council"
status = "active_subordinate"

[[sovereign_crates]]
id = "ceo"
crate = "arda-ceo"
status = "active_subordinate"
"#,
        )
        .expect("loop config");
        std::fs::create_dir_all(root.join("data/prometheus")).expect("prometheus data");
        std::fs::write(
            root.join("data/prometheus/autonomy_operating_loop_preflight.json"),
            serde_json::json!({
                "schema_version": "arda.autonomy_operating_loop_preflight.v1",
                "generated_at_utc": Utc::now().to_rfc3339(),
                "loop": {"missing_required_stages": []},
                "summary": {
                    "lane_count": 12,
                    "lane_configured_count": 12,
                    "lane_incomplete_count": 0
                }
            })
            .to_string(),
        )
        .expect("preflight");
        std::fs::create_dir_all(root.join("data/hades")).expect("hades data");
        std::fs::write(
            root.join("data/hades/autonomy_cleanup_approval_packets.json"),
            serde_json::json!({
                "schema_version": "arda.hades.cleanup_approval_packets.v1",
                "candidate_count": 0,
                "cleanup_authorized": false,
                "requires_operator_approval_for_mutation": true,
                "no_file_moves_or_deletes_performed": true,
                "packets": []
            })
            .to_string(),
        )
        .expect("cleanup packets");
        std::fs::create_dir_all(root.join("data/athena")).expect("athena data");
        std::fs::write(
            root.join("data/athena/external_source_lane_ledger.jsonl"),
            r#"{"schema_version":"arda.athena.external_source_lane.v1","source_id":"web","task_promotion_allowed":true,"canonical_url":"https://example.invalid/source","verification_status":"source_receipted"}
"#,
        )
        .expect("external ledger");
        std::fs::create_dir_all(root.join("data/council")).expect("council data");
        std::fs::write(root.join("data/council/agent_conversations.jsonl"), "")
            .expect("council ledger");
    }

    #[test]
    fn sovereign_adapters_use_canonical_governance_config_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical = dir
            .path()
            .join("config/governance/autonomy_operating_loop.toml");
        std::fs::create_dir_all(canonical.parent().expect("canonical parent"))
            .expect("canonical parent");
        std::fs::write(
            &canonical,
            r#"
[[sovereign_crates]]
id = "ceo"
crate = "arda-ceo"
status = "active_subordinate"
"#,
        )
        .expect("canonical config");

        let cfg = AutopilotConfig::from_root(dir.path());
        let projection =
            load_sovereign_adapters(dir.path(), &cfg, &[], &H2AProcessReport::default());

        assert!(projection.source_available);
        assert_eq!(projection.config_path, canonical.display().to_string());
        assert_eq!(projection.adapter_count, 1);
    }

    #[test]
    fn sovereign_adapters_hold_on_ambiguous_legacy_and_canonical_config_paths() {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical = dir
            .path()
            .join("config/governance/autonomy_operating_loop.toml");
        let legacy = dir.path().join("config/autonomy_operating_loop.toml");
        std::fs::create_dir_all(canonical.parent().expect("canonical parent"))
            .expect("canonical parent");
        std::fs::write(&canonical, "").expect("canonical config");
        std::fs::write(&legacy, "").expect("legacy config");

        let cfg = AutopilotConfig::from_root(dir.path());
        let projection =
            load_sovereign_adapters(dir.path(), &cfg, &[], &H2AProcessReport::default());

        assert!(!projection.source_available);
        assert!(projection
            .error
            .as_deref()
            .is_some_and(|error| error.contains("ambiguous_autonomy_config_paths")));
    }

    #[tokio::test]
    async fn read_only_cycle_reports_missing_readiness_evidence_without_creating_placeholders() {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical = dir
            .path()
            .join("config/governance/autonomy_operating_loop.toml");
        std::fs::create_dir_all(canonical.parent().expect("canonical parent"))
            .expect("canonical parent");
        std::fs::write(&canonical, "").expect("canonical config");

        let mut config = AutopilotConfig::from_root(dir.path());
        config.read_only = true;
        let mut autopilot = CeoAutopilot::from_world(config);
        let report = autopilot.run_cycle().await;

        assert_eq!(report.autonomy_readiness.decision, "hold");
        for reason in [
            "autonomy_preflight_missing",
            "hades_cleanup_approval_packets_missing",
            "athena_external_source_lane_ledger_missing",
        ] {
            assert!(report
                .autonomy_readiness
                .reasons
                .iter()
                .any(|item| item == reason));
        }
        for path in [
            "data/prometheus/autonomy_operating_loop_preflight.json",
            "data/hades/autonomy_cleanup_approval_packets.json",
            "data/athena/external_source_lane_ledger.jsonl",
        ] {
            assert!(
                !dir.path().join(path).exists(),
                "read-only cycle created {path}"
            );
        }
    }

    #[test]
    fn readiness_gate_holds_when_preflight_evidence_is_stale() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_allow_readiness_artifacts(dir.path());
        std::fs::write(
            dir.path()
                .join("data/prometheus/autonomy_operating_loop_preflight.json"),
            serde_json::json!({
                "schema_version": "arda.autonomy_operating_loop_preflight.v1",
                "generated_at_utc": "2000-01-01T00:00:00Z",
                "loop": {"missing_required_stages": []},
                "summary": {
                    "lane_count": 12,
                    "lane_configured_count": 12,
                    "lane_incomplete_count": 0
                }
            })
            .to_string(),
        )
        .expect("stale preflight");
        let cfg = AutopilotConfig::from_root(dir.path());
        let sovereign =
            load_sovereign_adapters(dir.path(), &cfg, &[], &H2AProcessReport::default());
        let hades = load_hades_introspection(dir.path());
        let council =
            load_council_runtime(dir.path(), true, &ObjectiveSelectionReport::default(), &[]);

        let gate = load_autonomy_readiness_gate(dir.path(), &hades, &sovereign, &council);

        assert_eq!(gate.decision, "hold");
        assert!(gate
            .reasons
            .iter()
            .any(|reason| reason == "autonomy_preflight_stale"));
    }

    #[test]
    fn autonomy_preflight_reports_configured_lanes_without_enabling_promotion() {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical = dir
            .path()
            .join("config/governance/autonomy_operating_loop.toml");
        std::fs::create_dir_all(canonical.parent().expect("canonical parent"))
            .expect("canonical parent");
        std::fs::write(
            &canonical,
            r#"
schema_version = "arda.autonomy_operating_loop.v1"

[loop]
stages = ["info", "ingest", "audit"]

[[lanes]]
id = "intake"
agent = "athena"
engine_interface = "knowledge_triage"
default_policy = "ledger_before_task"
"#,
        )
        .expect("canonical config");

        let report = inspect_autonomy_preflight(dir.path()).expect("preflight report");

        assert_eq!(
            report.schema_version,
            "arda.autonomy_operating_loop_preflight.v1"
        );
        assert_eq!(report.summary.lane_count, 1);
        assert_eq!(report.summary.lane_incomplete_count, 0);
        assert!(!report.task_promotion_allowed);
        assert!(!dir
            .path()
            .join("data/prometheus/autonomy_operating_loop_preflight.json")
            .exists());
    }

    #[test]
    fn writing_preflight_publishes_only_the_readiness_projection() {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical = dir
            .path()
            .join("config/governance/autonomy_operating_loop.toml");
        std::fs::create_dir_all(canonical.parent().expect("canonical parent"))
            .expect("canonical parent");
        std::fs::write(
            &canonical,
            "[loop]\nstages = [\"info\", \"ingest\", \"audit\"]\n",
        )
        .expect("canonical config");

        let output = write_autonomy_preflight(dir.path()).expect("write preflight");
        let payload: Value =
            serde_json::from_str(&std::fs::read_to_string(&output).expect("output"))
                .expect("preflight json");

        assert_eq!(payload["task_promotion_allowed"], false);
        assert_eq!(
            payload["schema_version"],
            "arda.autonomy_operating_loop_preflight.v1"
        );
        assert!(!dir.path().join("core/projects/tasks/queue.jsonl").exists());
        assert!(!dir.path().join("data/hades/action_queue.jsonl").exists());
    }

    #[tokio::test]
    async fn run_cycle_persists_approved_packet_plan_and_dispatches_apollo() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = AutopilotConfig::from_root(dir.path());
        write_allow_readiness_artifacts(dir.path());
        std::fs::create_dir_all(cfg.queue_path.parent().unwrap()).unwrap();
        std::fs::write(&cfg.queue_path, "").unwrap();
        std::fs::create_dir_all(cfg.objectives_path.parent().unwrap()).unwrap();
        std::fs::write(&cfg.objectives_path, "").unwrap();
        std::fs::create_dir_all(cfg.arandur_recommendations_path.parent().unwrap()).unwrap();
        std::fs::write(
            &cfg.arandur_recommendations_path,
            r#"{"recommendation_id":"reco-approved-refactor","review_required":false,"approval_packet":{"approval_id":"approval-reco-approved-refactor","status":"approved","approved_by":"operator","approved_at":"2026-05-22T00:00:00Z"},"candidate":{"id":"approved_refactor","owner":"prometheus","priority":"high","title":"Refactor module x"}}
"#,
        )
        .unwrap();

        let mut reg = AgentRegistry::new();
        reg.register(AgentCapabilities {
            agent_id: "ceo".into(),
            task_types: vec!["ops".into()],
            max_concurrent: 4,
            current_load: 0,
            success_rate: 1.0,
        });
        reg.register(AgentCapabilities {
            agent_id: "warden".into(),
            task_types: vec!["monitor".into()],
            max_concurrent: 4,
            current_load: 0,
            success_rate: 1.0,
        });
        reg.register(AgentCapabilities {
            agent_id: "prometheus".into(),
            task_types: vec!["analysis".into()],
            max_concurrent: 4,
            current_load: 0,
            success_rate: 1.0,
        });

        let mut auto = CeoAutopilot::new(cfg.clone(), reg);
        let report = auto.run_cycle().await;
        assert_eq!(report.objectives_processed, 1);
        let pc = &report.plans[0];
        assert!(!pc.queued_task_ids.is_empty());
        assert_eq!(
            pc.queue_operation
                .as_ref()
                .map(|operation| &operation.result_status),
            Some(&QueueOperationStatus::Appended)
        );
        assert!(
            !pc.apollo_dispatches.is_empty(),
            "operational tasks should dispatch through Apollo"
        );

        let inbox = std::fs::read_to_string(&cfg.objectives_path).unwrap();
        assert!(inbox.trim().is_empty());
    }

    #[tokio::test]
    async fn read_only_does_not_dispatch_or_emit_a2h() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = AutopilotConfig::from_root(dir.path());
        cfg.read_only = true;
        std::fs::create_dir_all(cfg.queue_path.parent().unwrap()).unwrap();
        std::fs::write(
            &cfg.queue_path,
            r#"{"id":"done1","title":"Done task","status":"completed","owner":"apollo"}
"#,
        )
        .unwrap();
        std::fs::create_dir_all(cfg.objectives_path.parent().unwrap()).unwrap();
        std::fs::write(
            &cfg.objectives_path,
            r#"{"id":"o1","statement":"deploy x"}
"#,
        )
        .unwrap();
        let mut auto = CeoAutopilot::new(
            cfg.clone(),
            super::super::bootstrap::seed_default_registry(),
        );
        let report = auto.run_cycle().await;
        assert!(report.plans[0].apollo_dispatches.is_empty());
        assert!(!report.plans[0].a2h_emitted);
        assert!(!cfg.a2h_path.exists());
        assert_eq!(report.outcomes_ingested, 0);
        assert!(!cfg.outcome_cursor_path.exists());
        assert_eq!(
            std::fs::read_to_string(&cfg.queue_path).unwrap(),
            r#"{"id":"done1","title":"Done task","status":"completed","owner":"apollo"}
"#
        );
    }

    #[tokio::test]
    async fn human_required_governance_blocks_queue_and_emits_a2h() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
        let cfg = AutopilotConfig::from_root(dir.path());
        write_allow_readiness_artifacts(dir.path());
        std::fs::create_dir_all(
            cfg.queue_path
                .parent()
                .unwrap_or_else(|| panic!("queue parent missing")),
        )
        .unwrap_or_else(|err| panic!("mkdir queue parent failed: {err}"));
        std::fs::write(&cfg.queue_path, "")
            .unwrap_or_else(|err| panic!("write queue failed: {err}"));
        std::fs::create_dir_all(
            cfg.objectives_path
                .parent()
                .unwrap_or_else(|| panic!("objectives parent missing")),
        )
        .unwrap_or_else(|err| panic!("mkdir objectives parent failed: {err}"));
        std::fs::write(
            &cfg.objectives_path,
            r#"{"id":"funds1","statement":"transfer funds to vendor"}
"#,
        )
        .unwrap_or_else(|err| panic!("write objectives failed: {err}"));

        let mut auto = CeoAutopilot::new(
            cfg.clone(),
            super::super::bootstrap::seed_default_registry(),
        );
        let report = auto.run_cycle().await;
        assert!(report.plans.is_empty());
        assert_eq!(report.objective_selection.objectives_considered, 1);
        assert_eq!(report.objective_selection.objectives_blocked_by_gate, 1);
        assert_eq!(
            report.objective_selection.candidates[0].governance_class,
            "funds_movement"
        );
        assert_eq!(
            report.objective_selection.candidates[0].review_gate,
            super::super::governance_policy::GovernanceGate::HumanRequired
        );
        assert!(report.objective_selection.candidates[0]
            .rejection_reason
            .as_deref()
            .unwrap_or_default()
            .starts_with("blocked_by_gate:HumanRequired:"));
        assert_eq!(
            std::fs::read_to_string(&cfg.queue_path).unwrap_or_default(),
            ""
        );
    }

    #[tokio::test]
    async fn triad_governance_propagates_oracle_quorum_evidence() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
        let cfg = AutopilotConfig::from_root(dir.path());
        std::fs::create_dir_all(
            cfg.queue_path
                .parent()
                .unwrap_or_else(|| panic!("queue parent missing")),
        )
        .unwrap_or_else(|err| panic!("mkdir queue parent failed: {err}"));
        std::fs::write(&cfg.queue_path, "")
            .unwrap_or_else(|err| panic!("write queue failed: {err}"));
        std::fs::create_dir_all(
            cfg.objectives_path
                .parent()
                .unwrap_or_else(|| panic!("objectives parent missing")),
        )
        .unwrap_or_else(|err| panic!("mkdir objectives parent failed: {err}"));
        std::fs::write(
            &cfg.objectives_path,
            r#"{"id":"reroute1","statement":"reroute provider traffic"}
"#,
        )
        .unwrap_or_else(|err| panic!("write objectives failed: {err}"));

        let mut auto = CeoAutopilot::new(
            cfg.clone(),
            super::super::bootstrap::seed_default_registry(),
        );
        let report = auto.run_cycle().await;
        assert!(report.plans.is_empty());
        assert_eq!(report.objective_selection.objectives_considered, 1);
        assert_eq!(report.objective_selection.objectives_blocked_by_gate, 1);
        assert_eq!(
            report.objective_selection.candidates[0].governance_class,
            "provider_reroute"
        );
        assert_eq!(
            report.objective_selection.candidates[0].review_gate,
            super::super::governance_policy::GovernanceGate::TriadQuorumRequired
        );
        assert!(report.objective_selection.candidates[0]
            .rejection_reason
            .as_deref()
            .unwrap_or_default()
            .starts_with("blocked_by_gate:TriadQuorumRequired:"));
        assert_eq!(
            std::fs::read_to_string(&cfg.queue_path).unwrap_or_default(),
            ""
        );
    }

    #[tokio::test]
    async fn read_only_benchmark_governance_gate_does_not_emit_or_delegate() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
        let mut cfg = AutopilotConfig::from_root(dir.path());
        cfg.read_only = true;
        std::fs::create_dir_all(
            cfg.queue_path
                .parent()
                .unwrap_or_else(|| panic!("queue parent missing")),
        )
        .unwrap_or_else(|err| panic!("mkdir queue parent failed: {err}"));
        std::fs::write(&cfg.queue_path, "")
            .unwrap_or_else(|err| panic!("write queue failed: {err}"));
        std::fs::create_dir_all(
            cfg.objectives_path
                .parent()
                .unwrap_or_else(|| panic!("objectives parent missing")),
        )
        .unwrap_or_else(|err| panic!("mkdir objectives parent failed: {err}"));
        std::fs::write(
            &cfg.objectives_path,
            r#"{"id":"bench1","statement":"run autonomy benchmark read-only cycle"}
"#,
        )
        .unwrap_or_else(|err| panic!("write objectives failed: {err}"));

        let mut auto = CeoAutopilot::new(
            cfg.clone(),
            super::super::bootstrap::seed_default_registry(),
        );
        let report = auto.run_cycle().await;
        assert!(report.plans.is_empty());
        assert_eq!(
            report.objective_selection.candidates[0].review_gate,
            super::super::governance_policy::GovernanceGate::ReadOnlyBenchmarkRequired
        );
        assert_eq!(report.objective_selection.objectives_blocked_by_gate, 1);
        assert_eq!(
            std::fs::read_to_string(&cfg.queue_path).unwrap_or_default(),
            ""
        );
        assert!(
            !cfg.state_path.exists(),
            "read-only benchmark must not mutate autopilot state snapshot at {}",
            cfg.state_path.display()
        );
    }

    #[tokio::test]
    async fn run_cycle_resumes_human_approved_a2h_objective() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = AutopilotConfig::from_root(dir.path());
        write_allow_readiness_artifacts(dir.path());
        std::fs::create_dir_all(cfg.queue_path.parent().unwrap()).unwrap();
        std::fs::write(&cfg.queue_path, "").unwrap();
        std::fs::create_dir_all(cfg.objectives_path.parent().unwrap()).unwrap();
        std::fs::write(&cfg.objectives_path, "").unwrap();

        let request_id = uuid::Uuid::new_v4();
        let obj = Objective {
            id: "human-approved".into(),
            statement: "Refactor module x".into(),
            constraints: vec![],
            deadline: None,
            success_criteria: vec![],
            tags: vec![],
        };
        append_pending_authorization(
            &cfg.a2h_pending_path,
            request_id,
            &obj,
            &GateDecision::Rejected {
                resonance: 0.2,
                concerns: vec!["needs human".into()],
            },
        )
        .unwrap();
        std::fs::create_dir_all(cfg.h2a_path.parent().unwrap()).unwrap();
        std::fs::write(
            &cfg.h2a_path,
            format!(
                r#"{{"request_id":"{request_id}","approved":true,"conditions":["watch logs"]}}"#
            ),
        )
        .unwrap();

        let mut auto = CeoAutopilot::new(
            cfg.clone(),
            super::super::bootstrap::seed_default_registry(),
        );
        let report = auto.run_cycle().await;
        assert_eq!(report.h2a.responses_processed, 1);
        assert_eq!(report.h2a.objectives_resumed, 1);
        assert_eq!(report.objectives_processed, 1);
        assert!(!report.plans[0].queued_task_ids.is_empty());
        let queue = std::fs::read_to_string(&cfg.queue_path).unwrap();
        assert!(queue.contains("\"oracle_conditions\":[\"watch logs\"]"));
        let pending = std::fs::read_to_string(&cfg.a2h_pending_path).unwrap();
        assert!(pending.contains("\"status\":\"resumed\""));
    }

    #[tokio::test]
    async fn operator_approved_recommendation_activates_queue_while_global_autonomy_is_held() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = AutopilotConfig::from_root(dir.path());
        std::fs::create_dir_all(cfg.arandur_recommendations_path.parent().unwrap()).unwrap();
        std::fs::write(
            &cfg.arandur_recommendations_path,
            r#"{"recommendation_id":"reco-approved","review_required":false,"approval_packet":{"id":"approval-reco-approved","status":"approved","approved_by":"operator","approved_at":"2026-08-13T00:00:00Z"},"candidate":{"id":"approved_task","owner":"prometheus","priority":"high","title":"Monitor approved queue activation"}}
"#,
        )
        .unwrap();

        let mut auto = CeoAutopilot::new(
            cfg.clone(),
            super::super::bootstrap::seed_default_registry(),
        );
        let report = auto.run_cycle().await;

        assert_eq!(report.autonomy_readiness.decision, "hold");
        assert_eq!(report.objectives_processed, 1);
        assert!(!report.plans[0].queued_task_ids.is_empty());
        assert_eq!(
            report.plans[0]
                .queue_operation
                .as_ref()
                .map(|operation| &operation.result_status),
            Some(&QueueOperationStatus::Appended)
        );
        let queue = std::fs::read_to_string(cfg.queue_path).unwrap();
        assert!(queue.contains("\"status\":\"pending\""));
    }

    #[tokio::test]
    async fn run_cycle_holds_delegation_when_cycle_joule_limit_exceeded() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = AutopilotConfig::from_root(dir.path());
        cfg.joule_cycle_limit = 1.0;
        write_allow_readiness_artifacts(dir.path());
        std::fs::create_dir_all(cfg.queue_path.parent().unwrap()).unwrap();
        std::fs::write(&cfg.queue_path, "").unwrap();
        std::fs::create_dir_all(cfg.objectives_path.parent().unwrap()).unwrap();
        std::fs::write(
            &cfg.objectives_path,
            r#"{"id":"o1","statement":"Refactor module x"}
"#,
        )
        .unwrap();

        let mut auto = CeoAutopilot::new(
            cfg.clone(),
            super::super::bootstrap::seed_default_registry(),
        );
        let report = auto.run_cycle().await;
        assert!(report.plans[0].joule_limited);
        assert!(report.plans[0].queued_task_ids.is_empty());
        assert_eq!(std::fs::read_to_string(&cfg.queue_path).unwrap(), "");
    }

    #[test]
    fn objective_selection_reports_no_action_when_inputs_empty() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = AutopilotConfig::from_root(dir.path());
        std::fs::create_dir_all(cfg.queue_path.parent().unwrap()).unwrap();
        std::fs::write(&cfg.queue_path, "").unwrap();

        let (_objectives, report) = select_cycle_objectives(
            &cfg,
            &ObjectiveDecomposer::default(),
            &GovernancePolicy::default(),
            Vec::new(),
            Vec::new(),
        );

        assert_eq!(report.status, "no_action");
        assert_eq!(report.objectives_considered, 0);
        assert_eq!(report.objectives_selected, 0);
        assert!(report.selected_objective_id.is_none());
        assert!(report
            .next_recommended_action
            .contains("add a bounded objective"));
    }

    #[test]
    fn objective_selection_uses_effective_latest_queue_records() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = AutopilotConfig::from_root(dir.path());
        std::fs::create_dir_all(cfg.queue_path.parent().unwrap()).unwrap();
        std::fs::write(
            &cfg.queue_path,
            concat!(
                r#"{"id":"stale-raw-1","source_record_id":"shared-source","title":"Refactor stale module","status":"pending","owner":"prometheus"}"#,
                "\n",
                r#"{"id":"stale-raw-2","source_record_id":"shared-source","title":"Refactor stale module","status":"completed","owner":"prometheus"}"#,
                "\n",
                r#"{"id":"live","title":"Refactor live module","status":"pending","owner":"prometheus","priority":"high"}"#,
                "\n"
            ),
        )
        .unwrap();

        let (objectives, report) = select_cycle_objectives(
            &cfg,
            &ObjectiveDecomposer::default(),
            &GovernancePolicy::default(),
            Vec::new(),
            Vec::new(),
        );

        assert_eq!(objectives.len(), 1);
        assert_eq!(objectives[0].objective.id, "live");
        assert_eq!(report.status, "selected");
        assert_eq!(report.effective_queue_open_count, 1);
        assert_eq!(report.stale_raw_queue_record_count, 1);
        assert_eq!(report.objectives_considered, 1);
        assert_eq!(report.objectives_selected, 1);
        assert!(report.no_selection_reason.is_none());
    }

    #[test]
    fn objective_selection_blocks_review_gated_effective_queue_records() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = AutopilotConfig::from_root(dir.path());
        std::fs::create_dir_all(cfg.queue_path.parent().unwrap()).unwrap();
        std::fs::write(
            &cfg.queue_path,
            r#"{"id":"hades","title":"archive production WARDEN queue","status":"pending","owner":"hades","action_class":"archive_or_retention"}
"#,
        )
        .unwrap();

        let (objectives, report) = select_cycle_objectives(
            &cfg,
            &ObjectiveDecomposer::default(),
            &GovernancePolicy::default(),
            Vec::new(),
            Vec::new(),
        );

        assert!(objectives.is_empty());
        assert_eq!(report.status, "blocked");
        assert_eq!(report.objectives_blocked_by_gate, 1);
        assert_eq!(
            report.candidates[0].review_gate,
            super::super::governance_policy::GovernanceGate::HadesReviewRequired
        );
        assert!(report.candidates[0]
            .rejection_reason
            .as_deref()
            .unwrap_or_default()
            .starts_with("blocked_by_gate:HadesReviewRequired:"));
    }

    #[test]
    fn objective_selection_reports_arandur_recommendations_as_review_gated() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
        let cfg = AutopilotConfig::from_root(dir.path());
        std::fs::create_dir_all(
            cfg.arandur_recommendations_path
                .parent()
                .unwrap_or_else(|| panic!("arandur recommendations parent missing")),
        )
        .unwrap_or_else(|err| panic!("mkdir arandur parent failed: {err}"));
        std::fs::write(
            &cfg.arandur_recommendations_path,
            r#"{"recommendation_id":"reco1","recommended_action":"Review mission backlog","review_required":true,"candidate":{"id":"mission_backlog","owner":"prometheus","priority":"high","title":"Review mission backlog readiness"}}
"#,
        )
        .unwrap_or_else(|err| panic!("write recommendations failed: {err}"));

        let (objectives, report) = select_cycle_objectives(
            &cfg,
            &ObjectiveDecomposer::default(),
            &GovernancePolicy::default(),
            Vec::new(),
            Vec::new(),
        );

        assert!(objectives.is_empty());
        assert_eq!(report.status, "blocked");
        assert_eq!(report.objectives_considered, 1);
        assert_eq!(report.objectives_blocked_by_gate, 1);
        assert_eq!(report.candidates[0].source_record_id, "reco1");
        assert_eq!(report.candidates[0].effective_status, "review_required");
        assert_eq!(
            report.candidates[0].governance_class,
            "arandur_recommendation"
        );
        assert_eq!(
            report.candidates[0].review_gate,
            super::super::governance_policy::GovernanceGate::ReviewRequired
        );
        assert!(report.candidates[0]
            .rejection_reason
            .as_deref()
            .unwrap_or_default()
            .contains("Arandur recommendation requires operator review"));
    }

    #[test]
    fn objective_selection_requires_approval_packet_for_cleared_arandur_recommendations() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
        let cfg = AutopilotConfig::from_root(dir.path());
        std::fs::create_dir_all(
            cfg.arandur_recommendations_path
                .parent()
                .unwrap_or_else(|| panic!("arandur recommendations parent missing")),
        )
        .unwrap_or_else(|err| panic!("mkdir arandur parent failed: {err}"));
        std::fs::write(
            &cfg.arandur_recommendations_path,
            r#"{"recommendation_id":"reco2","review_required":false,"candidate":{"id":"safe_refactor","owner":"prometheus","priority":"medium","title":"Refactor safe module"}}
"#,
        )
        .unwrap_or_else(|err| panic!("write recommendations failed: {err}"));

        let (objectives, report) = select_cycle_objectives(
            &cfg,
            &ObjectiveDecomposer::default(),
            &GovernancePolicy::default(),
            Vec::new(),
            Vec::new(),
        );

        assert!(objectives.is_empty());
        assert_eq!(report.status, "blocked");
        assert_eq!(report.objectives_blocked_by_gate, 1);
        assert_eq!(
            report.candidates[0].blocked_reason_code.as_deref(),
            Some("operator_approval_packet_missing")
        );
        assert!(report
            .blocked_candidate_groups
            .iter()
            .any(
                |group| group.reason_code == "operator_approval_packet_missing"
                    && group.governance_class == "arandur_recommendation"
            ));
    }

    #[test]
    fn objective_selection_can_use_operator_approval_packet_arandur_recommendations() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
        let cfg = AutopilotConfig::from_root(dir.path());
        std::fs::create_dir_all(
            cfg.arandur_recommendations_path
                .parent()
                .unwrap_or_else(|| panic!("arandur recommendations parent missing")),
        )
        .unwrap_or_else(|err| panic!("mkdir arandur parent failed: {err}"));
        std::fs::write(
            &cfg.arandur_recommendations_path,
            r#"{"recommendation_id":"reco2","review_required":false,"approval_packet":{"approval_id":"approval-reco2","status":"approved","approved_by":"operator","approved_at":"2026-05-21T00:00:00Z"},"candidate":{"id":"approved_gate","owner":"prometheus","priority":"medium","title":"Review approved automation gate"}}
"#,
        )
        .unwrap_or_else(|err| panic!("write recommendations failed: {err}"));

        let (objectives, report) = select_cycle_objectives(
            &cfg,
            &ObjectiveDecomposer::default(),
            &GovernancePolicy::default(),
            Vec::new(),
            Vec::new(),
        );

        assert_eq!(objectives.len(), 1);
        assert_eq!(objectives[0].objective.id, "approved_gate");
        assert_eq!(report.status, "selected");
        assert_eq!(report.objectives_selected, 1);
        assert_eq!(report.objectives_blocked_by_gate, 0);
        assert_eq!(
            report.selected_objective_id.as_deref(),
            Some("approved_gate")
        );
        assert_eq!(report.candidates[0].source_record_id, "reco2");
        assert_eq!(report.candidates[0].governance_class, "operator_approved");
        assert_eq!(
            report.candidates[0].review_gate,
            super::super::governance_policy::GovernanceGate::SafeAutonomous
        );
        assert_eq!(
            report.candidates[0].approval_packet_id.as_deref(),
            Some("approval-reco2")
        );
    }

    #[test]
    fn objective_selection_exposes_read_only_objective_packet_report() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
        let cfg = AutopilotConfig::from_root(dir.path());
        std::fs::create_dir_all(
            cfg.arandur_recommendations_path
                .parent()
                .unwrap_or_else(|| panic!("arandur recommendations parent missing")),
        )
        .unwrap_or_else(|err| panic!("mkdir arandur parent failed: {err}"));
        std::fs::write(
            &cfg.arandur_recommendations_path,
            concat!(
                r#"{"recommendation_id":"reco-approved","review_required":false,"approval_packet":{"approval_id":"approval-reco-approved","status":"approved","approved_by":"operator","approved_at":"2026-05-21T00:00:00Z"},"candidate":{"id":"approved_gate","owner":"prometheus","priority":"high","title":"Review approved automation gate"}}"#,
                "\n",
                r#"{"recommendation_id":"reco-blocked","review_required":true,"candidate":{"id":"blocked_gate","owner":"prometheus","priority":"medium","title":"Review blocked automation gate"}}"#,
                "\n"
            ),
        )
        .unwrap_or_else(|err| panic!("write recommendations failed: {err}"));

        let (objectives, report) = select_cycle_objectives(
            &cfg,
            &ObjectiveDecomposer::default(),
            &GovernancePolicy::default(),
            Vec::new(),
            Vec::new(),
        );

        assert_eq!(objectives.len(), 1);
        assert_eq!(
            report.objective_packet_report.mutation_policy,
            "read_only_report_only"
        );
        assert!(
            !report
                .objective_packet_report
                .canonical_queue_mutation_allowed
        );
        assert_eq!(report.objective_packet_report.packet_count, 2);
        assert_eq!(report.objective_packet_report.selected_packet_count, 1);
        assert_eq!(
            report
                .objective_packet_report
                .selected_candidate_id
                .as_deref(),
            Some("approved_gate")
        );
        let selected_packet = report
            .objective_packet_report
            .packets
            .iter()
            .find(|packet| packet.selected)
            .unwrap_or_else(|| panic!("selected packet missing"));
        assert_eq!(selected_packet.source_record_id, "reco-approved");
        assert_eq!(
            selected_packet.approval_packet_id.as_deref(),
            Some("approval-reco-approved")
        );
        assert!(!selected_packet.canonical_queue_mutation_allowed);
    }

    #[test]
    fn objective_selection_recognizes_executed_operator_approved_arandur_candidates() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
        let cfg = AutopilotConfig::from_root(dir.path());
        std::fs::create_dir_all(
            cfg.arandur_recommendations_path
                .parent()
                .unwrap_or_else(|| panic!("arandur recommendations parent missing")),
        )
        .unwrap_or_else(|err| panic!("mkdir arandur parent failed: {err}"));
        std::fs::write(
            &cfg.arandur_recommendations_path,
            concat!(
                r#"{"recommendation_id":"reco-arda","review_required":false,"approval_packet":{"approval_id":"arandur_approval_20260521T191923Z_candidate_arda_visualization_and_presence","status":"approved","approved_by":"operator","approved_at":"2026-05-21T19:19:23Z"},"candidate":{"id":"candidate_arda_visualization_and_presence","owner":"prometheus","priority":"high","title":"Execute ARDA visualization and presence"}}"#,
                "\n",
                r#"{"recommendation_id":"reco-ais","review_required":false,"approval_packet":{"approval_id":"arandur_approval_20260521T205014Z_candidate_ais_smb_os_and_relic","status":"approved","approved_by":"operator","approved_at":"2026-05-21T20:50:14Z"},"candidate":{"id":"candidate_ais_smb_os_and_relic","owner":"prometheus","priority":"high","title":"Execute AIS SMB OS and relic"}}"#,
                "\n"
            ),
        )
        .unwrap_or_else(|err| panic!("write recommendations failed: {err}"));
        std::fs::create_dir_all(
            cfg.queue_path
                .parent()
                .unwrap_or_else(|| panic!("queue parent missing")),
        )
        .unwrap_or_else(|err| panic!("mkdir queue parent failed: {err}"));
        std::fs::write(&cfg.queue_path, "")
            .unwrap_or_else(|err| panic!("write queue failed: {err}"));
        let receipt_path = dir
            .path()
            .join("audit/ARANDUR_OPERATOR_APPROVED_CANDIDATES_EXECUTION_2026-05-21/execution_receipt.json");
        std::fs::create_dir_all(
            receipt_path
                .parent()
                .unwrap_or_else(|| panic!("receipt parent missing")),
        )
        .unwrap_or_else(|err| panic!("mkdir receipt parent failed: {err}"));
        std::fs::write(
            &receipt_path,
            r#"{"contract":"arda.arandur.operator_approved_candidates_execution.v1","tasks":[{"candidate_id":"candidate_arda_visualization_and_presence","approval_packet_id":"arandur_approval_20260521T191923Z_candidate_arda_visualization_and_presence","status":"executed_verified"},{"candidate_id":"candidate_ais_smb_os_and_relic","approval_packet_id":"arandur_approval_20260521T205014Z_candidate_ais_smb_os_and_relic","status":"executed_verified"}]}"#,
        )
        .unwrap_or_else(|err| panic!("write receipt failed: {err}"));

        let (objectives, report) = select_cycle_objectives(
            &cfg,
            &ObjectiveDecomposer::default(),
            &GovernancePolicy::default(),
            Vec::new(),
            Vec::new(),
        );

        assert!(objectives.is_empty());
        assert_eq!(report.status, "no_action");
        assert_eq!(report.objectives_considered, 2);
        assert_eq!(report.objectives_selected, 0);
        assert_eq!(report.objectives_blocked_by_gate, 0);
        assert!(report.selected_objective_id.is_none());
        assert!(report.next_recommended_action.contains("already executed"));
        assert_eq!(
            std::fs::read_to_string(&cfg.queue_path).unwrap_or_default(),
            ""
        );
        for candidate in &report.candidates {
            assert_eq!(candidate.effective_status, "executed_verified");
            assert_eq!(candidate.governance_class, "operator_approved_executed");
            assert_eq!(
                candidate.review_gate,
                super::super::governance_policy::GovernanceGate::SafeAutonomous
            );
            assert!(candidate.completion_receipt_path.as_deref().is_some_and(|path| {
                path.ends_with("audit/ARANDUR_OPERATOR_APPROVED_CANDIDATES_EXECUTION_2026-05-21/execution_receipt.json")
            }));
            assert!(candidate
                .rejection_reason
                .as_deref()
                .unwrap_or_default()
                .contains("recognized_completed"));
        }
    }

    #[test]
    fn objective_selection_suppresses_superseded_open_recommendations_with_execution_receipts() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
        let cfg = AutopilotConfig::from_root(dir.path());
        std::fs::create_dir_all(
            cfg.arandur_recommendations_path
                .parent()
                .unwrap_or_else(|| panic!("arandur recommendations parent missing")),
        )
        .unwrap_or_else(|err| panic!("mkdir arandur parent failed: {err}"));
        std::fs::write(
            &cfg.arandur_recommendations_path,
            concat!(
                r#"{"recommendation_id":"open-arda","review_required":true,"candidate":{"id":"candidate_arda_visualization_and_presence","owner":"athena","priority":"high","title":"Review ARDA visualization and presence"}}"#,
                "\n",
                r#"{"recommendation_id":"open-ecst","review_required":true,"candidate":{"id":"candidate_ecst_mythos_research","owner":"athena","priority":"high","title":"Review ECST and MythOS research"}}"#,
                "\n",
                r#"{"recommendation_id":"approved-arda","review_required":false,"approval_packet":{"approval_id":"arandur_approval_20260521T191923Z_candidate_arda_visualization_and_presence","status":"approved","approved_by":"operator","approved_at":"2026-05-21T19:19:23Z"},"candidate":{"id":"candidate_arda_visualization_and_presence","owner":"prometheus","priority":"high","title":"Execute ARDA visualization and presence"}}"#,
                "\n"
            ),
        )
        .unwrap_or_else(|err| panic!("write recommendations failed: {err}"));
        let receipt_path = dir
            .path()
            .join("audit/ARANDUR_OPERATOR_APPROVED_CANDIDATES_EXECUTION_2026-05-21/execution_receipt.json");
        std::fs::create_dir_all(
            receipt_path
                .parent()
                .unwrap_or_else(|| panic!("receipt parent missing")),
        )
        .unwrap_or_else(|err| panic!("mkdir receipt parent failed: {err}"));
        std::fs::write(
            &receipt_path,
            r#"{"contract":"arda.arandur.operator_approved_candidates_execution.v1","tasks":[{"candidate_id":"candidate_arda_visualization_and_presence","approval_packet_id":"arandur_approval_20260521T191923Z_candidate_arda_visualization_and_presence","status":"executed_verified"}]}"#,
        )
        .unwrap_or_else(|err| panic!("write receipt failed: {err}"));

        let (objectives, report) = select_cycle_objectives(
            &cfg,
            &ObjectiveDecomposer::default(),
            &GovernancePolicy::default(),
            Vec::new(),
            Vec::new(),
        );

        assert!(objectives.is_empty());
        assert_eq!(report.status, "blocked");
        assert_eq!(report.objectives_considered, 2);
        assert_eq!(report.objectives_blocked_by_gate, 1);
        assert!(!report
            .candidates
            .iter()
            .any(|candidate| candidate.source_record_id == "open-arda"));
        assert!(report.candidates.iter().any(|candidate| {
            candidate.source_record_id == "approved-arda"
                && candidate.governance_class == "operator_approved_executed"
                && candidate.effective_status == "executed_verified"
        }));
        let packet = report
            .next_automation_gate_packet
            .as_ref()
            .unwrap_or_else(|| panic!("next automation gate packet missing"));
        assert_eq!(packet.recommendation_id, "open-ecst");
        assert_eq!(packet.candidate_id, "candidate_ecst_mythos_research");
    }

    #[test]
    fn objective_selection_groups_blocked_candidates_by_reason_class_and_packet() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
        let cfg = AutopilotConfig::from_root(dir.path());
        std::fs::create_dir_all(
            cfg.arandur_recommendations_path
                .parent()
                .unwrap_or_else(|| panic!("arandur recommendations parent missing")),
        )
        .unwrap_or_else(|err| panic!("mkdir arandur parent failed: {err}"));
        std::fs::create_dir_all(
            cfg.queue_path
                .parent()
                .unwrap_or_else(|| panic!("queue parent missing")),
        )
        .unwrap_or_else(|err| panic!("mkdir queue parent failed: {err}"));
        std::fs::write(
            &cfg.arandur_recommendations_path,
            r#"{"recommendation_id":"reco1","recommended_action":"Promote next automation gate","review_required":true,"candidate":{"id":"next_gate","owner":"prometheus","priority":"high","title":"Implement next reviewed automation gate"}}
"#,
        )
        .unwrap_or_else(|err| panic!("write recommendations failed: {err}"));
        std::fs::write(
            &cfg.queue_path,
            concat!(
                r#"{"id":"stale","title":"superseded raw queue task","status":"pending","owner":"prometheus","action_class":"safe_local_maintenance"}"#,
                "\n",
                r#"{"id":"stale","title":"superseded raw queue task","status":"completed","owner":"prometheus","action_class":"safe_local_maintenance"}"#,
                "\n",
                r#"{"id":"unknown","title":"operate mystery process","status":"pending","owner":"prometheus","action_class":"mystery_mutation"}"#,
                "\n",
                r#"{"id":"unsafe","title":"transfer funds to vendor","status":"pending","owner":"plutus","action_class":"funds_movement"}"#,
                "\n"
            ),
        )
        .unwrap_or_else(|err| panic!("write queue failed: {err}"));

        let (objectives, report) = select_cycle_objectives(
            &cfg,
            &ObjectiveDecomposer::default(),
            &GovernancePolicy::default(),
            Vec::new(),
            Vec::new(),
        );

        assert!(objectives.is_empty());
        assert_eq!(report.status, "blocked");
        assert_eq!(report.stale_raw_queue_record_count, 1);
        assert!(report.blocked_candidate_groups.iter().any(|group| {
            group.reason_code == "review_gated_recommendation_requires_operator_review"
                && group.governance_class == "arandur_recommendation"
                && group.count == 1
        }));
        assert!(report.blocked_candidate_groups.iter().any(|group| {
            group.reason_code == "stale_or_superseded_raw_queue_record"
                && group.governance_class == "raw_queue_record"
                && group.count == 1
        }));
        assert!(report.blocked_candidate_groups.iter().any(|group| {
            group.reason_code == "unknown_action_class"
                && group.governance_class == "mystery_mutation"
                && group.count == 1
        }));
        assert!(report.blocked_candidate_groups.iter().any(|group| {
            group.reason_code == "unsafe_human_required"
                && group.governance_class == "funds_movement"
                && group.count == 1
        }));
        let packet = report
            .next_automation_gate_packet
            .as_ref()
            .unwrap_or_else(|| panic!("next automation gate packet missing"));
        assert_eq!(packet.recommendation_id, "reco1");
        assert_eq!(packet.candidate_id, "next_gate");
        assert!(packet.requires_operator_approval);
        assert!(!packet.canonical_queue_mutation_allowed);
    }

    #[test]
    fn dispatch_retryable_only_retries_terminal_apollo_failures() {
        assert!(dispatch_retryable(&Dispatch::Submitted {
            task_id: "failed".into(),
            status: ExecutionStatus::Failed,
            joules: 0.0,
            transport: "in_process",
        }));
        assert!(dispatch_retryable(&Dispatch::Submitted {
            task_id: "timeout".into(),
            status: ExecutionStatus::Timeout,
            joules: 0.0,
            transport: "daemon",
        }));
        assert!(!dispatch_retryable(&Dispatch::Submitted {
            task_id: "ok".into(),
            status: ExecutionStatus::Completed,
            joules: 1.0,
            transport: "in_process",
        }));
        assert!(!dispatch_retryable(&Dispatch::Skipped {
            reason: "non-apollo".into(),
        }));
    }

    #[test]
    fn pending_plan_step_loader_finds_legacy_plan_steps() {
        let lines = [
            r#"{"id":"old1","status":"pending","plan_id":"plan_goal_daily_joulework_report_20260506","plan_step_index":0,"task_type":"collect_joule_samples"}"#,
            r#"{"id":"old2","status":"completed","plan_id":"plan_goal_daily_joulework_report_20260506","plan_step_index":1,"task_type":"summarize_by_provider_tier"}"#,
            r#"{"id":"analysis1","status":"pending","meta":{"objective_id":"obj1","plan_key":"plan"},"task_type":"analysis"}"#,
        ];
        let steps = pending_plan_steps_from_lines(lines.into_iter());
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].queue_id, "old1");
        assert_eq!(
            steps[0].objective_id,
            "plan_goal_daily_joulework_report_20260506"
        );
        assert_eq!(steps[0].plan.key, "step_0");
        assert_eq!(steps[0].plan.task_type, "collect_joule_samples");
    }

    #[test]
    fn pending_plan_step_loader_honors_latest_status_by_id() {
        let lines = [
            r#"{"id":"step1","status":"pending","plan_id":"plan1","plan_step_index":0,"task_type":"probe_seat"}"#,
            r#"{"id":"step1","status":"completed","plan_id":"plan1","plan_step_index":0,"task_type":"probe_seat"}"#,
        ];
        let steps = pending_plan_steps_from_lines(lines.into_iter());
        assert!(steps.is_empty());
    }

    #[test]
    fn hourly_delegated_joules_counts_recent_heartbeats() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("heartbeats.jsonl");
        let recent = Utc::now().to_rfc3339();
        let old = (Utc::now() - chrono::Duration::hours(2)).to_rfc3339();
        std::fs::write(
            &path,
            format!(
                "{{\"ts\":\"{recent}\",\"delegated_joules\":3.5}}\n{{\"ts\":\"{old}\",\"delegated_joules\":99.0}}\n"
            ),
        )
        .unwrap();
        assert_eq!(hourly_delegated_joules(&path), 3.5);
    }
}
