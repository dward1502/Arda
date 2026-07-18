#[cfg(test)]
mod canary {
    #[test]
    fn service_registry_surfaces_load() {
        let contract = arda_service_registry::ServiceContract {
            name: String::new(),
            kind: arda_service_registry::ServiceKind::Governance,
            command: String::new(),
            args: Vec::new(),
            working_directory: std::path::PathBuf::new(),
            environment: Vec::new(),
        };
        let _ = arda_service_registry::ServiceRecord::new(contract);
        let _ = arda_service_registry::ServiceStatus::Pending;
        let _ = arda_service_registry::ServiceHandle::default();
    }
}
