//! Authority marker for outpost payloads.
//!
//! Advisory-class observations may inform council/manwe/relic rendering after
//! governance review, but can never approve, reject, or mutate work.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityClass {
    #[serde(alias = "Advisory")]
    Advisory,
    #[serde(alias = "Presentation")]
    Presentation,
    #[serde(alias = "ExecutionProhibited")]
    ExecutionProhibited,
}

impl AuthorityClass {
    /// Observation authority can inform or render state, never execute work.
    pub const fn permits_execution(self) -> bool {
        false
    }
}

impl std::fmt::Display for AuthorityClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthorityClass::Advisory => write!(f, "advisory"),
            AuthorityClass::Presentation => write!(f, "presentation"),
            AuthorityClass::ExecutionProhibited => write!(f, "execution_prohibited"),
        }
    }
}
