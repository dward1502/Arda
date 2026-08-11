use arda_core::external_capability::ExternalCapability;
use arda_core::service_registry::{CapabilityHealth, CapabilityRegistry};
use arda_engine::adapters::{AdapterCatalog, AdapterKind};
use chrono::{Duration, TimeZone, Utc};

fn contract() -> ExternalCapability {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../config/adapters/hermes-workbench.external-capability.json");
    ExternalCapability::from_json_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn catalog_health_is_deterministic_stale_and_removable() {
    let mut catalog = AdapterCatalog::new();
    catalog
        .register(
            AdapterKind::External,
            contract(),
            true,
            CapabilityHealth::Unavailable,
            false,
        )
        .unwrap();
    let now = Utc.with_ymd_and_hms(2026, 8, 9, 12, 0, 0).unwrap();

    assert_eq!(
        catalog
            .observe_health("hermes-workbench", true, now)
            .unwrap(),
        CapabilityHealth::Ready
    );
    catalog.set_eligible("hermes-workbench", true).unwrap();
    catalog.refresh_staleness(now + Duration::days(2));
    let stale = catalog.get("hermes-workbench").unwrap();
    assert_eq!(stale.health, CapabilityHealth::Stale);
    assert!(!stale.eligible);

    catalog.remove("hermes-workbench").unwrap();
    let removed = catalog.get("hermes-workbench").unwrap();
    assert!(!removed.installed);
    assert_eq!(removed.health, CapabilityHealth::NotConfigured);
    assert!(!removed.eligible);
}

#[test]
fn all_adapter_kinds_project_provenance_without_duplicating_authority() {
    let mut catalog = AdapterCatalog::new();
    for (index, kind) in [
        AdapterKind::BuiltIn,
        AdapterKind::External,
        AdapterKind::Sidecar,
    ]
    .into_iter()
    .enumerate()
    {
        let mut candidate = contract();
        candidate.identity.adapter_id = format!("fixture-{index}");
        candidate.identity.candidate_id = format!("candidate-{index}");
        candidate.identity.version = format!("{index}");
        candidate.capabilities = [(
            format!("fixture.capability.{index}"),
            arda_core::external_capability::ExternalCapabilityMaturity::Preview,
        )]
        .into_iter()
        .collect();
        catalog
            .register(kind, candidate, true, CapabilityHealth::Ready, true)
            .unwrap();
    }

    let mut capabilities = CapabilityRegistry::new();
    catalog.register_capabilities(&mut capabilities).unwrap();
    assert_eq!(capabilities.records().count(), 3);
    assert!(capabilities.records().all(|record| {
        record.declaration.owner == "arda-engine"
            && !record.runtime.selected
            && record.runtime.eligible
    }));
}
