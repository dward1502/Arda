use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::Path;

const STALE_AFTER_SECONDS: i64 = 7_200;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeedDomainSummary {
    pub source: String,
    pub path: String,
    pub present: bool,
    pub valid_json: bool,
    pub bytes: Option<usize>,
    pub authority: Option<String>,
    pub generated_at_utc: Option<String>,
    pub drift_seconds: Option<i64>,
    pub freshness: String,
    pub anomaly: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChronosTypedFeed<T> {
    pub summary: FeedDomainSummary,
    pub model: Option<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChronosTypedStateFeeds {
    pub warden: ChronosTypedFeed<WardenFeedModel>,
    pub mnemosyne: ChronosTypedFeed<MnemosyneFeedModel>,
    pub plutus: ChronosTypedFeed<PlutusFeedModel>,
    pub charon: ChronosTypedFeed<CharonFeedModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WardenFeedModel {
    pub duty_count: usize,
    pub fleet_node_count: Option<usize>,
    pub stale_offline_total: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MnemosyneFeedModel {
    pub recent_memory_count: Option<usize>,
    pub event_type_count: Option<usize>,
    pub source_crate_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlutusFeedModel {
    pub provider_count: Option<usize>,
    pub governance_recent_record_count: Option<usize>,
    pub bacon_lite_passed_total: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CharonFeedModel {
    pub provider_count: Option<u64>,
    pub degraded_count: Option<u64>,
    pub cooldown_count: Option<u64>,
    pub recovery_failed_total: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChronosFeedSummary {
    pub feed_count: usize,
    pub present_count: usize,
    pub missing_count: usize,
    pub invalid_json_count: usize,
    pub stale_count: usize,
    pub max_age_seconds: Option<i64>,
    pub domains: Vec<FeedDomainSummary>,
    pub typed_feeds: ChronosTypedStateFeeds,
    pub anomalies: Vec<String>,
}

pub fn summarize_state_feeds(root: &Path, snapshot_time: DateTime<Utc>) -> ChronosFeedSummary {
    let domains: Vec<FeedDomainSummary> = feed_specs()
        .into_iter()
        .map(|(source, relative_path)| summarize_feed(root, source, relative_path, snapshot_time))
        .collect();

    let present_count = domains.iter().filter(|domain| domain.present).count();
    let missing_count = domains.iter().filter(|domain| !domain.present).count();
    let invalid_json_count = domains
        .iter()
        .filter(|domain| domain.present && !domain.valid_json)
        .count();
    let stale_count = domains
        .iter()
        .filter(|domain| domain.freshness == "stale")
        .count();
    let max_age_seconds = domains
        .iter()
        .filter_map(|domain| domain.drift_seconds)
        .max();
    let anomalies = domains
        .iter()
        .filter_map(|domain| domain.anomaly.clone())
        .collect();
    let typed_feeds = ChronosTypedStateFeeds::from_root_and_domains(root, &domains);

    ChronosFeedSummary {
        feed_count: domains.len(),
        present_count,
        missing_count,
        invalid_json_count,
        stale_count,
        max_age_seconds,
        domains,
        typed_feeds,
        anomalies,
    }
}

fn feed_specs() -> [(&'static str, &'static str); 4] {
    [
        ("warden", "core/state/warden_guardhouse.json"),
        ("mnemosyne", "core/state/memory_activity.json"),
        ("plutus", "core/state/plutus_runtime.json"),
        ("charon", "core/state/charon_router.json"),
    ]
}

fn summarize_feed(
    root: &Path,
    source: &str,
    relative_path: &str,
    snapshot_time: DateTime<Utc>,
) -> FeedDomainSummary {
    let path = root.join(relative_path);
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) => {
            let anomaly = format!("{source}:missing_feed");
            return FeedDomainSummary {
                source: source.to_string(),
                path: relative_path.to_string(),
                present: false,
                valid_json: false,
                bytes: None,
                authority: None,
                generated_at_utc: None,
                drift_seconds: None,
                freshness: "missing".to_string(),
                anomaly: Some(if error.kind() == std::io::ErrorKind::NotFound {
                    anomaly
                } else {
                    format!("{source}:read_error")
                }),
            };
        }
    };

    let bytes = content.len();
    let parsed = match serde_json::from_str::<Value>(&content) {
        Ok(value) => value,
        Err(_) => {
            return FeedDomainSummary {
                source: source.to_string(),
                path: relative_path.to_string(),
                present: true,
                valid_json: false,
                bytes: Some(bytes),
                authority: None,
                generated_at_utc: None,
                drift_seconds: None,
                freshness: "invalid_json".to_string(),
                anomaly: Some(format!("{source}:invalid_json")),
            };
        }
    };

    let authority = parsed
        .get("authority")
        .and_then(Value::as_str)
        .map(str::to_string);
    let generated_at_utc = parsed
        .get("generated_at_utc")
        .and_then(Value::as_str)
        .map(str::to_string);
    let drift_seconds = generated_at_utc
        .as_deref()
        .and_then(|timestamp| DateTime::parse_from_rfc3339(timestamp).ok())
        .map(|timestamp| {
            snapshot_time
                .signed_duration_since(timestamp.with_timezone(&Utc))
                .num_seconds()
                .abs()
        });

    let freshness = classify_freshness(drift_seconds);
    let anomaly = match freshness {
        "stale" => Some(format!("{source}:stale_feed")),
        "missing_timestamp" => Some(format!("{source}:missing_timestamp")),
        _ => None,
    };

    FeedDomainSummary {
        source: source.to_string(),
        path: relative_path.to_string(),
        present: true,
        valid_json: true,
        bytes: Some(bytes),
        authority,
        generated_at_utc,
        drift_seconds,
        freshness: freshness.to_string(),
        anomaly,
    }
}

fn classify_freshness(drift_seconds: Option<i64>) -> &'static str {
    match drift_seconds {
        Some(seconds) if seconds <= STALE_AFTER_SECONDS => "current",
        Some(_) => "stale",
        None => "missing_timestamp",
    }
}

impl ChronosTypedStateFeeds {
    fn from_root_and_domains(root: &Path, domains: &[FeedDomainSummary]) -> Self {
        Self {
            warden: typed_feed(root, domains, "warden", parse_warden_model),
            mnemosyne: typed_feed(root, domains, "mnemosyne", parse_mnemosyne_model),
            plutus: typed_feed(root, domains, "plutus", parse_plutus_model),
            charon: typed_feed(root, domains, "charon", parse_charon_model),
        }
    }
}

fn typed_feed<T>(
    root: &Path,
    domains: &[FeedDomainSummary],
    source: &str,
    parse: fn(&Value) -> T,
) -> ChronosTypedFeed<T> {
    let summary = domains
        .iter()
        .find(|domain| domain.source == source)
        .cloned()
        .unwrap_or_else(|| FeedDomainSummary {
            source: source.to_string(),
            path: String::new(),
            present: false,
            valid_json: false,
            bytes: None,
            authority: None,
            generated_at_utc: None,
            drift_seconds: None,
            freshness: "missing".to_string(),
            anomaly: Some(format!("{source}:missing_feed")),
        });
    let model = if summary.present && summary.valid_json {
        read_json(root.join(&summary.path)).map(|value| parse(&value))
    } else {
        None
    };

    ChronosTypedFeed { summary, model }
}

fn read_json(path: impl AsRef<Path>) -> Option<Value> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&content).ok()
}

fn parse_warden_model(value: &Value) -> WardenFeedModel {
    WardenFeedModel {
        duty_count: value
            .get("duties")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or_default(),
        fleet_node_count: value
            .pointer("/health/fleet_control/fleet_nodes")
            .and_then(Value::as_array)
            .map(Vec::len),
        stale_offline_total: value
            .pointer("/health/fleet_control/connection_cleanup/stale_offline_total")
            .and_then(Value::as_u64),
    }
}

fn parse_mnemosyne_model(value: &Value) -> MnemosyneFeedModel {
    MnemosyneFeedModel {
        recent_memory_count: value
            .pointer("/recent_activity/memories")
            .and_then(Value::as_array)
            .map(Vec::len),
        event_type_count: value
            .pointer("/distributions/event_types")
            .and_then(Value::as_object)
            .map(serde_json::Map::len),
        source_crate_count: value
            .pointer("/distributions/source_crates")
            .and_then(Value::as_object)
            .map(serde_json::Map::len),
    }
}

fn parse_plutus_model(value: &Value) -> PlutusFeedModel {
    PlutusFeedModel {
        provider_count: value
            .pointer("/runtime/economics/providers")
            .and_then(Value::as_array)
            .map(Vec::len),
        governance_recent_record_count: value
            .pointer("/runtime/governance/recent_records")
            .and_then(Value::as_array)
            .map(Vec::len),
        bacon_lite_passed_total: value
            .pointer("/runtime/governance/bacon_lite_passed_total")
            .and_then(Value::as_u64),
    }
}

fn parse_charon_model(value: &Value) -> CharonFeedModel {
    CharonFeedModel {
        provider_count: value
            .pointer("/arda_hints/provider_count")
            .and_then(Value::as_u64),
        degraded_count: value
            .pointer("/arda_hints/degraded_count")
            .and_then(Value::as_u64),
        cooldown_count: value
            .pointer("/arda_hints/cooldown_count")
            .and_then(Value::as_u64),
        recovery_failed_total: value
            .pointer("/bootstrap_recovery/summary/restart_failed_total")
            .and_then(Value::as_u64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use std::fs;

    #[test]
    fn summarizes_feed_domains_with_freshness_and_anomalies() {
        let root = std::env::temp_dir().join(format!(
            "annunimas_chronos_feed_summary_test_{}",
            Utc::now().timestamp_nanos_opt().expect("timestamp nanos")
        ));
        let state_dir = root.join("core/state");
        fs::create_dir_all(&state_dir).expect("state dir");
        fs::write(
            state_dir.join("warden_guardhouse.json"),
            r#"{"authority":"warden_system_projection","generated_at_utc":"2026-05-24T10:00:00Z","duties":["informant_network","drift_watch"],"health":{"fleet_control":{"fleet_nodes":[{"hostname":"core"},{"hostname":"edge"}],"connection_cleanup":{"stale_offline_total":1}}}}"#,
        )
        .expect("warden fixture");
        fs::write(
            state_dir.join("memory_activity.json"),
            r#"{"authority":"memory_activity_projection","generated_at_utc":"2026-05-24T07:30:00Z","distributions":{"event_types":{"task_completed":2},"source_crates":{"prometheus":2,"charon":1}},"recent_activity":{"memories":[{"id":"m1"},{"id":"m2"}]}}"#,
        )
        .expect("mnemosyne fixture");
        fs::write(state_dir.join("plutus_runtime.json"), "{not-json").expect("plutus fixture");

        let snapshot_time = Utc
            .with_ymd_and_hms(2026, 5, 24, 10, 30, 0)
            .single()
            .expect("snapshot time");
        let summary = summarize_state_feeds(&root, snapshot_time);

        assert_eq!(summary.feed_count, 4);
        assert_eq!(summary.present_count, 3);
        assert_eq!(summary.missing_count, 1);
        assert_eq!(summary.invalid_json_count, 1);
        assert_eq!(summary.stale_count, 1);
        assert_eq!(summary.max_age_seconds, Some(10_800));
        assert!(summary
            .anomalies
            .iter()
            .any(|anomaly| anomaly == "mnemosyne:stale_feed"));
        assert!(summary
            .anomalies
            .iter()
            .any(|anomaly| anomaly == "plutus:invalid_json"));
        assert!(summary
            .anomalies
            .iter()
            .any(|anomaly| anomaly == "charon:missing_feed"));

        let warden = summary
            .domains
            .iter()
            .find(|domain| domain.source == "warden")
            .expect("warden domain");
        assert_eq!(warden.freshness, "current");
        assert_eq!(warden.drift_seconds, Some(1_800));
        assert_eq!(
            warden.authority.as_deref(),
            Some("warden_system_projection")
        );
        let warden_model = summary
            .typed_feeds
            .warden
            .model
            .as_ref()
            .expect("warden typed model");
        assert_eq!(warden_model.duty_count, 2);
        assert_eq!(warden_model.fleet_node_count, Some(2));
        assert_eq!(warden_model.stale_offline_total, Some(1));
        let mnemosyne_model = summary
            .typed_feeds
            .mnemosyne
            .model
            .as_ref()
            .expect("mnemosyne typed model");
        assert_eq!(mnemosyne_model.recent_memory_count, Some(2));
        assert_eq!(mnemosyne_model.event_type_count, Some(1));
        assert_eq!(mnemosyne_model.source_crate_count, Some(2));
        assert!(summary.typed_feeds.plutus.model.is_none());
        assert!(summary.typed_feeds.charon.model.is_none());

        fs::remove_dir_all(root).expect("cleanup");
    }
}
