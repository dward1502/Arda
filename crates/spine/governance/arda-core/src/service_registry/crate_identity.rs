//! Local crate identity shim to avoid pulling in an external `crate_identity` crate.

use super::contract::ServiceContract;
use serde::{Deserialize, Serialize};

/// Stable identity for a crate/contract.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct CrateIdentity {
    pub name: String,
    pub version: String,
}

impl CrateIdentity {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }

    pub fn from_contract(contract: &ServiceContract) -> Self {
        Self::new(&contract.name, "0.1.0")
    }
}
