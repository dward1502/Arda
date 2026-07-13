use crate::{
    summarize_state_feeds, AnomalyDetector, ChronosFeedSummary, MovingAveragePredictor,
    ResourcePrediction, SystemMetrics, TimeSeries, TimeSeriesPoint,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChronosCapabilities {
    pub scheduler: bool,
    pub predictive_maintenance: bool,
    pub audit_orchestration: bool,
    pub time_series_analysis: bool,
    pub live_data_feeds: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChronosAuditRunnerSurface {
    pub mode: String,
    pub runner_status: String,
    pub task_definitions_path: String,
    pub receipt_path: String,
    pub receipt_count: u64,
    pub latest_receipt_at_utc: Option<DateTime<Utc>>,
    pub configured_audit_classes: Vec<String>,
    pub scheduled_task_count: u64,
    pub ready_task_count: u64,
    pub ready_task_ids: Vec<String>,
    pub scheduled_tasks: Vec<ChronosScheduledAuditTaskProjection>,
    pub next_runner_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChronosScheduledAuditTaskProjection {
    pub id: String,
    pub name: String,
    pub audit_class: String,
    pub owner: String,
    pub cadence: String,
    pub scheduled_time_utc: DateTime<Utc>,
    pub due: bool,
    pub read_only: bool,
    pub source_surfaces: Vec<ChronosAuditSurfaceStatus>,
    pub write_through: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChronosAuditSurfaceStatus {
    pub path: String,
    pub present: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChronosScheduledAuditTask {
    pub id: String,
    pub name: String,
    pub audit_class: String,
    pub scheduled_time_utc: DateTime<Utc>,
    pub cadence: String,
    pub owner: String,
    pub source_surfaces: Vec<String>,
    pub write_through: Vec<String>,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChronosAuditRunSummary {
    pub receipt_path: String,
    pub due_task_count: u64,
    pub written_receipt_count: u64,
    pub skipped_task_count: u64,
    pub receipt_statuses: Vec<ChronosAuditReceiptStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChronosAuditReceiptStatus {
    pub task_id: String,
    pub audit_class: String,
    pub status: String,
    pub missing_source_surfaces: Vec<String>,
    pub runner_receipt_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ChronosAuditTaskReceipt {
    schema_version: String,
    authority: String,
    generated_at_utc: DateTime<Utc>,
    task_id: String,
    task_name: String,
    audit_class: String,
    owner: String,
    cadence: String,
    scheduled_time_utc: DateTime<Utc>,
    due: bool,
    read_only: bool,
    status: String,
    runner_status: String,
    runner_receipt_path: Option<String>,
    source_surfaces: Vec<ChronosAuditSurfaceStatus>,
    missing_source_surfaces: Vec<String>,
    write_through: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ChronosAuditRunnerReceipt {
    schema_version: String,
    authority: String,
    generated_at_utc: DateTime<Utc>,
    task_id: String,
    audit_class: String,
    status: String,
    read_only: bool,
    mutation_policy: String,
    source_surfaces: Vec<ChronosAuditSurfaceStatus>,
    findings: Vec<String>,
    metrics: serde_json::Value,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ChronosAuditReceiptSummary {
    count: u64,
    latest_generated_at_utc: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ChronosScheduledAuditTaskFile {
    tasks: Vec<ChronosScheduledAuditTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChronosRuntimeSnapshot {
    pub schema_version: String,
    pub generated_at_utc: DateTime<Utc>,
    pub authority: String,
    pub status: String,
    pub mode: String,
    pub capabilities: ChronosCapabilities,
    pub state_feeds: Vec<crate::FeedDomainSummary>,
    pub feed_summary: ChronosFeedSummary,
    pub prediction: ResourcePrediction,
    pub anomalies: Vec<String>,
    pub time_series_summary: Option<crate::TimeSeriesSummary>,
    pub audit_runner: ChronosAuditRunnerSurface,
    pub next_integration_steps: Vec<String>,
}

pub fn build_runtime_snapshot(root: &Path, now: DateTime<Utc>) -> ChronosRuntimeSnapshot {
    let metrics = SystemMetrics {
        timestamp: now,
        cpu_usage: 0.0,
        memory_usage: 0.0,
        disk_usage: 0.0,
        network_in: 0,
        network_out: 0,
    };

    let mut predictor = MovingAveragePredictor::new(4);
    predictor.add_metric(metrics.clone());
    let prediction = predictor.predict();
    let anomalies = AnomalyDetector::new().check_anomaly(&metrics);

    let mut series = TimeSeries::new();
    series.add_point(TimeSeriesPoint {
        timestamp: now,
        value: prediction.predicted_cpu,
    });
    let time_series_summary = series.summarize();
    let feed_summary = summarize_state_feeds(root, now);
    let state_feeds = feed_summary.domains.clone();
    let live_data_feeds = feed_summary.present_count > 0;

    ChronosRuntimeSnapshot {
        schema_version: "annunimas.chronos-runtime.v1".to_string(),
        generated_at_utc: now,
        authority: "annunimas-chronos".to_string(),
        status: "baseline_active".to_string(),
        mode: "oneshot_runtime_projection".to_string(),
        capabilities: ChronosCapabilities {
            scheduler: true,
            predictive_maintenance: true,
            audit_orchestration: true,
            time_series_analysis: true,
            live_data_feeds,
        },
        state_feeds,
        feed_summary,
        prediction,
        anomalies,
        time_series_summary,
        audit_runner: scheduled_audit_runner_surface(root, now),
        next_integration_steps: chronos_next_integration_steps(root),
    }
}

pub fn execute_scheduled_audit_tasks(
    root: &Path,
    now: DateTime<Utc>,
) -> anyhow::Result<ChronosAuditRunSummary> {
    let task_definitions_path = "config/chronos_audit_tasks.json";
    let receipt_path = "data/chronos/audit_receipts.jsonl";
    let tasks = load_scheduled_audit_tasks(root, task_definitions_path);
    let due_tasks: Vec<_> = tasks
        .into_iter()
        .filter(|task| task.scheduled_time_utc <= now)
        .collect();
    let mut receipt_statuses = Vec::new();
    let mut receipts = Vec::new();

    for task in due_tasks {
        let source_surfaces: Vec<_> = task
            .source_surfaces
            .iter()
            .map(|surface| ChronosAuditSurfaceStatus {
                path: surface.clone(),
                present: root.join(surface).exists(),
            })
            .collect();
        let missing_source_surfaces: Vec<_> = source_surfaces
            .iter()
            .filter(|surface| !surface.present)
            .map(|surface| surface.path.clone())
            .collect();
        let status = if task.read_only {
            if missing_source_surfaces.is_empty() {
                "completed_read_only"
            } else {
                "completed_read_only_with_missing_sources"
            }
        } else {
            "skipped_mutating_task"
        }
        .to_string();

        let runner_receipt_path = if task.read_only {
            Some(write_bounded_audit_runner_receipt(
                root,
                now,
                &task,
                &source_surfaces,
                &missing_source_surfaces,
            )?)
        } else {
            None
        };

        receipt_statuses.push(ChronosAuditReceiptStatus {
            task_id: task.id.clone(),
            audit_class: task.audit_class.clone(),
            status: status.clone(),
            missing_source_surfaces: missing_source_surfaces.clone(),
            runner_receipt_path: runner_receipt_path.clone(),
        });

        if task.read_only {
            receipts.push(ChronosAuditTaskReceipt {
                schema_version: "annunimas.chronos-audit-receipt.v1".to_string(),
                authority: "annunimas-chronos".to_string(),
                generated_at_utc: now,
                task_id: task.id,
                task_name: task.name,
                audit_class: task.audit_class,
                owner: task.owner,
                cadence: task.cadence,
                scheduled_time_utc: task.scheduled_time_utc,
                due: true,
                read_only: task.read_only,
                status,
                runner_status: "bounded_read_only_runner_receipt_written".to_string(),
                runner_receipt_path,
                source_surfaces,
                missing_source_surfaces,
                write_through: task.write_through,
            });
        }
    }

    append_audit_receipts(root, receipt_path, &receipts)?;

    Ok(ChronosAuditRunSummary {
        receipt_path: receipt_path.to_string(),
        due_task_count: receipt_statuses.len() as u64,
        written_receipt_count: receipts.len() as u64,
        skipped_task_count: receipt_statuses.len().saturating_sub(receipts.len()) as u64,
        receipt_statuses,
    })
}

fn write_bounded_audit_runner_receipt(
    root: &Path,
    now: DateTime<Utc>,
    task: &ChronosScheduledAuditTask,
    source_surfaces: &[ChronosAuditSurfaceStatus],
    missing_source_surfaces: &[String],
) -> anyhow::Result<String> {
    let relative_path = format!(
        "audit/chronos-runs/{}/{}/runner_receipt.json",
        now.format("%Y-%m-%d"),
        task.id
    );
    let receipt = build_bounded_audit_runner_receipt(
        root,
        now,
        task,
        source_surfaces,
        missing_source_surfaces,
    );
    let path = root.join(&relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(&receipt)? + "\n")?;
    Ok(relative_path)
}

fn build_bounded_audit_runner_receipt(
    root: &Path,
    now: DateTime<Utc>,
    task: &ChronosScheduledAuditTask,
    source_surfaces: &[ChronosAuditSurfaceStatus],
    missing_source_surfaces: &[String],
) -> ChronosAuditRunnerReceipt {
    let (findings, metrics) = match task.audit_class.as_str() {
        "state_feed_freshness" => state_feed_freshness_metrics(root, now),
        "runtime_admission_pressure" => {
            source_surface_json_metrics(root, source_surfaces, "runtime_admission_sources_present")
        }
        "governance_receipt_continuity" => {
            governance_receipt_continuity_metrics(root, source_surfaces)
        }
        "fleet_informant_coverage" => fleet_informant_coverage_metrics(root, source_surfaces),
        "warden_repair_pressure_triage" => {
            warden_repair_pressure_triage_metrics(root, source_surfaces)
        }
        other => (
            vec![format!("{other}:unsupported_audit_class")],
            json!({"audit_class": other, "supported": false}),
        ),
    };
    let status = if missing_source_surfaces.is_empty() {
        "completed_read_only"
    } else {
        "completed_read_only_with_missing_sources"
    };

    ChronosAuditRunnerReceipt {
        schema_version: "annunimas.chronos-bounded-audit-runner.v1".to_string(),
        authority: "annunimas-chronos".to_string(),
        generated_at_utc: now,
        task_id: task.id.clone(),
        audit_class: task.audit_class.clone(),
        status: status.to_string(),
        read_only: true,
        mutation_policy: "audit_receipt_only_no_source_config_service_or_queue_mutation"
            .to_string(),
        source_surfaces: source_surfaces.to_vec(),
        findings,
        metrics,
    }
}

fn state_feed_freshness_metrics(
    root: &Path,
    now: DateTime<Utc>,
) -> (Vec<String>, serde_json::Value) {
    let summary = summarize_state_feeds(root, now);
    let mut findings = summary.anomalies.clone();
    if findings.is_empty() {
        findings.push("state_feed_freshness:all_configured_feeds_current".to_string());
    }
    (
        findings,
        json!({
            "feed_count": summary.feed_count,
            "present_count": summary.present_count,
            "missing_count": summary.missing_count,
            "invalid_json_count": summary.invalid_json_count,
            "stale_count": summary.stale_count,
            "max_age_seconds": summary.max_age_seconds,
        }),
    )
}

fn source_surface_json_metrics(
    root: &Path,
    source_surfaces: &[ChronosAuditSurfaceStatus],
    ok_finding: &str,
) -> (Vec<String>, serde_json::Value) {
    let mut valid_json_count = 0_usize;
    let mut invalid_json = Vec::new();
    for surface in source_surfaces.iter().filter(|surface| surface.present) {
        let Ok(raw) = fs::read_to_string(root.join(&surface.path)) else {
            invalid_json.push(surface.path.clone());
            continue;
        };
        if serde_json::from_str::<serde_json::Value>(&raw).is_ok() {
            valid_json_count += 1;
        } else {
            invalid_json.push(surface.path.clone());
        }
    }
    let mut findings = if invalid_json.is_empty() {
        vec![ok_finding.to_string()]
    } else {
        invalid_json
            .iter()
            .map(|path| format!("{path}:invalid_json"))
            .collect()
    };
    let missing_count = source_surfaces
        .iter()
        .filter(|surface| !surface.present)
        .count();
    if missing_count > 0 {
        findings.push(format!("missing_source_surface_count:{missing_count}"));
    }
    (
        findings,
        json!({
            "source_surface_count": source_surfaces.len(),
            "present_count": source_surfaces.iter().filter(|surface| surface.present).count(),
            "missing_count": missing_count,
            "valid_json_count": valid_json_count,
            "invalid_json_count": invalid_json.len(),
            "invalid_json": invalid_json,
        }),
    )
}

fn governance_receipt_continuity_metrics(
    root: &Path,
    source_surfaces: &[ChronosAuditSurfaceStatus],
) -> (Vec<String>, serde_json::Value) {
    let mut jsonl_counts = serde_json::Map::new();
    for surface in source_surfaces
        .iter()
        .filter(|surface| surface.present && surface.path.ends_with(".jsonl"))
    {
        let count = fs::read_to_string(root.join(&surface.path))
            .map(|raw| raw.lines().filter(|line| !line.trim().is_empty()).count())
            .unwrap_or_default();
        jsonl_counts.insert(surface.path.clone(), json!(count));
    }
    let (findings, metrics) =
        source_surface_json_metrics(root, source_surfaces, "governance_sources_present");
    (
        findings,
        json!({
            "surface_metrics": metrics,
            "jsonl_record_counts": jsonl_counts,
        }),
    )
}

fn fleet_informant_coverage_metrics(
    _root: &Path,
    source_surfaces: &[ChronosAuditSurfaceStatus],
) -> (Vec<String>, serde_json::Value) {
    let informant_surfaces: Vec<_> = source_surfaces
        .iter()
        .filter(|surface| surface.path.contains("/informants/"))
        .collect();
    let present_informants = informant_surfaces
        .iter()
        .filter(|surface| surface.present)
        .count();
    let missing_informants = informant_surfaces.len().saturating_sub(present_informants);
    let findings = if missing_informants == 0 {
        vec!["fleet_informant_coverage:all_configured_informants_present".to_string()]
    } else {
        vec![format!(
            "fleet_informant_coverage:missing_informants:{missing_informants}"
        )]
    };
    (
        findings,
        json!({
            "configured_informant_count": informant_surfaces.len(),
            "present_informant_count": present_informants,
            "missing_informant_count": missing_informants,
            "source_surface_count": source_surfaces.len(),
            "present_source_surface_count": source_surfaces.iter().filter(|surface| surface.present).count(),
        }),
    )
}

fn warden_repair_pressure_triage_metrics(
    root: &Path,
    source_surfaces: &[ChronosAuditSurfaceStatus],
) -> (Vec<String>, serde_json::Value) {
    let triage_surface = source_surfaces
        .iter()
        .find(|surface| surface.path.ends_with("repair_pressure_triage_last.json"));
    let triage = triage_surface
        .and_then(|surface| fs::read_to_string(root.join(&surface.path)).ok())
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok());
    let (surface_findings, surface_metrics) =
        warden_triage_source_surface_metrics(root, source_surfaces);
    let Some(triage) = triage else {
        let mut findings = surface_findings;
        findings
            .push("warden_repair_pressure_triage:missing_or_invalid_triage_receipt".to_string());
        return (
            findings,
            json!({
                "surface_metrics": surface_metrics,
                "triage_receipt_present": false,
            }),
        );
    };

    let status = triage
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let effective_attention = triage
        .pointer("/repair_pressure/effective_attention_required")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let repeated_repair_noise = triage
        .pointer("/repair_pressure/repeated_repair_noise")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let missing_informants = triage
        .pointer("/informant_coverage/unknown_nodes_missing_matching_informant_count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let matching_but_unknown = triage
        .pointer("/informant_coverage/unknown_nodes_with_matching_informant_count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let timer_count = triage
        .pointer("/timer_posture/timer_count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let healthy_timer_count = triage
        .pointer("/timer_posture/healthy_timer_count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);

    let mut findings = Vec::new();
    if status == "healthy" {
        findings.push("warden_repair_pressure_triage:healthy".to_string());
    } else {
        findings.push(format!("warden_repair_pressure_triage:status:{status}"));
    }
    if effective_attention > 0 {
        findings.push(format!(
            "repair_pressure:effective_attention:{effective_attention}"
        ));
    }
    if repeated_repair_noise > 0 {
        findings.push(format!(
            "repair_pressure:repeated_noise:{repeated_repair_noise}"
        ));
    }
    if missing_informants > 0 {
        findings.push(format!("informants:missing_matching:{missing_informants}"));
    }
    if matching_but_unknown > 0 {
        findings.push(format!(
            "informants:matching_file_present_but_projection_unknown:{matching_but_unknown}"
        ));
    }
    if timer_count != healthy_timer_count {
        findings.push(format!(
            "timers:healthy:{healthy_timer_count}/{timer_count}"
        ));
    }

    (
        findings,
        json!({
            "surface_metrics": surface_metrics,
            "triage_receipt_present": true,
            "triage_status": status,
            "effective_attention_required": effective_attention,
            "repeated_repair_noise": repeated_repair_noise,
            "missing_matching_informants": missing_informants,
            "matching_informant_but_projection_unknown": matching_but_unknown,
            "timer_count": timer_count,
            "healthy_timer_count": healthy_timer_count,
        }),
    )
}

fn warden_triage_source_surface_metrics(
    root: &Path,
    source_surfaces: &[ChronosAuditSurfaceStatus],
) -> (Vec<String>, serde_json::Value) {
    let mut valid_json_count = 0_usize;
    let mut valid_jsonl_count = 0_usize;
    let mut invalid_surfaces = Vec::new();

    for surface in source_surfaces.iter().filter(|surface| surface.present) {
        let Ok(raw) = fs::read_to_string(root.join(&surface.path)) else {
            invalid_surfaces.push(surface.path.clone());
            continue;
        };
        if surface.path.ends_with(".jsonl") {
            let valid = raw
                .lines()
                .filter(|line| !line.trim().is_empty())
                .all(|line| serde_json::from_str::<serde_json::Value>(line).is_ok());
            if valid {
                valid_jsonl_count += 1;
            } else {
                invalid_surfaces.push(surface.path.clone());
            }
        } else if serde_json::from_str::<serde_json::Value>(&raw).is_ok() {
            valid_json_count += 1;
        } else {
            invalid_surfaces.push(surface.path.clone());
        }
    }

    let missing_count = source_surfaces
        .iter()
        .filter(|surface| !surface.present)
        .count();
    let mut findings = if invalid_surfaces.is_empty() {
        vec!["warden_triage_sources_present".to_string()]
    } else {
        invalid_surfaces
            .iter()
            .map(|path| format!("{path}:invalid_json_or_jsonl"))
            .collect()
    };
    if missing_count > 0 {
        findings.push(format!("missing_source_surface_count:{missing_count}"));
    }

    (
        findings,
        json!({
            "source_surface_count": source_surfaces.len(),
            "present_count": source_surfaces.iter().filter(|surface| surface.present).count(),
            "missing_count": missing_count,
            "valid_json_count": valid_json_count,
            "valid_jsonl_count": valid_jsonl_count,
            "invalid_surface_count": invalid_surfaces.len(),
            "invalid_surfaces": invalid_surfaces,
        }),
    )
}

fn scheduled_audit_runner_surface(root: &Path, now: DateTime<Utc>) -> ChronosAuditRunnerSurface {
    let task_definitions_path = "config/chronos_audit_tasks.json";
    let receipt_path = "data/chronos/audit_receipts.jsonl";
    let tasks = load_scheduled_audit_tasks(root, task_definitions_path);
    let receipt_summary = audit_receipt_summary(root, receipt_path);
    let mut configured_classes = BTreeSet::new();
    let mut ready_task_ids = Vec::new();
    let mut scheduled_tasks = Vec::new();

    for task in &tasks {
        configured_classes.insert(task.audit_class.clone());
        let due = task.scheduled_time_utc <= now;
        if due {
            ready_task_ids.push(task.id.clone());
        }
        scheduled_tasks.push(ChronosScheduledAuditTaskProjection {
            id: task.id.clone(),
            name: task.name.clone(),
            audit_class: task.audit_class.clone(),
            owner: task.owner.clone(),
            cadence: task.cadence.clone(),
            scheduled_time_utc: task.scheduled_time_utc,
            due,
            read_only: task.read_only,
            source_surfaces: task
                .source_surfaces
                .iter()
                .map(|surface| ChronosAuditSurfaceStatus {
                    path: surface.clone(),
                    present: root.join(surface).exists(),
                })
                .collect(),
            write_through: task.write_through.clone(),
        });
    }

    let runner_status = if tasks.is_empty() {
        "no_scheduled_audit_tasks_configured"
    } else if ready_task_ids.is_empty() {
        "scheduled_audit_tasks_loaded_waiting"
    } else {
        "scheduled_audit_tasks_ready"
    };

    ChronosAuditRunnerSurface {
        mode: "scheduled_task_projection".to_string(),
        runner_status: runner_status.to_string(),
        task_definitions_path: task_definitions_path.to_string(),
        receipt_path: receipt_path.to_string(),
        receipt_count: receipt_summary.count,
        latest_receipt_at_utc: receipt_summary.latest_generated_at_utc,
        configured_audit_classes: configured_classes.into_iter().collect(),
        scheduled_task_count: tasks.len() as u64,
        ready_task_count: ready_task_ids.len() as u64,
        ready_task_ids,
        scheduled_tasks,
        next_runner_steps: vec![
            "execute read-only collectors before enabling mutating remediation".to_string(),
            "write append-only audit receipts with source evidence".to_string(),
            "promote Chronos from projection to bounded runner after receipt contract validation"
                .to_string(),
        ],
    }
}

fn load_scheduled_audit_tasks(root: &Path, relative_path: &str) -> Vec<ChronosScheduledAuditTask> {
    let path = root.join(relative_path);
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str::<ChronosScheduledAuditTaskFile>(&raw)
        .map(|file| file.tasks)
        .unwrap_or_default()
}

fn append_audit_receipts(
    root: &Path,
    relative_path: &str,
    receipts: &[ChronosAuditTaskReceipt],
) -> anyhow::Result<()> {
    if receipts.is_empty() {
        return Ok(());
    }
    let path = root.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    for receipt in receipts {
        let line = serde_json::to_string(receipt)?;
        writeln!(file, "{line}")?;
    }
    Ok(())
}

fn audit_receipt_summary(root: &Path, relative_path: &str) -> ChronosAuditReceiptSummary {
    let path = root.join(relative_path);
    let Ok(raw) = fs::read_to_string(path) else {
        return ChronosAuditReceiptSummary::default();
    };
    let mut count = 0_u64;
    let mut latest_generated_at_utc = None;
    for line in raw.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        count += 1;
        let Some(generated_at) = value
            .get("generated_at_utc")
            .and_then(|value| value.as_str())
        else {
            continue;
        };
        let Ok(parsed) = generated_at.parse::<DateTime<Utc>>() else {
            continue;
        };
        if latest_generated_at_utc
            .map(|latest| parsed > latest)
            .unwrap_or(true)
        {
            latest_generated_at_utc = Some(parsed);
        }
    }
    ChronosAuditReceiptSummary {
        count,
        latest_generated_at_utc,
    }
}

fn chronos_next_integration_steps(root: &Path) -> Vec<String> {
    if load_scheduled_audit_tasks(root, "config/chronos_audit_tasks.json").is_empty() {
        return vec![
            "define operator-approved scheduled audit tasks".to_string(),
            "promote CLI status readers beyond snapshot-only projection".to_string(),
            "persist temporal deltas for trend-aware admission pressure".to_string(),
        ];
    }

    vec![
        "execute scheduled read-only audit tasks and write receipts".to_string(),
        "promote CLI status readers beyond snapshot-only projection".to_string(),
        "persist temporal deltas for trend-aware admission pressure".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use std::fs;

    #[test]
    fn runtime_snapshot_surfaces_typed_feeds_audit_runner_and_next_steps() {
        let root = std::env::temp_dir().join(format!(
            "annunimas_chronos_runtime_snapshot_test_{}",
            Utc::now().timestamp_nanos_opt().expect("timestamp nanos")
        ));
        let state_dir = root.join("core/state");
        fs::create_dir_all(&state_dir).expect("state dir");
        fs::write(
            state_dir.join("warden_guardhouse.json"),
            r#"{"authority":"warden_system_projection","generated_at_utc":"2026-05-24T10:00:00Z","duties":["informant_network"],"health":{"fleet_control":{"fleet_nodes":[{"hostname":"core"}],"connection_cleanup":{"stale_offline_total":0}}}}"#,
        )
        .expect("warden fixture");
        let config_dir = root.join("config");
        let informant_dir = root.join("data/fleet/informants");
        fs::create_dir_all(&config_dir).expect("config dir");
        fs::create_dir_all(&informant_dir).expect("informant dir");
        fs::write(
            informant_dir.join("beelink_last.json"),
            r#"{"node":"beelink"}"#,
        )
        .expect("informant fixture");
        fs::write(
            config_dir.join("chronos_audit_tasks.json"),
            r#"{"tasks":[{"id":"fleet_informant_coverage_due","name":"Fleet informant coverage audit","audit_class":"fleet_informant_coverage","scheduled_time_utc":"2026-05-24T10:00:00Z","cadence":"hourly","owner":"warden","source_surfaces":["data/fleet/informants/beelink_last.json","data/fleet/informants/raspberrypi-aihat_last.json"],"write_through":["audit/chronos/fleet_informant_coverage.json"],"read_only":true}]}"#,
        )
        .expect("audit task fixture");

        let now = Utc
            .with_ymd_and_hms(2026, 5, 24, 10, 30, 0)
            .single()
            .expect("snapshot time");
        let snapshot = build_runtime_snapshot(&root, now);

        assert_eq!(snapshot.schema_version, "annunimas.chronos-runtime.v1");
        assert_eq!(snapshot.feed_summary.feed_count, 4);
        assert_eq!(snapshot.state_feeds.len(), 4);
        assert_eq!(
            snapshot
                .feed_summary
                .typed_feeds
                .warden
                .model
                .as_ref()
                .map(|model| model.fleet_node_count),
            Some(Some(1))
        );
        assert_eq!(snapshot.audit_runner.mode, "scheduled_task_projection");
        assert_eq!(
            snapshot.audit_runner.runner_status,
            "scheduled_audit_tasks_ready"
        );
        assert_eq!(
            snapshot.audit_runner.task_definitions_path,
            "config/chronos_audit_tasks.json"
        );
        assert_eq!(snapshot.audit_runner.scheduled_task_count, 1);
        assert_eq!(
            snapshot.audit_runner.ready_task_ids,
            vec!["fleet_informant_coverage_due"]
        );
        assert!(snapshot.audit_runner.scheduled_tasks[0]
            .source_surfaces
            .iter()
            .any(
                |surface| surface.path == "data/fleet/informants/beelink_last.json"
                    && surface.present
            ));
        assert!(snapshot.audit_runner.scheduled_tasks[0]
            .source_surfaces
            .iter()
            .any(
                |surface| surface.path == "data/fleet/informants/raspberrypi-aihat_last.json"
                    && !surface.present
            ));
        assert!(snapshot
            .next_integration_steps
            .iter()
            .any(|step| step.contains("CLI status")));

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn scheduled_audit_execution_writes_bounded_runner_receipts() {
        let root = std::env::temp_dir().join(format!(
            "annunimas_chronos_bounded_runner_test_{}",
            Utc::now().timestamp_nanos_opt().expect("timestamp nanos")
        ));
        let state_dir = root.join("core/state");
        let config_dir = root.join("config");
        fs::create_dir_all(&state_dir).expect("state dir");
        fs::create_dir_all(&config_dir).expect("config dir");
        fs::write(
            state_dir.join("warden_guardhouse.json"),
            r#"{"authority":"warden_system_projection","generated_at_utc":"2026-05-24T10:00:00Z","duties":["drift_watch"]}"#,
        )
        .expect("warden fixture");
        fs::write(
            config_dir.join("chronos_audit_tasks.json"),
            r#"{"tasks":[{"id":"state_feed_due","name":"State feed due","audit_class":"state_feed_freshness","scheduled_time_utc":"2026-05-24T10:00:00Z","cadence":"hourly","owner":"chronos","source_surfaces":["core/state/warden_guardhouse.json"],"write_through":["audit/chronos-runs/","data/chronos/audit_receipts.jsonl"],"read_only":true}]}"#,
        )
        .expect("task fixture");

        let now = Utc
            .with_ymd_and_hms(2026, 5, 24, 10, 30, 0)
            .single()
            .expect("snapshot time");
        let summary = execute_scheduled_audit_tasks(&root, now).expect("execute scheduled audits");

        assert_eq!(summary.due_task_count, 1);
        assert_eq!(summary.written_receipt_count, 1);
        let runner_path = summary.receipt_statuses[0]
            .runner_receipt_path
            .as_deref()
            .expect("runner receipt path");
        assert!(runner_path.starts_with("audit/chronos-runs/2026-05-24/state_feed_due/"));
        let runner_receipt: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(root.join(runner_path)).expect("read runner receipt"),
        )
        .expect("parse runner receipt");
        assert_eq!(
            runner_receipt
                .get("schema_version")
                .and_then(serde_json::Value::as_str),
            Some("annunimas.chronos-bounded-audit-runner.v1")
        );
        assert_eq!(
            runner_receipt
                .get("mutation_policy")
                .and_then(serde_json::Value::as_str),
            Some("audit_receipt_only_no_source_config_service_or_queue_mutation")
        );
        assert!(root.join("data/chronos/audit_receipts.jsonl").exists());

        fs::remove_dir_all(root).expect("cleanup");
    }
}
