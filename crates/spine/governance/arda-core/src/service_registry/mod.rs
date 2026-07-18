//! Foundational service registry / contract / crate_identity.

pub mod contract;
pub mod crate_identity;
pub mod registry;
pub mod service;
pub mod test_support;

pub use contract::{ServiceContract, ServiceKind, ServiceSchemaVersion};
pub use crate_identity::CrateIdentity;
pub use registry::{RegistryError, ServiceRegistry};
pub use service::{
    ArdaServiceRegistryStatus, ContinuityConfig, ContractConfig, GovernanceConfig, ServiceHandle,
    ServiceRecord, ServiceRegistryState, ServiceRegistryStateValidator, ServiceStatus,
};
