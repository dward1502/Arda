// Snapshot of fleet-derived provider state and admission receipts.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetBootstrapSnapshot {
    pub generated_at_utc: String,
    pub providers: Vec<FleetProviderSnapshot>,
    pub receipts_path: std::path::PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetProviderSnapshot {
    pub id: String,
    pub base_url: String,
    pub model_id: String,
    pub healthy: bool,
    pub ts_utc: String,
}

impl FleetBootstrapSnapshot {
    pub fn new(providers: Vec<FleetProviderSnapshot>) -> Self {
        Self { generated_at_utc: chrono::Utc::now().to_rfc3339(), providers, receipts_path: std::path::PathBuf::from("data/prometheus/runtime_admission_shed_receipts.jsonl") }
    }
}

impl FleetProviderSnapshot {
    pub fn new(id: impl Into<String>, base_url: impl Into<String>, model_id: impl Into<String>, healthy: bool) -> Self {
        Self { id: id.into(), base_url: base_url.into(), model_id: model_id.into(), healthy, ts_utc: chrono::Utc::now().to_rfc3339() }
    }
}
