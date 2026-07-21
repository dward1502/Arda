// sigil: REPAIR

pub mod config;
pub mod error;
pub mod routing_adapter;
pub mod types;

#[cfg(feature = "adaptive")]
pub mod adaptive;
pub use config::ManweConfig;
pub use types::{ManweRequestEnvelope, ModelState, ProviderState, RouteDecision};
