#[tokio::test]
async fn engine_runs_orome_manual_dispatch_smoke_path() {
    let report = arda_engine::orome::manual_smoke_dispatch()
        .await
        .expect("orome smoke dispatch");

    assert!(report.receipt.dispatched);
    assert_eq!(report.receipt.provider_id, "manual-smoke");
    assert_eq!(report.receipt.attempts, 1);
    assert!(report.receipt.streaming);
    assert_eq!(report.receipt.chunks_sent, 1);
    assert_eq!(report.metrics.succeeded, 1);
    assert!(report
        .hud_surfaces
        .contains(&"provider_metrics".to_string()));
    assert!(report.hud_surfaces.contains(&"human_plan".to_string()));
}
