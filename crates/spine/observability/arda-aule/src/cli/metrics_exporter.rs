// sigil: SCROLL
//
// Prometheus exposition exporter for the Arda fleet.
//
// Adapted from `Annunimas/crates/annunimas-cli/src/commands/metrics.rs` at
// commit c0b91edcdf777338b1eb42945b2adb4a3eaf6d7d. The Arda adaptation makes
// the runtime root explicit and attaches the exporter to `arda-cli`.
//
// MIT License
//
// Copyright (c) 2026 Daniel Ward
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
//
// Reads JSON snapshots emitted by other crates and re-emits them as
// `metric_name{label="x"} value` text on `/metrics`. Designed to run
// once per fleet node and be scraped by a single Prometheus server.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use prometheus::{Encoder, Gauge, IntGauge, IntGaugeVec, Opts, Registry, TextEncoder};
use serde_json::Value;
use sysinfo::System;
use tokio::time::interval;
use tracing::{info, warn};

use super::MetricsCommands;

#[derive(Clone)]
struct ExporterState {
    registry: Arc<Registry>,
    families: Arc<MetricFamilies>,
    root: PathBuf,
    system_metrics: bool,
}

struct MetricFamilies {
    autonomy_mode: IntGaugeVec,
    autonomy_violations: IntGauge,
    safety_score: Gauge,
    athena_age_min: Gauge,
    queue_depth: IntGaugeVec,
    backlog_total: IntGauge,
    active_internal_tasks: IntGauge,
    queue_latest_open_total: IntGauge,
    queue_raw_queued_rows_total: IntGauge,
    queue_stale_raw_queued_rows_total: IntGauge,
    pressure_guard_status: IntGaugeVec,
    pressure_guard_violations_total: IntGauge,
    pressure_guard_oversize_files_total: IntGauge,
    audit_health_status: IntGaugeVec,
    charon_failure_budget: IntGaugeVec,
    refresh_success: IntGauge,
    refresh_last_unix: IntGauge,
    node_time_seconds: Gauge,
    node_boot_time_seconds: Gauge,
    node_load1: Gauge,
    node_load5: Gauge,
    node_load15: Gauge,
    node_cpu_online: IntGauge,
    node_memory_memtotal_bytes: IntGauge,
    node_memory_memfree_bytes: IntGauge,
    node_memory_memavailable_bytes: IntGauge,
    node_memory_memused_bytes: IntGauge,
    node_swap_total_bytes: IntGauge,
    node_swap_free_bytes: IntGauge,
    node_swap_used_bytes: IntGauge,
}

impl MetricFamilies {
    fn new() -> Result<Self> {
        Ok(Self {
            autonomy_mode: IntGaugeVec::new(
                Opts::new(
                    "annunimas_autonomy_mode",
                    "Autonomy runtime mode flag (1=current mode)",
                ),
                &["mode"],
            )?,
            autonomy_violations: IntGauge::new(
                "annunimas_autonomy_violations",
                "Count of active autonomy guard violations",
            )?,
            safety_score: Gauge::new("annunimas_safety_score", "Composite safety score (0..1)")?,
            athena_age_min: Gauge::new(
                "annunimas_athena_lookup_age_minutes",
                "Minutes since most recent ATHENA digest entry",
            )?,
            queue_depth: IntGaugeVec::new(
                Opts::new("annunimas_queue_depth", "Pending records per work queue"),
                &["queue"],
            )?,
            backlog_total: IntGauge::new(
                "annunimas_backlog_total_records",
                "Total records in projects backlog",
            )?,
            active_internal_tasks: IntGauge::new(
                "annunimas_active_internal_tasks",
                "Total active internal tasks across all queues",
            )?,
            queue_latest_open_total: IntGauge::new(
                "annunimas_queue_latest_open_total",
                "Latest-by-id open project task total from queue hygiene",
            )?,
            queue_raw_queued_rows_total: IntGauge::new(
                "annunimas_queue_raw_queued_rows_total",
                "Raw queued rows in append-only project task ledger",
            )?,
            queue_stale_raw_queued_rows_total: IntGauge::new(
                "annunimas_queue_stale_raw_queued_rows_total",
                "Raw queued rows superseded by terminal same-id records",
            )?,
            pressure_guard_status: IntGaugeVec::new(
                Opts::new(
                    "annunimas_pressure_guard_status",
                    "Pressure guard status flag (1=current status)",
                ),
                &["status"],
            )?,
            pressure_guard_violations_total: IntGauge::new(
                "annunimas_pressure_guard_violations_total",
                "Pressure guard violation count from latest pressure report",
            )?,
            pressure_guard_oversize_files_total: IntGauge::new(
                "annunimas_pressure_guard_oversize_files_total",
                "Oversize file count from latest pressure guard filesystem scan",
            )?,
            audit_health_status: IntGaugeVec::new(
                Opts::new(
                    "annunimas_audit_health_status",
                    "Audit health status flag (1=current status)",
                ),
                &["surface", "status"],
            )?,
            charon_failure_budget: IntGaugeVec::new(
                Opts::new(
                    "annunimas_charon_failure_budget_remaining",
                    "Remaining failure budget per Charon provider",
                ),
                &["provider_id"],
            )?,
            refresh_success: IntGauge::new(
                "annunimas_metrics_exporter_refresh_success",
                "1 if last refresh cycle completed without error",
            )?,
            refresh_last_unix: IntGauge::new(
                "annunimas_metrics_exporter_refresh_unix_seconds",
                "Unix timestamp of last successful refresh",
            )?,
            node_time_seconds: Gauge::new(
                "node_time_seconds",
                "System time in seconds since epoch",
            )?,
            node_boot_time_seconds: Gauge::new(
                "node_boot_time_seconds",
                "Node boot time in seconds since epoch",
            )?,
            node_load1: Gauge::new("node_load1", "1m load average")?,
            node_load5: Gauge::new("node_load5", "5m load average")?,
            node_load15: Gauge::new("node_load15", "15m load average")?,
            node_cpu_online: IntGauge::new("node_cpu_online", "Number of online CPUs")?,
            node_memory_memtotal_bytes: IntGauge::new(
                "node_memory_MemTotal_bytes",
                "Total system memory in bytes",
            )?,
            node_memory_memfree_bytes: IntGauge::new(
                "node_memory_MemFree_bytes",
                "Free system memory in bytes",
            )?,
            node_memory_memavailable_bytes: IntGauge::new(
                "node_memory_MemAvailable_bytes",
                "Available system memory in bytes",
            )?,
            node_memory_memused_bytes: IntGauge::new(
                "node_memory_MemUsed_bytes",
                "Used system memory in bytes",
            )?,
            node_swap_total_bytes: IntGauge::new(
                "node_memory_SwapTotal_bytes",
                "Total swap in bytes",
            )?,
            node_swap_free_bytes: IntGauge::new(
                "node_memory_SwapFree_bytes",
                "Free swap in bytes",
            )?,
            node_swap_used_bytes: IntGauge::new(
                "node_memory_SwapUsed_bytes",
                "Used swap in bytes",
            )?,
        })
    }

    fn register(&self, r: &Registry, system_metrics: bool) -> Result<()> {
        r.register(Box::new(self.autonomy_mode.clone()))?;
        r.register(Box::new(self.autonomy_violations.clone()))?;
        r.register(Box::new(self.safety_score.clone()))?;
        r.register(Box::new(self.athena_age_min.clone()))?;
        r.register(Box::new(self.queue_depth.clone()))?;
        r.register(Box::new(self.backlog_total.clone()))?;
        r.register(Box::new(self.active_internal_tasks.clone()))?;
        r.register(Box::new(self.queue_latest_open_total.clone()))?;
        r.register(Box::new(self.queue_raw_queued_rows_total.clone()))?;
        r.register(Box::new(self.queue_stale_raw_queued_rows_total.clone()))?;
        r.register(Box::new(self.pressure_guard_status.clone()))?;
        r.register(Box::new(self.pressure_guard_violations_total.clone()))?;
        r.register(Box::new(self.pressure_guard_oversize_files_total.clone()))?;
        r.register(Box::new(self.audit_health_status.clone()))?;
        r.register(Box::new(self.charon_failure_budget.clone()))?;
        r.register(Box::new(self.refresh_success.clone()))?;
        r.register(Box::new(self.refresh_last_unix.clone()))?;
        if system_metrics {
            r.register(Box::new(self.node_time_seconds.clone()))?;
            r.register(Box::new(self.node_boot_time_seconds.clone()))?;
            r.register(Box::new(self.node_load1.clone()))?;
            r.register(Box::new(self.node_load5.clone()))?;
            r.register(Box::new(self.node_load15.clone()))?;
            r.register(Box::new(self.node_cpu_online.clone()))?;
            r.register(Box::new(self.node_memory_memtotal_bytes.clone()))?;
            r.register(Box::new(self.node_memory_memfree_bytes.clone()))?;
            r.register(Box::new(self.node_memory_memavailable_bytes.clone()))?;
            r.register(Box::new(self.node_memory_memused_bytes.clone()))?;
            r.register(Box::new(self.node_swap_total_bytes.clone()))?;
            r.register(Box::new(self.node_swap_free_bytes.clone()))?;
            r.register(Box::new(self.node_swap_used_bytes.clone()))?;
        }
        Ok(())
    }
}

fn read_json(path: &Path) -> Option<Value> {
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn first_i64(obj: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<i64> {
    for k in keys {
        if let Some(v) = obj.get(*k).and_then(|x| x.as_i64()) {
            return Some(v);
        }
    }
    None
}

fn refresh(state: &ExporterState) {
    let root = &state.root;
    let f = &state.families;

    // --- core/state/autonomy_runtime.json ---
    if let Some(v) = read_json(&root.join("core/state/autonomy_runtime.json")) {
        let mode = v.get("mode").and_then(|s| s.as_str()).unwrap_or("unknown");
        for candidate in ["normal", "degraded", "unknown"] {
            f.autonomy_mode
                .with_label_values(&[candidate])
                .set(if candidate == mode { 1 } else { 0 });
        }
        let n = v
            .get("violations")
            .and_then(|x| x.as_array())
            .map(|a| a.len() as i64)
            .unwrap_or(0);
        f.autonomy_violations.set(n);
    } else {
        warn!("autonomy_runtime.json missing or unreadable");
    }

    // --- core/state/queue_hygiene.json ---
    let queue_hygiene = root.join("core/state/queue_hygiene.json");
    if let Some(v) = read_json(&queue_hygiene) {
        if let Some(metrics) = v.get("metrics").and_then(|x| x.as_object()) {
            if let Some(n) = first_i64(metrics, &["latest_open_total"]) {
                f.queue_latest_open_total.set(n);
            }
            if let Some(n) = first_i64(metrics, &["raw_queued_rows_total"]) {
                f.queue_raw_queued_rows_total.set(n);
            }
            if let Some(n) = first_i64(metrics, &["stale_raw_queued_rows_total"]) {
                f.queue_stale_raw_queued_rows_total.set(n);
            }
        }
    }

    // --- core/state/runtime_admission_pressure.json ---
    let pressure = root.join("core/state/runtime_admission_pressure.json");
    if let Some(v) = read_json(&pressure) {
        let status = v
            .get("status")
            .and_then(|x| x.as_str())
            .unwrap_or("unknown");
        for candidate in ["ok", "alert", "error", "unknown"] {
            f.pressure_guard_status
                .with_label_values(&[candidate])
                .set(if candidate == status { 1 } else { 0 });
        }
        let violation_total = v
            .get("violations")
            .and_then(|x| x.as_array())
            .map(|items| items.len() as i64)
            .unwrap_or(0);
        f.pressure_guard_violations_total.set(violation_total);
        let oversize_total = v
            .pointer("/observed/storage_pressure/oversize_files_gte_100mb")
            .and_then(|x| x.as_i64())
            .unwrap_or(0);
        f.pressure_guard_oversize_files_total.set(oversize_total);
    }

    // --- audit health projection ---
    set_audit_health(
        f,
        "pressure_guard",
        read_json(&root.join("core/state/runtime_admission_pressure.json"))
            .as_ref()
            .and_then(|v| v.get("status"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown"),
    );
    set_audit_health(
        f,
        "repeated_audit",
        read_json(&root.join("core/state/repeated_audit_status.json"))
            .as_ref()
            .and_then(|v| {
                v.get("gate_status")
                    .or_else(|| v.get("status"))
                    .or_else(|| v.pointer("/summary/status"))
            })
            .and_then(|v| v.as_str())
            .unwrap_or("unknown"),
    );
    set_audit_health(
        f,
        "queue_hygiene",
        if read_json(&root.join("core/state/queue_hygiene.json")).is_some() {
            "ok"
        } else {
            "unknown"
        },
    );

    // --- core/metrics/by_crate/prometheus/queue_observability.json ---
    let qpath = root.join("core/metrics/by_crate/prometheus/queue_observability.json");
    if let Some(v) = read_json(&qpath) {
        if let Some(b) = v.get("breakdown").and_then(|x| x.as_object()) {
            for (queue, body) in b {
                let body_obj = match body.as_object() {
                    Some(o) => o,
                    None => continue,
                };
                let depth = first_i64(
                    body_obj,
                    &["pending_records", "queued", "pending_deep", "open"],
                )
                .unwrap_or(0);
                f.queue_depth
                    .with_label_values(&[queue.as_str()])
                    .set(depth);
            }
        }
        if let Some(s) = v.get("summary").and_then(|x| x.as_object()) {
            if let Some(n) = first_i64(s, &["backlog_total_records"]) {
                f.backlog_total.set(n);
            }
            if let Some(n) = first_i64(s, &["total_active_internal_tasks"]) {
                f.active_internal_tasks.set(n);
            }
        }
    }

    // --- core/metrics/by_crate/prometheus/ops_dashboard.json (charon section) ---
    let ops = root.join("core/metrics/by_crate/prometheus/ops_dashboard.json");
    if let Some(v) = read_json(&ops) {
        if let Some(arr) = v
            .pointer("/charon/provider_failure_budgets")
            .and_then(|x| x.as_array())
        {
            for entry in arr {
                let pid = entry
                    .get("provider_id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("unknown");
                let budget = entry
                    .get("failure_budget_remaining")
                    .or_else(|| entry.get("failures_remaining"))
                    .or_else(|| entry.get("failures"))
                    .and_then(|x| x.as_i64())
                    .unwrap_or(0);
                f.charon_failure_budget
                    .with_label_values(&[pid])
                    .set(budget);
            }
        }
    }

    // --- data/prometheus/autonomy_budget_last.json (signals block) ---
    let abud = root.join("data/prometheus/autonomy_budget_last.json");
    if let Some(v) = read_json(&abud) {
        if let Some(s) = v.get("signals") {
            if let Some(n) = s.get("athena_lookup_age_minutes").and_then(|x| x.as_f64()) {
                f.athena_age_min.set(n);
            }
            if let Some(n) = s.get("safety_score").and_then(|x| x.as_f64()) {
                f.safety_score.set(n);
            }
        }
    }

    if state.system_metrics {
        refresh_system_metrics(f);
    }

    f.refresh_success.set(1);
    f.refresh_last_unix.set(chrono::Utc::now().timestamp());
}

fn set_audit_health(f: &MetricFamilies, surface: &str, status: &str) {
    for candidate in ["ok", "pass", "warn", "alert", "error", "unknown"] {
        f.audit_health_status
            .with_label_values(&[surface, candidate])
            .set(if candidate == status { 1 } else { 0 });
    }
}

fn refresh_system_metrics(f: &MetricFamilies) {
    let mut system = System::new_all();
    system.refresh_all();

    let now = chrono::Utc::now().timestamp() as f64;
    let uptime = System::uptime() as f64;
    let load = System::load_average();

    f.node_time_seconds.set(now);
    f.node_boot_time_seconds.set((now - uptime).max(0.0));
    f.node_load1.set(load.one);
    f.node_load5.set(load.five);
    f.node_load15.set(load.fifteen);
    f.node_cpu_online.set(system.cpus().len() as i64);
    f.node_memory_memtotal_bytes
        .set(system.total_memory() as i64);
    f.node_memory_memfree_bytes.set(system.free_memory() as i64);
    f.node_memory_memavailable_bytes
        .set(system.available_memory() as i64);
    f.node_memory_memused_bytes.set(system.used_memory() as i64);
    f.node_swap_total_bytes.set(system.total_swap() as i64);
    f.node_swap_free_bytes.set(system.free_swap() as i64);
    f.node_swap_used_bytes.set(system.used_swap() as i64);
}

async fn metrics_handler(State(state): State<ExporterState>) -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let mut buf = Vec::new();
    if let Err(e) = encoder.encode(&state.registry.gather(), &mut buf) {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("encode: {e}")).into_response();
    }
    match String::from_utf8(buf) {
        Ok(body) => (
            [(axum::http::header::CONTENT_TYPE, encoder.format_type())],
            body,
        )
            .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("utf8: {e}")).into_response(),
    }
}

async fn health_handler() -> &'static str {
    "ok"
}

async fn audit_health_handler(State(state): State<ExporterState>) -> Json<Value> {
    let root = &state.root;
    let pressure = read_json(&root.join("core/state/runtime_admission_pressure.json"));
    let repeated = read_json(&root.join("core/state/repeated_audit_status.json"));
    let queue_hygiene = read_json(&root.join("core/state/queue_hygiene.json"));
    let pressure_status = pressure
        .as_ref()
        .and_then(|v| v.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let repeated_status = repeated
        .as_ref()
        .and_then(|v| {
            v.get("gate_status")
                .or_else(|| v.get("status"))
                .or_else(|| v.pointer("/summary/status"))
        })
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let queue_latest_open = queue_hygiene
        .as_ref()
        .and_then(|v| v.pointer("/metrics/latest_open_total"))
        .and_then(Value::as_u64);
    let status = if matches!(pressure_status, "alert" | "error")
        || matches!(repeated_status, "alert" | "error" | "fail")
    {
        "alert"
    } else if queue_hygiene.is_none() || pressure.is_none() {
        "unknown"
    } else {
        "ok"
    };
    Json(serde_json::json!({
        "status": status,
        "surfaces": {
            "pressure_guard": pressure_status,
            "repeated_audit": repeated_status,
            "queue_hygiene": if queue_hygiene.is_some() { "ok" } else { "unknown" }
        },
        "queue_latest_open_total": queue_latest_open,
        "paths": {
            "pressure_guard": "core/state/runtime_admission_pressure.json",
            "repeated_audit": "core/state/repeated_audit_status.json",
            "queue_hygiene": "core/state/queue_hygiene.json"
        }
    }))
}

fn build_state(root: PathBuf, system_metrics: bool) -> Result<ExporterState> {
    let registry = Registry::new();
    let families = MetricFamilies::new().context("build metric families")?;
    families
        .register(&registry, system_metrics)
        .context("register families")?;
    Ok(ExporterState {
        registry: Arc::new(registry),
        families: Arc::new(families),
        root,
        system_metrics,
    })
}

pub(crate) async fn handle(command: MetricsCommands) -> Result<()> {
    match command {
        MetricsCommands::Serve {
            root,
            bind,
            port,
            refresh_secs,
            system_metrics,
        } => {
            let state = build_state(root, system_metrics)?;
            refresh(&state);

            let bg = state.clone();
            tokio::spawn(async move {
                let mut tick = interval(Duration::from_secs(refresh_secs.max(1)));
                tick.tick().await;
                loop {
                    tick.tick().await;
                    refresh(&bg);
                }
            });

            let app = Router::new()
                .route("/metrics", get(metrics_handler))
                .route("/health", get(health_handler))
                .route("/health/audit", get(audit_health_handler))
                .with_state(state);

            let addr = format!("{bind}:{port}");
            info!(%addr, refresh_secs, "Arda metrics exporter listening");
            let listener = tokio::net::TcpListener::bind(&addr)
                .await
                .with_context(|| format!("bind {addr}"))?;
            axum::serve(listener, app).await.context("axum serve")?;
            Ok(())
        }
        MetricsCommands::Snapshot {
            root,
            system_metrics,
        } => {
            let state = build_state(root, system_metrics)?;
            refresh(&state);
            let encoder = TextEncoder::new();
            let mut buf = Vec::new();
            encoder.encode(&state.registry.gather(), &mut buf)?;
            print!("{}", String::from_utf8(buf)?);
            Ok(())
        }
    }
}
