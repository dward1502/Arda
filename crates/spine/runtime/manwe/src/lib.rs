//! `manwe` — the Arda inference gateway (static local root).
//!
//! Frozen contract (REFACTOR_PLAN §0/§2): the binary listens on
//! `127.0.0.1:7171` and serves OpenAI-compatible chat completions against a
//! static provider catalog. This crate root exposes the small public surface
//! other Arda crates (e.g. `arda-engine`) can depend on without binding to any
//! single inference SDK: the `Charon*` trait shims, the provider catalog, and
//! the charon→gateway bridge types.

pub mod charon_remote;
pub mod config;
pub mod gateway;
pub mod provider;
pub mod route;
pub mod routing_adapter;
pub mod transport;

pub use charon_remote::{CharonRemote, GatewayDependencyInjection, GatewayProviders};
pub use gateway::{ProviderRecord, SpannedManweGateway};
pub use provider::ProviderCatalog;
pub use route::{CharonCore, CharonGovernance, CharonMnemosyne, CharonPlutus};
pub use routing_adapter::AdaptiveRoutingAdapter;
pub use transport::{ApiTransport, CharonTransport, Transport};
