use arda_mandos::classify_rumil_evidence;
use arda_rumil::{AuditReportCompleteness, RumilEvidenceClass, RumilEvidenceReference};
use chrono::Utc;
use uuid::Uuid;

#[test]
fn mandos_preserves_rumil_classes_and_rejects_partial_evidence_from_scoring() {
    let reference = RumilEvidenceReference {
        audit_id: Uuid::from_u128(1),
        project_id: Uuid::from_u128(2),
        packet_reference: "data/warden/rumil_audits/a.json".to_string(),
        packet_sha256: "sha256:packet".to_string(),
        generated_at_utc: Utc::now(),
        completeness: AuditReportCompleteness::Partial,
        classes: vec![
            RumilEvidenceClass::ToolBacked,
            RumilEvidenceClass::Heuristic,
            RumilEvidenceClass::Historical,
            RumilEvidenceClass::Partial,
            RumilEvidenceClass::Unavailable,
        ],
        stale_baseline: true,
        rejected_providers: vec!["cargo-audit".to_string()],
        missing_evidence: vec!["cargo_audit".to_string()],
        authority: "advisory_read_only".to_string(),
        execution_authorized: false,
    };

    let classified = classify_rumil_evidence(&reference).unwrap();
    assert_eq!(classified.classes, reference.classes);
    assert!(!classified.accepted_for_reasoning);
    assert!(classified.degraded);
    assert!(classified
        .degraded_reasons
        .contains(&"partial_coverage".to_string()));
    assert!(classified
        .degraded_reasons
        .contains(&"stale_baseline".to_string()));
    assert_eq!(classified.packet_sha256, "sha256:packet");
    assert!(!classified.execution_authorized);
}

#[test]
fn mandos_accepts_complete_tool_backed_receipt_as_advisory_reasoning_evidence() {
    let reference = RumilEvidenceReference {
        audit_id: Uuid::from_u128(3),
        project_id: Uuid::from_u128(4),
        packet_reference: "packets/a.json".to_string(),
        packet_sha256: "sha256:complete".to_string(),
        generated_at_utc: Utc::now(),
        completeness: AuditReportCompleteness::Complete,
        classes: vec![RumilEvidenceClass::ToolBacked],
        stale_baseline: false,
        rejected_providers: vec![],
        missing_evidence: vec![],
        authority: "advisory_read_only".to_string(),
        execution_authorized: false,
    };

    let classified = classify_rumil_evidence(&reference).unwrap();
    assert!(classified.accepted_for_reasoning);
    assert!(!classified.degraded);
    assert_eq!(classified.authority, "advisory_reasoning_evidence");
    assert!(!classified.execution_authorized);
}
