use arda_rumil::{
    build_memory_observation, compare_baselines, normalize_finding, AuditBaseline, AuditReport,
    AuditReportCompleteness, Comparison, ContractRootIdentity, FindingConfidenceClass,
    FindingDisposition, FindingDraft, FindingFeedback, FindingSeverity, FindingStatus,
    InventorySummary, RevisionRelation,
};
use chrono::{TimeZone, Utc};
use uuid::Uuid;

fn project_id() -> Uuid {
    Uuid::parse_str("018f0000-0000-7000-8000-000000000001").unwrap()
}

fn audit_id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

fn draft(summary: &str) -> FindingDraft {
    FindingDraft {
        category: "rust.security".into(),
        severity: FindingSeverity::High,
        status: FindingStatus::New,
        confidence_class: FindingConfidenceClass::ToolBacked,
        path_or_scope: "crates/example/Cargo.toml".into(),
        evidence_identity: "rumil.cargo_audit.v1:RUSTSEC-0001".into(),
        summary: summary.into(),
        recommendation: Some("upgrade dependency".into()),
        evidence_refs: vec!["receipt:b".into(), "receipt:a".into()],
        provider_id: Some("rumil.cargo_audit.v1".into()),
        source_command_id: Some(audit_id(99)),
        review_required: true,
    }
}

#[test]
fn stable_finding_id_ignores_provider_order_and_run_specific_command_id() {
    let first = normalize_finding(project_id(), audit_id(1), draft("advisory present")).unwrap();
    let mut reordered = draft("advisory present");
    reordered.evidence_refs.reverse();
    reordered.source_command_id = Some(audit_id(100));
    let second = normalize_finding(project_id(), audit_id(2), reordered).unwrap();

    assert_eq!(first.finding_id, second.finding_id);
    assert_eq!(first.evidence_refs, vec!["receipt:a", "receipt:b"]);
    assert_eq!(first.status, FindingStatus::New);
    assert!(!first.mutation_allowed);
}

#[test]
fn normalization_rejects_absolute_paths_and_missing_evidence_identity() {
    let mut absolute = draft("bad path");
    absolute.path_or_scope = "/home/operator/project".into();
    assert!(normalize_finding(project_id(), audit_id(1), absolute).is_err());

    let mut missing = draft("missing identity");
    missing.evidence_identity.clear();
    assert!(normalize_finding(project_id(), audit_id(1), missing).is_err());
}

#[test]
fn comparison_is_idempotent_order_independent_and_classifies_lifecycle() {
    let persistent_prior = normalize_finding(project_id(), audit_id(1), draft("same")).unwrap();
    let persistent_current = normalize_finding(project_id(), audit_id(2), draft("same")).unwrap();

    let mut changed_prior_draft = draft("old summary");
    changed_prior_draft.evidence_identity = "logical:changed".into();
    let changed_prior = normalize_finding(project_id(), audit_id(1), changed_prior_draft).unwrap();
    let mut changed_current_draft = draft("new summary");
    changed_current_draft.evidence_identity = "logical:changed".into();
    let changed_current =
        normalize_finding(project_id(), audit_id(2), changed_current_draft).unwrap();

    let mut resolved_draft = draft("gone");
    resolved_draft.evidence_identity = "logical:resolved".into();
    let resolved = normalize_finding(project_id(), audit_id(1), resolved_draft).unwrap();

    let mut new_draft = draft("new");
    new_draft.evidence_identity = "logical:new".into();
    let new_finding = normalize_finding(project_id(), audit_id(2), new_draft).unwrap();

    let prior = AuditBaseline {
        audit_id: audit_id(1),
        project_id: project_id(),
        source_revision: Some("a".into()),
        findings: vec![resolved.clone(), changed_prior, persistent_prior],
    };
    let current = AuditBaseline {
        audit_id: audit_id(2),
        project_id: project_id(),
        source_revision: Some("b".into()),
        findings: vec![
            new_finding.clone(),
            persistent_current.clone(),
            changed_current.clone(),
        ],
    };

    let first = compare_baselines(&current, &prior, RevisionRelation::Ahead).unwrap();
    let mut reversed_current = current.clone();
    reversed_current.findings.reverse();
    let mut reversed_prior = prior.clone();
    reversed_prior.findings.reverse();
    let second =
        compare_baselines(&reversed_current, &reversed_prior, RevisionRelation::Ahead).unwrap();

    assert_eq!(first, second);
    assert_eq!(first.new_findings, vec![new_finding.finding_id]);
    assert_eq!(
        first.persistent_findings,
        vec![persistent_current.finding_id]
    );
    assert_eq!(first.changed_findings, vec![changed_current.finding_id]);
    assert_eq!(first.resolved_findings, vec![resolved.finding_id]);
    assert!(first.stale_findings.is_empty());
    assert!(first.unverifiable_findings.is_empty());
}

#[test]
fn stale_and_unverifiable_findings_are_disclosed_separately() {
    let mut stale_draft = draft("stale");
    stale_draft.evidence_identity = "logical:stale".into();
    stale_draft.status = FindingStatus::Stale;
    let stale = normalize_finding(project_id(), audit_id(2), stale_draft).unwrap();

    let mut unknown_draft = draft("unknown");
    unknown_draft.evidence_identity = "logical:unknown".into();
    unknown_draft.status = FindingStatus::Unverifiable;
    unknown_draft.confidence_class = FindingConfidenceClass::Unavailable;
    let unknown = normalize_finding(project_id(), audit_id(2), unknown_draft).unwrap();

    let prior = AuditBaseline::empty(audit_id(1), project_id(), Some("a".into()));
    let current = AuditBaseline {
        audit_id: audit_id(2),
        project_id: project_id(),
        source_revision: Some("b".into()),
        findings: vec![unknown.clone(), stale.clone()],
    };
    let comparison = compare_baselines(&current, &prior, RevisionRelation::Ahead).unwrap();

    assert_eq!(comparison.stale_findings, vec![stale.finding_id]);
    assert_eq!(comparison.unverifiable_findings, vec![unknown.finding_id]);
    assert!(comparison.new_findings.is_empty());
}

#[test]
fn baseline_comparison_rejects_cross_project_history_and_duplicate_ids() {
    let prior = AuditBaseline::empty(audit_id(1), project_id(), None);
    let other = AuditBaseline::empty(audit_id(2), audit_id(999), None);
    assert!(compare_baselines(&other, &prior, RevisionRelation::Unknown).is_err());

    let finding = normalize_finding(project_id(), audit_id(2), draft("duplicate")).unwrap();
    let duplicate = AuditBaseline {
        audit_id: audit_id(2),
        project_id: project_id(),
        source_revision: None,
        findings: vec![finding.clone(), finding],
    };
    assert!(compare_baselines(&duplicate, &prior, RevisionRelation::Unknown).is_err());
}

#[test]
fn feedback_is_explicit_and_does_not_mutate_classifier_policy() {
    let feedback = FindingFeedback::new(
        audit_id(7),
        FindingDisposition::FalsePositive,
        "generated fixture is intentionally vulnerable",
        "operator",
        Utc.with_ymd_and_hms(2026, 8, 2, 12, 0, 0).unwrap(),
    )
    .unwrap();

    assert_eq!(feedback.disposition, FindingDisposition::FalsePositive);
    assert!(!feedback.mutates_classifier);
    assert!(FindingFeedback::new(
        audit_id(7),
        FindingDisposition::Accepted,
        "",
        "operator",
        Utc::now(),
    )
    .is_err());
}

fn report() -> AuditReport {
    AuditReport {
        audit_id: audit_id(2),
        project_id: project_id(),
        project_kind: "cargo_workspace".into(),
        root_identity: ContractRootIdentity {
            project_id: project_id(),
            name: "fixture".into(),
            kind: "cargo_workspace".into(),
            remote_url: None,
        },
        source_revision: Some("b".into()),
        profile_id: "generic-rust-v1".into(),
        generated_at_utc: Utc::now(),
        completed_at_utc: Some(Utc::now()),
        completeness: AuditReportCompleteness::Partial,
        capabilities_requested: vec![],
        capabilities_completed: vec![],
        capabilities_unavailable: vec![],
        inventory_summary: InventorySummary::default(),
        tree_digest: None,
        file_record_references: vec![],
        package_records: serde_json::Value::Null,
        module_records: serde_json::Value::Null,
        dependency_graph_reference: None,
        command_receipts: vec!["receipt:provider".into()],
        finding_references: vec![],
        organization_plan_reference: None,
        comparison_reference: None,
        exclusions: vec![],
        truncation: vec![],
        warnings: vec![],
        errors: vec![],
        authority: "review_only".into(),
    }
}

#[test]
fn memory_projection_contains_counts_and_receipts_but_no_raw_source() {
    let finding = normalize_finding(project_id(), audit_id(2), draft("advisory present")).unwrap();
    let comparison = Comparison {
        comparison_id: audit_id(3),
        current_audit_id: audit_id(2),
        prior_audit_id: audit_id(1),
        identity_match: true,
        revision_relation: RevisionRelation::Ahead,
        new_findings: vec![finding.finding_id],
        persistent_findings: vec![],
        changed_findings: vec![],
        resolved_findings: vec![],
        stale_findings: vec![],
        unverifiable_findings: vec![],
        baseline_warnings: vec![],
    };

    let observation = build_memory_observation(&report(), &[finding], Some(&comparison));
    assert_eq!(observation.finding_counts["high"], 1);
    assert_eq!(observation.receipt_refs, vec!["receipt:provider"]);
    assert!(!observation.summary.contains("advisory present"));
    assert_eq!(observation.comparison_digest, Some(audit_id(3).to_string()));
    assert_eq!(observation.provenance, "arda.rumil.audit-report.v1");
}
