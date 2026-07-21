//! Phase D typed surface for supervisor observability.

use std::path::PathBuf;

/// Resolve generated proto message roots for health/model and route/governance
/// surfaces.
pub fn generated_proto_roots() -> Vec<PathBuf> {
    vec![
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/grpc"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/contract"),
    ]
}
