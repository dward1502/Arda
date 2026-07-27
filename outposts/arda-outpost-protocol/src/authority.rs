//! Authority marker for outpost payloads.
//!
//! Advisory-class observations may inform council/manwe/relic rendering after
//! governance review, but can never approve, reject, or mutate work.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthorityClass {
    Advisory,
    Presentation,
    ExecutionProhibited,
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
