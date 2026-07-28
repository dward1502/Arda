use crate::BudgetAlert;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub const PLUTUS_FINANCE_SCHEMA_VERSION: &str = "arda.plutus.finance.v1";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FinanceMetrics {
    pub spend_total: f64,
    pub credit_total: f64,
    pub budget_usage_percent: f64,
    pub ledger_last_account: Option<String>,
    pub ledger_last_amount: Option<f64>,
    pub streamed_events: u64,
    pub budget_alert: Option<BudgetAlert>,
    pub snapshot_age_seconds: Option<u64>,
    pub snapshot_stale: bool,
    pub transport_requests_total: u64,
    pub transport_elapsed_micros_average: f64,
    pub transport_elapsed_micros_max: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinanceStreamReport {
    pub schema_version: String,
    pub metrics: FinanceMetrics,
}

pub fn finance_stream_report(home: PathBuf) -> Result<FinanceStreamReport> {
    let metrics = finance_metrics(home.clone())?;
    Ok(FinanceStreamReport {
        schema_version: PLUTUS_FINANCE_SCHEMA_VERSION.to_owned(),
        metrics,
    })
}

pub fn finance_metrics(home: PathBuf) -> Result<FinanceMetrics> {
    let runtime_path = home.join("runtime_status.json");
    let mut metrics = FinanceMetrics {
        budget_usage_percent: 100.0,
        budget_alert: Some(BudgetAlert::Critical),
        snapshot_stale: true,
        ..FinanceMetrics::default()
    };
    if !runtime_path.exists() {
        return Ok(metrics);
    }
    let raw = fs::read_to_string(runtime_path)?;
    let snapshot: serde_json::Value = serde_json::from_str(&raw)?;
    metrics.spend_total = snapshot
        .get("economics")
        .and_then(|value| value.get("spend_total"))
        .and_then(|value| value.as_f64())
        .unwrap_or_default();
    metrics.credit_total = snapshot
        .get("ledger")
        .and_then(|value| value.get("credit_total"))
        .and_then(|value| value.as_f64())
        .unwrap_or_default();
    metrics.budget_usage_percent = snapshot
        .get("economics")
        .and_then(|value| value.get("budget_usage_percent"))
        .and_then(|value| value.as_f64())
        .unwrap_or_default();
    metrics.budget_alert = snapshot
        .get("economics")
        .and_then(|value| value.get("budget_alert"))
        .and_then(|value| serde_json::from_value(value.clone()).ok());
    metrics.snapshot_age_seconds = snapshot_age_seconds(&home);
    metrics.snapshot_stale = metrics.snapshot_age_seconds.is_none_or(|age| age > 300);
    metrics.ledger_last_account = snapshot
        .get("ledger")
        .and_then(|value| value.get("last_credit_account"))
        .and_then(|value| value.as_str())
        .map(str::to_owned);
    metrics.ledger_last_amount = snapshot
        .get("ledger")
        .and_then(|value| value.get("last_credit_amount"))
        .and_then(|value| value.as_f64());
    metrics.transport_requests_total = snapshot
        .get("transport_latency")
        .and_then(|value| value.get("requests_total"))
        .and_then(|value| value.as_u64())
        .unwrap_or_default();
    metrics.transport_elapsed_micros_average = snapshot
        .get("transport_latency")
        .and_then(|value| value.get("elapsed_micros_average"))
        .and_then(|value| value.as_f64())
        .unwrap_or_default();
    metrics.transport_elapsed_micros_max = snapshot
        .get("transport_latency")
        .and_then(|value| value.get("elapsed_micros_max"))
        .and_then(|value| value.as_u64())
        .unwrap_or_default();
    metrics.streamed_events = event_stream_total(home);
    Ok(metrics)
}

fn snapshot_age_seconds(home: &std::path::Path) -> Option<u64> {
    fs::metadata(home.join("runtime_status.json"))
        .ok()?
        .modified()
        .ok()?
        .elapsed()
        .ok()
        .map(|age| age.as_secs())
}

fn event_stream_total(home: PathBuf) -> u64 {
    let path = home.join("runtime_events.jsonl");
    if !path.exists() {
        return 0;
    }
    fs::read_to_string(path)
        .map(|raw| raw.lines().filter(|line| !line.trim().is_empty()).count() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn finance_metrics_reads_runtime_snapshot() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("runtime_status.json"),
            serde_json::to_vec(&json!({
                "schema_version": "arda.plutus.runtime.v2",
                "economics": {
                    "spend_total": 12.5,
                    "budget_usage_percent": 0.45,
                    "budget_alert": "warning",
                },
                "ledger": {
                    "credit_total": 8.25,
                    "last_credit_account": "acme",
                    "last_credit_amount": 2.0,
                },
                "transport_latency": {
                    "requests_total": 4,
                    "elapsed_micros_average": 12.5,
                    "elapsed_micros_max": 20,
                },
            }))
            .expect("snapshot"),
        )
        .expect("write snapshot");

        let metrics = finance_metrics(dir.path().to_path_buf()).expect("metrics");
        assert!((metrics.spend_total - 12.5).abs() < f64::EPSILON);
        assert!((metrics.credit_total - 8.25).abs() < f64::EPSILON);
        assert!((metrics.budget_usage_percent - 0.45).abs() < f64::EPSILON);
        assert_eq!(metrics.budget_alert, Some(BudgetAlert::Warning));
        assert!(!metrics.snapshot_stale);
        assert_eq!(metrics.transport_requests_total, 4);
        assert_eq!(metrics.transport_elapsed_micros_max, 20);
        assert_eq!(metrics.ledger_last_account.as_deref(), Some("acme"));
        assert!((metrics.ledger_last_amount.unwrap_or(0.0) - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn finance_metrics_counts_streamed_events() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("runtime_status.json"),
            serde_json::to_vec(&json!({"economics": {}, "ledger": {}})).expect("snapshot"),
        )
        .expect("write snapshot");
        std::fs::write(
            dir.path().join("runtime_events.jsonl"),
            "{\"event\": 1}\n\n{\"event\": 2}\n",
        )
        .expect("events");

        let metrics = finance_metrics(dir.path().to_path_buf()).expect("metrics");
        assert_eq!(metrics.streamed_events, 2);
    }

    #[test]
    fn finance_stream_report_wraps_schema_version() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("runtime_status.json"),
            serde_json::to_vec(&json!({"economics": {}, "ledger": {}})).expect("snapshot"),
        )
        .expect("write snapshot");

        let report = finance_stream_report(dir.path().to_path_buf()).expect("report");
        assert_eq!(report.schema_version, PLUTUS_FINANCE_SCHEMA_VERSION);
        assert_eq!(report.metrics.spend_total, 0.0);
    }

    #[test]
    fn finance_metrics_returns_defaults_when_snapshot_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let metrics = finance_metrics(dir.path().to_path_buf()).expect("metrics");
        assert!((metrics.budget_usage_percent - 100.0).abs() < f64::EPSILON);
        assert_eq!(metrics.budget_alert, Some(BudgetAlert::Critical));
        assert!(metrics.snapshot_stale);
        assert_eq!(metrics.streamed_events, 0);
    }
}
