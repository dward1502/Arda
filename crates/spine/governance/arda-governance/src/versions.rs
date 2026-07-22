//! Policy/scorer semantic versions carried by governance receipts.

pub const TRIAD_POLICY_VERSION: &str = "arda.governance.triad.v2";
pub const GOVERNANCE_CHAIN_POLICY_VERSION: &str = "structured_evidence_v2";
pub const RESONANCE_POLICY_VERSION: &str = "arda.governance.resonance.v2";
pub const BACON_LITE_POLICY_VERSION: &str = "arda.governance.bacon_lite.v2";
pub const GAME_THEORY_POLICY_VERSION: &str = "capability_weighted_unit_interval_v2";
pub const LOVE_EQUATION_POLICY_VERSION: &str = "arda.governance.love_proxy.v2";
pub const JOULEWORK_POLICY_VERSION: &str = "arda.governance.joulework.v2";

pub(crate) fn legacy_triad_policy_version() -> String {
    "heuristic_local_v1".to_string()
}

pub(crate) fn legacy_resonance_policy_version() -> String {
    "ecst_compatibility_v1".to_string()
}

pub(crate) fn legacy_bacon_lite_policy_version() -> String {
    "bacon_lite_v1".to_string()
}

pub(crate) fn legacy_game_theory_policy_version() -> String {
    "capability_weighted_local_v1".to_string()
}

pub(crate) fn legacy_love_equation_policy_version() -> String {
    "love_proxy_v1".to_string()
}

pub(crate) fn legacy_joulework_policy_version() -> String {
    "joulework_v1".to_string()
}
