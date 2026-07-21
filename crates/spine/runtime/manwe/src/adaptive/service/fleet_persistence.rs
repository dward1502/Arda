// Snapshot of fleet-derived provider state and admission receipts.

use std::path::Path;

use crate::adaptive::service::types::ProviderState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetBootstrapSnapshot {
    pub generated_at_utc: String,
    pub schema_version: String,
    pub summary: FleetBootstrapSummary,
    pub providers: Vec<FleetProviderSnapshot>,
    pub receipts_path: std::path::PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FleetBootstrapSummary {
    pub providers_total: usize,
    pub providers_healthy: usize,
    pub providers_degraded: usize,
    pub providers_offline: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetProviderSnapshot {
    pub id: String,
    pub manwe_provider_id: Option<String>,
    pub target_id: Option<String>,
    pub display_name: Option<String>,
    pub base_url: Option<String>,
    pub models_url: Option<String>,
    pub health_url: Option<String>,
    pub status: String,
    pub models: Vec<String>,
    pub has_live_endpoint: bool,
    pub provider_hint: Option<String>,
    pub healthy: bool,
    pub intentional_offline: bool,
    pub last_error: Option<String>,
    pub ts_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdmissionsShedReceipt {
    pub ts_utc: String,
    pub source: String,
    pub provider_id: String,
    pub model_id: Option<String>,
    pub reason: String,
    pub detail: serde_json::Value,
}

impl FleetBootstrapSnapshot {
    pub fn new(providers: Vec<FleetProviderSnapshot>) -> Self {
        let providers_total = providers.len();
        let providers_healthy = providers
            .iter()
            .filter(|p| p.healthy && !p.intentional_offline)
            .count();
        let providers_degraded =
            providers.iter().filter(|p| matches!(p.status.as_str(), "degraded")).count();
        let providers_offline =
            providers.iter().filter(|p| matches!(p.status.as_str(), "offline")).count();
        Self {
            generated_at_utc: chrono::Utc::now().to_rfc3339(),
            schema_version: "arda.fleet-bootstrap.v2".to_string(),
            summary: FleetBootstrapSummary {
                providers_total,
                providers_healthy,
                providers_degraded,
                providers_offline,
            },
            providers,
            receipts_path: std::path::PathBuf::from(
                "data/prometheus/runtime_admission_shed_receipts.jsonl",
            ),
        }
    }

    pub fn from_provider_states(providers: &[ProviderState]) -> Self {
        let snapshots: Vec<FleetProviderSnapshot> = providers
            .iter()
            .map(|provider| {
                let status = if provider.enabled {
                    if provider.healthy {
                        if provider.in_cooldown {
                            "degraded"
                        } else {
                            "online"
                        }
                    } else {
                        "offline"
                    }
                } else {
                    "disabled"
                };
                FleetProviderSnapshot {
                    id: provider.id.clone(),
                    manwe_provider_id: Some(provider.id.clone()),
                    target_id: None,
                    display_name: Some(provider.name.clone()),
                    base_url: provider.base_url.clone(),
                    models_url: None,
                    health_url: None,
                    status: status.to_string(),
                    models: provider.models.iter().map(|model| model.id.clone()).collect(),
                    has_live_endpoint: provider.healthy,
                    provider_hint: Some(provider.driver.clone()),
                    healthy: provider.healthy,
                    intentional_offline: !provider.enabled,
                    last_error: provider.last_error.clone(),
                    ts_utc: chrono::Utc::now().to_rfc3339(),
                }
            })
            .collect();
        Self::new(snapshots)
    }
}

impl FleetProviderSnapshot {
    pub fn new(
        id: impl Into<String>,
        base_url: impl Into<String>,
        model_id: impl Into<String>,
        healthy: bool,
    ) -> Self {
        Self {
            id: id.into(),
            manwe_provider_id: None,
            target_id: None,
            display_name: None,
            base_url: Some(base_url.into()),
            models_url: None,
            health_url: None,
            status: if healthy { "online" } else { "offline" }.to_string(),
            models: vec![model_id.into()],
            has_live_endpoint: healthy,
            provider_hint: None,
            healthy,
            intentional_offline: !healthy,
            last_error: if healthy { None } else { Some("unknown".to_string()) },
            ts_utc: chrono::Utc::now().to_rfc3339(),
        }
    }
}

impl AdmissionsShedReceipt {
    pub fn new(
        source: impl Into<String>,
        provider_id: impl Into<String>,
        model_id: Option<impl Into<String>>,
        reason: impl Into<String>,
        detail: serde_json::Value,
    ) -> Self {
        Self {
            ts_utc: chrono::Utc::now().to_rfc3339(),
            source: source.into(),
            provider_id: provider_id.into(),
            model_id: model_id.map(|m| m.into()),
            reason: reason.into(),
            detail,
        }
    }
}

pub fn write_fleet_bootstrap_json(
    path: &Path,
    snapshot: &FleetBootstrapSnapshot,
) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes =
        serde_json::to_vec_pretty(snapshot).map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

pub fn append_admissions_shed_receipt(
    path: &Path,
    receipt: &AdmissionsShedReceipt,
) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    use std::io::Write;
    let line =
        serde_json::to_string(&receipt).map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
    writeln!(file, "{line}")?;
    file.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_provider_states_into_bootstrap_snapshot() {
        let providers = vec![
            ProviderState {
                id: "edge_core".into(),
                name: "Core Hub".into(),
                ..ProviderState::default()
            },
            ProviderState {
                id: "edge_guard".into(),
                name: "Guard House".into(),
                enabled: false,
                ..ProviderState::default()
            },
        ];
        let snapshot = FleetBootstrapSnapshot::from_provider_states(&providers);
        assert_eq!(snapshot.providers.len(), 2);
        assert_eq!(snapshot.summary.providers_total, 2);
        assert_eq!(snapshot.summary.providers_offline, 0);
        assert_eq!(snapshot.providers[0].status, "online");
        assert_eq!(snapshot.providers[1].status, "disabled");
    }

    #[test]
    fn appends_jsonl_receipt() {
        let dir = std::env::temp_dir().join("arda-fleet-persistence");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("receipts.jsonl");
        let receipt = AdmissionsShedReceipt::new(
            "unit",
            "edge_core",
            Some("LFM"),
            "tests",
            serde_json::json!({"ok": true}),
        );
        append_admissions_shed_receipt(&path, &receipt).unwrap();
        let line = std::fs::read_to_string(&path).unwrap();
        assert!(line.contains("\"provider_id\":\"edge_core\""));
        assert!(line.ends_with('\n'));
    }

    #[test]
    fn writes_bootstrap_snapshot_atomically() {
        let dir = std::env::temp_dir().join("arda-fleet-bootstrap");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("fleet_bootstrap.json");
        let providers = vec![FleetProviderSnapshot::new(
            "edge_core",
            "http://core:9337/v1",
            "LFM",
            true,
        )];
        let snapshot = FleetBootstrapSnapshot::new(providers);
        write_fleet_bootstrap_json(&path, &snapshot).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let parsed: FleetBootstrapSnapshot = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed.providers.len(), 1);
        assert_eq!(parsed.providers[0].status, "online");
        assert_eq!(parsed.schema_version, "arda.fleet-bootstrap.v2");
    }
}
