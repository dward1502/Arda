// sigil: REPAIR
//! Resonance Calculation
//!
//! Task resonance scoring based on ECST components.
//! Phi harmonic measures golden ratio alignment across three dimensions.

use crate::audio::AudioGovernance;
use crate::triad::{GateOutcome, GovernanceChainResult, TriadResult};
use crate::triad_philosopher::{
    derive_alignment_signals, interpret_alignment, PhilosopherAction, TriadPhilosopherVerdict,
};
use crate::vision::VisionGovernance;
use crate::{evaluate_love_dynamics, profile_joulework, LoveDynamicsInput};
use arda_core::task::{Task, TaskStatus};
use serde::{Deserialize, Serialize};

/// Planned removal release for compatibility APIs that synthesize Triad purity.
pub const COMPATIBILITY_RESONANCE_REMOVAL_VERSION: &str = "0.3.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriadPuritySource {
    LiveTriad,
    LiveGovernanceChain,
    Absent,
    CompatibilityDefault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhiHarmonicSource {
    GoldenRatioTaskFlowHeuristic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhiHarmonicSemantic {
    NotEmpiricalAutonomyProof,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PhiHarmonicComponents {
    pub composite_score: f64,
    pub source: PhiHarmonicSource,
    pub semantic: PhiHarmonicSemantic,
    pub time_score: f64,
    pub resource_score: f64,
    pub question_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonanceComponents {
    pub time_harmony: f64,
    pub status_coherence: f64,
    pub triad_purity: f64,
    /// Source of the Triad purity component so callers can distinguish live
    /// Triad scoring from compatibility defaults or absent data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triad_purity_source: Option<TriadPuritySource>,
    pub joule_balance: f64,
    pub phi_harmonic: f64,
    /// Source label for the phi harmonic component.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phi_harmonic_source: Option<PhiHarmonicSource>,
    /// Semantic boundary for the phi harmonic component.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phi_harmonic_semantic: Option<PhiHarmonicSemantic>,
    /// Planning:execution ratio phi score (0-100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phi_time_score: Option<f64>,
    /// Estimated:actual Joule ratio phi score (0-100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phi_resource_score: Option<f64>,
    /// Clarifications requested:resolved ratio phi score (0-100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phi_question_score: Option<f64>,
    pub freq_harmony: f64,
    /// Audio environmental coherence (0-100), if available
    pub audio_coherence: Option<f64>,
    /// Vision perceptual coherence (0-100), if available
    pub vision_coherence: Option<f64>,
    /// Love Dynamics projected empathy (0-1) derived from current task signals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub love_projected_empathy: Option<f64>,
    /// Love Dynamics empathy delta derived from `dE/dt = beta * (C - D) * E`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub love_delta_empathy: Option<f64>,
    /// Triad Philosopher action derived from the alignment signals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub philosopher_action: Option<PhilosopherAction>,
    /// Triad Philosopher normalized alignment score (0-1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub philosopher_alignment_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonanceScore {
    pub value: f64,
    pub ecst_components: Option<ResonanceComponents>,
    /// Deterministic reflective arbitration over evidence, independence,
    /// Love Dynamics, and JouleWork.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triad_philosopher: Option<TriadPhilosopherVerdict>,
}

/// Golden ratio constants
const PHI: f64 = 1.618033988749895;
const PHI_INV: f64 = 0.618033988749895;

/// Phi harmonic scorer using Gaussian falloff
/// Perfect phi alignment = 100, deviation costs score
fn phi_score(actual: f64, target: f64) -> f64 {
    if target == 0.0 {
        return 50.0;
    }
    let deviation = (actual - target).abs() / target;
    (100.0 * (-deviation * deviation * 3.0).exp()).min(100.0)
}

/// Calculate phi harmonic across three dimensions:
/// 1. TIME RATIO - planning:execution should approach phi_inverse (0.618)
/// 2. RESOURCE RATIO - estimated:actual joule cost should approach phi (1.618)
/// 3. QUESTION RATIO - questions_asked:answers should approach phi_inverse (0.618)
///
/// This is a golden-ratio task-flow heuristic for dashboard/operator evidence,
/// not empirical proof of autonomy quality or safety.
pub fn phi_harmonic_components(task: &Task) -> PhiHarmonicComponents {
    // 1. TIME RATIO - planning:execution
    let time_score = {
        let plan = task.planning_duration_secs();
        let exec = task.execution_duration_secs();
        if exec > 0.0 {
            phi_score(plan / exec, PHI_INV)
        } else {
            50.0 // incomplete task, neutral
        }
    };

    // 2. RESOURCE RATIO - estimated:actual joule cost
    let resource_score = {
        let estimated = task.joule_cost_estimated;
        let actual = task.joule_cost_actual;
        if actual > 0.0 && estimated > 0.0 {
            phi_score(estimated / actual, PHI)
        } else {
            50.0
        }
    };

    // 3. QUESTION RATIO - clarifications requested:resolved
    let question_score = {
        let asked = task.clarifications_requested as f64;
        let answered = task.clarifications_resolved as f64;
        if answered > 0.0 {
            phi_score(asked / answered, PHI_INV)
        } else if asked == 0.0 {
            75.0 // no questions needed, reasonably good
        } else {
            25.0 // asked questions, none resolved
        }
    };

    let composite_score = time_score * 0.30 + resource_score * 0.45 + question_score * 0.25;

    PhiHarmonicComponents {
        composite_score,
        source: PhiHarmonicSource::GoldenRatioTaskFlowHeuristic,
        semantic: PhiHarmonicSemantic::NotEmpiricalAutonomyProof,
        time_score,
        resource_score,
        question_score,
    }
}

/// Calculate phi harmonic composite score (0-100).
pub fn phi_harmonic(task: &Task) -> f64 {
    phi_harmonic_components(task).composite_score
}

/// Calculate joule balance - efficiency of estimated vs actual
pub fn joule_balance(task: &Task) -> f64 {
    let estimated = task.joule_cost_estimated;
    let actual = task.joule_cost_actual;

    if actual == 0.0 || estimated == 0.0 {
        return 50.0; // neutral if no data
    }

    // Efficiency = how close estimated was to actual
    // Perfect = 1.0, higher or lower both cost points
    let ratio = estimated / actual;
    if (ratio - 1.0).abs() < 0.1 {
        100.0 // within 10%
    } else if (ratio - 1.0).abs() < 0.25 {
        80.0 // within 25%
    } else if (ratio - 1.0).abs() < 0.5 {
        60.0 // within 50%
    } else {
        40.0 // off by more than 50%
    }
}

/// Calculate task resonance score (0-100) using the legacy synthetic Triad signal.
///
/// When `audio` or `vision` governance signals are provided, they contribute
/// to the overall score as additional coherence dimensions (5% weight each),
/// and the base weights are normalized to sum to the remaining budget.
#[deprecated(
    since = "0.1.0",
    note = "synthetic Triad purity is removed in 0.3.0; evaluate once and call calculate_resonance_with_triad or calculate_resonance_with_governance_chain"
)]
pub fn calculate_resonance(
    task: &Task,
    audio: Option<&AudioGovernance>,
    vision: Option<&VisionGovernance>,
) -> ResonanceScore {
    calculate_resonance_with_triad_signal(
        task,
        TriadPuritySignal::compatibility_default(),
        audio,
        vision,
    )
}

/// Calculate resonance using a live default-Triad result while preserving the
/// legacy `calculate_resonance` call shape for callers that have no live Triad
/// evidence yet.
pub fn calculate_resonance_with_triad(
    task: &Task,
    triad: &TriadResult,
    audio: Option<&AudioGovernance>,
    vision: Option<&VisionGovernance>,
) -> ResonanceScore {
    calculate_resonance_with_triad_signal(
        task,
        TriadPuritySignal::from_triad_result(triad),
        audio,
        vision,
    )
}

/// Calculate resonance using a live configurable governance-chain result while
/// preserving all non-Triad component semantics.
///
/// ```
/// use arda_core::Task;
/// use arda_governance::{
///     calculate_resonance_with_governance_chain, evaluate_governance_chain,
///     GovernanceChainConfig, TriadPuritySource,
/// };
///
/// let task = Task::new("verify source evidence and fallback", "governance");
/// let chain = evaluate_governance_chain(&task, &GovernanceChainConfig::default_triad());
/// let score = calculate_resonance_with_governance_chain(&task, &chain, None, None);
/// assert_eq!(
///     score.ecst_components.unwrap().triad_purity_source,
///     Some(TriadPuritySource::LiveGovernanceChain)
/// );
/// ```
pub fn calculate_resonance_with_governance_chain(
    task: &Task,
    chain: &GovernanceChainResult,
    audio: Option<&AudioGovernance>,
    vision: Option<&VisionGovernance>,
) -> ResonanceScore {
    calculate_resonance_with_triad_signal(
        task,
        TriadPuritySignal::from_governance_chain(chain),
        audio,
        vision,
    )
}

/// Calculate resonance while explicitly recording that no Triad evidence exists.
///
/// This is intended for degraded paths that cannot evaluate governance. Production
/// callers should prefer a live Triad or governance-chain result.
pub fn calculate_resonance_without_governance(
    task: &Task,
    audio: Option<&AudioGovernance>,
    vision: Option<&VisionGovernance>,
) -> ResonanceScore {
    calculate_resonance_with_triad_signal(task, TriadPuritySignal::absent(), audio, vision)
}

fn calculate_resonance_with_triad_signal(
    task: &Task,
    triad_signal: TriadPuritySignal,
    audio: Option<&AudioGovernance>,
    vision: Option<&VisionGovernance>,
) -> ResonanceScore {
    // Time harmony: faster completion = higher score
    let elapsed = (task.updated_at - task.created_at).num_seconds() as f64;
    let time_harmony = (100.0 / (1.0 + elapsed / 60.0)).min(100.0);
    let freq_harmony = time_harmony; // Simplified

    // Status coherence
    let status_coherence = match task.status {
        TaskStatus::Complete => 100.0,
        TaskStatus::Running => 60.0,
        TaskStatus::Pending => 30.0,
        TaskStatus::Failed { .. } => 10.0,
        TaskStatus::Retry { .. } => 40.0,
    };

    // Phi harmonic
    let phi_components = phi_harmonic_components(task);
    let phi = phi_components.composite_score;

    // Joule balance
    let joule = joule_balance(task);

    // Audio coherence
    let audio_coherence = audio.map(|a| a.coherence_score());

    // Vision coherence
    let vision_coherence = vision.map(|v| v.coherence_score);

    // Determine weight budget: base signals always present.
    // Each optional signal that is present takes 0.05 from the base pool.
    let extra_weight = audio_coherence.as_ref().map(|_| 0.05).unwrap_or(0.0)
        + vision_coherence.as_ref().map(|_| 0.05).unwrap_or(0.0);
    let base_scale = 1.0 - extra_weight;
    let triad = triad_signal.value;

    // Overall score remains backward-compatible when no live Triad/governance
    // evidence is supplied; live evidence changes only the Triad component.
    let value = (time_harmony * 0.25 * base_scale
        + status_coherence * 0.30 * base_scale
        + triad * 0.20 * base_scale
        + joule * 0.15 * base_scale
        + phi * 0.10 * base_scale
        + audio_coherence.unwrap_or(0.0) * 0.05
        + vision_coherence.unwrap_or(0.0) * 0.05)
        .min(100.0);

    let base_components = ResonanceComponents {
        time_harmony,
        status_coherence,
        triad_purity: triad,
        triad_purity_source: Some(triad_signal.source),
        joule_balance: joule,
        phi_harmonic: phi,
        phi_harmonic_source: Some(phi_components.source),
        phi_harmonic_semantic: Some(phi_components.semantic),
        phi_time_score: Some(phi_components.time_score),
        phi_resource_score: Some(phi_components.resource_score),
        phi_question_score: Some(phi_components.question_score),
        freq_harmony,
        audio_coherence,
        vision_coherence,
        love_projected_empathy: None,
        love_delta_empathy: None,
        philosopher_action: None,
        philosopher_alignment_score: None,
    };
    let love = evaluate_love_dynamics(LoveDynamicsInput {
        empathy: status_coherence / 100.0,
        cooperation: (triad_signal.cooperation_value + joule / 100.0 + phi / 100.0) / 3.0,
        defection: 1.0 - ((joule / 100.0 + phi / 100.0) / 2.0),
        beta: 0.50,
        delta_time: 1.0,
    });
    let joule_profile = profile_joulework(task);
    let signals = derive_alignment_signals(task, &love, &joule_profile, &base_components);
    let philosopher = interpret_alignment(signals);
    let components = ResonanceComponents {
        love_projected_empathy: Some(love.projected_empathy),
        love_delta_empathy: Some(love.delta_empathy),
        philosopher_action: Some(philosopher.action),
        philosopher_alignment_score: Some(philosopher.alignment_score),
        ..base_components
    };

    ResonanceScore {
        value,
        ecst_components: Some(components),
        triad_philosopher: Some(philosopher),
    }
}

/// Backward-compatible resonance calculation without audio/vision signals.
#[deprecated(
    since = "0.1.0",
    note = "synthetic Triad purity is removed in 0.3.0; evaluate once and call calculate_resonance_with_triad or calculate_resonance_with_governance_chain"
)]
#[allow(deprecated)]
// phase1-migration: compatibility API retained only through arda-governance 0.2.x.
pub fn calculate_resonance_basic(task: &Task) -> ResonanceScore {
    calculate_resonance(task, None, None)
}

struct TriadPuritySignal {
    value: f64,
    cooperation_value: f64,
    source: TriadPuritySource,
}

impl TriadPuritySignal {
    fn absent() -> Self {
        Self {
            value: 0.0,
            cooperation_value: 0.0,
            source: TriadPuritySource::Absent,
        }
    }

    fn compatibility_default() -> Self {
        let value = triad_purity(0.7);
        Self {
            value,
            cooperation_value: value,
            source: TriadPuritySource::CompatibilityDefault,
        }
    }

    fn from_triad_result(triad: &TriadResult) -> Self {
        let passed = [triad.aurelius, triad.bacon, triad.sun_tzu]
            .into_iter()
            .filter(|outcome| matches!(outcome, GateOutcome::Pass))
            .count();
        Self::from_pass_ratio(passed, 3, TriadPuritySource::LiveTriad)
    }

    fn from_governance_chain(chain: &GovernanceChainResult) -> Self {
        let passed = chain
            .lenses
            .iter()
            .filter(|lens| matches!(lens.outcome, GateOutcome::Pass))
            .count();
        Self::from_pass_ratio(
            passed,
            chain.lenses.len(),
            TriadPuritySource::LiveGovernanceChain,
        )
    }

    fn from_pass_ratio(passed: usize, total: usize, source: TriadPuritySource) -> Self {
        let value = if total == 0 {
            0.0
        } else {
            (passed as f64 / total as f64) * 100.0
        };
        Self {
            value,
            cooperation_value: value / 100.0,
            source,
        }
    }
}

/// Placeholder triad purity - would integrate with triad.rs
fn triad_purity(default: f64) -> f64 {
    default
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;
    use arda_core::{Task, TaskStatus};

    #[test]
    fn phi_harmonic_rewards_balanced_task_flow() {
        let mut task = Task::new("ingest source because evidence matters", "ingest");
        task.planning_started_at = Some(task.created_at);
        task.execution_started_at = Some(task.created_at + chrono::TimeDelta::seconds(6));
        task.updated_at = task.created_at + chrono::TimeDelta::seconds(16);
        task.joule_cost_estimated = 1.618;
        task.joule_cost_actual = 1.0;
        task.clarifications_requested = 1;
        task.clarifications_resolved = 2;

        let score = phi_harmonic(&task);
        assert!(score > 50.0);
    }

    #[test]
    fn joule_balance_penalizes_large_mismatch() {
        let mut task = Task::new("route request", "dispatch");
        task.joule_cost_estimated = 5.0;
        task.joule_cost_actual = 15.0;

        assert_eq!(joule_balance(&task), 40.0);
    }

    #[test]
    fn phi_harmonic_discloses_neutral_incomplete_components() {
        let task = Task::new("classify local note", "governance");

        let components = phi_harmonic_components(&task);

        assert_eq!(
            components.source,
            PhiHarmonicSource::GoldenRatioTaskFlowHeuristic
        );
        assert_eq!(
            components.semantic,
            PhiHarmonicSemantic::NotEmpiricalAutonomyProof
        );
        assert_eq!(components.time_score, 50.0);
        assert_eq!(components.resource_score, 50.0);
        assert_eq!(components.question_score, 75.0);
        assert_eq!(components.composite_score, phi_harmonic(&task));
    }

    #[test]
    fn phi_harmonic_discloses_no_question_and_unresolved_question_semantics() {
        let no_questions = Task::new("classify local note", "governance");
        assert_eq!(phi_harmonic_components(&no_questions).question_score, 75.0);

        let mut unresolved = Task::new("classify ambiguous note", "governance");
        unresolved.clarifications_requested = 2;
        unresolved.clarifications_resolved = 0;
        assert_eq!(phi_harmonic_components(&unresolved).question_score, 25.0);
    }

    #[test]
    fn phi_harmonic_discloses_resource_ratio_component() {
        let mut aligned = Task::new("measure work", "governance");
        aligned.joule_cost_estimated = PHI;
        aligned.joule_cost_actual = 1.0;

        let mut misaligned = Task::new("measure work", "governance");
        misaligned.joule_cost_estimated = 5.0;
        misaligned.joule_cost_actual = 1.0;

        let aligned_resource = phi_harmonic_components(&aligned).resource_score;
        let misaligned_resource = phi_harmonic_components(&misaligned).resource_score;

        assert_eq!(aligned_resource, 100.0);
        assert!(misaligned_resource < aligned_resource);
    }

    #[test]
    fn resonance_components_include_phi_semantic_metadata() {
        let task = Task::new("show phi evidence", "governance");

        let components = calculate_resonance_basic(&task)
            .ecst_components
            .expect("resonance components should be present");

        assert_eq!(
            components.phi_harmonic_source,
            Some(PhiHarmonicSource::GoldenRatioTaskFlowHeuristic)
        );
        assert_eq!(
            components.phi_harmonic_semantic,
            Some(PhiHarmonicSemantic::NotEmpiricalAutonomyProof)
        );
        assert_eq!(components.phi_time_score, Some(50.0));
        assert_eq!(components.phi_resource_score, Some(50.0));
        assert_eq!(components.phi_question_score, Some(75.0));
    }

    #[test]
    fn calculate_resonance_returns_components() {
        let mut task = Task::new("summarize latest notes", "query");
        task.status = TaskStatus::Complete;
        task.updated_at = task.created_at + chrono::TimeDelta::seconds(30);
        task.joule_cost_estimated = 2.0;
        task.joule_cost_actual = 2.1;

        let score = calculate_resonance_basic(&task);
        assert!(score.value > 0.0);
        assert!(score.ecst_components.is_some());
    }

    #[test]
    fn resonance_components_disclose_compatibility_defaulted_triad_purity() {
        let mut task = Task::new("summarize latest notes", "query");
        task.status = TaskStatus::Complete;

        let components = calculate_resonance_basic(&task)
            .ecst_components
            .expect("resonance components should be present");

        assert_eq!(components.triad_purity, 0.7);
        assert_eq!(
            components.triad_purity_source,
            Some(TriadPuritySource::CompatibilityDefault)
        );
    }

    #[test]
    fn resonance_can_use_live_governance_chain_without_changing_compatibility_path() {
        use crate::triad::{
            GateOutcome, GovernanceChainConfig, GovernanceLensVerdict, GovernanceReviewMode,
        };

        let mut task = Task::new("summarize latest notes", "query");
        task.status = TaskStatus::Complete;

        let compatibility_score = calculate_resonance_basic(&task);
        let compatibility = compatibility_score
            .ecst_components
            .expect("compatibility components should be present");
        assert_eq!(
            compatibility.triad_purity_source,
            Some(TriadPuritySource::CompatibilityDefault)
        );

        let live_chain = GovernanceChainResult {
            chain_id: "default_triad".to_string(),
            chain_version: "heuristic_local_v1".to_string(),
            profile_source: "config/governance/philosophers.toml".to_string(),
            review_mode: GovernanceReviewMode::HeuristicLocal,
            profile_maturity: "draft_human_authored".to_string(),
            required_passes: 2,
            autonomous_blocking_enabled: false,
            passed: false,
            veto_reason: Some("AURELIUS_FAIL|BACON_FAIL".to_string()),
            lenses: vec![
                GovernanceLensVerdict {
                    lens_id: "aurelius".to_string(),
                    display_name: "Marcus Aurelius".to_string(),
                    profile_id: Some("aurelius".to_string()),
                    outcome: GateOutcome::Fail,
                    score: 0.10,
                    pass_threshold: 0.60,
                },
                GovernanceLensVerdict {
                    lens_id: "bacon".to_string(),
                    display_name: "Francis Bacon".to_string(),
                    profile_id: Some("bacon".to_string()),
                    outcome: GateOutcome::Fail,
                    score: 0.20,
                    pass_threshold: 0.50,
                },
                GovernanceLensVerdict {
                    lens_id: "sun_tzu".to_string(),
                    display_name: "Sun Tzu".to_string(),
                    profile_id: Some("sun_tzu".to_string()),
                    outcome: GateOutcome::Pass,
                    score: 0.80,
                    pass_threshold: 0.50,
                },
            ],
        };

        let live = calculate_resonance_with_governance_chain(&task, &live_chain, None, None);
        let live_components = live
            .ecst_components
            .expect("live governance components should be present");

        assert_eq!(
            live_components.triad_purity_source,
            Some(TriadPuritySource::LiveGovernanceChain)
        );
        assert!((live_components.triad_purity - (100.0 / 3.0)).abs() < 1e-9);
        assert_eq!(
            compatibility.status_coherence,
            live_components.status_coherence
        );
        assert_eq!(compatibility.joule_balance, live_components.joule_balance);
        assert!(live.value > compatibility_score.value);

        let default_chain = GovernanceChainConfig::default_triad();
        let evaluated = calculate_resonance_with_governance_chain(
            &task,
            &crate::triad::evaluate_governance_chain(&task, &default_chain),
            None,
            None,
        );
        assert_eq!(
            evaluated
                .ecst_components
                .expect("evaluated governance components should be present")
                .triad_purity_source,
            Some(TriadPuritySource::LiveGovernanceChain)
        );
    }

    #[test]
    fn resonance_live_triad_path_discloses_source_and_pass_ratio() {
        use crate::triad::{GateOutcome, GovernanceReviewMode, TriadResult};

        let mut task = Task::new("summarize latest notes", "query");
        task.status = TaskStatus::Complete;

        let compatibility_score = calculate_resonance_basic(&task);
        let compatibility = compatibility_score
            .ecst_components
            .expect("compatibility components should be present");

        let triad = TriadResult {
            chain_id: "default_triad".to_string(),
            chain_version: "heuristic_local_v1".to_string(),
            profile_source: "config/governance/philosophers.toml".to_string(),
            review_mode: GovernanceReviewMode::HeuristicLocal,
            profile_maturity: "draft_human_authored".to_string(),
            aurelius: GateOutcome::Fail,
            bacon: GateOutcome::Conditional,
            sun_tzu: GateOutcome::Pass,
            aurelius_score: 0.20,
            bacon_score: 0.42,
            sun_tzu_score: 0.80,
            passed: false,
            veto_reason: Some("AURELIUS_FAIL".to_string()),
        };

        let live_score = calculate_resonance_with_triad(&task, &triad, None, None);
        let live = live_score
            .ecst_components
            .expect("live Triad components should be present");

        assert_eq!(
            compatibility.triad_purity_source,
            Some(TriadPuritySource::CompatibilityDefault)
        );
        assert_eq!(live.triad_purity_source, Some(TriadPuritySource::LiveTriad));
        assert!((live.triad_purity - (100.0 / 3.0)).abs() < 1e-9);
        assert_eq!(compatibility.status_coherence, live.status_coherence);
        assert_eq!(compatibility.joule_balance, live.joule_balance);
        assert_eq!(compatibility.phi_harmonic, live.phi_harmonic);
        assert_ne!(compatibility.triad_purity, live.triad_purity);
    }

    #[test]
    fn live_chain_fail_conditional_and_pass_combinations_are_distinct() {
        use crate::triad::{GovernanceChainResult, GovernanceLensVerdict, GovernanceReviewMode};

        fn chain(outcomes: [GateOutcome; 3]) -> GovernanceChainResult {
            let ids = ["aurelius", "bacon", "sun_tzu"];
            let lenses = ids
                .into_iter()
                .zip(outcomes)
                .map(|(id, outcome)| GovernanceLensVerdict {
                    lens_id: id.to_string(),
                    display_name: id.to_string(),
                    profile_id: Some(id.to_string()),
                    outcome,
                    score: match outcome {
                        GateOutcome::Pass => 0.8,
                        GateOutcome::Conditional => 0.4,
                        GateOutcome::Fail => 0.2,
                    },
                    pass_threshold: 0.5,
                })
                .collect::<Vec<_>>();
            GovernanceChainResult {
                chain_id: "phase_1_fixture".to_string(),
                chain_version: "live_v1".to_string(),
                profile_source: "fixture".to_string(),
                review_mode: GovernanceReviewMode::HeuristicLocal,
                profile_maturity: "fixture".to_string(),
                required_passes: 2,
                autonomous_blocking_enabled: false,
                passed: lenses
                    .iter()
                    .filter(|lens| lens.outcome == GateOutcome::Pass)
                    .count()
                    >= 2,
                veto_reason: None,
                lenses,
            }
        }

        let mut task = Task::new("evaluate live chain evidence", "governance");
        task.status = TaskStatus::Complete;
        let failed = calculate_resonance_with_governance_chain(
            &task,
            &chain([GateOutcome::Fail, GateOutcome::Fail, GateOutcome::Fail]),
            None,
            None,
        );
        let conditional = calculate_resonance_with_governance_chain(
            &task,
            &chain([
                GateOutcome::Pass,
                GateOutcome::Conditional,
                GateOutcome::Fail,
            ]),
            None,
            None,
        );
        let passing = calculate_resonance_with_governance_chain(
            &task,
            &chain([GateOutcome::Pass, GateOutcome::Pass, GateOutcome::Pass]),
            None,
            None,
        );

        let failed_components = failed.ecst_components.expect("failed components");
        let conditional_components = conditional.ecst_components.expect("conditional components");
        let passing_components = passing.ecst_components.expect("passing components");
        assert_eq!(failed_components.triad_purity, 0.0);
        assert!((conditional_components.triad_purity - (100.0 / 3.0)).abs() < 1e-9);
        assert_eq!(passing_components.triad_purity, 100.0);
        assert!(failed.value < conditional.value);
        assert!(conditional.value < passing.value);
        assert_eq!(
            passing_components.triad_purity_source,
            Some(TriadPuritySource::LiveGovernanceChain)
        );
    }

    #[test]
    fn degraded_resonance_explicitly_serializes_absent_triad_source() {
        let task = Task::new("degraded path", "governance");
        let score = calculate_resonance_without_governance(&task, None, None);
        let encoded = serde_json::to_value(score).expect("serialize resonance");

        assert_eq!(encoded["ecst_components"]["triad_purity_source"], "absent");
        assert_eq!(encoded["ecst_components"]["triad_purity"], 0.0);
    }

    #[test]
    fn calculate_resonance_with_audio_and_vision() {
        use crate::audio::AudioGovernance;
        use crate::vision::{VisionGovernance, VisionSignal};

        let mut task = Task::new("render asset", "forge");
        task.status = TaskStatus::Complete;
        task.updated_at = task.created_at + chrono::TimeDelta::seconds(30);
        task.joule_cost_estimated = 2.0;
        task.joule_cost_actual = 2.1;

        let audio = AudioGovernance::default();
        let vision = VisionGovernance::assess(vec![VisionSignal {
            iteration: 1,
            match_score: 0.85,
            score_delta: 0.1,
            missing: vec![],
            wrong: vec![],
            strengths: vec!["clean edges".into()],
        }]);

        let score = calculate_resonance(&task, Some(&audio), Some(&vision));
        assert!(score.value > 0.0);
        let components = score
            .ecst_components
            .expect("resonance components should be present");
        assert!(components.audio_coherence.is_some());
        assert!(components.vision_coherence.is_some());
    }
}
