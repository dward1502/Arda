#![cfg(feature = "full-cli")]
// sigil: REPAIR
//! Task-type vocabulary normalization between the decomposer and
//! `core/realm/boot.toml` `[joulework.base_costs]`.
//!
//! Decomposer emits semantic types (analysis, ops, build, monitor, …);
//! boot.toml uses canonical labour types (ingest, analyze, synthesize,
//! decide, communicate, monitor, archive). This is the single source of
//! truth that maps between them.

/// Canonical labour types used by `core/realm/boot.toml [joulework.base_costs]`.
pub const CANONICAL_TYPES: &[&str] = &[
    "ingest",
    "analyze",
    "synthesize",
    "decide",
    "communicate",
    "monitor",
    "archive",
];

/// Map a decomposer task_type to its canonical labour type.
/// Returns the input unchanged if it is already canonical.
pub fn canonical(decomposer_type: &str) -> &'static str {
    match decomposer_type {
        "ingest" | "research" => "ingest",
        "analyze" | "analysis" => "analyze",
        "synthesize" | "synthesis" => "synthesize",
        "decide" | "ops" | "policy" | "build" => "decide",
        "communicate" | "comms" => "communicate",
        "monitor" => "monitor",
        "archive" => "archive",
        _ => "decide",
    }
}

/// True if a task_type targets Apollo (operational executor) rather than a
/// human-driven realm. Apollo handles concrete, low-risk operational dispatch.
pub fn is_apollo_dispatchable(task_type: &str) -> bool {
    matches!(canonical(task_type), "decide" | "monitor" | "archive")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maps_decomposer_to_canonical() {
        assert_eq!(canonical("analysis"), "analyze");
        assert_eq!(canonical("synthesis"), "synthesize");
        assert_eq!(canonical("ops"), "decide");
        assert_eq!(canonical("research"), "ingest");
        assert_eq!(canonical("monitor"), "monitor");
        assert_eq!(canonical("unknown"), "decide");
    }
    #[test]
    fn flags_apollo_dispatchable() {
        assert!(is_apollo_dispatchable("ops"));
        assert!(is_apollo_dispatchable("monitor"));
        assert!(!is_apollo_dispatchable("analysis"));
    }
}
