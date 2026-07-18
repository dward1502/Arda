// sigil: REPAIR
//! ATHENA transport.
//!
//! Extracted from `arda-varda/src/transport/`. It currently owns the portable
//! transport configuration and `expand_home`, while transport implementations
//! remain in `arda-varda` during the incremental split.

pub mod config;
pub mod error;
pub mod home;

pub use config::DaemonConfig;
pub use error::TransportError;
pub use home::expand_home;

#[cfg(test)]
mod tests {
    #[test]
    fn transport_crate_loads() {
        let _ = crate::expand_home("~/data/athena/athena.sock");
        let _ = crate::DaemonConfig::default();
        let _ = crate::TransportError {
            agent: "athena-transport",
            message: "verify the extracted surfaces compile".to_string(),
        };
    }
}
