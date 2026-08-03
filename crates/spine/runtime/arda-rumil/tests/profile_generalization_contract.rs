#[cfg(feature = "walkdir")]
use std::fs;

#[cfg(feature = "walkdir")]
use arda_rumil::audit_with_profile;
use arda_rumil::{builtin_profile, validate_execution_target, ExecutionTarget, ProjectKind};
#[cfg(feature = "crypto")]
use arda_rumil::{import_legacy_hades_findings, RumilEvidenceClass};
#[cfg(feature = "crypto")]
use chrono::{TimeZone, Utc};
#[cfg(feature = "walkdir")]
use tempfile::tempdir;
#[cfg(feature = "crypto")]
use uuid::Uuid;

#[test]
fn builtins_cover_arda_and_four_general_project_kinds() {
    for (id, kind) in [
        ("arda-v1", ProjectKind::Arda),
        ("generic-rust-v1", ProjectKind::Rust),
        ("generic-node-v1", ProjectKind::Node),
        ("generic-python-v1", ProjectKind::Python),
        ("generic-mixed-v1", ProjectKind::Mixed),
    ] {
        let profile = builtin_profile(id).expect("built-in profile");
        assert_eq!(profile.project_kind, kind);
        profile.validate().expect("safe declarative profile");
        assert!(!profile.inventory.exclusions.is_empty());
        assert!(!profile.retention.packet_class.is_empty());
        assert!(!profile.organization.mutation_authorized);
    }
}

#[test]
#[cfg(feature = "walkdir")]
fn generalized_profiles_run_bounded_inventory_without_mutation() {
    let root = tempdir().unwrap();
    fs::write(
        root.path().join("Cargo.toml"),
        "[package]\nname='fixture'\n",
    )
    .unwrap();
    fs::write(root.path().join("package.json"), "{}").unwrap();
    fs::write(
        root.path().join("pyproject.toml"),
        "[project]\nname='fixture'\n",
    )
    .unwrap();

    for id in [
        "generic-rust-v1",
        "generic-node-v1",
        "generic-python-v1",
        "generic-mixed-v1",
    ] {
        let profile = builtin_profile(id).unwrap();
        let report = audit_with_profile(root.path(), &profile).unwrap();
        assert!(!report.entries.is_empty());
        assert!(report.entries.len() as u64 <= profile.inventory.max_files);
    }
}

#[test]
fn pi_target_is_inventory_only_and_host_owns_provider_execution() {
    let rust = builtin_profile("generic-rust-v1").unwrap();
    assert!(validate_execution_target(&rust, ExecutionTarget::Host).is_ok());
    assert!(validate_execution_target(&rust, ExecutionTarget::PiCollector).is_err());

    let mut pi = rust.clone();
    pi.providers.clear();
    pi.execution_role = ExecutionTarget::PiCollector;
    assert!(validate_execution_target(&pi, ExecutionTarget::PiCollector).is_ok());
}

#[test]
#[cfg(feature = "crypto")]
fn hades_findings_import_as_historical_baseline_with_explicit_provenance() {
    let raw = serde_json::to_vec(&serde_json::json!({
        "findings": [{
            "category": "documentation",
            "severity": "low",
            "path": "README.md",
            "summary": "legacy retained finding"
        }]
    }))
    .unwrap();
    let imported = import_legacy_hades_findings(
        "data/hades/audit/summary.json",
        &raw,
        Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap(),
        Uuid::from_u128(10),
        Uuid::from_u128(20),
        "field_mapped",
    )
    .unwrap();

    assert_eq!(imported.provenance.legacy_source, "hades");
    assert!(imported.provenance.historical_only);
    assert!(!imported.provenance.native_rumil_evidence);
    assert_eq!(imported.baseline.findings.len(), 1);
    assert_eq!(
        imported.baseline.findings[0].confidence_class,
        arda_rumil::FindingConfidenceClass::Historical
    );
    assert!(imported.baseline.findings[0].review_required);
    assert!(!imported.baseline.findings[0].mutation_allowed);
    assert!(RumilEvidenceClass::Historical > RumilEvidenceClass::ToolBacked);
}
