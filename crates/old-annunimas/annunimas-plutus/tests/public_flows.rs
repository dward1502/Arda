use annunimas_core::JouleWorkMeasurementSource;
use annunimas_plutus::{CostModelConfig, JouleWorkTracker, JouleWorkUnit, PlutusService};

#[tokio::test]
async fn public_economics_and_status_flow() {
    let dir = tempfile::tempdir().expect("tempdir");
    let service = PlutusService::from_home(dir.path()).expect("service");

    service
        .register_model(CostModelConfig {
            provider: "openai".to_owned(),
            input_rate: 0.001,
            output_rate: 0.002,
            batch_size: 1000,
        })
        .await
        .expect("register");
    let cost = service
        .record_spend("openai", 100, 50)
        .await
        .expect("spend");
    assert!(cost.is_some());

    let status = service.status().await.expect("status");
    assert_eq!(status["authority"], "plutus_service");
    assert!(
        status["economics"]["total_spend"]
            .as_f64()
            .unwrap_or_default()
            > 0.0
    );
}

#[tokio::test]
async fn public_work_credit_and_relationship_flow() {
    let dir = tempfile::tempdir().expect("tempdir");
    let service = PlutusService::from_home(dir.path()).expect("service");

    service
        .track_work(
            "athena",
            2.0,
            JouleWorkUnit::Reasoning,
            Some("task_public_1".to_owned()),
        )
        .await
        .expect("track");
    service.credit("athena", 4.0).await.expect("credit");
    let score = service
        .record_relationship("athena", "hermes", 0.9, 0.8, 0.85)
        .await
        .expect("relationship");
    assert!(score > 0.0);

    let status = service.status().await.expect("status");
    let accounts = status["ledger"]["accounts"].as_array().expect("accounts");
    assert!(accounts.iter().any(|row| {
        row["account"].as_str() == Some("athena") && row["balance"].as_f64() == Some(4.0)
    }));
    assert!(status["joulework"]["total"].as_f64().unwrap_or_default() > 0.0);
    assert!(
        status["love_equation"]["relationships_total"]
            .as_u64()
            .unwrap_or(0)
            >= 1
    );
}

#[tokio::test]
async fn joulework_tracker_reports_measurement_source_confidence_and_legacy_defaults() {
    let tracker = JouleWorkTracker::new();

    tracker
        .track_work(
            "hades",
            1.0,
            JouleWorkUnit::Compute,
            Some("legacy-default".to_owned()),
        )
        .await;
    tracker
        .track_work_with_source(
            "charon",
            2.0,
            JouleWorkUnit::Reasoning,
            Some("provider-observed".to_owned()),
            JouleWorkMeasurementSource::ProviderUsageReport,
            1.2,
        )
        .await;

    let summary = tracker.summary().await;
    assert_eq!(summary.default_fallback_total, 1.0);
    assert_eq!(summary.observed_total, 4.0);
    assert_eq!(summary.average_confidence, 0.5);
    assert_eq!(
        summary
            .by_source
            .get(&JouleWorkMeasurementSource::DefaultFallback)
            .copied(),
        Some(1.0)
    );
    assert_eq!(
        summary
            .by_source
            .get(&JouleWorkMeasurementSource::ProviderUsageReport)
            .copied(),
        Some(4.0)
    );

    let status = tracker.status_snapshot().await;
    assert_eq!(status["measurement_metadata"]["observed_total"], 4.0);
    assert_eq!(
        status["measurement_metadata"]["default_fallback_total"],
        1.0
    );
    assert_eq!(status["measurement_metadata"]["average_confidence"], 0.5);
    assert_eq!(
        status["measurement_metadata"]["autonomy_truth_warning"],
        true
    );
    assert_eq!(
        status["measurement_metadata"]["default_fallback_autonomy_truth"],
        false
    );
    assert!(status["by_source"]
        .as_array()
        .expect("sources")
        .iter()
        .any(|row| {
            row["source"].as_str() == Some("provider_usage_report")
                && row["amount"].as_f64() == Some(4.0)
        }));
}
