//! Trait shims between the standalone gateway crate and the authority crates.
//!
//! The types are intentionally minimal so the gateway can build while the
//! real authority wiring is being ported from `old-annunimas`.
//!
//! When a shim fails at runtime, the gateway surfaces the authority gap via
//! diagnostics instead of crashing.

use std::any::Any;
use std::fmt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthorityShimError {
    #[error("not implemented in standalone gateway crate")]
    NotImplemented,
    #[error("authority unavailable: {0}")]
    Unavailable(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
}

pub type AuthorityShimResult<T> = Result<T, AuthorityShimError>;

pub trait ManweCore: Send + Sync + fmt::Debug {
    fn record_outcome(&self, _event: &dyn Any) -> AuthorityShimResult<()> {
        Err(AuthorityShimError::NotImplemented)
    }

    fn state_snapshot(&self) -> AuthorityShimResult<Vec<Box<dyn Any + Send + Sync>>> {
        Err(AuthorityShimError::NotImplemented)
    }
}

pub trait ManweGovernance: Send + Sync + fmt::Debug {
    fn validate_send(&self, _context: &dyn Any) -> AuthorityShimResult<bool> {
        Err(AuthorityShimError::NotImplemented)
    }
}

pub trait ManweMnemosyne: Send + Sync + fmt::Debug {
    fn ingest_outcome(&self, _event: &dyn Any) -> AuthorityShimResult<()> {
        Err(AuthorityShimError::NotImplemented)
    }
}

pub trait ManwePlutus: Send + Sync + fmt::Debug {
    fn meter_request(&self, _context: &dyn Any) -> AuthorityShimResult<()> {
        Err(AuthorityShimError::NotImplemented)
    }
}
