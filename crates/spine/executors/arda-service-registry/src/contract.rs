//! Service contracts for the registry.
//!
//! Each supported service declares a `ServiceContract` so the registry can
//! normalize inputs before delegating to a process supervisor. The contract
//! is intentionally lightweight: a structured identity plus the minimal set
//! of fission controls the daemon supports today.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Stable identifier for a registered service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceKind {
    Governance,
    Mnemosyne,
    Plutus,
    Oracle,
    Charon,
    Gateway,
}

impl std::fmt::Display for ServiceKind {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            ServiceKind::Governance => "governance",
            ServiceKind::Mnemosyne => "mnemosyne",
            ServiceKind::Plutus => "plutus",
            ServiceKind::Oracle => "oracle",
            ServiceKind::Charon => "charon",
            ServiceKind::Gateway => "gateway",
        };
        write!(fmt, "{label}")
    }
}

impl std::str::FromStr for ServiceKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "governance" => Ok(Self::Governance),
            "mnemosyne" => Ok(Self::Mnemosyne),
            "plutus" => Ok(Self::Plutus),
            "oracle" => Ok(Self::Oracle),
            "charon" => Ok(Self::Charon),
            "gateway" => Ok(Self::Gateway),
            other => Err(format!("unknown service kind: {other}")),
        }
    }
}

/// Lifecycle contract for a registered service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceContract {
    pub name: String,
    pub kind: ServiceKind,
    pub command: String,
    pub args: Vec<String>,
    pub working_directory: PathBuf,
    pub environment: Vec<(String, String)>,
}

impl ServiceContract {
    pub fn new(
        name: impl Into<String>,
        kind: ServiceKind,
        command: impl Into<String>,
        working_directory: impl Into<std::path::PathBuf>,
    ) -> Self {
        Self {
            name: name.into(),
            kind,
            command: command.into(),
            args: Vec::new(),
            working_directory: working_directory.into(),
            environment: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_roundtrip() {
        for kind in [
            ServiceKind::Governance,
            ServiceKind::Mnemosyne,
            ServiceKind::Plutus,
            ServiceKind::Oracle,
            ServiceKind::Charon,
            ServiceKind::Gateway,
        ] {
            let round = kind.to_string().parse::<ServiceKind>().unwrap();
            assert_eq!(kind, round);
        }
    }
}
