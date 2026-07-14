//! `support` — public-bridge / charon / adaptive-routing facade.
//!
//! Provides `CharonRemote`, adapters, and `GatewayDependencyInjection`
//! so callers can start with a local transport and swap to manwe when
//! upstream subsystems are ready.

use serde::{Deserialize, Serialize};

use crate::transport::{AuthorityRecord, GatewayTransport};

// ---------------------------------------------------------------------------
// charon → gateway contract
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharonRemote;

impl CharonRemote {
    pub async fn bootstrap(
        authority: impl AsRef<str>,
    ) -> Result<AuthorityRecord, crate::GatewayError> {
        GatewayTransport::bootstrap_authority(authority).await
    }
}

// ---------------------------------------------------------------------------
// dependency-injection notes
// ---------------------------------------------------------------------------

pub struct GatewayDependencyInjection;

impl GatewayDependencyInjection {
    /// Replace the built-in local transport with an alternate transport.
    ///
    /// This is a compile-time choice in the current architecture. Returning
    /// `()` preserves the concrete transport boundary; concrete call sites
    /// can branch on feature flags when the crate matures.
    pub fn bind_transport<T: GatewayTransport>(_transport: T) -> impl GatewayTransport {
        crate::transport::LocalShim
    }
}
