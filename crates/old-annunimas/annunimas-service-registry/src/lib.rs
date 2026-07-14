// sigil: ANKH
//! Annunimas Service Registry - Blueprint Crate
//!
//! This crate provides the foundational service registry functionality for the Annunimas system.
//! It serves as a **blueprint** for service registration, discovery, and governance.
//!
//! # Overview
//!
//! The service registry is a core component of Annunimas that:
//! - Defines service contracts and governance requirements
//! - Manages service registration and dependency tracking
//! - Calculates optimal service startup order
//! - Provides status reporting and system integration points
//!
//! # Modules
//!
//! - [`contract`]: Service contract definitions and governance baselines
//! - [`registry`]: Service registry implementation and state management
//! - [`service`]: Service status and system integration definitions
//!
//! # Features
//!
//! - **Contract Validation**: Governance policy enforcement
//! - **Dependency Management**: Service dependency tracking and cycle detection
//! - **Startup Ordering**: Topological sort for deterministic service startup
//! - **Status Reporting**: System health and governance compliance status
//!
//! # Usage
//!
//! This is a **blueprint crate**. For production use, extend this blueprint in your own crate.
//! See the module documentation for details on how to extend the registry functionality.

pub mod contract;
pub mod registry;
pub mod service;

/// Returns the crate identity string
pub fn crate_identity() -> &'static str {
    "annunimas-service-registry"
}
