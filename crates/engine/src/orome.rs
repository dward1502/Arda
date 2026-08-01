//! Live arda-orome interface wiring owned by the engine.

use arda_orome::provider::{
    DispatchMetricsSnapshot, DispatchReceipt, ManualTransport, ProviderConfig, ProviderRuntime,
    ProviderType, RoutingIntent, TransportRequest,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct OromeSmokeReport {
    pub schema_version: &'static str,
    pub receipt: DispatchReceipt,
    pub metrics: DispatchMetricsSnapshot,
    pub hud_surfaces: Vec<String>,
}

/// Exercise the interface package without network or external side effects.
/// The resulting typed receipt is suitable for CLI and HUD status projections.
pub async fn manual_smoke_dispatch() -> anyhow::Result<OromeSmokeReport> {
    let runtime = ProviderRuntime::new(vec![ProviderConfig {
        id: "manual-smoke".to_string(),
        kind: ProviderType::Custom,
        name: "Manual smoke transport".to_string(),
        endpoint: "manual://engine-smoke".to_string(),
        capabilities: vec![
            "streaming".to_string(),
            "edge_local_only".to_string(),
            "receipt_required".to_string(),
        ],
    }]);
    let result = runtime
        .dispatch(
            RoutingIntent::direct("manual-smoke"),
            TransportRequest::new("engine-smoke", "arda-orome interface probe").streaming(true),
            &ManualTransport::test(),
        )
        .await;
    let receipt = result.receipts.into_iter().next().ok_or_else(|| {
        anyhow::anyhow!(result
            .error
            .unwrap_or_else(|| "missing receipt".to_string()))
    })?;
    Ok(OromeSmokeReport {
        schema_version: "arda.engine.orome_smoke.v1",
        receipt,
        metrics: runtime.metrics(),
        hud_surfaces: vec![
            "provider_metrics".to_string(),
            "human_plan".to_string(),
            "governance_receipts".to_string(),
        ],
    })
}
