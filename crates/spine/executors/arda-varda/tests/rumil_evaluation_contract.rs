use arda_rumil::{AuditReportCompleteness, RumilEvidenceClass, RumilEvidenceReference};
use arda_varda::{evaluate_rumil_evidence, RumilEvaluationDisposition};
use chrono::Utc;
use uuid::Uuid;

fn reference(classes: Vec<RumilEvidenceClass>) -> RumilEvidenceReference {
    RumilEvidenceReference {
        audit_id: Uuid::from_u128(1),
        project_id: Uuid::from_u128(2),
        packet_reference: "data/warden/rumil_audits/a.json".to_string(),
        packet_sha256: "sha256:packet".to_string(),
        generated_at_utc: Utc::now(),
        completeness: if classes.contains(&RumilEvidenceClass::Partial) {
            AuditReportCompleteness::Partial
        } else {
            AuditReportCompleteness::Complete
        },
        classes,
        stale_baseline: false,
        rejected_providers: vec![],
        missing_evidence: vec![],
        authority: "advisory_read_only".to_string(),
        execution_authorized: false,
    }
}

#[test]
fn varda_accepts_only_complete_tool_backed_receipts_as_advisory_evidence() {
    let receipt = evaluate_rumil_evidence(&reference(vec![RumilEvidenceClass::ToolBacked]));
    assert_eq!(
        receipt.disposition,
        RumilEvaluationDisposition::AcceptedAdvisory
    );
    assert!(receipt.accepted_for_evaluation);
    assert!(!receipt.execution_authorized);
    assert_eq!(receipt.packet_sha256, "sha256:packet");
}

#[test]
fn varda_routes_partial_stale_rejected_or_missing_receipts_to_review() {
    let mut evidence = reference(vec![
        RumilEvidenceClass::ToolBacked,
        RumilEvidenceClass::Heuristic,
        RumilEvidenceClass::Historical,
        RumilEvidenceClass::Partial,
        RumilEvidenceClass::Unavailable,
    ]);
    evidence.stale_baseline = true;
    evidence.rejected_providers = vec!["cargo-audit".to_string()];
    evidence.missing_evidence = vec!["cargo_audit".to_string()];

    let receipt = evaluate_rumil_evidence(&evidence);
    assert_eq!(
        receipt.disposition,
        RumilEvaluationDisposition::ReviewRequired
    );
    assert!(!receipt.accepted_for_evaluation);
    assert!(receipt
        .review_reasons
        .contains(&"partial_coverage".to_string()));
    assert!(receipt
        .review_reasons
        .contains(&"stale_baseline".to_string()));
    assert!(receipt
        .review_reasons
        .contains(&"rejected_provider".to_string()));
    assert!(receipt
        .review_reasons
        .contains(&"missing_evidence".to_string()));
    assert!(!receipt.execution_authorized);
}
