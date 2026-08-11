use arda_rumil::{
    plan_organization, InventoryReport, OrganizationIssueKind, OrganizationObservation,
    OrganizationPlanStatus, OrganizationProfile, OrganizationRisk, OrganizationRule, TreeEntry,
    TreeEntryKind,
};
use chrono::{TimeZone, Utc};
use uuid::Uuid;

fn project_id() -> Uuid {
    Uuid::parse_str("018f0000-0000-7000-8000-000000000005").unwrap()
}

fn audit_id() -> Uuid {
    Uuid::parse_str("018f0000-0000-7000-8000-000000000050").unwrap()
}

fn observed_at() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 2, 18, 0, 0).unwrap()
}

fn file(path: &str, digest: Option<&str>) -> TreeEntry {
    TreeEntry {
        relative_path: path.into(),
        kind: TreeEntryKind::File,
        size_bytes: Some(8),
        content_sha256: digest.map(str::to_string),
        mime_or_extension: None,
        executable: Some(false),
        symlink_target_relative: None,
        redaction_state: arda_rumil::RedactionState::Observed,
        observed_at_utc: observed_at(),
    }
}

fn inventory(entries: Vec<TreeEntry>) -> InventoryReport {
    InventoryReport {
        entries,
        total_bytes_seen: 0,
        truncation_reasons: Vec::new(),
        exclusion_summary: Vec::new(),
    }
}

fn profile(rules: Vec<OrganizationRule>) -> OrganizationProfile {
    OrganizationProfile {
        profile_id: "generic-project-organization-v1".into(),
        enabled_rules: rules,
        required_root_documents: vec!["README.md".into(), "INDEX.md".into()],
        allowed_output_roots: vec!["target".into(), "dist".into(), "build".into()],
    }
}

#[test]
fn missing_root_documents_are_candidates_only_when_enabled() {
    let enabled = plan_organization(
        audit_id(),
        project_id(),
        &profile(vec![OrganizationRule::MissingRootDocument]),
        &inventory(vec![file("src/lib.rs", None)]),
        &[],
        observed_at(),
    )
    .unwrap();
    assert_eq!(enabled.plan.candidates.len(), 2);
    assert_eq!(
        enabled.plan.candidates[0].candidate_type,
        "missing_root_document"
    );
    assert_eq!(enabled.plan.candidates[0].risk, OrganizationRisk::Low);

    let disabled = plan_organization(
        audit_id(),
        project_id(),
        &profile(Vec::new()),
        &inventory(vec![file("src/lib.rs", None)]),
        &[],
        observed_at(),
    )
    .unwrap();
    assert!(disabled.plan.candidates.is_empty());
}

#[test]
fn duplicate_case_folded_paths_are_deterministic_and_review_only() {
    let bundle = plan_organization(
        audit_id(),
        project_id(),
        &profile(vec![OrganizationRule::DuplicatePath]),
        &inventory(vec![
            file("Docs/Guide.md", None),
            file("docs/guide.md", None),
        ]),
        &[],
        observed_at(),
    )
    .unwrap();

    assert_eq!(bundle.plan.candidates.len(), 1);
    assert_eq!(
        bundle.plan.candidates[0].affected_paths,
        vec!["Docs/Guide.md", "docs/guide.md"]
    );
    assert!(bundle.plan.no_delete);
    assert!(bundle.plan.no_move);
    assert!(bundle.plan.no_rewrite);
    assert!(bundle.plan.operator_review_required);
    assert!(!bundle.plan.mutation_authorized);
    assert_eq!(bundle.plan.status, OrganizationPlanStatus::Proposed);
}

#[test]
fn observed_checks_are_project_neutral_and_profile_gated() {
    let observations = vec![
        OrganizationObservation {
            kind: OrganizationIssueKind::StaleGeneratedArtifact,
            path: "generated/schema.json".into(),
            evidence_refs: vec!["receipt:mtime".into()],
            affected_paths: vec!["generated/schema.json".into()],
            detail: "older than the configured generation baseline".into(),
        },
        OrganizationObservation {
            kind: OrganizationIssueKind::MisplacedOutput,
            path: "src/report.json".into(),
            evidence_refs: vec!["receipt:path-policy".into()],
            affected_paths: vec!["src/report.json".into()],
            detail: "output is outside an allowed output root".into(),
        },
        OrganizationObservation {
            kind: OrganizationIssueKind::DocumentationDrift,
            path: "README.md".into(),
            evidence_refs: vec!["receipt:docs-check".into()],
            affected_paths: vec!["README.md", "Cargo.toml"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            detail: "documented package name differs from metadata".into(),
        },
    ];
    let profile = profile(vec![
        OrganizationRule::StaleGeneratedArtifact,
        OrganizationRule::DocumentationDrift,
    ]);
    let bundle = plan_organization(
        audit_id(),
        project_id(),
        &profile,
        &inventory(vec![file("README.md", None)]),
        &observations,
        observed_at(),
    )
    .unwrap();

    let kinds: Vec<_> = bundle
        .plan
        .candidates
        .iter()
        .map(|candidate| candidate.candidate_type.as_str())
        .collect();
    assert_eq!(
        kinds,
        vec!["documentation_drift", "stale_generated_artifact"]
    );
    assert!(bundle
        .plan
        .candidates
        .iter()
        .all(|candidate| !candidate.evidence_refs.is_empty()));
}

#[test]
fn empty_plan_still_emits_a_deterministic_dry_run_receipt() {
    let profile = profile(Vec::new());
    let first = plan_organization(
        audit_id(),
        project_id(),
        &profile,
        &inventory(vec![file("README.md", None)]),
        &[],
        observed_at(),
    )
    .unwrap();
    let second = plan_organization(
        audit_id(),
        project_id(),
        &profile,
        &inventory(vec![file("README.md", None)]),
        &[],
        observed_at(),
    )
    .unwrap();

    assert!(first.plan.candidates.is_empty());
    assert_eq!(first.receipt.candidate_count, 0);
    assert_eq!(first.receipt.receipt_id, second.receipt.receipt_id);
    assert!(first.receipt.dry_run);
    assert!(!first.receipt.filesystem_mutated);
    assert_eq!(first.receipt.authority, "review_only");
}

#[test]
fn mutation_handoff_is_declared_but_not_implemented() {
    let bundle = plan_organization(
        audit_id(),
        project_id(),
        &profile(Vec::new()),
        &inventory(Vec::new()),
        &[],
        observed_at(),
    )
    .unwrap();

    assert!(!bundle.mutation_handoff.implemented);
    assert!(!bundle.mutation_handoff.direct_mutation_available);
    assert!(bundle.mutation_handoff.operator_approval_required);
    assert!(bundle.mutation_handoff.exact_path_and_digest_required);
    assert_eq!(
        bundle.mutation_handoff.governance_contract,
        "external_operator_governance_receipt"
    );
}

#[test]
fn arda_rust_and_non_rust_inventories_use_the_same_planner() {
    let profile = profile(vec![OrganizationRule::MissingRootDocument]);
    let arda = plan_organization(
        audit_id(),
        project_id(),
        &profile,
        &inventory(vec![
            file("Cargo.toml", None),
            file("README.md", None),
            file("INDEX.md", None),
            file("crates/spine/runtime/arda-rumil/src/lib.rs", None),
        ]),
        &[],
        observed_at(),
    )
    .unwrap();
    let rust = plan_organization(
        audit_id(),
        project_id(),
        &profile,
        &inventory(vec![file("Cargo.toml", None), file("src/lib.rs", None)]),
        &[],
        observed_at(),
    )
    .unwrap();
    let non_rust = plan_organization(
        audit_id(),
        project_id(),
        &profile,
        &inventory(vec![file("package.json", None), file("src/index.js", None)]),
        &[],
        observed_at(),
    )
    .unwrap();

    assert!(arda.plan.candidates.is_empty());
    assert_eq!(rust.plan.candidates, non_rust.plan.candidates);
    assert!(rust
        .plan
        .candidates
        .iter()
        .all(|candidate| !candidate.recommended_action.contains("Soterion")));
}

#[cfg(feature = "crypto")]
#[test]
fn legacy_hades_report_import_is_historical_only() {
    let imported = arda_rumil::import_legacy_hades_organization_report(
        "data/hades/organization_plan_last.json",
        br#"{"no_delete":true}"#,
        observed_at(),
        project_id(),
        audit_id(),
        "metadata_only",
    )
    .unwrap();

    assert!(imported.historical_only);
    assert!(!imported.native_rumil_evidence);
    assert_eq!(imported.legacy_contract, "hades.organization-plan.legacy");
    assert_eq!(imported.mapping_quality, "metadata_only");
    assert_eq!(imported.legacy_sha256.len(), 64);
}
