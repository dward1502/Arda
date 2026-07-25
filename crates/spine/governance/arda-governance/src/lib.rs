// sigil: REPAIR
//! Stable governance primitives for Arda applications.
//!
//! The crate owns deterministic governance evaluation, profile/config parsing,
//! resonance and alignment projection, readiness reporting, and governance
//! evidence records. It does not own transport, persistence backends, process
//! orchestration, or autonomous policy enforcement.
//!
//! Filesystem access is explicit: use [`GovernancePaths`] or pass a path to a
//! loader. See the crate README for the compatibility and feature policy.

pub mod audio;
pub mod bacon_lite;
pub mod empirical_distrust;
pub mod environmental;
pub mod evidence;
pub mod game_theory;
pub mod joulework;
pub mod love_dynamics;
pub mod love_equation;
pub mod metrics;
pub mod nonconformist_bee;
pub mod normalization;
pub mod operator;
pub mod paths;
pub mod philosopher_profiles;
pub mod readiness;
pub mod realm_policy;
pub mod resonance;
pub mod scorer;
pub mod solar;
pub mod triad;
pub mod triad_philosopher;
pub mod versions;
pub mod vision;

pub use audio::{capture_audio_governance, AudioEnvironment, AudioGovernance};
pub use bacon_lite::{
    bacon_lite_validate, build_bacon_lite_event, enqueue_bacon_lite, global_bacon_lite_counters,
    read_bacon_lite_summary, read_latest_bacon_lite_event, record_bacon_lite, record_bacon_lite_to,
    BaconLiteAggregate, BaconLiteBackpressurePolicy, BaconLiteEnqueueError, BaconLiteEvent,
    BaconLiteLedgerSummary, BaconLiteLogPaths, BaconLiteReadWindow, BaconLiteResult,
    BaconLiteWriter, BaconLiteWriterConfig, BaconLiteWriterCounters, MalformedLineBehavior,
};
pub use empirical_distrust::{
    assess_empirical_distrust, EmpiricalDistrustAssessment, EmpiricalDistrustVerdict,
};
pub use environmental::{
    audio_signal, collect_environmental_signals, environmental_coherence, vision_signal,
    EnvironmentalAdvisory, EnvironmentalCoherence, GovernanceSignal, GovernanceSignalEnvelope,
    GovernanceSignalSource, MeasurementQuality, SignalFreshness, SignalHealth,
    DEFAULT_FRESHNESS_WINDOW_SECS, ENVIRONMENTAL_POLICY_VERSION,
};
pub use evidence::{
    assess_governance_evidence, GovernanceEvidence, GovernanceEvidenceAnchor,
    GovernanceEvidenceAssessment, GovernanceEvidenceContext, GovernanceEvidenceGrade,
    GovernanceScoringSource, GOVERNANCE_EVIDENCE_SCHEMA_VERSION,
};
pub use game_theory::{
    game_theory_score, GameTheory, GameTheoryConfidenceBand, GameTheoryPolicy,
    GameTheorySelectionKind, GameTheorySelectionResult,
};
pub use joulework::{profile_joulework, JouleWorkProfile};
pub use love_dynamics::{
    evaluate_love_dynamics, LoveDynamicsInput, LoveDynamicsScore, LoveDynamicsTrend,
};
#[allow(deprecated)]
pub use love_equation::{
    love_dynamics_compatibility_proxy, love_equation_score, LoveEquationScore,
};
pub use metrics::{
    global_governance_metrics, GovernanceCounterSnapshot, GovernanceHistogramBucket,
    GovernanceHistogramSnapshot, GovernanceMetrics, GovernanceMetricsSnapshot,
};
pub use nonconformist_bee::{
    assess_nonconformist_bee, NonconformistBeeAssessment, NonconformistBeeVerdict,
};
pub use normalization::{normalize_legacy_unit_or_percent, UnitInterval};
pub use operator::{
    build_governance_status_report, render_governance_status_human, CompactPhilosopherEvidence,
    GovernanceDecisionConfidenceBand, GovernanceOperatorDecision, GovernanceReadinessGap,
    GovernanceStatusReport,
};
pub use paths::GovernancePaths;
pub use philosopher_profiles::{
    load_philosopher_profiles, load_philosopher_profiles_from_str, PhilosopherLifecycleReceipt,
    PhilosopherProfile, PhilosopherProfileError, PhilosopherProfileMaturity, PhilosopherProfileSet,
    PhilosopherProfileSourceKind, PhilosopherProfileStatus, PhilosopherProfileStatusProjection,
};
pub use readiness::{
    apply_independent_review_receipts, default_governance_readiness_report,
    evaluate_readiness_level, governance_readiness_report_with_independent_reviews,
    missing_evidence_for_level, GovernanceIndependentReviewAuthority,
    GovernanceIndependentReviewReceipt, GovernanceIndependentReviewVerdict,
    GovernanceReadinessEvidence, GovernanceReadinessLevel, GovernanceReadinessReport,
    GovernanceReadinessRequirement, GovernanceSubsystemReadiness,
};
pub use realm_policy::{
    evaluate_realm_governance, load_realm_policy, load_realm_policy_from_str,
    RealmBlockingControls, RealmGovernanceVerdict, RealmPolicyConfig, RealmPolicyError,
    RealmPolicyReloadReceipt, RealmPolicyReloadStatus, RealmPolicyRule, RealmPolicyScope,
    RealmPolicyStore, ResolvedRealmPolicy, RuntimeBlockingAuthority, RuntimeBlockingDecision,
};
#[allow(deprecated)]
pub use resonance::{
    calculate_resonance, calculate_resonance_basic, calculate_resonance_with_governance_chain,
    calculate_resonance_with_triad, calculate_resonance_without_governance,
    PhilosopherInfluencePolicy, ResonanceComponents, ResonanceScore, TriadPuritySource,
    COMPATIBILITY_RESONANCE_REMOVAL_VERSION,
};
pub use scorer::{
    score_governance_with_timeout, GovernanceScoreCacheStatus, GovernanceScoreFuture,
    GovernanceScoreReceipt, GovernanceScoreRequest, GovernanceScorer, GovernanceScorerError,
    GovernanceScorerErrorKind, GovernanceScorerState, LocalGovernanceScorer,
};
#[cfg(feature = "llm-scorer")]
pub use scorer::{
    LlmGovernanceScorer, LlmGovernanceScorerConfig, LlmScoreBackend, LlmScoreBackendFuture,
    LlmScoreResponse,
};
pub use solar::{
    fetch_solar_geomag, solar_multiplier, SolarClient, SolarEndpointConfig, SolarGeomagData,
};
pub use triad::{
    evaluate_governance_chain, load_governance_chain, load_governance_chain_from_str,
    triad_validate, GateOutcome, GovernanceChainConfig, GovernanceChainError,
    GovernanceChainResult, GovernanceGateName, GovernanceLensConfig, GovernanceLensVerdict,
    GovernanceReviewMode, GovernanceVetoCode, GovernanceVetoReason, LiveTriad, TriadConfig,
    TriadResult,
};
pub use triad_philosopher::{
    derive_alignment_signals, interpret_alignment, AlignmentSignals, PhilosopherAction,
    TriadPhilosopherVerdict,
};
pub use versions::{
    BACON_LITE_POLICY_VERSION, GAME_THEORY_POLICY_VERSION, GOVERNANCE_CHAIN_POLICY_VERSION,
    JOULEWORK_POLICY_VERSION, LOVE_EQUATION_POLICY_VERSION, RESONANCE_POLICY_VERSION,
    TRIAD_POLICY_VERSION,
};
pub use vision::{VisionConvergence, VisionGovernance, VisionSignal};
