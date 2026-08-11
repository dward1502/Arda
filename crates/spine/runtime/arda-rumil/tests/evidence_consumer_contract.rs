use arda_rumil::{project_evidence_reference, AuditReport, Finding, RumilEvidenceClass};
use chrono::{TimeZone, Utc};
use uuid::Uuid;

fn report() -> AuditReport {
    serde_json::from_value(serde_json::json!({
        "audit_id": Uuid::from_u128(10),
        "project_id": Uuid::from_u128(20),
        "project_kind": "rust",
        "root_identity": {
            "project_id": Uuid::from_u128(20),
            "name": "fixture",
            "kind": "rust",
            "remote_url": null
        },
        "source_revision": "abc123",
        "profile_id": "generic-rust-v1",
        "generated_at_utc": "2026-08-03T00:00:00Z",
        "completed_at_utc": "2026-08-03T00:00:01Z",
        "completeness": "partial",
        "capabilities_requested": ["inventory", "cargo_audit"],
        "capabilities_completed": [{
            "capability": "inventory",
            "status": "completed",
            "provider_id": "arda-rumil.generic-inventory",
            "detail": null
        }],
        "capabilities_unavailable": [{
            "capability": "cargo_audit",
            "status": "rejected",
            "provider_id": "cargo-audit",
            "detail": "provider rejected by policy"
        }],
        "inventory_summary": {
            "total_files": 2,
            "total_directories": 1,
            "total_symlinks": 0,
            "total_bytes": 10,
            "sampled_files": 2
        },
        "tree_digest": "sha256:tree",
        "file_record_references": ["records.json"],
        "package_records": {},
        "module_records": {},
        "dependency_graph_reference": null,
        "command_receipts": ["receipts/cargo-check.json"],
        "finding_references": ["findings.json"],
        "organization_plan_reference": null,
        "comparison_reference": "comparison.json",
        "exclusions": [],
        "truncation": ["file_count_budget"],
        "warnings": ["prior baseline is stale"],
        "errors": [],
        "authority": "advisory_read_only"
    }))
    .unwrap()
}

fn finding(confidence: &str) -> Finding {
    serde_json::from_value(serde_json::json!({
        "finding_id": Uuid::new_v4(),
        "audit_id": Uuid::from_u128(10),
        "category": "fixture",
        "severity": "low",
        "status": "new",
        "confidence_class": confidence,
        "path_or_scope": ".",
        "summary": "bounded fixture finding",
        "recommendation": null,
        "evidence_refs": ["receipt:fixture"],
        "provider_id": null,
        "source_command_id": null,
        "prior_finding_id": null,
        "review_required": true,
        "mutation_allowed": false
    }))
    .unwrap()
}

#[test]
fn projection_discloses_every_evidence_class_without_source_content() {
    let evidence = project_evidence_reference(
        &report(),
        &[
            finding("tool_backed"),
            finding("heuristic"),
            finding("historical"),
        ],
        "data/warden/rumil_audits/audit.json",
        "sha256:packet",
        true,
    )
    .unwrap();

    assert_eq!(
        evidence.classes,
        vec![
            RumilEvidenceClass::ToolBacked,
            RumilEvidenceClass::Heuristic,
            RumilEvidenceClass::Historical,
            RumilEvidenceClass::Partial,
            RumilEvidenceClass::Unavailable,
        ]
    );
    assert!(evidence.stale_baseline);
    assert_eq!(evidence.rejected_providers, vec!["cargo-audit"]);
    assert_eq!(evidence.missing_evidence, vec!["cargo_audit"]);
    assert_eq!(evidence.authority, "advisory_read_only");
    assert!(!evidence.execution_authorized);
    assert!(!serde_json::to_string(&evidence)
        .unwrap()
        .contains("bounded fixture finding"));
}

#[test]
fn projection_rejects_filesystem_and_unbound_packet_references() {
    let now = Utc.with_ymd_and_hms(2026, 8, 3, 0, 0, 0).unwrap();
    assert!(
        project_evidence_reference(&report(), &[], "/tmp/audit.json", "sha256:x", false).is_err()
    );
    assert!(project_evidence_reference(&report(), &[], "audit.json", "", false).is_err());
    assert!(now <= report().generated_at_utc);
}
