#[cfg(test)]
mod tests {
    use arda_service_registry::contract::{ServiceContract, ServiceKind};
    use arda_service_registry::registry::{RegistryError, ServiceRegistry};
    use arda_service_registry::service::{ServiceHandle, ServiceRecord, ServiceStatus};

    #[test]
    fn registers_a_new_contract_with_default_status() {
        let mut registry = ServiceRegistry::new();
        let contract = ServiceContract::new("gateway", ServiceKind::Gateway, "manwe", ".");
        let record = registry.upsert_contract(contract).unwrap();

        assert_eq!(record.status, ServiceStatus::Pending);
        assert_eq!(record.contract.kind, ServiceKind::Gateway);
    }

    #[test]
    fn returns_pending_after_fresh_start() {
        let contract = ServiceContract::new("harness", ServiceKind::Gateway, "arda", ".");
        let registry = ServiceRegistry::from_snapshot(vec![ServiceRecord::new(contract)]);

        let snapshot = registry.snapshot();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].status, ServiceStatus::Pending);
    }

    #[test]
    fn moves_running_once_started_by_name() {
        let mut registry = ServiceRegistry::new();
        registry.upsert_contract(ServiceContract::new("alpha", ServiceKind::Gateway, "a", ".")).unwrap();
        let record = registry.upsert_contract(ServiceContract::new("beta", ServiceKind::Gateway, "b", ".")).unwrap();

        registry.mark_started("beta", ServiceHandle::new(1)).unwrap();

        let snap = registry.snapshot();
        let beta = snap.iter().find(|r| r.contract.name == "beta").unwrap();
        assert_eq!(beta.status, ServiceStatus::Running);

        let alpha = snap.iter().find(|r| r.contract.name == "alpha").unwrap();
        assert_eq!(alpha.status, ServiceStatus::Pending);
    }

    #[test]
    fn unregister_purges_contract() {
        let mut registry = ServiceRegistry::new();
        registry.upsert_contract(ServiceContract::new("tmp", ServiceKind::Gateway, "x", ".")).unwrap();
        registry.unregister("tmp").unwrap();
        assert!(registry.get("tmp").is_none());
    }

    #[test]
    fn by_kind_filters_services() {
        let mut registry = ServiceRegistry::new();
        registry.upsert_contract(ServiceContract::new("g1", ServiceKind::Governance, "g", ".")).unwrap();
        registry.upsert_contract(ServiceContract::new("g2", ServiceKind::Governance, "g", ".")).unwrap();
        registry.upsert_contract(ServiceContract::new("m1", ServiceKind::Mnemosyne, "m", ".")).unwrap();

        assert_eq!(registry.by_kind(ServiceKind::Governance).len(), 2);
        assert_eq!(registry.by_kind(ServiceKind::Mnemosyne).len(), 1);
        assert!(registry.by_kind(ServiceKind::Oracle).is_empty());
    }
}
