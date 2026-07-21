#![cfg(feature = "full-cli")]
use super::super::*;
use anyhow::{bail, Context};
use fs2::FileExt;
use sha1::{Digest, Sha1};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};

const MANWE_STATE_PATH: &str = "data/manwe/state.jsonl";
const MANWE_GOVERNANCE_EVENTS_PATH: &str = "data/manwe/governance_events.jsonl";
const MANWE_TELEMETRY_SUMMARIES_PATH: &str = "data/manwe/telemetry_summaries.jsonl";

pub(crate) fn handle(command: PrometheusdManweCommands) -> anyhow::Result<()> {
    match command {
        PrometheusdManweCommands::TelemetryReport {
            root,
            since,
            write,
            justification,
            limit,
        } => {
            let root = resolve_root(root);
            let value = build_manwe_telemetry_report(
                &root,
                Some(since.as_str()),
                write,
                justification.as_deref(),
                limit,
            )?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
    }
    Ok(())
}

fn resolve_root(root: Option<String>) -> PathBuf {
    root.map(PathBuf::from)
        .or_else(|| std::env::var("ARDA_ROOT").ok().map(PathBuf::from))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn build_manwe_telemetry_report(
    root: &Path,
    since: Option<&str>,
    write: bool,
    justification: Option<&str>,
    limit: usize,
) -> anyhow::Result<serde_json::Value> {
    if write && justification.unwrap_or("").trim().is_empty() {
        bail!(
            "--write requires non-empty --justification for review-gated telemetry summary append"
        );
    }

    let state_path = root.join(MANWE_STATE_PATH);
    let governance_path = root.join(MANWE_GOVERNANCE_EVENTS_PATH);
    let summary_path = root.join(MANWE_TELEMETRY_SUMMARIES_PATH);
    let cutoff = parse_since_cutoff(since)?;

    let mut summary = TelemetrySummary {
        state_path: state_path.display().to_string(),
        governance_events_path: governance_path.display().to_string(),
        summary_path: summary_path.display().to_string(),
        since: since.unwrap_or("all").to_string(),
        ..Default::default()
    };

    ingest_jsonl(&state_path, cutoff, limit, &mut summary, "state")?;
    ingest_jsonl(&governance_path, cutoff, limit, &mut summary, "governance")?;

    let idempotency_key = telemetry_idempotency_key(&summary);
    let record = json!({
        "contract": "arda.manwe.telemetry_summary.v1",
        "summary_id": idempotency_key,
        "authority": "agent_generated",
        "review_required": true,
        "status": if write { "candidate_append" } else { "dry_run" },
        "generated_at_utc": Utc::now().to_rfc3339(),
        "justification": justification,
        "period": {
            "since": summary.since,
            "first_event_ts": summary.first_ts,
            "last_event_ts": summary.last_ts,
        },
        "source_paths": {
            "state": summary.state_path,
            "governance_events": summary.governance_events_path,
            "summary_ledger": summary.summary_path,
        },
        "source_health": {
            "state_lines_read": summary.state_lines_read,
            "governance_lines_read": summary.governance_lines_read,
            "state_events_used": summary.state_events_used,
            "governance_events_used": summary.governance_events_used,
            "malformed_state_events": summary.malformed_state_events,
            "malformed_governance_events": summary.malformed_governance_events,
            "limit_per_file": limit,
        },
        "event_counts": summary.event_counts,
        "route_success_count": summary.route_success_count,
        "route_failure_count": summary.route_failure_count,
        "provider_selection_counts": top_map(&summary.provider_selection_counts, 24),
        "model_selection_counts": top_map(&summary.model_selection_counts, 24),
        "task_type_counts": top_map(&summary.task_type_counts, 24),
        "failure_reason_counts": top_map(&summary.failure_reason_counts, 24),
        "provider_failure_counts": top_map(&summary.provider_failure_counts, 24),
        "cooldown_counts": top_map(&summary.cooldown_counts, 24),
        "echo_gate_action_counts": top_map(&summary.echo_gate_action_counts, 12),
        "observed_providers": summary.observed_providers.iter().collect::<Vec<_>>(),
        "evidence_fingerprint_sha1": summary.evidence_fingerprint,
        "caveats": [
            "JSONL telemetry is append-only operational evidence, not human-reviewed performance scoring.",
            "Live Prometheus counters are not included in this offline summary; use /metrics for process-lifetime latency histograms.",
            "Malformed lines are counted and skipped to preserve read tolerance."
        ],
        "recommendations": build_recommendations(&summary),
    });

    if !write {
        return Ok(record);
    }

    append_summary_idempotent(&summary_path, &record, &idempotency_key)
}

#[derive(Default, Clone)]
struct TelemetrySummary {
    state_path: String,
    governance_events_path: String,
    summary_path: String,
    since: String,
    first_ts: Option<String>,
    last_ts: Option<String>,
    state_lines_read: usize,
    governance_lines_read: usize,
    state_events_used: usize,
    governance_events_used: usize,
    malformed_state_events: usize,
    malformed_governance_events: usize,
    route_success_count: u64,
    route_failure_count: u64,
    event_counts: BTreeMap<String, u64>,
    provider_selection_counts: BTreeMap<String, u64>,
    model_selection_counts: BTreeMap<String, u64>,
    task_type_counts: BTreeMap<String, u64>,
    failure_reason_counts: BTreeMap<String, u64>,
    provider_failure_counts: BTreeMap<String, u64>,
    cooldown_counts: BTreeMap<String, u64>,
    echo_gate_action_counts: BTreeMap<String, u64>,
    observed_providers: BTreeSet<String>,
    evidence_fingerprint: String,
}

fn ingest_jsonl(
    path: &Path,
    cutoff: Option<chrono::DateTime<Utc>>,
    limit: usize,
    summary: &mut TelemetrySummary,
    source: &str,
) -> anyhow::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut hasher = Sha1::new();
    let mut used = 0usize;
    for line in reader.lines() {
        let line = line?;
        if source == "state" {
            summary.state_lines_read += 1;
        } else {
            summary.governance_lines_read += 1;
        }
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => {
                if source == "state" {
                    summary.malformed_state_events += 1;
                } else {
                    summary.malformed_governance_events += 1;
                }
                continue;
            }
        };
        let ts = value.get("ts").and_then(|v| v.as_str()).map(str::to_string);
        if let (Some(cutoff), Some(ts)) = (cutoff, ts.as_deref()) {
            if parse_event_ts(ts)
                .map(|parsed| parsed < cutoff)
                .unwrap_or(false)
            {
                continue;
            }
        }
        if limit > 0 && used >= limit {
            break;
        }
        used += 1;
        hasher.update(line.as_bytes());
        hasher.update(b"\n");
        observe_event(summary, &value, source);
    }
    let digest = format!("{:x}", hasher.finalize());
    if summary.evidence_fingerprint.is_empty() {
        summary.evidence_fingerprint = digest;
    } else {
        let combined = format!("{}{}", summary.evidence_fingerprint, digest);
        summary.evidence_fingerprint = format!("{:x}", Sha1::digest(combined.as_bytes()));
    }
    Ok(())
}

fn observe_event(summary: &mut TelemetrySummary, value: &serde_json::Value, source: &str) {
    if source == "state" {
        summary.state_events_used += 1;
    } else {
        summary.governance_events_used += 1;
    }
    if let Some(ts) = value.get("ts").and_then(|v| v.as_str()) {
        let ts = ts.to_string();
        if summary.first_ts.as_ref().map(|t| ts < *t).unwrap_or(true) {
            summary.first_ts = Some(ts.clone());
        }
        if summary.last_ts.as_ref().map(|t| ts > *t).unwrap_or(true) {
            summary.last_ts = Some(ts);
        }
    }
    let event = value
        .get("event")
        .or_else(|| value.get("event_type"))
        .or_else(|| value.get("type"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    inc(&mut summary.event_counts, event);
    let payload = value.get("payload").unwrap_or(value);
    if let Some(task_type) = payload.get("task_type").and_then(|v| v.as_str()) {
        inc(&mut summary.task_type_counts, task_type);
    }
    match event {
        "route_selected" => {
            summary.route_success_count += 1;
            if let Some(provider) = payload.get("provider_id").and_then(|v| v.as_str()) {
                inc(&mut summary.provider_selection_counts, provider);
                summary.observed_providers.insert(provider.to_string());
            }
            if let Some(model) = payload.get("model_id").and_then(|v| v.as_str()) {
                inc(&mut summary.model_selection_counts, model);
            }
        }
        "route_failed" => {
            summary.route_failure_count += 1;
            if let Some(provider) = payload.get("provider_id").and_then(|v| v.as_str()) {
                inc(&mut summary.provider_failure_counts, provider);
                summary.observed_providers.insert(provider.to_string());
            }
            let reason = payload
                .get("reason")
                .or_else(|| payload.get("error"))
                .and_then(|v| v.as_str())
                .unwrap_or("unspecified");
            inc(&mut summary.failure_reason_counts, reason);
        }
        "provider_result" | "model_result" => {
            if let Some(provider) = payload.get("provider_id").and_then(|v| v.as_str()) {
                summary.observed_providers.insert(provider.to_string());
                let ok = payload.get("ok").and_then(|v| v.as_bool()).unwrap_or(true);
                if !ok {
                    inc(&mut summary.provider_failure_counts, provider);
                    let reason = payload
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("provider_result_not_ok");
                    inc(&mut summary.failure_reason_counts, reason);
                }
            }
        }
        "provider_cooldown" | "route_cooldown_bypass" => {
            if let Some(provider) = payload.get("provider_id").and_then(|v| v.as_str()) {
                inc(&mut summary.cooldown_counts, provider);
                summary.observed_providers.insert(provider.to_string());
            }
        }
        "echo_gate" => {
            let action = payload
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            inc(&mut summary.echo_gate_action_counts, action);
        }
        _ => {}
    }
}

fn parse_since_cutoff(since: Option<&str>) -> anyhow::Result<Option<chrono::DateTime<Utc>>> {
    let Some(raw) = since else { return Ok(None) };
    let raw = raw.trim();
    if raw.is_empty() || raw.eq_ignore_ascii_case("all") {
        return Ok(None);
    }
    if let Some(days) = raw.strip_suffix('d').and_then(|v| v.parse::<i64>().ok()) {
        return Ok(Some(Utc::now() - chrono::Duration::days(days)));
    }
    if let Some(hours) = raw.strip_suffix('h').and_then(|v| v.parse::<i64>().ok()) {
        return Ok(Some(Utc::now() - chrono::Duration::hours(hours)));
    }
    parse_event_ts(raw).map(Some).with_context(|| {
        format!("unsupported --since value '{raw}' (use all, 7d, 24h, or RFC3339)")
    })
}

fn parse_event_ts(raw: &str) -> anyhow::Result<chrono::DateTime<Utc>> {
    Ok(chrono::DateTime::parse_from_rfc3339(raw)?.with_timezone(&Utc))
}

fn inc(map: &mut BTreeMap<String, u64>, key: &str) {
    *map.entry(key.to_string()).or_insert(0) += 1;
}

fn top_map(map: &BTreeMap<String, u64>, limit: usize) -> Vec<serde_json::Value> {
    let mut pairs: Vec<_> = map.iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    pairs
        .into_iter()
        .take(limit)
        .map(|(key, count)| json!({ "key": key, "count": count }))
        .collect()
}

fn build_recommendations(summary: &TelemetrySummary) -> Vec<String> {
    let mut out = Vec::new();
    if summary.route_failure_count > 0 {
        out.push("Review top provider_failure_counts and failure_reason_counts before changing routing weights.".to_string());
    }
    if !summary.cooldown_counts.is_empty() {
        out.push("Providers entering cooldown should be checked for quota, payload compatibility, or local service health drift.".to_string());
    }
    if summary.malformed_state_events > 0 || summary.malformed_governance_events > 0 {
        out.push("Repair or quarantine malformed telemetry lines so long-run reports remain fully parseable.".to_string());
    }
    if summary.state_events_used == 0 && summary.governance_events_used == 0 {
        out.push(
            "No telemetry events matched this window; widen --since or verify dManwe ledger paths."
                .to_string(),
        );
    }
    out.push("Keep this summary review-gated; do not use it to mutate routing policy without a separate operator-approved change.".to_string());
    out
}

fn telemetry_idempotency_key(summary: &TelemetrySummary) -> String {
    let canonical = json!({
        "contract": "arda.manwe.telemetry_summary.v1",
        "since": summary.since,
        "first_ts": summary.first_ts,
        "last_ts": summary.last_ts,
        "state_events_used": summary.state_events_used,
        "governance_events_used": summary.governance_events_used,
        "event_counts": summary.event_counts,
        "provider_selection_counts": summary.provider_selection_counts,
        "model_selection_counts": summary.model_selection_counts,
        "failure_reason_counts": summary.failure_reason_counts,
        "cooldown_counts": summary.cooldown_counts,
        "echo_gate_action_counts": summary.echo_gate_action_counts,
        "evidence_fingerprint_sha1": summary.evidence_fingerprint,
    });
    format!(
        "manwe_telemetry_{}",
        Sha1::digest(canonical.to_string().as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}

fn append_summary_idempotent(
    path: &Path,
    record: &serde_json::Value,
    idempotency_key: &str,
) -> anyhow::Result<serde_json::Value> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)
        .with_context(|| format!("open summary ledger {}", path.display()))?;
    file.lock_exclusive()?;
    let result = (|| -> anyhow::Result<serde_json::Value> {
        let mut existing = String::new();
        file.seek(SeekFrom::Start(0))?;
        file.read_to_string(&mut existing)?;
        for line in existing.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
                if value
                    .get("summary_id")
                    .and_then(|v| v.as_str())
                    .map(|id| id == idempotency_key)
                    .unwrap_or(false)
                {
                    let mut value = value;
                    value["status"] = json!("already_recorded_idempotent_noop");
                    return Ok(value);
                }
            }
        }
        let mut to_write = record.clone();
        to_write["status"] = json!("recorded");
        file.seek(SeekFrom::End(0))?;
        writeln!(file, "{}", serde_json::to_string(&to_write)?)?;
        file.flush()?;
        Ok(to_write)
    })();
    let _ = file.unlock();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("arda-cli-{name}-{unique}"));
        fs::create_dir_all(root.join("data/manwe")).expect("mkdirs");
        root
    }

    fn write_sample_logs(root: &Path) {
        fs::write(
            root.join(MANWE_STATE_PATH),
            concat!(
                "{\"event\":\"route_selected\",\"payload\":{\"provider_id\":\"cerebras\",\"model_id\":\"qwen\",\"task_type\":\"code\"},\"ts\":\"2026-05-18T00:00:00Z\"}\n",
                "{\"event\":\"route_failed\",\"payload\":{\"provider_id\":\"openrouter\",\"reason\":\"rate_limited\",\"task_type\":\"code\"},\"ts\":\"2026-05-18T00:01:00Z\"}\n",
                "not json\n"
            ),
        )
        .expect("state");
        fs::write(
            root.join(MANWE_GOVERNANCE_EVENTS_PATH),
            concat!(
                "{\"event\":\"echo_gate\",\"payload\":{\"action\":\"Proceed\",\"task_type\":\"code\"},\"ts\":\"2026-05-18T00:02:00Z\"}\n",
                "{\"event\":\"route_selected\",\"payload\":{\"provider_id\":\"mistral\",\"model_id\":\"devstral\",\"task_type\":\"code\"},\"ts\":\"2026-05-18T00:03:00Z\"}\n"
            ),
        )
        .expect("governance");
    }

    #[test]
    fn manwe_telemetry_report_dry_run_is_review_gated_without_writing() {
        let root = temp_root("manwe-telemetry-dry-run");
        write_sample_logs(&root);
        let report = build_manwe_telemetry_report(&root, Some("all"), false, None, 100)
            .expect("telemetry report");
        assert_eq!(report["contract"], "arda.manwe.telemetry_summary.v1");
        assert_eq!(report["authority"], "agent_generated");
        assert_eq!(report["review_required"], true);
        assert_eq!(report["status"], "dry_run");
        assert_eq!(report["route_success_count"], 2);
        assert_eq!(report["route_failure_count"], 1);
        assert_eq!(report["source_health"]["malformed_state_events"], 1);
        assert!(!root.join(MANWE_TELEMETRY_SUMMARIES_PATH).exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn manwe_telemetry_write_requires_justification() {
        let root = temp_root("manwe-telemetry-justification");
        write_sample_logs(&root);
        let err = build_manwe_telemetry_report(&root, Some("all"), true, Some("  "), 100)
            .expect_err("missing justification should fail");
        assert!(err.to_string().contains("--write requires"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn manwe_telemetry_write_is_append_only_and_idempotent() {
        let root = temp_root("manwe-telemetry-write");
        write_sample_logs(&root);
        let first = build_manwe_telemetry_report(
            &root,
            Some("all"),
            true,
            Some("operator reviewed dManwe telemetry summary"),
            100,
        )
        .expect("first write");
        let second = build_manwe_telemetry_report(
            &root,
            Some("all"),
            true,
            Some("operator reviewed dManwe telemetry summary"),
            100,
        )
        .expect("second write");
        assert_eq!(first["status"], "recorded");
        assert_eq!(second["status"], "already_recorded_idempotent_noop");
        assert_eq!(first["summary_id"], second["summary_id"]);
        let ledger =
            fs::read_to_string(root.join(MANWE_TELEMETRY_SUMMARIES_PATH)).expect("ledger");
        assert_eq!(ledger.lines().count(), 1);
        let _ = fs::remove_dir_all(root);
    }
}
