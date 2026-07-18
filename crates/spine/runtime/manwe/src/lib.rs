// sigil: REPAIR
//! `manwe` — the Arda inference gateway (static local root).
//!
//! Active refactor: this crate root no longer re-exports the adaptive
//! sub-tree.  The public surface is deliberately small so the rest of
//! Arda can depend on stable placeholders while the `adaptive` feature is
//! still being split out of the legacy monolith.

pub mod charon_remote;
pub mod config;
pub mod gateway;
pub mod provider;
pub mod route;
pub mod routing_adapter;

// Local transport stubs remain available for call sites that only need the
// interface shim.
pub mod transport;

// Adaptive gateway services are feature-gated so default builds don't pull in
// the full inference routing stack.
#[cfg(feature = "adaptive")]
pub mod adaptive;

#[cfg(feature = "adaptive")]
pub mod service;

pub use charon_remote::{CharonRemote, GatewayDependencyInjection, GatewayProviders};
pub use gateway::{ProviderRecord, SpannedManweGateway};
pub use provider::ProviderCatalog;
pub use route::{CharonCore, CharonGovernance, CharonMnemosyne, CharonPlutus};
#[cfg(feature = "adaptive")]
pub use service::CharonService;
pub use transport::{ApiTransport, CharonTransport, Transport};
