use arda_outpost_scout::{
    AuditFollowupRequest, AuditFollowupSection, ScoutAuditRequest, ScoutAuditService,
};
use arda_rumil::{AuditReportCompleteness, AuditRequest};
use chrono::{Duration, Utc};
use tempfile::tempdir;
use uuid::Uuid;

fn request(project_id: Uuid, expires_at_utc: chrono::DateTime<Utc>) -> ScoutAuditRequest {
    ScoutAuditRequest {
        root: "project".into(),
        project_name: "fixture".to_string(),
        project_kind: "rust".to_string(),
        remote_url: None,
        request: AuditRequest {
            request_id: Uuid::new_v4(),
            project_id,
            profile_id: "warden-read-only".to_string(),
            source_revision_expectation: None,
            requested_capabilities: vec!["inventory".to_string()],
            root_policy: "bounded_request_root".to_string(),
            path_exclusions: vec!["private".to_string()],
            file_count_budget: 100,
            byte_budget: 1024 * 1024,
            source_excerpt_budget: 4096,
            command_timeout_seconds: 5,
            provider_allowlist: Vec::new(),
            redaction_policy: vec!["default_secrets".to_string()],
            prior_audit_id: None,
            requested_by: "node-pi5-warden".to_string(),
            expires_at_utc,
            authority: "advisory_read_only".to_string(),
        },
    }
}

fn fixture() -> tempfile::TempDir {
    let root = tempdir().expect("fixture root");
    let project = root.path().join("project");
    std::fs::create_dir_all(project.join("src")).expect("source directory");
    std::fs::write(
        project.join("Cargo.toml"),
        "[package]\nname='fixture'\nversion='0.1.0'\n",
    )
    .expect("manifest");
    std::fs::write(project.join("src/lib.rs"), "pub fn fixture() {}\n").expect("source");
    root
}

#[test]
fn warden_executes_bounded_audit_and_persists_large_packet_outside_memory() {
    let root = fixture();
    let service = ScoutAuditService::new(root.path(), "node-pi5-warden");
    let outcome = service
        .execute(
            request(Uuid::new_v4(), Utc::now() + Duration::minutes(5)),
            Utc::now(),
        )
        .expect("bounded audit");

    assert!(!outcome.replayed);
    assert_eq!(
        outcome.report.completeness,
        AuditReportCompleteness::Complete
    );
    assert!(outcome.packet_path.starts_with("data/warden/rumil_audits/"));
    assert!(root.path().join(&outcome.packet_path).is_file());
    assert_eq!(
        outcome.observation.payload["audit_id"],
        outcome.report.audit_id.to_string()
    );
    assert!(outcome.observation.payload.get("file_records").is_none());

    let packet: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.path().join(&outcome.packet_path)).expect("packet bytes"),
    )
    .expect("packet json");
    assert!(packet["file_records"]
        .as_array()
        .is_some_and(|records| !records.is_empty()));
}

#[test]
fn replay_is_idempotent_and_does_not_append_another_receipt() {
    let root = fixture();
    let service = ScoutAuditService::new(root.path(), "node-pi5-warden");
    let request = request(Uuid::new_v4(), Utc::now() + Duration::minutes(5));

    let first = service
        .execute(request.clone(), Utc::now())
        .expect("first audit");
    let replay = service.execute(request, Utc::now()).expect("replay audit");

    assert!(!first.replayed);
    assert!(replay.replayed);
    assert_eq!(replay.report.audit_id, first.report.audit_id);
    assert_eq!(replay.packet_sha256, first.packet_sha256);
    let receipts =
        std::fs::read_to_string(root.path().join("data/warden/rumil_audit_receipts.jsonl"))
            .expect("receipt ledger");
    assert_eq!(receipts.lines().count(), 1);
}

#[test]
fn reused_request_id_with_changed_content_is_rejected_as_an_idempotency_conflict() {
    let root = fixture();
    let service = ScoutAuditService::new(root.path(), "node-pi5-warden");
    let request = request(Uuid::new_v4(), Utc::now() + Duration::minutes(5));
    service
        .execute(request.clone(), Utc::now())
        .expect("first audit");
    let mut conflict = request;
    conflict.request.byte_budget += 1;

    let error = service
        .execute(conflict, Utc::now())
        .expect_err("idempotency conflict");
    assert!(error.to_string().contains("different audit request"));
}

#[test]
fn bounded_scan_reports_partial_completeness_instead_of_overclaiming() {
    let root = fixture();
    let service = ScoutAuditService::new(root.path(), "node-pi5-warden");
    let mut request = request(Uuid::new_v4(), Utc::now() + Duration::minutes(5));
    request.request.file_count_budget = 1;

    let outcome = service.execute(request, Utc::now()).expect("partial audit");

    assert_eq!(
        outcome.report.completeness,
        AuditReportCompleteness::Partial
    );
    assert!(!outcome.report.truncation.is_empty());
}

#[test]
fn expired_or_non_advisory_requests_are_rejected_before_audit_files_exist() {
    let root = fixture();
    let service = ScoutAuditService::new(root.path(), "node-pi5-warden");
    let expired = request(Uuid::new_v4(), Utc::now() - Duration::seconds(1));
    assert!(service.execute(expired, Utc::now()).is_err());

    let mut elevated = request(Uuid::new_v4(), Utc::now() + Duration::minutes(5));
    elevated.request.authority = "execute".to_string();
    assert!(service.execute(elevated, Utc::now()).is_err());
    assert!(!root.path().join("data/warden/rumil_audits").exists());
}

#[test]
fn followup_reads_only_the_stored_packet_by_audit_id() {
    let root = fixture();
    let service = ScoutAuditService::new(root.path(), "node-pi5-warden");
    let outcome = service
        .execute(
            request(Uuid::new_v4(), Utc::now() + Duration::minutes(5)),
            Utc::now(),
        )
        .expect("audit");

    let followup = service
        .followup(AuditFollowupRequest {
            audit_id: outcome.report.audit_id,
            sections: vec![
                AuditFollowupSection::Summary,
                AuditFollowupSection::FileRecords,
            ],
            path_prefix: Some("src".to_string()),
            file_record_limit: 10,
        })
        .expect("follow-up");

    assert_eq!(followup.audit_id, outcome.report.audit_id);
    assert!(followup.summary.is_some());
    assert_eq!(followup.file_records.len(), 2);
    assert!(followup
        .file_records
        .iter()
        .all(|record| record.path == "src" || record.path.starts_with("src/")));
}
