//! In-memory registry for Arda services.

use std::collections::HashMap;

use crate::service_registry::contract::{ServiceContract, ServiceKind};
use crate::service_registry::service::{ServiceHandle, ServiceRecord, ServiceStatus};

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("service {0} already registered")]
    Duplicate(String),
    #[error("service {0} not found")]
    NotFound(String),
    #[error("invalid contract for {0}: {1}")]
    Invalid(String, String),
}

#[derive(Debug, Clone, Default)]
pub struct ServiceRegistry {
    records: HashMap<String, ServiceRecord>,
}

impl ServiceRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn records(&self) -> &HashMap<String, ServiceRecord> {
        &self.records
    }

    /// Load a snapshot from an in-memory map.
    pub fn from_snapshot(records: Vec<ServiceRecord>) -> Self {
        let mut registry = Self::new();
        for record in records {
            let _ = registry.upsert_contract(record.contract.clone());
        }
        registry
    }

    /// Walk every registered service, returning a shallow copy.
    pub fn snapshot(&self) -> Vec<ServiceRecord> {
        self.records.values().cloned().collect()
    }

    /// Validate and insert a contract. Returns the new `ServiceRecord`.
    ///
    /// Rejects duplicate names with `RegistryError::Duplicate`; use
    /// `unregister` first if a contract must be re-registered.
    pub fn upsert_contract(
        &mut self,
        contract: ServiceContract,
    ) -> Result<ServiceRecord, RegistryError> {
        if contract.name.trim().is_empty() {
            return Err(RegistryError::Invalid(
                contract.name,
                "service name must not be empty".into(),
            ));
        }

        if self.records.contains_key(&contract.name) {
            return Err(RegistryError::Duplicate(contract.name));
        }

        let record = ServiceRecord::new(contract);
        self.records
            .insert(record.contract.name.clone(), record.clone());

        Ok(record)
    }

    /// Remove a contract by name.
    pub fn unregister(&mut self, name: &str) -> Result<ServiceRecord, RegistryError> {
        self.records
            .remove(name)
            .ok_or_else(|| RegistryError::NotFound(name.to_string()))
    }

    /// Mark a service handle as attached to the registry entry.
    pub fn mark_started(&mut self, name: &str, handle: ServiceHandle) -> Result<(), RegistryError> {
        let entry = self
            .records
            .get_mut(name)
            .ok_or_else(|| RegistryError::NotFound(name.to_string()))?;
        entry.status = ServiceStatus::Running;
        entry.handle = Some(handle);
        Ok(())
    }

    /// Fetch a copy of one service record.
    pub fn get(&self, name: &str) -> Option<&ServiceRecord> {
        self.records.get(name)
    }

    /// Iterate services of the requested kind.
    pub fn by_kind(&self, kind: ServiceKind) -> Vec<&ServiceRecord> {
        self.records
            .values()
            .filter(|record| record.contract.kind == kind)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_registration_rejected() {
        let contract = ServiceContract::new("alpha", ServiceKind::Gateway, "cmd", ".");
        let mut registry = ServiceRegistry::new();
        registry.upsert_contract(contract.clone()).unwrap();
        let dup = registry.upsert_contract(contract);
        assert!(matches!(dup, Err(RegistryError::Duplicate(_))));
    }

    #[test]
    fn from_snapshot_skips_duplicates() {
        let contract = ServiceContract::new("alpha", ServiceKind::Gateway, "cmd", ".");
        let registry = ServiceRegistry::from_snapshot(vec![
            ServiceRecord::new(contract.clone()),
            ServiceRecord::new(contract.clone()),
            ServiceRecord::new(contract),
        ]);
        assert_eq!(registry.snapshot().len(), 1);
    }

    #[test]
    fn snapshot_round_trip_preserves_record() {
        let contract = ServiceContract::new("alpha", ServiceKind::Gateway, "cmd", ".");
        let mut registry = ServiceRegistry::new();
        registry.upsert_contract(contract.clone()).unwrap();
        let restored = ServiceRegistry::from_snapshot(registry.snapshot());
        let record = restored.get("alpha").unwrap();
        assert_eq!(record.contract.name, "alpha");
    }

    #[test]
    fn empty_service_name_is_rejected() {
        let mut registry = ServiceRegistry::new();
        let contract = ServiceContract::new("", ServiceKind::Gateway, "cmd", ".");
        let err = registry.upsert_contract(contract).unwrap_err();
        assert!(
            matches!(err, RegistryError::Invalid(_, _)),
            "expected Invalid error, got {err:?}"
        );
    }
}
