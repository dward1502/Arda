// sigil: REPAIR
//! Annunimas Governance Module
//!
//! Governance, Triad validation, and resonance scoring.

pub mod audio;
pub mod bacon_lite;
pub mod game_theory;
pub mod joulework;
pub mod love_dynamics;
pub mod love_equation;
pub mod philosopher_profiles;
pub mod readiness;
pub mod resonance;
pub mod solar;
pub mod triad;
pub mod triad_philosopher;
pub mod vision;

pub use audio::{capture_audio_governance, AudioEnvironment, AudioGovernance};
pub use bacon_lite::{bacon_lite_validate, record_bacon_lite, BaconLiteEvent, BaconLiteResult};
pub use game_theory::{
    game_theory_score, GameTheory, GameTheoryPolicy, GameTheorySelectionKind,
    GameTheorySelectionResult,
};
pub use joulework::{profile_joulework, JouleWorkProfile};
pub use love_dynamics::{
    evaluate_love_dynamics, LoveDynamicsInput, LoveDynamicsScore, LoveDynamicsTrend,
};
pub use love_equation::{love_equation_score, LoveEquationScore};
pub use philosopher_profiles::{
    load_philosopher_profiles, load_philosopher_profiles_from_str, PhilosopherProfile,
    PhilosopherProfileError, PhilosopherProfileMaturity, PhilosopherProfileSet,
    PhilosopherProfileStatus, PhilosopherProfileStatusProjection,
};
pub use readiness::{
    apply_independent_review_receipts, default_governance_readiness_report,
    evaluate_readiness_level, governance_readiness_report_with_independent_reviews,
    missing_evidence_for_level, GovernanceIndependentReviewAuthority,
    GovernanceIndependentReviewReceipt, GovernanceIndependentReviewVerdict,
    GovernanceReadinessEvidence, GovernanceReadinessLevel, GovernanceReadinessReport,
    GovernanceReadinessRequirement, GovernanceSubsystemReadiness,
};
pub use resonance::{
    calculate_resonance, calculate_resonance_basic, calculate_resonance_with_governance_chain,
    calculate_resonance_with_triad, ResonanceComponents, ResonanceScore, TriadPuritySource,
};
pub use triad::{
    evaluate_governance_chain, load_governance_chain, load_governance_chain_from_str,
    triad_validate, GateOutcome, GovernanceChainConfig, GovernanceChainError,
    GovernanceChainResult, GovernanceLensConfig, GovernanceLensVerdict, GovernanceReviewMode,
    LiveTriad, TriadConfig, TriadResult,
};
pub use triad_philosopher::{
    derive_alignment_signals, interpret_alignment, AlignmentSignals, PhilosopherAction,
    TriadPhilosopherVerdict,
};
pub use vision::{VisionConvergence, VisionGovernance, VisionSignal};
