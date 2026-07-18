//! `standard-library` facade for the service registry.
//!
//! Arda composes runtime behavior through small external crates so `engine`
//! stays lean. This module glues those facades together without depending on
//! any heavyweight runtime crate directly.

pub mod contract;
pub mod registry;
pub mod service;
pub mod test_support;

pub use contract::ServiceContract;
pub use contract::ServiceKind;
pub use registry::RegistryError;
pub use registry::ServiceRegistry;
pub use service::{ServiceHandle, ServiceRecord, ServiceStatus};
