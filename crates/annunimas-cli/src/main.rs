// sigil: REPAIR
use annunimas_apollo::transport::ipc::send_command as apollo_ipc_send_command;
use annunimas_apollo::transport::{
    expand_home as apollo_expand_home, ApolloDaemon, ApolloDaemonConfig,
};
use annunimas_apollo::{
    ApolloService, ExecutionPriority, ExecutionRequest, InterruptionAttachmentRequest,
};
use annunimas_athena::ingest::{
    crawl4ai_fetch_markdown, resolve_crawl_provider_order, scrapling_fetch_markdown, AthenaStore,
    BatchIngestReport,
};
use annunimas_athena::transport::ipc::send_command as athena_ipc_send_command;
use annunimas_athena::transport::{expand_home, AthenaDaemon, AthenaDaemonConfig};
use annunimas_charon::transport::ipc::send_command as charon_ipc_send_command;
use annunimas_charon::transport::{CharonDaemon, CharonDaemonConfig};
use annunimas_charon::{CharonRequestEnvelope, CharonService};
use annunimas_core::config::Config;
use annunimas_core::ledger::Ledger;
use annunimas_core::task::Task;
use annunimas_core::AipkgManifest;
use annunimas_hades::transport::ipc::send_command as hades_ipc_send_command;
use annunimas_hades::transport::{HadesDaemon, HadesDaemonConfig};
use annunimas_hades::{HadesService, QuorumProof};
use annunimas_hermes::transport::ipc::send_command as hermes_ipc_send_command;
use annunimas_hermes::transport::{HermesDaemon, HermesDaemonConfig};
use annunimas_hermes::{
    BoardroomPost, DiscordBot, HermesService, InboundMessage, InterruptionMessage,
};
use annunimas_mnemosyne::transport::ipc::send_command as mnemosyne_ipc_send_command;
use annunimas_mnemosyne::transport::{MnemosyneDaemon, MnemosyneDaemonConfig};
use annunimas_mnemosyne::{InformantEvent, MnemosyneService};
use annunimas_oracle::transport::ipc::send_command as oracle_ipc_send_command;
use annunimas_oracle::transport::{
    expand_home as oracle_expand_home, OracleDaemon, OracleDaemonConfig,
};
use annunimas_oracle::{OracleQuery, OracleService};
use annunimas_plutus::transport::ipc::send_command as plutus_ipc_send_command;
use annunimas_plutus::transport::{
    expand_home as plutus_expand_home, PlutusDaemon, PlutusDaemonConfig,
};
use annunimas_plutus::{CostModelConfig, JouleWorkUnit, PlutusService};
use annunimas_prometheus::transport::ipc::send_command as prometheus_ipc_send_command;
use annunimas_prometheus::transport::{PrometheusDaemon, PrometheusDaemonConfig};
use annunimas_prometheus::{Pipeline, PrometheusService};
use chrono::Utc;
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::json;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing_subscriber::EnvFilter;

mod cli_bootstrap;
mod cli_dispatch;
mod cli_interactive;
mod commands;
mod export_surface;
mod ipc_bridge;
mod observability;
mod policy_guard;
mod support;

use cli_bootstrap::{
    build_provider, build_router, default_runtime_socket, load_config, load_env_files,
    set_runtime_defaults,
};
use cli_dispatch::execute as execute_cli;
use cli_interactive::{
    format_decision_prompt_message, maybe_send_illuvatar_decision_prompt, resolve_athena_source_id,
};
use commands::chronos::ChronosCommands;
use commands::learning::LearningCommands;
use commands::loop_cmd::{HaltCommands, LoopCommands, WardenCommands};
use commands::state::StateCommands;
use commands::{
    aipkg, apollo, arandur, athena, charon, chronos, control, forge, hades, hermes, learning,
    loop_cmd, metrics, mnemosyne, onboarding, onboarding::OnboardingCommands, operating, oracle,
    pipeline, plutus, prometheus, state, utility,
};
use export_surface::ExportCommands;
use ipc_bridge::{
    apollo_call_or_local, athena_call_or_local, athena_ingest_batch_chunk,
    build_aipkg_preflight_receipt, charon_call_or_local, charon_call_or_local_async,
    hades_call_or_local, hermes_call_or_local, hermes_call_or_local_async, load_aipkg_manifest,
    merge_batch_report, mnemosyne_call_or_local, oracle_call_or_local, parse_execution_priority,
    parse_joulework_unit, parse_json_input, plutus_call_or_local, prometheus_call_or_local,
    socket_path_from_env,
};
use observability::{
    build_governance_observation, build_operations_briefing, build_ops_dashboard,
    disk_usage_percent, format_operations_briefing_text, persist_governance_observation,
    persist_queue_observability, queue_observability_snapshot,
};
use policy_guard::{
    default_governance_weights, default_signal_thresholds, enforce_policy_guard,
    load_active_ruleset, load_system_control_state, set_active_ruleset,
};
use support::{
    format_status_output, format_tools_output, list_agent_personalities, run_maintenance_cycle,
    runtime_surface, set_agent_personality, spawn_maintenance_cycle,
};

fn default_loopback_addr(port: u16) -> String {
    format!("{}:{port}", "127.0.0.1")
}

pub(crate) fn annunimas_root() -> PathBuf {
    env::var("ANNUNIMAS_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .and_then(Path::parent)
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."))
        })
}

pub(crate) fn home_root() -> PathBuf {
    env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| {
        annunimas_root()
            .parent()
            .unwrap_or(Path::new("/"))
            .to_path_buf()
    })
}

#[derive(Parser)]
#[command(name = "annunimas", about = "Annunimas Agent Orchestration System")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, default_value = "config/default.toml")]
    config: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a task through the pipeline
    Run {
        #[arg(short = 't', long)]
        task_type: String,
        description: String,
    },
    /// List registered agents and capabilities
    Tools,
    /// Show system status
    Status,
    /// Export generated Annunimas operational surfaces
    Export {
        #[command(subcommand)]
        command: ExportCommands,
    },
    /// Operate governance/runtime control surfaces
    Control {
        #[command(subcommand)]
        command: ControlCommands,
    },
    /// Operate executive council decision surfaces
    Council {
        #[command(subcommand)]
        command: CouncilCommands,
    },
    /// Evaluate venture/project opportunities against operating memory
    Venture {
        #[command(subcommand)]
        command: VentureCommands,
    },
    /// Operate utility surfaces migrated from legacy scripts
    Utility {
        #[command(subcommand)]
        command: UtilityCommands,
    },
    /// Operate bounded project-task pipeline workflows
    Pipeline {
        #[command(subcommand)]
        command: PipelineCommands,
    },
    /// Operate sovereign `.aipkg` package-law workflows
    Aipkg {
        #[command(subcommand)]
        command: AipkgCommands,
    },
    /// Operate ATHENA local service/corpus workflows
    Athena {
        #[command(subcommand)]
        command: AthenaCommands,
    },
    /// Operate PROMETHEUS executive service workflows
    Prometheus {
        #[command(subcommand)]
        command: PrometheusCommands,
    },
    /// Operate autonomous learning loop workflow surfaces
    Learning {
        #[command(subcommand)]
        command: LearningCommands,
    },
    /// Operate CHARON inference routing workflows
    Charon {
        #[command(subcommand)]
        command: CharonCommands,
    },
    /// Operate MNEMOSYNE memory service workflows
    Mnemosyne {
        #[command(subcommand)]
        command: MnemosyneCommands,
    },
    /// Operate HADES lifecycle workflows
    Hades {
        #[command(subcommand)]
        command: HadesCommands,
    },
    /// Operate HERMES communications workflows
    Hermes {
        #[command(subcommand)]
        command: HermesCommands,
    },
    /// Operate CHRONOS temporal workflow visibility
    Chronos {
        #[command(subcommand)]
        command: ChronosCommands,
    },
    /// Operate APOLLO workflow runtime
    Apollo {
        #[command(subcommand)]
        command: ApolloCommands,
    },
    /// Operate PLUTUS JouleWork runtime
    Plutus {
        #[command(subcommand)]
        command: PlutusCommands,
    },
    /// Operate ORACLE governance runtime
    Oracle {
        #[command(subcommand)]
        command: OracleCommands,
    },
    /// Serve Prometheus exposition for fleet scraping
    Metrics {
        #[command(subcommand)]
        command: MetricsCommands,
    },
    /// Run onboarding discovery, checklist, and proposal flows
    Onboarding {
        #[command(subcommand)]
        command: OnboardingCommands,
    },
    /// Validate on-disk agent state against the v0.1 contract
    State {
        #[command(subcommand)]
        command: StateCommands,
    },
    /// Halt switch for the autonomy loop
    Halt {
        #[command(subcommand)]
        command: HaltCommands,
    },
    /// Warden loop alert status and management
    Warden {
        #[command(subcommand)]
        command: WardenCommands,
    },
    /// Phase 1 autonomy loop driver (seed-goals, tick, …)
    Loop {
        #[command(subcommand)]
        command: LoopCommands,
    },
    /// Operate FORGE-MIND 3D asset authoring workflows (BlenderMCP bridge)
    Forge {
        #[command(subcommand)]
        command: ForgeCommands,
    },
    /// Vision-feedback iterate loop (alias: fi)
    ///
    /// Each round: forge generate → bpy render 3 angles → vision-LLM compares
    /// against a reference → governance scores the iteration. Accepts at
    /// --accept-threshold (default 0.85) or stops at --budget-iters (default 5).
    #[clap(alias = "fi")]
    Iterate {
        /// Path to the target reference image (what the asset should look like).
        target_image: String,
        /// ARDA asset id (e.g. `desk_left_surface`).
        #[arg(long)]
        asset_id: String,
        /// Asset domain folder under `apps/arda-hud/src/assets/scene/`.
        #[arg(long, default_value = "world")]
        domain: String,
        /// Initial positive prompt describing the asset.
        prompt: String,
        /// Negative prompt override.
        #[arg(long)]
        negative: Option<String>,
        /// Override the ARDA assets root.
        #[arg(long)]
        assets_root: Option<String>,
        /// Scene binding written to metadata.json.
        #[arg(long)]
        scene_binding: Option<String>,
        /// Material family written to metadata.json.
        #[arg(long, default_value = "world_terminal_housing")]
        material_family: String,
        /// Max iteration rounds.
        #[arg(long, default_value_t = 5)]
        budget_iters: u32,
        /// Vision match score to accept (0–1, default 0.85).
        #[arg(long, default_value_t = 0.85)]
        accept_threshold: f64,
        /// Override ComfyUI base URL.
        #[arg(long)]
        comfyui_addr: Option<String>,
        /// ComfyUI per-call timeout seconds (default 1800).
        #[arg(long)]
        timeout_secs: Option<u64>,
        /// Override vision-LLM endpoint (default http://annunimas-server:8081).
        #[arg(long)]
        vision_addr: Option<String>,
        /// Vision-LLM model alias (default qwen2.5-vl-7b-instruct).
        #[arg(long)]
        vision_model: Option<String>,
        /// Vision-LLM per-call timeout seconds (default 240).
        #[arg(long)]
        vision_timeout_secs: Option<u64>,
        /// Python interpreter with `bpy` for angle renders.
        #[arg(long)]
        bpy_python: Option<String>,
        /// Comma-separated camera angles (default front,three_quarter,side).
        #[arg(long)]
        angles: Option<String>,
        /// Render width/height in px (default 768).
        #[arg(long, default_value_t = 768)]
        render_size: u32,
    },
}

#[derive(Subcommand)]
pub(crate) enum ForgeCommands {
    /// Upgrade an existing ARDA asset GLB through the Blender bridge.
    ///
    /// Ephemeral by default: each run opens a fresh Blender scene, imports
    /// the existing GLB, applies the upgrade script, exports back, and writes
    /// the ARDA metadata.json sidecar.
    Upgrade {
        /// ARDA asset id (e.g. `desk_left_surface`, `upper_monitor_1`).
        asset_id: String,
        /// Asset domain folder under `apps/arda-hud/src/assets/scene/`.
        #[arg(long, default_value = "world")]
        domain: String,
        /// Built-in template name: `prompt_1` (default), `prompt_2` (baking), `prompt_3` (monitor).
        #[arg(long, default_value = "prompt_1")]
        template: String,
        /// Path to a raw Blender-Python script (overrides --template and --prompt-file).
        #[arg(long)]
        script: Option<String>,
        /// Path to a natural-language prompt file. Routed through Charon to a local LLM
        /// for Python translation. Not used in v0 — placeholder for future wiring.
        #[arg(long)]
        prompt_file: Option<String>,
        /// Override the ARDA assets root (default: apps/arda-hud/src/assets/scene).
        #[arg(long)]
        assets_root: Option<String>,
        /// Scene binding written to metadata.json (defaults to asset_id).
        #[arg(long)]
        scene_binding: Option<String>,
        /// Material family written to metadata.json.
        #[arg(long, default_value = "world_district_structure")]
        material_family: String,
        /// Persistent mode: do not reset the Blender scene before running.
        #[arg(long, default_value_t = false)]
        persistent: bool,
        /// Force GLB export on/off. Default follows the template's recommendation
        /// (Prompt 2 baking defaults off; Prompts 1 and 3 default on).
        #[arg(long)]
        export_glb: Option<bool>,
    },
    /// Show forge-mind status and configured Blender bridge endpoint.
    Status,
    /// Apply the ARDA monitor materialization pass to an existing GLB.
    MaterializeMonitor {
        /// ARDA asset id (e.g. `upper_monitor_1`).
        asset_id: String,
        /// Asset domain folder under `apps/arda-hud/src/assets/scene/`.
        #[arg(long, default_value = "world")]
        domain: String,
        /// Override the ARDA assets root.
        #[arg(long)]
        assets_root: Option<String>,
    },
    /// Generate a new ARDA asset from a text prompt via ComfyUI + Hunyuan3D-2.
    ///
    /// Submits a text→3D workflow to ComfyUI, downloads the resulting GLB,
    /// and drops it plus an ARDA-contract metadata.json into
    /// `apps/arda-hud/src/assets/scene/<domain>/<asset_id>/`.
    Generate {
        /// Positive prompt describing the asset.
        prompt: String,
        /// ARDA asset id (becomes the folder + filename).
        #[arg(long)]
        asset_id: String,
        /// Asset domain folder under `apps/arda-hud/src/assets/scene/`.
        #[arg(long, default_value = "world")]
        domain: String,
        /// Negative prompt. Defaults to a strong "no humans/text/clutter" baseline.
        #[arg(long)]
        negative: Option<String>,
        /// Override the ARDA assets root.
        #[arg(long)]
        assets_root: Option<String>,
        /// Scene binding written to metadata.json (defaults to asset_id).
        #[arg(long)]
        scene_binding: Option<String>,
        /// Material family written to metadata.json.
        #[arg(long, default_value = "world_terminal_housing")]
        material_family: String,
        /// Override ComfyUI base URL (else FORGE_COMFYUI_ADDR or http://annunimas-server:8188).
        #[arg(long)]
        comfyui_addr: Option<String>,
        /// Per-call workflow timeout in seconds (default 1800).
        #[arg(long)]
        timeout_secs: Option<u64>,
        /// SDXL seed.
        #[arg(long)]
        sdxl_seed: Option<i64>,
        /// SDXL sampler steps.
        #[arg(long)]
        sdxl_steps: Option<u32>,
        /// SDXL CFG scale.
        #[arg(long)]
        sdxl_cfg: Option<f64>,
        /// Hunyuan3D seed.
        #[arg(long)]
        hy3d_seed: Option<i64>,
        /// Hunyuan3D sampler steps.
        #[arg(long)]
        hy3d_steps: Option<u32>,
        /// Hunyuan3D octree resolution (256 = fast, 384 = balanced, 512 = slow + detailed).
        #[arg(long)]
        hy3d_octree: Option<u32>,
        /// Run Prompt 1 (topology cleanup) over the generated GLB via the Blender bridge.
        #[arg(long, default_value_t = false)]
        post_cleanup: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum MetricsCommands {
    /// Run the exporter as an HTTP server (scrape via /metrics)
    Serve {
        #[arg(long, default_value = "0.0.0.0")]
        bind: String,
        #[arg(long, default_value_t = 9101)]
        port: u16,
        #[arg(long, default_value_t = 15)]
        refresh_secs: u64,
        #[arg(long, default_value_t = false)]
        system_metrics: bool,
    },
    /// Print one snapshot of exposition to stdout and exit
    Snapshot {
        #[arg(long, default_value_t = false)]
        system_metrics: bool,
    },
}

#[derive(Subcommand)]
enum AipkgCommands {
    /// Validate a `.aipkg` manifest against the sovereign core law
    ValidateManifest { manifest_path: String },
    /// Emit a zero-work preflight receipt for a `.aipkg` manifest
    Preflight {
        manifest_path: String,
        #[arg(long)]
        runtime_profile: Option<String>,
        #[arg(long)]
        out: Option<String>,
    },
}

#[derive(Subcommand)]
enum AthenaCommands {
    /// Start ATHENA daemon (IPC + optional HTTP/SSE)
    Start {
        #[arg(long, default_value_t = default_runtime_socket("athena.sock"))]
        socket_path: String,
        #[arg(long, default_value_t = default_loopback_addr(5111))]
        http_addr: String,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        http_enabled: bool,
    },
    /// Show ATHENA corpus status
    Status,
    /// Show ATHENA Knowledge Vault v0 scaffold/status without scanning the corpus
    VaultStatus,
    /// Ingest a source into local ATHENA corpus
    Ingest {
        input: String,
        #[arg(long, default_value = "cli")]
        submitted_by: String,
        #[arg(long, default_value = "manual cli ingest")]
        task_context: String,
    },
    IngestBatch {
        input_file: String,
        #[arg(long, default_value = "cli")]
        submitted_by: String,
        #[arg(long, default_value = "manual cli batch ingest")]
        task_context: String,
        #[arg(long, default_value_t = 250)]
        batch_size: usize,
        #[arg(long, default_value_t = 300)]
        max_receipts: usize,
    },
    /// Ingest URL lines/bookmarks into ATHENA via the shared ingest pipeline
    ImportUrls {
        input_file: String,
        #[arg(long, default_value = "cli")]
        submitted_by: String,
        #[arg(long, default_value = "manual corpus URL import")]
        task_context: String,
    },
    /// Import an X bookmarks JSON export as XBookmark sources
    ImportXBookmarks {
        input_file: String,
        #[arg(long, default_value = "cli")]
        submitted_by: String,
        #[arg(long, default_value = "X bookmarks export import")]
        task_context: String,
    },
    /// Search X via Hermes Agent x_search and ingest returned post URLs as XPost sources
    ImportXSearch {
        query: String,
        #[arg(long, default_value_t = 10)]
        limit: usize,
        #[arg(long, default_value = "hermes")]
        hermes_bin: String,
        #[arg(long)]
        capture_file: Option<PathBuf>,
        #[arg(long, default_value = "hermes-x-search")]
        submitted_by: String,
        #[arg(long, default_value = "Hermes x_search assisted import")]
        task_context: String,
    },
    /// Import ChatGPT/Claude/generic AI chat exports as ChatExport sources
    ImportAiChats {
        input_file: String,
        #[arg(long, default_value = "cli")]
        submitted_by: String,
        #[arg(long, default_value = "AI chat export import")]
        task_context: String,
    },
    /// Ingest a bounded wave of local human-readable note files
    HumanCorpusWave {
        #[arg(long, default_value_t = 8)]
        limit: usize,
    },
    /// Scan /human into classifier JSONL without moving or promoting files
    HumanScan {
        #[arg(long, default_value = "human")]
        human_root: PathBuf,
        #[arg(long, default_value = "data/athena/human_ingestion_results.jsonl")]
        output: PathBuf,
        #[arg(
            long,
            default_value = "data/athena/human_contradiction_candidates.jsonl"
        )]
        contradictions: PathBuf,
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Extract and ingest a bounded wave of local documents and archives
    HumanDocumentWave {
        #[arg(long, default_value_t = 8)]
        limit: usize,
    },
    /// Fetch markdown via local crawl4ai and capture it into ATHENA
    Crawl {
        url: String,
        #[arg(long, default_value = "fit")]
        filter: String,
        #[arg(long)]
        query: Option<String>,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        ingest: bool,
        #[arg(long, default_value = "cli")]
        submitted_by: String,
        #[arg(long, default_value = "crawl4ai assisted ingest")]
        task_context: String,
    },
    /// Query local ATHENA corpus
    Query {
        query: String,
        #[arg(long, default_value_t = 8)]
        limit: usize,
    },
    /// Trigger deep analysis for a source id
    Deep {
        source_id: String,
        #[arg(long, default_value = "manual cli deep")]
        reason: String,
    },
    /// Process queued deep-analysis items
    DeepProcess {
        #[arg(long, default_value_t = 25)]
        limit: usize,
        #[arg(long, default_value_t = false)]
        retry_failed: bool,
    },
    /// Print digest ledger entries (optionally filtered by source id)
    Digest {
        source_id: Option<String>,
        #[arg(long, default_value_t = 25)]
        limit: usize,
    },
    /// Show policy-readiness gate outcomes for deep sources
    PolicyReadiness {
        #[arg(long, default_value_t = 25)]
        limit: usize,
    },
    /// Queue remediation tasks for policy-readiness blockers
    PolicyPromote {
        #[arg(long, default_value_t = 25)]
        limit: usize,
        #[arg(long, default_value_t = false)]
        reevaluate: bool,
    },
    /// Harvest opposing viewpoints for a source and queue them for deep analysis
    OppositionHarvest {
        source_id: String,
        #[arg(long)]
        topic: Option<String>,
        #[arg(long, default_value = "cli")]
        submitted_by: String,
    },
    /// Build a review-gated packet intake surface from a Phase 2F audit packet directory
    PacketIntake {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long)]
        packet_dir: String,
        #[arg(long, default_value = "data/athena/packet_intake.jsonl")]
        output: String,
        #[arg(long, default_value_t = false)]
        write: bool,
    },
    /// Build review-gated promotion candidates from an ATHENA packet directory
    PacketPromotionSurface {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long)]
        packet_dir: String,
        #[arg(long, default_value = "audit/ATHENA_PACKET_PROMOTION_SURFACE.md")]
        output: String,
        #[arg(long, default_value_t = false)]
        write: bool,
    },
    /// Generate deterministic planning tasks from ATHENA evidence
    GeneratePlanningTasks {
        source_id: String,
        #[arg(long, default_value_t = 5)]
        limit: usize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum OpsBriefingFormat {
    Json,
    Text,
}

#[derive(Subcommand)]
enum PrometheusCommands {
    /// Start PROMETHEUS daemon (IPC + optional HTTP/SSE)
    Start {
        #[arg(long, default_value_t = default_runtime_socket("prometheus.sock"))]
        socket_path: String,
        #[arg(long, default_value_t = default_loopback_addr(5113))]
        http_addr: String,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        http_enabled: bool,
    },
    /// Show executive status
    Status,
    /// Show unified CEO operations dashboard
    OpsDashboard,
    /// Show compact read-only operations briefing from latest receipts/state
    OpsBriefing {
        #[arg(long)]
        root: Option<String>,
        #[arg(long, value_enum, default_value = "json")]
        format: OpsBriefingFormat,
    },
    /// Run scheduled maintenance sweep (CHARON + HADES + optional prune)
    Maintenance {
        #[arg(long, default_value = "scheduled")]
        sweep_type: String,
        #[arg(long, default_value_t = 300)]
        cooldown_seconds: i64,
        #[arg(long, default_value_t = false)]
        r#async: bool,
        #[arg(long, default_value_t = false)]
        prune: bool,
        #[arg(long, default_value_t = 92)]
        prune_threshold_pct: u8,
    },
    /// Show order-of-battle roster
    Roster,
    /// Show recent machine thoughts
    Thoughts {
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// View pending (or all) Illuvatar escalations
    Escalate {
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long, default_value_t = false)]
        include_resolved: bool,
    },
    /// Resolve an escalation by id
    ResolveEscalation {
        escalation_id: String,
        #[arg(long, default_value = "resolved by operator")]
        note: String,
    },
    /// Reconcile stale Prometheus runtime orders/escalations; dry-run unless --apply is set
    ReconcileRuntime {
        #[arg(long)]
        before: String,
        #[arg(long, default_value_t = false)]
        apply: bool,
        #[arg(long, default_value = "resolved by Prometheus runtime reconciliation")]
        note: String,
    },
    /// Open/report/close a live council fanout session through HERMES
    CouncilFanout {
        topic: String,
        #[arg(long = "participant")]
        participants: Vec<String>,
        #[arg(long)]
        context: Option<String>,
    },
    /// Show latest execution intents (stateful lifecycle view)
    ExecutionIntents {
        #[arg(long, default_value_t = 50)]
        limit: usize,
        #[arg(long, default_value_t = false)]
        include_terminal: bool,
    },
    /// Show latest startup recovery summary for execution intents
    ExecutionIntentRecovery,
    /// Transition execution intent state
    TransitionIntent {
        intent_id: String,
        status: String,
        #[arg(long)]
        note: Option<String>,
    },
    /// Compact execution intents by retention/max-keep policy
    CompactIntents {
        #[arg(long, default_value_t = 14)]
        retention_days: i64,
        #[arg(long, default_value_t = 5000)]
        max_keep: usize,
    },
    /// Detect runtime drift against hashed baselines and optionally auto-open reconcile tasks
    DriftCheck {
        #[arg(long, default_value_t = false)]
        reconcile: bool,
    },
    /// Set or update an agent personality profile
    PersonalitySet {
        agent_id: String,
        personality: String,
        #[arg(long, default_value = "balanced")]
        comms_style: String,
        #[arg(long)]
        notes: Option<String>,
    },
    /// List personality profiles
    PersonalityList,
    /// Manage active CEO ruleset profile
    Ruleset {
        #[command(subcommand)]
        command: RulesetCommands,
    },
    /// CEO autopilot — run an autonomous cycle (decompose → gate → delegate → dispatch)
    Autopilot {
        #[command(subcommand)]
        command: AutopilotCommands,
    },
    /// Arandur operational learning protocol and state surfaces
    Arandur {
        #[command(subcommand)]
        command: ArandurCommands,
    },
    /// Charon telemetry synthesis and review-gated report surfaces
    Charon {
        #[command(subcommand)]
        command: PrometheusCharonCommands,
    },
}

#[derive(Subcommand)]
enum PrometheusCharonCommands {
    /// Synthesize Charon routing/governance telemetry into a review-gated append-only summary
    TelemetryReport {
        #[arg(long)]
        root: Option<String>,
        /// Window to summarize: all, 7d, 24h, or RFC3339 timestamp
        #[arg(long, default_value = "7d")]
        since: String,
        /// Append to data/charon/telemetry_summaries.jsonl; dry-run by default
        #[arg(long, default_value_t = false)]
        write: bool,
        /// Required with --write to preserve operator review trail
        #[arg(long)]
        justification: Option<String>,
        /// Maximum events to read per source file after filtering; 0 means unlimited
        #[arg(long, default_value_t = 0)]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum ArandurCommands {
    /// Show Arandur runtime state
    Status {
        #[arg(long)]
        root: Option<String>,
    },
    /// Initialize or refresh Arandur Level 1 runtime state
    Initialize {
        #[arg(long)]
        root: Option<String>,
    },
    /// Append a review-gated operational learning episode
    RecordEpisode {
        #[arg(long)]
        root: Option<String>,
        #[arg(long, default_value = "review")]
        episode_type: String,
        #[arg(long)]
        summary: String,
        #[arg(long = "evidence")]
        evidence: Vec<String>,
        #[arg(long)]
        recommendation: Option<String>,
    },
    /// Read task queue, ATHENA packet state, runtime state, and episode ledger
    Observe {
        #[arg(long)]
        root: Option<String>,
    },
    /// Emit a read-only Arandur/PROMETHEUS/ATHENA/governance system map
    SystemMap {
        #[arg(long)]
        root: Option<String>,
    },
    /// Emit review-gated system improvement candidates from the read-only system map
    ImprovementScan {
        #[arg(long)]
        root: Option<String>,
    },
    /// Review an ATHENA Phase 2F-style packet without promoting it
    ReviewPacket {
        #[arg(long)]
        root: Option<String>,
        #[arg(long, default_value = "audit/HUMAN_INBOX_PHASE2F_2026-05-17")]
        packet_dir: String,
    },
    /// Append review-required next-action recommendations to Arandur ledger
    RecommendNext {
        #[arg(long)]
        root: Option<String>,
        /// Report candidate recommendations without appending to data/arandur/recommendations.jsonl
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    /// Run one bounded observe/orient/decide/verify/reflect Arandur cycle
    Cycle {
        #[arg(long)]
        root: Option<String>,
        #[arg(long, default_value = "audit/HUMAN_INBOX_PHASE2F_2026-05-17")]
        packet_dir: String,
        /// Append review-required recommendations; otherwise recommendation generation is dry-run only
        #[arg(long, default_value_t = false)]
        append_recommendations: bool,
        /// Append a review-gated episode record for this cycle
        #[arg(long, default_value_t = false)]
        record_episode: bool,
    },
    /// Run the Arandur read-only safety benchmark without mutating ledgers or queue state
    Benchmark {
        #[arg(long)]
        root: Option<String>,
        #[arg(long, default_value = "audit/HUMAN_INBOX_PHASE2F_2026-05-17")]
        packet_dir: String,
    },
    /// Summarize the Arandur recommendation ledger and review gates
    Recommendations {
        #[arg(long)]
        root: Option<String>,
    },
    /// Report Level 1 to Level 2 readiness gates without mutation approval
    Readiness {
        #[arg(long)]
        root: Option<String>,
    },
    /// Report or explicitly write a Level 2 runtime promotion after readiness gates pass
    PromoteLevel {
        #[arg(long)]
        root: Option<String>,
        #[arg(long, default_value_t = 2)]
        target: u8,
        /// Mutate core/state/arandur/runtime.json only after gates pass and approval-note is present
        #[arg(long, default_value_t = false)]
        write: bool,
        /// Human/operator approval note required when --write is set
        #[arg(long)]
        approval_note: Option<String>,
    },
    /// Report bounded Level 2 mutation classes without mutating state
    MutationClasses {
        #[arg(long)]
        root: Option<String>,
    },
    /// Verify a bounded supervised mutation against pre/post evidence
    VerifyMutation {
        #[arg(long)]
        root: Option<String>,
        #[arg(long)]
        mutation_class: String,
        #[arg(long)]
        target_path: String,
        #[arg(long)]
        pre_sha1: Option<String>,
        #[arg(long)]
        pre_bytes: Option<u64>,
        /// Append the verification record to data/arandur/mutation_evidence.jsonl
        #[arg(long, default_value_t = false)]
        write_report: bool,
    },
    /// Emit review-gated rollback evidence without automatic rollback mutation
    RollbackReport {
        #[arg(long)]
        root: Option<String>,
        #[arg(long)]
        mutation_class: String,
        #[arg(long)]
        target_path: String,
        #[arg(long)]
        reason: String,
        /// Append the rollback report to data/arandur/mutation_evidence.jsonl
        #[arg(long, default_value_t = false)]
        write_report: bool,
    },
    /// Plan bounded Phase 6B scout missions without executing network or mutation actions
    ScoutPlan {
        #[arg(long)]
        root: Option<String>,
        /// Mission scope to shape read-only scout mission candidates
        #[arg(long, default_value = "public internet opportunity scouting")]
        scope: String,
        /// Maximum number of candidate scout missions to emit
        #[arg(long, default_value_t = 3)]
        limit: usize,
    },
    /// Execute a bounded Phase 6C scout intake from operator-provided evidence, never raw network promotion
    ScoutExecute {
        #[arg(long)]
        root: Option<String>,
        /// Stable scout mission identifier from scout-plan or operator review
        #[arg(long)]
        mission_id: String,
        /// Operator-reviewed scout scope
        #[arg(long, default_value = "public internet opportunity scouting")]
        scope: String,
        /// Source URL(s) examined outside the command; recorded as citations only
        #[arg(long = "source-url")]
        source_urls: Vec<String>,
        /// Evidence/notes file to summarize into the review-gated scout ledger
        #[arg(long)]
        evidence_file: Option<String>,
        /// Append to data/arandur/scout_findings.jsonl; default is dry-run report only
        #[arg(long, default_value_t = false)]
        write: bool,
        /// Required operator justification when --write is set
        #[arg(long)]
        justification: Option<String>,
    },
    /// Aggregate Phase 6D patterns across Arandur ledgers into review-gated synthesis candidates
    PatternSynthesis {
        #[arg(long)]
        root: Option<String>,
        /// Append to data/arandur/pattern_synthesis.jsonl; default is dry-run report only
        #[arg(long, default_value_t = false)]
        write: bool,
        /// Required operator justification when --write is set
        #[arg(long)]
        justification: Option<String>,
    },
    /// Promote approved Phase 6D patterns into bounded review-gated mission candidate packets
    MissionPromotion {
        #[arg(long)]
        root: Option<String>,
        /// Append to data/arandur/mission_candidates.jsonl; default is dry-run report only
        #[arg(long, default_value_t = false)]
        write: bool,
        /// Required operator justification when --write is set
        #[arg(long)]
        justification: Option<String>,
    },
    /// Review approved Phase 6E mission candidates into bounded Phase 6F review packets
    MissionReview {
        #[arg(long)]
        root: Option<String>,
        /// Optional mission_candidate_id filter; default reviews all eligible candidates
        #[arg(long)]
        candidate_id: Option<String>,
        /// Append to data/arandur/mission_reviews.jsonl; default is dry-run report only
        #[arg(long, default_value_t = false)]
        write: bool,
        /// Required operator justification when --write is set
        #[arg(long)]
        justification: Option<String>,
    },
    /// Prepare Phase 6G human approval requests from reviewed mission packets without queue mutation
    MissionApprovalRequest {
        #[arg(long)]
        root: Option<String>,
        /// Optional source_mission_candidate_id filter; default requests approval for all eligible reviews
        #[arg(long)]
        candidate_id: Option<String>,
        /// Append to data/arandur/mission_approval_requests.jsonl; default is dry-run report only
        #[arg(long, default_value_t = false)]
        write: bool,
        /// Required operator justification when --write is set
        #[arg(long)]
        justification: Option<String>,
    },
    /// Record an explicit append-only operator decision for a Phase 6G approval request
    MissionApprovalDecision {
        #[arg(long)]
        root: Option<String>,
        /// Phase 6G approval_request_id being decided
        #[arg(long)]
        approval_request_id: String,
        /// Decision status to append; Phase 6J currently accepts approved only
        #[arg(long, default_value = "approved")]
        status: String,
        /// Required operator justification for the decision append
        #[arg(long)]
        justification: String,
    },
    /// Convert approved Phase 6G requests into bounded Phase 6H queue proposals without queue mutation
    MissionQueueProposal {
        #[arg(long)]
        root: Option<String>,
        /// Optional source_mission_candidate_id filter; default proposes all eligible approval requests
        #[arg(long)]
        candidate_id: Option<String>,
        /// Append to data/arandur/mission_queue_proposals.jsonl; default is dry-run report only
        #[arg(long, default_value_t = false)]
        write: bool,
        /// Required operator justification when --write is set
        #[arg(long)]
        justification: Option<String>,
    },
    /// Prepare Phase 6I queue write requests from Phase 6H proposals without canonical queue mutation
    MissionQueueWriteRequest {
        #[arg(long)]
        root: Option<String>,
        /// Optional source_mission_candidate_id filter; default prepares all eligible queue proposals
        #[arg(long)]
        candidate_id: Option<String>,
        /// Append to data/arandur/mission_queue_write_requests.jsonl; default is dry-run report only
        #[arg(long, default_value_t = false)]
        write: bool,
        /// Required operator justification when --write is set
        #[arg(long)]
        justification: Option<String>,
    },
    /// Execute Phase 6J approved queue write requests into canonical task queue
    ExecuteQueueWrite {
        #[arg(long)]
        root: Option<String>,
        /// Optional source_mission_candidate_id filter; default executes all eligible approved write requests
        #[arg(long)]
        candidate_id: Option<String>,
        /// Append to core/projects/tasks/queue.jsonl; default is dry-run report only
        #[arg(long, default_value_t = false)]
        write: bool,
        /// Required operator justification when --write is set
        #[arg(long)]
        justification: Option<String>,
    },
    /// Append a bounded ARDA presence event for HUD/native scene ingestion
    PresenceEvent {
        #[arg(long)]
        root: Option<String>,
        /// Stable event id; defaults to a deterministic id from agent, timestamp, and correlation id
        #[arg(long)]
        event_id: Option<String>,
        /// Presence agent identifier
        #[arg(long, default_value = "arandur")]
        agent: String,
        /// Bounded presence mode binding, e.g. observing, advising, escalating, offline
        #[arg(long)]
        mode: String,
        /// Bounded attention binding, e.g. idle, focused, elevated, critical
        #[arg(long)]
        attention: String,
        /// Bounded accent binding, e.g. cyan, gold, amber, red, violet
        #[arg(long)]
        accent: String,
        /// Named scene anchor target, not arbitrary transform data
        #[arg(long, default_value = "boardroom.hologram_anchor")]
        anchor_target: String,
        /// Optional mission id attached to this presence transition
        #[arg(long)]
        mission_id: Option<String>,
        /// Optional replay/idempotency correlation id
        #[arg(long)]
        correlation_id: Option<String>,
        /// Optional event timestamp; defaults to current UTC RFC3339 timestamp
        #[arg(long)]
        timestamp_utc: Option<String>,
    },
    /// Report effective canonical queue backlog after append-only supersession
    MissionBacklog {
        #[arg(long)]
        root: Option<String>,
    },
    /// Inspect or decide the next supervised automation gate candidate
    Gate {
        #[arg(long)]
        root: Option<String>,
        #[arg(value_enum)]
        action: ArandurGateAction,
        /// Candidate/objective id for approve and deny decisions
        #[arg(long)]
        objective_id: Option<String>,
        /// Required operator justification for approve and deny decisions
        #[arg(long)]
        justification: Option<String>,
    },
}

#[derive(Clone, Debug, ValueEnum)]
enum ArandurGateAction {
    Next,
    Blocked,
    Approve,
    Deny,
}

#[derive(Subcommand)]
enum CouncilCommands {
    /// Emit evidence-linked system improvement recommendations
    RecommendImprovements {
        #[arg(long)]
        root: Option<String>,
        #[arg(long, default_value = "annunimas")]
        scope: String,
        #[arg(long, default_value_t = 10)]
        limit: usize,
        /// Append review-required recommendations to the Arandur ledger
        #[arg(long, default_value_t = false)]
        append_recommendations: bool,
    },
}

#[derive(Subcommand)]
enum VentureCommands {
    /// Evaluate a venture/project idea from ATHENA and MNEMOSYNE evidence
    Evaluate {
        query: String,
        #[arg(long)]
        root: Option<String>,
        #[arg(long, default_value = "venture")]
        scope: String,
        #[arg(long, default_value_t = 5)]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum AutopilotCommands {
    /// Run a single autopilot cycle and print the report
    Once {
        #[arg(long)]
        root: Option<String>,
        #[arg(long, default_value_t = false)]
        read_only: bool,
    },
    /// Run the autopilot loop until interrupted
    Run {
        #[arg(long)]
        root: Option<String>,
        #[arg(long, default_value_t = 30)]
        interval: u64,
        #[arg(long, default_value_t = false)]
        read_only: bool,
    },
    /// Show the most recent cycle report from data/ceo/autopilot.state.json
    Status {
        #[arg(long)]
        root: Option<String>,
    },
    /// Discover and classify knowledge sources without task-queue mutation
    KnowledgeTriage {
        #[arg(long)]
        root: Option<String>,
        /// Force dry-run/read-only execution; this is also the default unless --write is set
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        /// Write core/knowledge JSONL artifacts; omitted means dry-run only
        #[arg(long, default_value_t = false)]
        write: bool,
        /// Override discovery roots; repeatable. Defaults to human, docs, docs/plans, ../Eregion.
        #[arg(long = "source-root")]
        source_roots: Vec<String>,
    },
    /// Promote eligible safe-local knowledge candidates to internal tasks; dry-run unless --write is set
    PromoteKnowledgeTasks {
        #[arg(long)]
        root: Option<String>,
        /// Promotion lane selector. Currently only safe-local is write-eligible.
        #[arg(long, default_value = "safe-local")]
        lane: String,
        /// Force dry-run/read-only execution; this is also the default unless --write is set
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        /// Append safe-local tasks and promotion receipts; omitted means dry-run only
        #[arg(long, default_value_t = false)]
        write: bool,
        /// Explicit operator approval evidence required for write-mode task queue mutation
        #[arg(long = "approval-evidence")]
        approval_evidence: Option<String>,
        /// Override discovery roots; repeatable. Defaults to human, docs, docs/plans, ../Eregion.
        #[arg(long = "source-root")]
        source_roots: Vec<String>,
    },
    /// Evaluate promoted knowledge tasks for bounded Arandur execution; dry-run unless --write is set
    ExecuteKnowledgeTasks {
        #[arg(long)]
        root: Option<String>,
        /// Force dry-run/read-only execution; this is also the default unless --write is set
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        /// Append Arandur execution guard receipts; omitted means dry-run only
        #[arg(long, default_value_t = false)]
        write: bool,
    },
}

#[derive(Subcommand)]
enum RulesetCommands {
    /// Show active ruleset state and policy
    Status,
    /// Set active ruleset profile
    Set {
        /// Profile name: annunimas_totality or citadel_business
        profile: String,
        #[arg(long, default_value = "manual operator update")]
        reason: String,
        #[arg(long)]
        expires_at_utc: Option<String>,
    },
}

#[derive(Subcommand)]
enum MnemosyneCommands {
    /// Start MNEMOSYNE daemon (IPC + optional HTTP/SSE)
    Start {
        #[arg(long, default_value_t = default_runtime_socket("mnemosyne.sock"))]
        socket_path: String,
        #[arg(long, default_value_t = default_loopback_addr(5115))]
        http_addr: String,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        http_enabled: bool,
    },
    /// Show memory service status
    Status,
    /// Show configured memory storage paths
    Paths,
    /// Print memory statistics
    Stats,
    /// Encode a manual informant event into memory
    Encode {
        #[arg(long, default_value = "manual")]
        event_type: String,
        #[arg(long, default_value = "cli_mneme")]
        informant_id: String,
        #[arg(long, default_value = "illuvatar")]
        crate_name: String,
        content: String,
        #[arg(long)]
        confidence: Option<f64>,
        #[arg(long = "tag")]
        tags: Vec<String>,
    },
    /// Recall recent memories
    RecallRecent {
        #[arg(long, default_value_t = 24)]
        hours: i64,
        #[arg(long)]
        crate_name: Option<String>,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long)]
        query: Option<String>,
        #[arg(long, default_value_t = 12)]
        limit: usize,
    },
    /// Recall memory_seed conclusions from the knowledge triage registry
    KnowledgeSeeds {
        #[arg(long)]
        query: Option<String>,
        #[arg(long, default_value_t = 12)]
        limit: usize,
    },
    /// Trigger a consolidation sweep
    Consolidate {
        #[arg(long, default_value_t = 24)]
        hours: i64,
    },
    /// Print identity state summary
    IdentityState,
    /// Sync Obsidian/human vault into Mnemosyne memory/index
    ObsidianSync {
        #[arg(long, default_value = "human/.obsidian")]
        vault_path: String,
        #[arg(long, default_value_t = 200)]
        max_files: usize,
    },
}

#[derive(Subcommand)]
enum CharonCommands {
    /// Start CHARON daemon (IPC + optional HTTP/SSE)
    Start {
        #[arg(long, default_value_t = default_runtime_socket("charon.sock"))]
        socket_path: String,
        #[arg(long, default_value_t = default_loopback_addr(5110))]
        http_addr: String,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        http_enabled: bool,
    },
    /// Show CHARON status
    Status {
        /// Emit JSON. Status output is JSON by default; this flag is accepted for script compatibility.
        #[arg(long, default_value_t = false)]
        json: bool,
        /// Emit compact single-line JSON instead of pretty JSON.
        #[arg(long, default_value_t = false)]
        compact: bool,
    },
    /// Show full provider state
    State,
    /// Show provider list
    Providers,
    /// Show recent CHARON route audit grouped by agent/task/provider/model
    RouteAudit {
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Show CHARON provider truth, quota, fallback, and observed route rollup
    Observability,
    /// Show compact CHARON operator summary: providers, routable models, cooldowns, usage, failures
    OperatorSummary {
        /// Emit compact single-line JSON instead of pretty JSON.
        #[arg(long, default_value_t = false)]
        compact: bool,
    },
    /// Run or plan the CHARON adaptive-routing eval suite
    Eval {
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        dry_run: bool,
    },
    /// Route a request envelope by task type
    Route {
        #[arg(long, default_value = "athena")]
        agent_id: String,
        #[arg(long)]
        force_provider_id: Option<String>,
        #[arg(long = "exclude-provider-id")]
        exclude_provider_ids: Vec<String>,
        task_type: String,
        prompt: String,
        #[arg(long, default_value = "normal")]
        priority: String,
    },
    /// Route and forward request to OpenAI-compatible provider
    Proxy {
        #[arg(long, default_value = "athena")]
        agent_id: String,
        #[arg(long)]
        force_provider_id: Option<String>,
        #[arg(long = "exclude-provider-id")]
        exclude_provider_ids: Vec<String>,
        task_type: String,
        prompt: String,
        #[arg(long, default_value = "normal")]
        priority: String,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        dry_run: bool,
    },
    /// Force cooldown on provider id
    Cooldown {
        provider_id: String,
        #[arg(long, default_value_t = 60)]
        seconds: i64,
    },
    /// Report provider result for circuit-breaker/health tracking
    ProviderResult {
        provider_id: String,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        ok: bool,
        #[arg(long)]
        latency_ms: Option<u64>,
        #[arg(long)]
        error: Option<String>,
    },
    /// Reload CHARON provider config from TOML
    ReloadConfig,
    /// Show CHARON paths
    Paths,
    /// Probe each enabled provider with a canonical OpenAI tool-call
    /// payload; report status, latency and first line of any error body.
    /// Bypasses Echo Gate, scoring and cooldown so connectivity/payload
    /// problems surface directly.
    Probe {
        /// Limit probe to provider ids matching one of these (substring match).
        #[arg(long = "only")]
        only: Vec<String>,
        /// Skip the tools/tool_choice fields (probe plain chat instead).
        #[arg(long, default_value_t = false)]
        no_tools: bool,
        /// Per-request timeout in seconds.
        #[arg(long, default_value_t = 15)]
        timeout_secs: u64,
    },
    /// Hit each enabled provider's /models endpoint and print the
    /// catalog of currently-available model IDs. Use this when
    /// hardcoded model IDs in config drift out of sync with reality.
    Discover {
        /// Limit discovery to provider ids matching one of these (substring match).
        #[arg(long = "only")]
        only: Vec<String>,
        /// Per-request timeout in seconds.
        #[arg(long, default_value_t = 15)]
        timeout_secs: u64,
        /// Include only model IDs containing any of these needles (substring match).
        #[arg(long = "grep")]
        grep: Vec<String>,
    },
}

#[derive(Subcommand)]
enum HadesCommands {
    /// Start HADES daemon (IPC + optional HTTP/SSE)
    Start {
        #[arg(long, default_value_t = default_runtime_socket("hades.sock"))]
        socket_path: String,
        #[arg(long, default_value_t = default_loopback_addr(5112))]
        http_addr: String,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        http_enabled: bool,
    },
    /// Show current HADES status
    Status,
    /// Trigger immediate sweep
    Sweep {
        #[arg(long, default_value = "manual")]
        sweep_type: String,
        #[arg(long)]
        path: Option<String>,
    },
    /// Show action queue
    Queue {
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Query audit log
    Log {
        #[arg(long, default_value_t = 100)]
        limit: usize,
        #[arg(long)]
        event_filter: Option<String>,
    },
    /// Generate a dry-run HADES task queue compaction plan and approval packet
    QueueCompactionPlan {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long, default_value = "audit/hades-queue-compaction-runs")]
        out_dir: String,
        #[arg(long, default_value = "operator")]
        operator_id: String,
        #[arg(long, default_value_t = false)]
        approved: bool,
    },
    /// Apply an approved HADES task queue compaction plan; dry-run unless --apply is set
    QueueCompactionApply {
        #[arg(long)]
        plan: String,
        #[arg(long)]
        approval_packet: String,
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long, default_value = "audit/hades-queue-compaction-runs/rollback")]
        rollback_dir: String,
        #[arg(long, default_value_t = false)]
        apply: bool,
    },
    /// Queue file removal
    Remove {
        file: String,
        #[arg(long, default_value = "orchestrator")]
        authorized_by: String,
        #[arg(long = "quorum-approver")]
        quorum_approvers: Vec<String>,
        #[arg(long = "quorum-evidence")]
        quorum_evidence: Vec<String>,
        #[arg(long)]
        quorum_asserted_at_utc: Option<String>,
    },
    /// Import ATHENA human scan records into HADES lifecycle review queue
    ImportHumanReviews {
        #[arg(long, default_value = "data/athena/human_ingestion_results.jsonl")]
        input: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Run read-only HADES lifecycle audit detectors over plans, human notes, and task queues
    LifecycleAudit {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Project HADES lifecycle findings into review queue (L2)
    LifecycleReviewQueue {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Generate policy-level HADES lifecycle automation report without authorizing cleanup (L2)
    LifecyclePolicyReport {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Measure Soterion/README/INDEX organization coverage without mutating files
    OrganizationAudit {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Generate dry-run HADES organization candidates without mutating source files
    OrganizationPlan {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long, default_value = "docs/operations")]
        scope: String,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Generate an operator approval packet for bounded HADES organization apply candidates
    OrganizationApprovalPacket {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long, default_value = "docs/operations")]
        scope: String,
        #[arg(long, default_value_t = 100)]
        limit: usize,
        #[arg(long, default_value = "operator")]
        operator_id: String,
        #[arg(long, default_value_t = false)]
        approved: bool,
        #[arg(long, default_value = "data/hades/organization_approval_packet.json")]
        out_path: String,
    },
    /// Execute a bounded approved HADES organization apply packet; dry-run unless --apply is set
    OrganizationApply {
        #[arg(long, default_value = "data/hades/organization_approval_packet.json")]
        approval_packet: String,
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long, default_value_t = false)]
        apply: bool,
    },
    /// Project WARDEN raw queue plus HADES policy report into an operator review packet (L3, no mutation)
    WardenHadesReviewPacket {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(
            long,
            default_value = "crates/annunimas-hades/data/warden/informant_queue.jsonl"
        )]
        raw_queue: String,
        #[arg(long, default_value_t = 100)]
        limit: usize,
        #[arg(
            long,
            default_value = "audit/WARDEN_QUEUE_TRIAGE_2026-05-19/operator_review_packet"
        )]
        out_dir: String,
    },
    /// Record a signed operator decision for selected WARDEN/HADES review IDs only (L3, no mutation)
    WardenHadesApprovalPacket {
        #[arg(
            long,
            default_value = "audit/WARDEN_QUEUE_TRIAGE_2026-05-19/operator_review_packet/warden_hades_operator_review_packet.json"
        )]
        review_packet: String,
        #[arg(long = "review-id", value_delimiter = ',')]
        review_ids: Vec<String>,
        #[arg(long, default_value = "operator")]
        operator_id: String,
        #[arg(long, default_value = "defer_retain_raw")]
        decision: String,
        #[arg(long, default_value = "manual_operator_gate")]
        evidence: String,
        #[arg(
            long,
            default_value = "audit/WARDEN_QUEUE_TRIAGE_2026-05-19/operator_review_packet/warden_hades_operator_signed_approval_packet.json"
        )]
        out_path: String,
    },
    /// Generate WARDEN/HADES dry-run receipt from a signed approval packet (L3, no mutation)
    WardenHadesDryRunReceipt {
        #[arg(
            long,
            default_value = "audit/WARDEN_QUEUE_TRIAGE_2026-05-19/operator_review_packet/warden_hades_operator_signed_approval_packet.json"
        )]
        approval_packet: String,
        #[arg(
            long,
            default_value = "audit/WARDEN_QUEUE_TRIAGE_2026-05-19/operator_review_packet/warden_hades_operator_review_packet.json"
        )]
        review_packet: String,
        #[arg(long, default_value = "retain_acknowledgement")]
        action: String,
        #[arg(
            long,
            default_value = "audit/WARDEN_QUEUE_TRIAGE_2026-05-19/operator_review_packet/warden_hades_dry_run_receipt.json"
        )]
        out_path: String,
    },
    /// Record a signed mutation-specific WARDEN/HADES approval packet for planning only (L3, no mutation)
    WardenHadesMutationApprovalPacket {
        #[arg(
            long,
            default_value = "audit/WARDEN_QUEUE_TRIAGE_2026-05-19/operator_review_packet/warden_hades_dry_run_receipt.json"
        )]
        dry_run_receipt: String,
        #[arg(long, default_value = "operator")]
        operator_id: String,
        #[arg(long, default_value = "archive_after_approval")]
        action: String,
        #[arg(long, default_value = "manual_operator_mutation_gate")]
        evidence: String,
        #[arg(
            long,
            default_value = "audit/WARDEN_QUEUE_TRIAGE_2026-05-19/operator_review_packet/warden_hades_operator_signed_mutation_approval_packet.json"
        )]
        out_path: String,
    },
    /// Generate WARDEN/HADES mutation plan receipt from a mutation-specific approval packet (L3, no mutation)
    WardenHadesMutationPlanReceipt {
        #[arg(
            long,
            default_value = "audit/WARDEN_QUEUE_TRIAGE_2026-05-19/operator_review_packet/warden_hades_operator_signed_mutation_approval_packet.json"
        )]
        mutation_approval_packet: String,
        #[arg(
            long,
            default_value = "audit/WARDEN_QUEUE_TRIAGE_2026-05-19/operator_review_packet/warden_hades_operator_review_packet.json"
        )]
        review_packet: String,
        #[arg(
            long,
            default_value = "audit/WARDEN_QUEUE_TRIAGE_2026-05-19/operator_review_packet/warden_hades_dry_run_receipt.json"
        )]
        dry_run_receipt: String,
        #[arg(long, default_value = "archive_after_approval")]
        action: String,
        #[arg(
            long,
            default_value = "audit/WARDEN_QUEUE_TRIAGE_2026-05-19/operator_review_packet/warden_hades_mutation_plan_receipt.json"
        )]
        out_path: String,
    },
    /// Record final-apply WARDEN/HADES approval packet after fresh hash and rollback checks (L3, no mutation)
    WardenHadesFinalApplyApprovalPacket {
        #[arg(
            long,
            default_value = "audit/WARDEN_QUEUE_TRIAGE_2026-05-19/operator_review_packet/warden_hades_mutation_plan_receipt.json"
        )]
        mutation_plan_receipt: String,
        #[arg(long, default_value = "operator")]
        operator_id: String,
        #[arg(long, default_value = "archive_after_approval")]
        action: String,
        #[arg(long)]
        rollback_plan: String,
        #[arg(long, default_value = "manual_operator_final_apply_gate")]
        evidence: String,
        #[arg(
            long,
            default_value = "audit/WARDEN_QUEUE_TRIAGE_2026-05-19/operator_review_packet/warden_hades_final_apply_approval_packet.json"
        )]
        out_path: String,
    },
    /// Execute approved WARDEN/HADES final apply archive gate with fresh hash checks (L4, archive only)
    WardenHadesFinalApplyExecution {
        #[arg(
            long,
            default_value = "audit/WARDEN_QUEUE_TRIAGE_2026-05-19/operator_review_packet/warden_hades_final_apply_approval_packet.json"
        )]
        final_apply_approval_packet: String,
        #[arg(long, default_value = "archive_after_approval")]
        action: String,
        #[arg(
            long,
            default_value = "audit/WARDEN_QUEUE_TRIAGE_2026-05-19/operator_review_packet/warden_hades_final_apply_archive.jsonl"
        )]
        archive_path: String,
        #[arg(
            long,
            default_value = "audit/WARDEN_QUEUE_TRIAGE_2026-05-19/operator_review_packet/warden_hades_final_apply_execution_receipt.json"
        )]
        receipt_path: String,
    },
    /// Generate operator approval packet for archive/quarantine/cleanup candidates (L3)
    LifecycleApprovalPacket {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long, default_value_t = 100)]
        limit: usize,
        #[arg(long, default_value = "data/hades/lifecycle_approval_packet.json")]
        out_path: String,
    },
    /// Execute gated lifecycle cleanup planner; dry-run by default and no destructive work without approval (L4)
    LifecycleCleanup {
        #[arg(long, default_value = "data/hades/lifecycle_approval_packet.json")]
        approval_packet: String,
        #[arg(
            long,
            default_value = "data/hades/lifecycle_cleanup_rollback_evidence.json"
        )]
        rollback_out: String,
        #[arg(long, default_value_t = false)]
        apply: bool,
    },
    /// Audit recommendation/task outcomes without mutating state
    AuditOutcomes {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long, default_value_t = 25)]
        limit: usize,
    },
    /// Show HADES storage paths
    Paths,
}

#[derive(Subcommand)]
enum HermesCommands {
    /// Start HERMES daemon (IPC + optional HTTP/SSE)
    Start {
        #[arg(long, default_value_t = default_runtime_socket("hermes.sock"))]
        socket_path: String,
        #[arg(long, default_value_t = default_loopback_addr(5117))]
        http_addr: String,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        http_enabled: bool,
    },
    /// Start the Serenity Discord slash-command bot for HERMES
    DiscordStart {
        #[arg(long)]
        token: Option<String>,
        /// Read the Discord bot token from this environment variable after loading config/.env.
        #[arg(long)]
        token_env: Option<String>,
        #[arg(long)]
        application_id: Option<u64>,
        #[arg(long)]
        guild_id: Option<u64>,
        #[arg(long)]
        channel_id: Option<u64>,
        #[arg(long)]
        ready_message: Option<String>,
    },
    /// Plan/read-only dry-run Discord channel administration
    DiscordChannels {
        #[command(subcommand)]
        command: DiscordChannelsCommands,
    },
    /// Show communications status
    Status,
    /// Show configured providers
    Providers,
    /// Show running subcomponents
    Subcomponents,
    /// Show recent boardroom posts
    Boardroom {
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Classify inbound message intent
    Classify {
        source: String,
        sender: String,
        content: String,
        #[arg(long)]
        channel: Option<String>,
        #[arg(long, default_value_t = false)]
        illuvatar: bool,
    },
    /// Queue outbound message
    Send {
        provider: String,
        channel: String,
        subject: String,
        body: String,
        #[arg(long, default_value_t = false)]
        stream: bool,
    },
    /// Retry recent outbound queue items with provider backoff policy
    RetryOutbound {
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Retry reroute dead-letter queue entries
    RetryRerouteDlq {
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Post boardroom entry
    BoardroomPost {
        from_agent: String,
        subject: String,
        body: String,
    },
    /// Trigger calendar sync
    CalendarSync,
    /// Ingest external webhook-style message
    IngestExternal {
        provider: String,
        sender: String,
        content: String,
        #[arg(long)]
        channel: Option<String>,
        #[arg(long, default_value_t = false)]
        illuvatar: bool,
    },
    /// Poll providers once and ingest inbound messages
    PollOnce,
    /// Capture an interruption while allowing in-flight async tasks to continue
    Interrupt {
        content: String,
        #[arg(long, default_value = "voice")]
        source: String,
        #[arg(long, default_value = "operator")]
        sender: String,
        #[arg(long)]
        channel: Option<String>,
        #[arg(long)]
        run_id: Option<String>,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long)]
        task_id: Option<String>,
    },
    /// Fan out a high-priority Illuvatar directive to all providers
    IlluvatarFanout {
        content: String,
        #[arg(long, default_value = "discord")]
        provider: String,
        #[arg(long, default_value = "illuvatar")]
        sender: String,
        #[arg(long)]
        channel: Option<String>,
    },
    /// Send 1-3 options prompt (A/B/C) and store mapping for auto-apply on reply
    DecisionPrompt {
        provider: String,
        channel: String,
        question: String,
        #[arg(long, default_value = "discord")]
        source: String,
        #[arg(long, default_value = "illuvatar")]
        sender: String,
        #[arg(long, default_value = "Option A")]
        a_label: String,
        #[arg(long)]
        a_action: String,
        #[arg(long, default_value = "Option B")]
        b_label: String,
        #[arg(long)]
        b_action: String,
        #[arg(long)]
        c_label: Option<String>,
        #[arg(long)]
        c_action: Option<String>,
    },
    /// Open a council thread in boardroom
    CouncilOpen {
        topic: String,
        #[arg(long, value_delimiter = ',')]
        participants: Vec<String>,
    },
    /// Add council report from an agent
    CouncilReport {
        session_id: String,
        from_agent: String,
        body: String,
    },
    /// Close a council thread
    CouncilClose {
        session_id: String,
        #[arg(long, default_value = "closed by operator")]
        outcome: String,
    },
    /// Project a boardroom quorum review packet without Discord dispatch
    BoardroomQuorum {
        session_id: String,
        topic: String,
        #[arg(long, value_delimiter = ',')]
        evidence_path: Vec<String>,
        #[arg(long)]
        oracle_query_id: Option<String>,
        #[arg(long)]
        oracle_verdict_path: Option<PathBuf>,
        #[arg(long)]
        charon_route_evidence: Option<String>,
        #[arg(long, default_value_t = 2)]
        quorum_threshold: usize,
        #[arg(long, value_delimiter = ',')]
        approval: Vec<String>,
        #[arg(long, default_value_t = false)]
        render: bool,
    },
    /// Dispatch a passed boardroom quorum packet after explicit operator approval
    BoardroomQuorumDispatch {
        packet_id: String,
        provider: String,
        channel: String,
        #[arg(long)]
        operator_approval_note: String,
    },
    /// Show HERMES storage paths
    Paths,
}

#[derive(Subcommand)]
enum DiscordChannelsCommands {
    /// Build a read-only plan for required Annunimas Discord channels
    Plan,
    /// Apply channel plan in dry-run mode unless explicit future mutating support is added
    Apply {
        #[arg(long, default_value_t = true, action = clap::ArgAction::SetTrue)]
        dry_run: bool,
        #[arg(long, default_value_t = false)]
        approve: bool,
    },
}

#[derive(Subcommand)]
enum ApolloCommands {
    Start {
        #[arg(long, default_value_t = default_runtime_socket("apollo.sock"))]
        socket_path: String,
        #[arg(long, default_value_t = default_loopback_addr(5118))]
        http_addr: String,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        http_enabled: bool,
    },
    Status,
    Submit {
        task_id: String,
        agent_id: String,
        #[arg(long, default_value = "{}")]
        payload: String,
        #[arg(long, default_value = "normal")]
        priority: String,
        #[arg(long, default_value_t = 60)]
        timeout_secs: u64,
    },
    Execute {
        task_id: String,
    },
    /// Execute a task only after explicit approval evidence is supplied
    ExecuteApproved {
        task_id: String,
        #[arg(long = "approval-evidence")]
        approval_evidence: String,
    },
    Interrupt {
        task_id: String,
        content: String,
        #[arg(long, default_value = "voice")]
        source: String,
        #[arg(long, default_value = "operator")]
        sender: String,
        #[arg(long, default_value = "note")]
        disposition: String,
        #[arg(long)]
        run_id: Option<String>,
        #[arg(long)]
        session_id: Option<String>,
    },
    Paths,
}

#[derive(Subcommand)]
enum PlutusCommands {
    Start {
        #[arg(long, default_value_t = default_runtime_socket("plutus.sock"))]
        socket_path: String,
        #[arg(long, default_value_t = default_loopback_addr(5119))]
        http_addr: String,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        http_enabled: bool,
    },
    Status,
    RegisterModel {
        provider: String,
        input_rate: f64,
        output_rate: f64,
        #[arg(long, default_value_t = 1000)]
        batch_size: usize,
    },
    RecordSpend {
        provider: String,
        input_tokens: usize,
        output_tokens: usize,
    },
    TrackWork {
        agent_id: String,
        amount: f64,
        #[arg(long, default_value = "reasoning")]
        unit: String,
        #[arg(long)]
        task_id: Option<String>,
    },
    Credit {
        account: String,
        amount: f64,
    },
    Relationship {
        from: String,
        to: String,
        trust: f64,
        attention: f64,
        reciprocity: f64,
    },
    Paths,
}

#[derive(Subcommand)]
enum OracleCommands {
    Start {
        #[arg(long, default_value_t = default_runtime_socket("oracle.sock"))]
        socket_path: String,
        #[arg(long, default_value_t = default_loopback_addr(5120))]
        http_addr: String,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        http_enabled: bool,
    },
    Status,
    Evaluate {
        task: String,
        #[arg(long)]
        id: Option<String>,
        #[arg(long, default_value = "operator")]
        requester: String,
        #[arg(long = "context")]
        context: Vec<String>,
    },
    Verdicts {
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    /// Emit governance autonomy-readiness JSON for every governance subsystem
    Readiness,
    /// Emit Triad Philosopher bootstrap profile status without enabling blocking autonomy
    PhilosopherProfiles {
        #[arg(long, default_value = "config/governance/philosophers.toml")]
        profiles_path: String,
        #[arg(long, value_enum, default_value_t = PhilosopherProfilesFormat::Json)]
        format: PhilosopherProfilesFormat,
    },
    Paths,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum PhilosopherProfilesFormat {
    /// Pretty JSON status projection for contract consumers
    Json,
    /// Human-readable compact operator summary
    Compact,
    /// Minimal JSON status surface for dashboard/status integration
    Status,
}

#[derive(Subcommand)]
enum PipelineCommands {
    /// Execute bounded project-task queue rules
    ProjectTaskExecutor,
    /// Project Flywheel work-packet readiness without mutating the task queue
    FlywheelPacketReadiness,
    /// Dispatch one ready Flywheel packet through Hermes Agent; dry-run unless --write is set
    FlywheelDispatch {
        #[arg(long)]
        task_id: Option<String>,
        #[arg(long, default_value_t = false)]
        write: bool,
    },
    /// Produce or write a Flywheel review receipt; dry-run unless --write is set
    FlywheelReviewReceipt {
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        dispatch_receipt: Option<String>,
        #[arg(long = "changed-file")]
        changed_files: Vec<String>,
        #[arg(long = "verification")]
        verification: Vec<String>,
        #[arg(long)]
        diff_review: String,
        #[arg(long, default_value = "review_required")]
        recommendation: String,
        #[arg(long)]
        notes: Option<String>,
        #[arg(long, default_value_t = false)]
        write: bool,
    },
    /// Emit async user intake tasks from HERMES inbound messages
    EmitAsyncUserIntakeTasks,
    /// Run ATHENA async user intake executor
    RunAsyncUserIntakeExecutor,
    /// Emit human corpus digest tasks from digest plan
    EmitHumanCorpusDigestTasks,
    /// Reconcile completed human corpus digest tasks
    ReconcileHumanCorpusDigestTasks,
    /// Emit source absorption tasks from promote-now candidates
    EmitSourceAbsorptionTasks,
    /// Run downstream source absorption executor
    RunSourceAbsorptionExecutor,
    /// Reconcile downstream source absorption completion evidence
    ReconcileSourceAbsorptionDownstream,
    /// Emit Platform OS migration/boundary review evidence without destructive extraction
    PlatformOsMigrationExecutor,
}

#[allow(clippy::large_enum_variant)]
#[derive(Subcommand)]
enum ControlCommands {
    /// Render runtime/env surfaces from operator profile
    SyncOperatorProfile {
        #[arg(default_value = "config/operator_profile.json")]
        profile_path: String,
        #[arg(long, default_value = "config/runtime.generated.env")]
        runtime_out: String,
        #[arg(long, default_value = "config/.env.generated")]
        env_out: String,
        #[arg(long, default_value_t = false)]
        apply_runtime: bool,
        #[arg(long, default_value_t = false)]
        apply_env: bool,
        #[arg(long, default_value_t = false)]
        force_env: bool,
        #[arg(long, default_value_t = false)]
        apply_control_policy: bool,
    },
    /// Evaluate sovereign launch preflight posture
    LaunchPreflight {
        #[arg(long, default_value = "data/prometheus/launch_preflight_last.json")]
        report_path: String,
        #[arg(long, default_value = "core/state/governance_runtime.json")]
        governance_path: String,
        #[arg(long, default_value = "core/state/runtime_budget_policy.json")]
        budget_path: String,
        #[arg(long, default_value = "core/state/runtime_admission_pressure.json")]
        pressure_path: String,
        #[arg(long, default_value = "core/state/runtime_topology.json")]
        topology_path: String,
        #[arg(long, default_value = "prometheus,charon,hermes")]
        degraded_agents: String,
        #[arg(long, default_value_t = true)]
        enforce_swap: bool,
        #[arg(long, default_value_t = false)]
        enforce_exit: bool,
    },
    /// Apply bounded OpenCode route governor recommendations
    ApplyOpencodeRouteGovernor {
        #[arg(long, default_value = "core/state/model_control_surface.json")]
        model_control_path: String,
        #[arg(long, default_value = "config/opencode_agent_routes.toml")]
        routes_path: String,
        #[arg(long, default_value = "core/state/opencode_route_governor.json")]
        state_path: String,
    },
    /// Apply bounded runtime recovery route shift into the route matrix
    ApplyRuntimeRecoveryRouteGovernor {
        #[arg(long, default_value = "core/state/runtime_admission_recovery.json")]
        recovery_path: String,
        #[arg(long, default_value = "core/state/charon_router.json")]
        charon_router_path: String,
        #[arg(long, default_value = "config/model_route_matrix.toml")]
        route_matrix_path: String,
        #[arg(
            long,
            default_value = "core/state/runtime_recovery_route_governor.json"
        )]
        state_path: String,
    },
    /// Run bounded runtime admission recovery executor
    RunRuntimeAdmissionRecoveryExecutor {
        #[arg(long, default_value = "core/state/runtime_admission_recovery.json")]
        recovery_path: String,
        #[arg(
            long,
            default_value = "core/state/runtime_admission_recovery_executor.json"
        )]
        out_path: String,
        #[arg(long)]
        timeout_seconds: Option<u64>,
    },
    /// Mirror output accounting candidates into non-destructive accounting storage
    SyncOutputAccounting {
        #[arg(long, default_value = "core/state/output_topology.json")]
        topology_path: String,
        #[arg(long, default_value = "core/state/output_accounting.json")]
        state_path: String,
        #[arg(long, default_value = "data/prometheus/output_accounting_runs.jsonl")]
        ledger_path: String,
        #[arg(long, default_value = "data/accounting/output_mirror")]
        mirror_root: String,
    },
    /// Compact runtime build cache under bounded age and pressure rules
    PruneRuntimeBuildCache {
        #[arg(long, default_value = "core/state/runtime_build_cache.json")]
        out_path: String,
    },
    /// Move HADES backup files into archive layout and emit layout state
    OrganizeHadesBackups {
        #[arg(long, default_value = "data/hades")]
        hades_root: String,
        #[arg(long, default_value = "core/state/hades_charon_layout.json")]
        out_path: String,
    },
    /// Reconcile duplicate or cleared escalation events
    ReconcileEscalations {
        #[arg(long, default_value = "data/prometheus/escalations.jsonl")]
        escalations_path: String,
        #[arg(long, default_value = "core/state/autonomy_runtime.json")]
        autonomy_runtime_path: String,
        #[arg(long, default_value = "core/state/runtime_admission_pressure.json")]
        pressure_report_path: String,
    },
    /// Record human or triad approval for a governed action class
    ApproveHumanAugmentation {
        decision_class: String,
        #[arg(long)]
        command_signature: Option<String>,
        #[arg(long = "approver")]
        approvers: Vec<String>,
        #[arg(long = "evidence")]
        evidence: Vec<String>,
        #[arg(long)]
        expires_at_utc: Option<String>,
        #[arg(long)]
        note: Option<String>,
        #[arg(long, default_value = "approved")]
        status: String,
        #[arg(long, default_value = "core/state/human_augmentation_approval.json")]
        approvals_path: String,
        #[arg(long, default_value = "core/state/human_augmentation_runtime.json")]
        runtime_out: String,
    },
    /// Record a CEO council session and refresh runtime state
    RecordCeoCouncilSession {
        objective: String,
        #[arg(long, default_value = "arandur")]
        ceo_identity: String,
        #[arg(long, default_value = "warden")]
        cto_identity: String,
        #[arg(long, default_value = "hybrid")]
        cto_mode: String,
        #[arg(long, default_value = "discord")]
        ingress: String,
        #[arg(long)]
        channel_ref: Option<String>,
        #[arg(long, default_value = "lightweight")]
        loop_class: String,
        #[arg(long, default_value = "routine_maintenance")]
        decision_class: String,
        #[arg(long, default_value_t = false)]
        triad_required: bool,
        #[arg(long = "participant")]
        participants: Vec<String>,
        #[arg(long = "proposal")]
        proposals: Vec<String>,
        #[arg(long = "objection")]
        objections: Vec<String>,
        #[arg(long)]
        synthesis: Option<String>,
        #[arg(long, default_value = "proposed")]
        outcome_status: String,
        #[arg(long, default_value_t = false)]
        human_escalated: bool,
        #[arg(long = "validator")]
        validators_invoked: Vec<String>,
        #[arg(long = "memory-lane")]
        memory_lanes: Vec<String>,
        #[arg(long = "memory-write")]
        memory_writes: Vec<String>,
        #[arg(long, default_value_t = false)]
        promoted_private_memory: bool,
        #[arg(long, default_value = "core/state/ceo_council_sessions.json")]
        sessions_path: String,
        #[arg(long, default_value = "core/state/ceo_council_runtime.json")]
        runtime_out: String,
    },
}

#[derive(Subcommand)]
enum UtilityCommands {
    /// Render operator runtime status
    OperatorRuntimeStatus,
    /// Summarize professionalization audit closeout ledgers without refreshing generated state
    ProfessionalizationAuditCloseout {
        #[arg(long, default_value = "audit/PROFESSIONALIZATION_AUDIT_2026-05-25")]
        audit_dir: String,
    },
    /// Create a sovereign crate spawn blueprint
    CreateCrateSpawnBlueprint {
        crate_name: String,
        #[arg(long, default_value = "operations")]
        realm: String,
        #[arg(long, default_value = "crates")]
        output_root: String,
        #[arg(long, default_value_t = false)]
        force: bool,
        #[arg(long, default_value_t = false)]
        productizable: bool,
    },
    /// Repair and compact HADES JSONL stores
    RepairHadesStores {
        #[arg(long, default_value_t = false)]
        apply: bool,
        #[arg(long, default_value = "data/hades/repair_report.json")]
        report_path: String,
    },
    /// Stamp Soterion sigils across repository text assets
    StampSoterionSigils {
        #[arg(long, default_value_t = false)]
        apply: bool,
    },
    /// Fetch markdown through the ATHENA Scrapling shim path
    ScraplingFetch {
        #[arg(long)]
        url: String,
        #[arg(long)]
        filter: String,
        #[arg(long)]
        query: Option<String>,
    },
    /// List resolved Hermes agent bridge targets
    HermesAgentEdgeBridgeListTargets {
        #[arg(long)]
        config: Option<String>,
        #[arg(long, default_value = "core/edge/targets.toml")]
        targets: String,
    },
    /// Check SSH reachability and Hermes presence for a bridge target
    HermesAgentEdgeBridgePreflight {
        #[arg(long)]
        node: String,
        #[arg(long)]
        config: Option<String>,
        #[arg(long, default_value = "core/edge/targets.toml")]
        targets: String,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    /// Probe a remote Hermes worker
    HermesAgentEdgeBridgeProbe {
        #[arg(long)]
        node: String,
        #[arg(long)]
        config: Option<String>,
        #[arg(long, default_value = "core/edge/targets.toml")]
        targets: String,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    /// Dispatch a prompt to a remote Hermes worker
    HermesAgentEdgeBridgeDispatch {
        #[arg(long)]
        node: String,
        #[arg(long)]
        prompt: String,
        #[arg(long)]
        config: Option<String>,
        #[arg(long, default_value = "core/edge/targets.toml")]
        targets: String,
        #[arg(long)]
        toolsets: Option<String>,
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        cwd: Option<String>,
        #[arg(long, default_value_t = false)]
        query_memory: bool,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    /// Adapt a Hermes Agent gateway/background result into a HERMES subagent receipt
    HermesAgentGatewayReceipt {
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        background_task_id: Option<String>,
        #[arg(long, default_value = "discord")]
        platform: String,
        #[arg(long, default_value = "work-stream")]
        channel: String,
        #[arg(long, default_value = "completed")]
        status: String,
        #[arg(long)]
        summary: String,
        #[arg(long = "verification")]
        verification: Vec<String>,
        #[arg(long = "changed-file")]
        changed_files: Vec<String>,
        #[arg(long = "blocker")]
        blockers: Vec<String>,
        #[arg(long)]
        next_action: Option<String>,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        dry_run: bool,
    },
    /// Read-only Phase 6 readiness audit for the Hermes Agent Discord gateway
    HermesAgentGatewayActivationCheck,
    /// Read-only remote confidence snapshot for ARDA/Hermes primary consoles and Discord confidence surface
    RemoteConfidence,
    /// Publish the remote confidence snapshot to the local ARDA runtime state file; no external send
    RemoteConfidencePublish,
    /// Classify safe-local work-cycle candidates and write only a local report
    SafeLocalWorkCyclePreflight,
    /// Refresh a bounded permission-profile scope window and write an audit receipt
    PermissionScopeRefresh {
        #[arg(long, default_value = "ceo_operator")]
        profile: String,
        #[arg(long, default_value = "network")]
        scope: String,
        #[arg(long, default_value_t = 2)]
        ttl_hours: u64,
        #[arg(long)]
        reason: String,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    /// Append a local-only agent conversation record for ARDA/council projection
    AgentConversationAppend {
        #[arg(long)]
        conversation_id: String,
        #[arg(long)]
        topic: String,
        #[arg(long)]
        speaker_agent: String,
        #[arg(long)]
        seat: String,
        #[arg(long, default_value = "observation")]
        message_class: String,
        #[arg(long, default_value = "informational")]
        actionability: String,
        #[arg(long, default_value = "read_only")]
        risk_lane: String,
        #[arg(long)]
        summary: String,
        #[arg(long)]
        related_plan: Option<String>,
        #[arg(long)]
        related_task: Option<String>,
        #[arg(long)]
        related_scout_request: Option<String>,
        #[arg(long)]
        confidence: Option<String>,
        #[arg(long = "source-link")]
        source_links: Vec<String>,
        #[arg(long = "receipt-link")]
        receipt_links: Vec<String>,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    /// Append a local-only Athena scout request record for ARDA projection
    ScoutRequestAppend {
        #[arg(long)]
        scout_request_id: String,
        #[arg(long)]
        requester_agent: String,
        #[arg(long)]
        question: String,
        #[arg(long, default_value = "implementation_notes")]
        desired_output_type: String,
        #[arg(long, default_value = "repo_allowed")]
        allowed_sources: String,
        #[arg(long, default_value = "read_only")]
        risk_lane: String,
        #[arg(long, default_value = "requested")]
        status: String,
        #[arg(long)]
        target_plan: String,
        #[arg(long)]
        target_task: Option<String>,
        #[arg(long)]
        expires_at_utc: Option<String>,
        #[arg(long)]
        staleness_policy: Option<String>,
        #[arg(long)]
        notes: Option<String>,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    /// Append a local-only Athena scout finding record and refresh scout runtime projection
    ScoutFindingAppend {
        #[arg(long)]
        scout_finding_id: String,
        #[arg(long)]
        scout_request_id: String,
        #[arg(long, default_value = "athena")]
        source_agent: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        summary: String,
        #[arg(long, default_value = "repo_allowed")]
        source_policy: String,
        #[arg(long, default_value = "found")]
        status: String,
        #[arg(long, default_value = "read_only")]
        risk_lane: String,
        #[arg(long)]
        confidence: Option<String>,
        #[arg(long = "source-link")]
        source_links: Vec<String>,
        #[arg(long = "follow-up-task")]
        recommended_follow_up_tasks: Vec<String>,
        #[arg(long = "receipt-link")]
        receipt_links: Vec<String>,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    /// Regenerate the local scout runtime summary from Athena scout ledgers
    ScoutRuntimeRefresh,
    /// Append local producer evidence for remote-confidence conversation/scout projection
    RemoteConfidenceProducerProof,
    /// Append a project task or pivot record
    TaskPivot {
        title: String,
        #[arg(long)]
        id: Option<String>,
        #[arg(long, default_value = "prometheus")]
        owner: String,
        #[arg(long, default_value = "high")]
        priority: String,
        #[arg(long, default_value = "queued")]
        status: String,
        #[arg(long)]
        result: Option<String>,
        #[arg(long)]
        notes: Option<String>,
        #[arg(long)]
        origin: Option<String>,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long = "meta")]
        meta: Vec<String>,
        #[arg(long = "glyph")]
        glyph: Vec<String>,
        #[arg(long)]
        sigil: Option<String>,
        #[arg(long)]
        queued_at_utc: Option<String>,
        #[arg(long)]
        completed_at_utc: Option<String>,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    load_env_files()?;
    set_runtime_defaults()?;
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let cli = Cli::parse();
    let config = load_config(&cli.config);
    enforce_policy_guard(&cli.command)?;
    execute_cli(cli, &config).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_operator_command_surface_for_key_subsystems() {
        let cli = Cli::try_parse_from(["annunimas", "status"]).expect("parse status");
        assert!(matches!(cli.command, Commands::Status));

        let cli = Cli::try_parse_from(["annunimas", "halt", "status"]).expect("parse halt status");
        assert!(matches!(
            cli.command,
            Commands::Halt {
                command: HaltCommands::Status { .. }
            }
        ));

        let cli =
            Cli::try_parse_from(["annunimas", "warden", "status"]).expect("parse warden status");
        assert!(matches!(
            cli.command,
            Commands::Warden {
                command: WardenCommands::Status { .. }
            }
        ));

        let cli = Cli::try_parse_from([
            "annunimas",
            "hermes",
            "send",
            "discord",
            "ops",
            "subject",
            "body",
        ])
        .expect("parse hermes send");
        assert!(matches!(
            cli.command,
            Commands::Hermes {
                command: HermesCommands::Send { .. }
            }
        ));

        let cli = Cli::try_parse_from([
            "annunimas",
            "hermes",
            "boardroom-quorum",
            "council_gate_36",
            "Gate 3.6 boardroom quorum",
            "--evidence-path",
            "data/hermes/council_sessions.jsonl,audit/oracle.jsonl",
            "--oracle-query-id",
            "oracle_gate_36",
            "--oracle-verdict-path",
            "audit/oracle.jsonl",
            "--charon-route-evidence",
            "edge_hub_3080:nous-hermes",
            "--quorum-threshold",
            "2",
            "--approval",
            "aurelius,bacon",
            "--render",
        ])
        .expect("parse hermes boardroom-quorum");
        assert!(matches!(
            cli.command,
            Commands::Hermes {
                command: HermesCommands::BoardroomQuorum { render: true, .. }
            }
        ));

        let cli = Cli::try_parse_from([
            "annunimas",
            "hermes",
            "boardroom-quorum-dispatch",
            "packet_gate_36",
            "discord",
            "ops-boardroom",
            "--operator-approval-note",
            "operator approved quorum packet dispatch",
        ])
        .expect("parse hermes boardroom-quorum-dispatch");
        assert!(matches!(
            cli.command,
            Commands::Hermes {
                command: HermesCommands::BoardroomQuorumDispatch { .. }
            }
        ));

        let cli = Cli::try_parse_from([
            "annunimas",
            "utility",
            "hermes-agent-gateway-receipt",
            "--task-id",
            "tsk_smoke",
            "--summary",
            "gateway smoke ok",
            "--dry-run=false",
        ])
        .expect("parse gateway receipt write mode");
        assert!(matches!(
            cli.command,
            Commands::Utility {
                command: UtilityCommands::HermesAgentGatewayReceipt { dry_run: false, .. }
            }
        ));

        let cli = Cli::try_parse_from([
            "annunimas",
            "charon",
            "route",
            "--agent-id",
            "athena",
            "--force-provider-id",
            "edge_hub_3080",
            "--exclude-provider-id",
            "openai",
            "code",
            "route this",
        ])
        .expect("parse charon route");
        assert!(matches!(
            cli.command,
            Commands::Charon {
                command: CharonCommands::Route { .. }
            }
        ));

        let cli = Cli::try_parse_from([
            "annunimas",
            "athena",
            "crawl",
            "https://example.com",
            "--filter",
            "fit",
        ])
        .expect("parse athena crawl");
        assert!(matches!(
            cli.command,
            Commands::Athena {
                command: AthenaCommands::Crawl { .. }
            }
        ));

        let cli = Cli::try_parse_from([
            "annunimas",
            "aipkg",
            "preflight",
            "spec/aipkg/v0.1/manifest.example.json",
            "--runtime-profile",
            "local-sovereign",
        ])
        .expect("parse aipkg preflight");
        assert!(matches!(
            cli.command,
            Commands::Aipkg {
                command: AipkgCommands::Preflight { .. }
            }
        ));

        let cli = Cli::try_parse_from([
            "annunimas",
            "prometheus",
            "ops-briefing",
            "--format",
            "text",
        ])
        .expect("parse prometheus ops-briefing text format");
        assert!(matches!(
            cli.command,
            Commands::Prometheus {
                command: PrometheusCommands::OpsBriefing {
                    format: OpsBriefingFormat::Text,
                    ..
                }
            }
        ));

        let cli = Cli::try_parse_from(["annunimas", "prometheus", "arandur", "observe"])
            .expect("parse arandur observe");
        assert!(matches!(
            cli.command,
            Commands::Prometheus {
                command: PrometheusCommands::Arandur {
                    command: ArandurCommands::Observe { .. }
                }
            }
        ));

        let cli = Cli::try_parse_from([
            "annunimas",
            "prometheus",
            "arandur",
            "review-packet",
            "--packet-dir",
            "audit/HUMAN_INBOX_PHASE2F_2026-05-17",
        ])
        .expect("parse arandur review-packet");
        assert!(matches!(
            cli.command,
            Commands::Prometheus {
                command: PrometheusCommands::Arandur {
                    command: ArandurCommands::ReviewPacket { .. }
                }
            }
        ));

        let cli = Cli::try_parse_from([
            "annunimas",
            "prometheus",
            "arandur",
            "recommend-next",
            "--dry-run",
        ])
        .expect("parse arandur recommend-next dry-run");
        assert!(matches!(
            cli.command,
            Commands::Prometheus {
                command: PrometheusCommands::Arandur {
                    command: ArandurCommands::RecommendNext { dry_run: true, .. }
                }
            }
        ));

        let cli = Cli::try_parse_from(["annunimas", "prometheus", "arandur", "readiness"])
            .expect("parse arandur readiness");
        assert!(matches!(
            cli.command,
            Commands::Prometheus {
                command: PrometheusCommands::Arandur {
                    command: ArandurCommands::Readiness { .. }
                }
            }
        ));

        let cli = Cli::try_parse_from(["annunimas", "oracle", "readiness"])
            .expect("parse oracle readiness");
        assert!(matches!(
            cli.command,
            Commands::Oracle {
                command: OracleCommands::Readiness
            }
        ));

        let cli = Cli::try_parse_from(["annunimas", "chronos", "status", "--format", "compact"])
            .expect("parse chronos status compact");
        assert!(matches!(
            cli.command,
            Commands::Chronos {
                command: ChronosCommands::Status { .. }
            }
        ));

        let cli = Cli::try_parse_from(["annunimas", "prometheus", "arandur", "mutation-classes"])
            .expect("parse arandur mutation-classes");
        assert!(matches!(
            cli.command,
            Commands::Prometheus {
                command: PrometheusCommands::Arandur {
                    command: ArandurCommands::MutationClasses { .. }
                }
            }
        ));

        let cli = Cli::try_parse_from(["annunimas", "prometheus", "arandur", "mission-backlog"])
            .expect("parse arandur mission-backlog");
        assert!(matches!(
            cli.command,
            Commands::Prometheus {
                command: PrometheusCommands::Arandur {
                    command: ArandurCommands::MissionBacklog { .. }
                }
            }
        ));

        let cli = Cli::try_parse_from([
            "annunimas",
            "prometheus",
            "arandur",
            "presence-event",
            "--event-id",
            "presence_test",
            "--mode",
            "advising",
            "--attention",
            "elevated",
            "--accent",
            "cyan",
            "--anchor-target",
            "boardroom.hologram_anchor",
        ])
        .expect("parse arandur presence-event");
        assert!(matches!(
            cli.command,
            Commands::Prometheus {
                command: PrometheusCommands::Arandur {
                    command: ArandurCommands::PresenceEvent { .. }
                }
            }
        ));

        let cli = Cli::try_parse_from(["annunimas", "onboarding", "detect", "--write"])
            .expect("parse onboarding detect");
        assert!(matches!(
            cli.command,
            Commands::Onboarding {
                command: OnboardingCommands::Detect { .. }
            }
        ));

        let cli = Cli::try_parse_from([
            "annunimas",
            "prometheus",
            "arandur",
            "verify-mutation",
            "--mutation-class",
            "recommendation_ledger_append",
            "--target-path",
            "data/arandur/recommendations.jsonl",
            "--pre-sha1",
            "abc123",
            "--pre-bytes",
            "10",
        ])
        .expect("parse arandur verify-mutation");
        assert!(matches!(
            cli.command,
            Commands::Prometheus {
                command: PrometheusCommands::Arandur {
                    command: ArandurCommands::VerifyMutation { .. }
                }
            }
        ));

        let cli = Cli::try_parse_from([
            "annunimas",
            "prometheus",
            "arandur",
            "rollback-report",
            "--mutation-class",
            "recommendation_ledger_append",
            "--target-path",
            "data/arandur/recommendations.jsonl",
            "--reason",
            "verification failed",
        ])
        .expect("parse arandur rollback-report");
        assert!(matches!(
            cli.command,
            Commands::Prometheus {
                command: PrometheusCommands::Arandur {
                    command: ArandurCommands::RollbackReport { .. }
                }
            }
        ));

        let cli = Cli::try_parse_from([
            "annunimas",
            "apollo",
            "submit",
            "task-1",
            "athena",
            "--payload",
            "{\"op\":\"ingest\"}",
        ])
        .expect("parse apollo submit");
        assert!(matches!(
            cli.command,
            Commands::Apollo {
                command: ApolloCommands::Submit { .. }
            }
        ));
    }

    #[test]
    fn parses_destructive_hades_remove_with_quorum_arguments() {
        let cli = Cli::try_parse_from([
            "annunimas",
            "hades",
            "remove",
            "data/tmp.log",
            "--quorum-approver",
            "aurelius",
            "--quorum-approver",
            "bacon",
            "--quorum-evidence",
            "ticket-123",
        ])
        .expect("parse hades remove");

        match cli.command {
            Commands::Hades {
                command:
                    HadesCommands::Remove {
                        file,
                        quorum_approvers,
                        quorum_evidence,
                        ..
                    },
            } => {
                assert_eq!(file, "data/tmp.log");
                assert_eq!(quorum_approvers, vec!["aurelius", "bacon"]);
                assert_eq!(quorum_evidence, vec!["ticket-123"]);
            }
            _ => panic!("unexpected command variant"),
        }
    }

    #[test]
    fn parses_warden_security_monitoring_enforcement_command() {
        let cli = Cli::try_parse_from([
            "annunimas",
            "warden",
            "enforce",
            "--state-root",
            "core/state",
            "--out",
            "core/state/warden_security_enforcement.json",
            "--findings",
            "data/warden/security_enforcement.jsonl",
        ])
        .expect("parse warden enforce");

        assert!(matches!(
            cli.command,
            Commands::Warden {
                command: WardenCommands::Enforce { apply: false, .. }
            }
        ));
    }

    #[test]
    fn parses_hades_lifecycle_l2_l3_l4_commands() {
        let l2 = Cli::try_parse_from([
            "annunimas",
            "hades",
            "lifecycle-review-queue",
            "--root",
            ".",
            "--limit",
            "5",
        ])
        .expect("parse lifecycle review queue");
        assert!(matches!(
            l2.command,
            Commands::Hades {
                command: HadesCommands::LifecycleReviewQueue { limit: 5, .. }
            }
        ));

        let l3 = Cli::try_parse_from([
            "annunimas",
            "hades",
            "lifecycle-approval-packet",
            "--root",
            ".",
            "--out-path",
            "data/hades/packet.json",
        ])
        .expect("parse lifecycle approval packet");
        assert!(matches!(
            l3.command,
            Commands::Hades {
                command: HadesCommands::LifecycleApprovalPacket { .. }
            }
        ));

        let l4 = Cli::try_parse_from([
            "annunimas",
            "hades",
            "lifecycle-cleanup",
            "--approval-packet",
            "data/hades/packet.json",
            "--rollback-out",
            "data/hades/rollback.json",
        ])
        .expect("parse lifecycle cleanup dry run");
        assert!(matches!(
            l4.command,
            Commands::Hades {
                command: HadesCommands::LifecycleCleanup { apply: false, .. }
            }
        ));
    }

    #[test]
    fn tools_output_lists_hardened_subsystems() {
        let config = Config::default();
        let output = format_tools_output(&config, &["athena", "hermes", "apollo"]);
        assert!(output.contains("Hardened Subsystems:"));
        assert!(output.contains("athena"));
        assert!(output.contains("warden"));
        assert!(output.contains("apollo"));
        assert!(output.contains("LLM Provider:"));
    }

    #[test]
    fn status_output_reports_runtime_paths_and_shared_state() {
        let config = Config::default();
        let output = format_status_output(&config, "config/default.toml", "ollama", "qwen");
        assert!(output.contains("Runtime Paths:"));
        assert!(output.contains("ANNUNIMAS_ATHENA_SOCKET = "));
        assert!(output.contains("athena.sock"));
        assert!(output.contains("ANNUNIMAS_APOLLO_SOCKET = "));
        assert!(output.contains("apollo.sock"));
        assert!(output.contains("Shared State:"));
        assert!(output.contains("core/state/world.json"));
        assert!(output.contains("Queues:"));
        assert!(output.contains("core/projects/tasks/queue.jsonl"));
        assert!(output.contains("core/queue/queue.jsonl"));
    }

    #[test]
    fn resolves_athena_source_id_from_raw_input_digest_match() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let temp_root = std::env::temp_dir().join(format!("annunimas-cli-athena-resolve-{unique}"));
        let digest_dir = temp_root.join("data/athena");
        std::fs::create_dir_all(&digest_dir).expect("mkdirs");
        std::fs::write(
            digest_dir.join("digest.jsonl"),
            concat!(
                "{\"id\":\"src_old\",\"raw_input\":\"/tmp/other.md\",\"url\":null,\"book_ref\":\"/tmp/old.jsonl\"}\n",
                "{\"id\":\"src_test\",\"raw_input\":\"/tmp/federated.md\",\"url\":null,\"book_ref\":\"/tmp/test.jsonl\"}\n"
            ),
        )
        .expect("write digest");

        let original = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&temp_root).expect("set cwd");
        let resolved = resolve_athena_source_id("/tmp/federated.md");
        std::env::set_current_dir(original).expect("restore cwd");
        let _ = std::fs::remove_dir_all(&temp_root);

        assert_eq!(resolved, "src_test");
    }

    #[test]
    fn aipkg_preflight_receipt_marks_runtime_mismatch() {
        let manifest = AipkgManifest {
            manifest_version: "0.1".into(),
            package_id: "org.annunimas.demo".into(),
            version: "0.1.0".into(),
            package_digest:
                "sha256:1111111111111111111111111111111111111111111111111111111111111111".into(),
            runtime_profile: "local-sovereign".into(),
            preflight: annunimas_core::AipkgPreflight {
                zero_work_required: true,
                compatibility_required: true,
                quote_required: true,
            },
            governance: annunimas_core::AipkgGovernance {
                triad_required: true,
                bacon_lite_required: true,
                joulework_budget_required: true,
                love_eq_guard_required: true,
                soterion_trace_required: true,
            },
            receipts: annunimas_core::AipkgReceiptPolicy {
                preflight_required: true,
                execution_required: true,
                validation_required: true,
                settlement_optional: true,
                signatures_required: true,
            },
        };
        let receipt = build_aipkg_preflight_receipt(
            "spec/aipkg/v0.1/manifest.example.json",
            &manifest,
            Some("wasm-wasi"),
        )
        .expect("receipt");
        assert_eq!(
            receipt.get("status").and_then(|v| v.as_str()).unwrap_or(""),
            "profile_mismatch"
        );
        assert_eq!(
            receipt["compatibility"]["compatible"].as_bool(),
            Some(false)
        );
    }
}
