use arda_outpost_scout::{
    SurveyReport,
    suggestion::{analyze_survey, AdvisoryLevel},
};
use arda_outpost_scout::observation::CrateStatus;

fn fake_crate_observation() -> arda_outpost_scout::CrateObservation {
    arda_outpost_scout::CrateObservation {
        path: "fixtures/fake-crate".to_string(),
        name: "fake-crate".to_string(),
        purpose: Some("fixture for tests".to_string()),
        status: CrateStatus::Active,
        key_entrypoints: vec!["src/lib.rs".to_string()],
        test_surface: vec!["tests/fake.rs".to_string()],
        dependencies: vec!["serde".to_string()],
        dev_patterns: vec![],
        observed_at: chrono::Utc::now(),
    }
}

#[test]
fn advisory_reports_use_active_as_base() {
    let report = SurveyReport::new("node-pi5-warden", vec![fake_crate_observation()]);
    let advisory = analyze_survey(&report);
    assert_eq!(advisory.max_level, AdvisoryLevel::Action);
}

#[test]
fn survey_report_validates_schema_and_attributes() {
    let report = SurveyReport::new("node-pi5-warden", vec![fake_crate_observation()]);
    assert_eq!(report.schema_version, arda_outpost_protocol::SCHEMA_VERSION);
    assert_eq!(report.observations.len(), 1);
    assert_eq!(report.source, "node-pi5-warden");
}
