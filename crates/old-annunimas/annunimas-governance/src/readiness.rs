// sigil: REPAIR
//! Governance autonomy-readiness projection.
//!
//! G9 turns autonomy language into scoped, evidence-backed readiness records.
//! The default projection is intentionally conservative: it reports current
//! source metadata and runtime receipts without promoting any subsystem to full
//! autonomy readiness unless every required evidence class is present.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceReadinessLevel {
    DocumentedOnly,
    HeuristicLocal,
    SourceMetadataDisclosed,
    RuntimeReceipted,
    PolicyEnforced,
    IndependentReviewReceipted,
    AutonomyReadyForScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceReadinessEvidence {
    pub documentation: bool,
    pub local_heuristic: bool,
    pub source_metadata: bool,
    pub runtime_receipts: bool,
    pub policy_enforcement: bool,
    pub independent_review_receipts: bool,
    pub scoped_autonomy_policy: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceIndependentReviewAuthority {
    OperatorReviewed,
    ExternalAudit,
    GeneratedArtifact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceIndependentReviewVerdict {
    ApprovedForEvidence,
    NeedsFollowUp,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceIndependentReviewReceipt {
    pub receipt_id: String,
    pub subsystem: String,
    pub reviewer: String,
    pub authority: GovernanceIndependentReviewAuthority,
    pub verdict: GovernanceIndependentReviewVerdict,
    pub evidence_uri: String,
    pub independent_from_primary_heuristic: bool,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceReadinessRequirement {
    pub level: GovernanceReadinessLevel,
    pub required_evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceSubsystemReadiness {
    pub subsystem: String,
    pub current_level: GovernanceReadinessLevel,
    pub claimed_level: GovernanceReadinessLevel,
    pub evidence: GovernanceReadinessEvidence,
    pub missing_evidence: Vec<String>,
    pub receipts: Vec<String>,
    pub maturity_label: String,
    pub autonomy_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceReadinessReport {
    pub schema_version: String,
    pub contract: String,
    pub default_autonomy_ready: bool,
    pub subsystems: Vec<GovernanceSubsystemReadiness>,
    pub requirements: Vec<GovernanceReadinessRequirement>,
}

impl GovernanceReadinessEvidence {
    pub fn documented_only() -> Self {
        Self {
            documentation: true,
            local_heuristic: false,
            source_metadata: false,
            runtime_receipts: false,
            policy_enforcement: false,
            independent_review_receipts: false,
            scoped_autonomy_policy: false,
        }
    }

    pub fn heuristic_local() -> Self {
        Self {
            local_heuristic: true,
            ..Self::documented_only()
        }
    }

    pub fn source_metadata_disclosed() -> Self {
        Self {
            source_metadata: true,
            ..Self::heuristic_local()
        }
    }

    pub fn runtime_receipted() -> Self {
        Self {
            runtime_receipts: true,
            ..Self::source_metadata_disclosed()
        }
    }

    pub fn policy_enforced() -> Self {
        Self {
            policy_enforcement: true,
            ..Self::runtime_receipted()
        }
    }
}

impl GovernanceIndependentReviewReceipt {
    pub fn counts_as_independent_evidence_for(&self, subsystem: &str) -> bool {
        self.subsystem == subsystem
            && self.independent_from_primary_heuristic
            && self.verdict == GovernanceIndependentReviewVerdict::ApprovedForEvidence
            && matches!(
                self.authority,
                GovernanceIndependentReviewAuthority::OperatorReviewed
                    | GovernanceIndependentReviewAuthority::ExternalAudit
            )
            && !self.receipt_id.trim().is_empty()
            && !self.reviewer.trim().is_empty()
            && !self.evidence_uri.trim().is_empty()
    }
}

pub fn apply_independent_review_receipts(
    mut evidence: GovernanceReadinessEvidence,
    subsystem: &str,
    receipts: &[GovernanceIndependentReviewReceipt],
) -> GovernanceReadinessEvidence {
    evidence.independent_review_receipts = receipts
        .iter()
        .any(|receipt| receipt.counts_as_independent_evidence_for(subsystem));
    evidence
}

pub fn evaluate_readiness_level(
    evidence: &GovernanceReadinessEvidence,
) -> GovernanceReadinessLevel {
    if evidence.documentation
        && evidence.local_heuristic
        && evidence.source_metadata
        && evidence.runtime_receipts
        && evidence.policy_enforcement
        && evidence.independent_review_receipts
        && evidence.scoped_autonomy_policy
    {
        GovernanceReadinessLevel::AutonomyReadyForScope
    } else if evidence.documentation
        && evidence.local_heuristic
        && evidence.source_metadata
        && evidence.runtime_receipts
        && evidence.policy_enforcement
        && evidence.independent_review_receipts
    {
        GovernanceReadinessLevel::IndependentReviewReceipted
    } else if evidence.documentation
        && evidence.local_heuristic
        && evidence.source_metadata
        && evidence.runtime_receipts
        && evidence.policy_enforcement
    {
        GovernanceReadinessLevel::PolicyEnforced
    } else if evidence.documentation
        && evidence.local_heuristic
        && evidence.source_metadata
        && evidence.runtime_receipts
    {
        GovernanceReadinessLevel::RuntimeReceipted
    } else if evidence.documentation && evidence.local_heuristic && evidence.source_metadata {
        GovernanceReadinessLevel::SourceMetadataDisclosed
    } else if evidence.documentation && evidence.local_heuristic {
        GovernanceReadinessLevel::HeuristicLocal
    } else {
        GovernanceReadinessLevel::DocumentedOnly
    }
}

pub fn missing_evidence_for_level(
    evidence: &GovernanceReadinessEvidence,
    claimed_level: GovernanceReadinessLevel,
) -> Vec<String> {
    required_evidence_names(claimed_level)
        .into_iter()
        .filter(|name| !has_evidence(evidence, name))
        .map(str::to_string)
        .collect()
}

pub fn default_governance_readiness_report() -> GovernanceReadinessReport {
    governance_readiness_report_with_independent_reviews(&[])
}

pub fn governance_readiness_report_with_independent_reviews(
    independent_reviews: &[GovernanceIndependentReviewReceipt],
) -> GovernanceReadinessReport {
    let subsystems = vec![
        subsystem(
            "triad_gate",
            GovernanceReadinessEvidence::source_metadata_disclosed(),
            GovernanceReadinessLevel::AutonomyReadyForScope,
            vec![
                "crates/annunimas-governance/src/triad.rs".to_string(),
                "docs/contracts/GOVERNANCE_CURRENT_STATE.md".to_string(),
            ],
            "heuristic_local",
            independent_reviews,
        ),
        subsystem(
            "triad_dispatch_policy",
            GovernanceReadinessEvidence::policy_enforced(),
            GovernanceReadinessLevel::PolicyEnforced,
            vec![
                "crates/annunimas-core/src/governance_gates.rs".to_string(),
                "crates/annunimas-core/src/loop_engine.rs".to_string(),
            ],
            "policy_record_and_proceed_default",
            independent_reviews,
        ),
        subsystem(
            "philosopher_profiles",
            GovernanceReadinessEvidence::source_metadata_disclosed(),
            GovernanceReadinessLevel::AutonomyReadyForScope,
            vec![
                "config/governance/philosophers.toml".to_string(),
                "docs/contracts/governance-philosopher-profiles.md".to_string(),
            ],
            "draft_human_authored",
            independent_reviews,
        ),
        subsystem(
            "resonance_governance_chain",
            GovernanceReadinessEvidence::source_metadata_disclosed(),
            GovernanceReadinessLevel::RuntimeReceipted,
            vec![
                "crates/annunimas-governance/src/resonance.rs".to_string(),
                "docs/contracts/GOVERNANCE_CURRENT_STATE.md".to_string(),
            ],
            "live_source_disclosed_when_explicitly_called",
            independent_reviews,
        ),
        subsystem(
            "game_theory_selection",
            GovernanceReadinessEvidence::source_metadata_disclosed(),
            GovernanceReadinessLevel::AutonomyReadyForScope,
            vec![
                "crates/annunimas-governance/src/game_theory.rs".to_string(),
                "docs/contracts/GOVERNANCE_CURRENT_STATE.md".to_string(),
            ],
            "capability_weighted_heuristic_not_autonomous_consensus",
            independent_reviews,
        ),
        subsystem(
            "joulework_measurement",
            GovernanceReadinessEvidence::runtime_receipted(),
            GovernanceReadinessLevel::RuntimeReceipted,
            vec![
                "docs/contracts/joulework-measurement-contract.md".to_string(),
                "crates/annunimas-core/src/task.rs".to_string(),
            ],
            "source_aware_default_fallback_not_autonomy_truth",
            independent_reviews,
        ),
        subsystem(
            "love_dynamics",
            GovernanceReadinessEvidence::source_metadata_disclosed(),
            GovernanceReadinessLevel::RuntimeReceipted,
            vec![
                "crates/annunimas-governance/src/love_dynamics.rs".to_string(),
                "docs/contracts/GOVERNANCE_CURRENT_STATE.md".to_string(),
            ],
            "canonical_formula_source_disclosed",
            independent_reviews,
        ),
    ];

    GovernanceReadinessReport {
        schema_version: "annunimas.governance.readiness.v1".to_string(),
        contract: "docs/contracts/governance-autonomy-readiness.md".to_string(),
        default_autonomy_ready: subsystems.iter().all(|subsystem| subsystem.autonomy_ready),
        subsystems,
        requirements: readiness_requirements(),
    }
}

fn subsystem(
    subsystem: &str,
    evidence: GovernanceReadinessEvidence,
    claimed_level: GovernanceReadinessLevel,
    mut receipts: Vec<String>,
    maturity_label: &str,
    independent_reviews: &[GovernanceIndependentReviewReceipt],
) -> GovernanceSubsystemReadiness {
    let evidence = apply_independent_review_receipts(evidence, subsystem, independent_reviews);
    receipts.extend(
        independent_reviews
            .iter()
            .filter(|receipt| receipt.counts_as_independent_evidence_for(subsystem))
            .map(|receipt| receipt.receipt_id.clone()),
    );
    let current_level = evaluate_readiness_level(&evidence);
    let missing_evidence = missing_evidence_for_level(&evidence, claimed_level);
    GovernanceSubsystemReadiness {
        subsystem: subsystem.to_string(),
        current_level,
        claimed_level,
        evidence,
        missing_evidence,
        receipts,
        maturity_label: maturity_label.to_string(),
        autonomy_ready: current_level == GovernanceReadinessLevel::AutonomyReadyForScope,
    }
}

fn readiness_requirements() -> Vec<GovernanceReadinessRequirement> {
    [
        GovernanceReadinessLevel::DocumentedOnly,
        GovernanceReadinessLevel::HeuristicLocal,
        GovernanceReadinessLevel::SourceMetadataDisclosed,
        GovernanceReadinessLevel::RuntimeReceipted,
        GovernanceReadinessLevel::PolicyEnforced,
        GovernanceReadinessLevel::IndependentReviewReceipted,
        GovernanceReadinessLevel::AutonomyReadyForScope,
    ]
    .into_iter()
    .map(|level| GovernanceReadinessRequirement {
        level,
        required_evidence: required_evidence_names(level)
            .into_iter()
            .map(str::to_string)
            .collect(),
    })
    .collect()
}

fn required_evidence_names(level: GovernanceReadinessLevel) -> Vec<&'static str> {
    match level {
        GovernanceReadinessLevel::DocumentedOnly => vec!["documentation"],
        GovernanceReadinessLevel::HeuristicLocal => vec!["documentation", "local_heuristic"],
        GovernanceReadinessLevel::SourceMetadataDisclosed => {
            vec!["documentation", "local_heuristic", "source_metadata"]
        }
        GovernanceReadinessLevel::RuntimeReceipted => vec![
            "documentation",
            "local_heuristic",
            "source_metadata",
            "runtime_receipts",
        ],
        GovernanceReadinessLevel::PolicyEnforced => vec![
            "documentation",
            "local_heuristic",
            "source_metadata",
            "runtime_receipts",
            "policy_enforcement",
        ],
        GovernanceReadinessLevel::IndependentReviewReceipted => vec![
            "documentation",
            "local_heuristic",
            "source_metadata",
            "runtime_receipts",
            "policy_enforcement",
            "independent_review_receipts",
        ],
        GovernanceReadinessLevel::AutonomyReadyForScope => vec![
            "documentation",
            "local_heuristic",
            "source_metadata",
            "runtime_receipts",
            "policy_enforcement",
            "independent_review_receipts",
            "scoped_autonomy_policy",
        ],
    }
}

fn has_evidence(evidence: &GovernanceReadinessEvidence, name: &str) -> bool {
    match name {
        "documentation" => evidence.documentation,
        "local_heuristic" => evidence.local_heuristic,
        "source_metadata" => evidence.source_metadata,
        "runtime_receipts" => evidence.runtime_receipts,
        "policy_enforcement" => evidence.policy_enforcement,
        "independent_review_receipts" => evidence.independent_review_receipts,
        "scoped_autonomy_policy" => evidence.scoped_autonomy_policy,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readiness_downgrades_when_independent_receipts_are_missing() {
        let evidence = GovernanceReadinessEvidence::policy_enforced();
        assert_eq!(
            evaluate_readiness_level(&evidence),
            GovernanceReadinessLevel::PolicyEnforced
        );
        assert_eq!(
            missing_evidence_for_level(&evidence, GovernanceReadinessLevel::AutonomyReadyForScope),
            vec!["independent_review_receipts", "scoped_autonomy_policy"]
        );
    }

    #[test]
    fn valid_independent_review_receipt_advances_only_to_receipted_level() {
        let receipts = vec![GovernanceIndependentReviewReceipt {
            receipt_id: "review-triad-dispatch-001".to_string(),
            subsystem: "triad_dispatch_policy".to_string(),
            reviewer: "hades".to_string(),
            authority: GovernanceIndependentReviewAuthority::OperatorReviewed,
            verdict: GovernanceIndependentReviewVerdict::ApprovedForEvidence,
            evidence_uri: "audit/governance/triad-dispatch-independent-review.json".to_string(),
            independent_from_primary_heuristic: true,
            notes: "reviewed dispatch policy evidence and default non-blocking semantics"
                .to_string(),
        }];

        let evidence = apply_independent_review_receipts(
            GovernanceReadinessEvidence::policy_enforced(),
            "triad_dispatch_policy",
            &receipts,
        );

        assert!(evidence.independent_review_receipts);
        assert!(!evidence.scoped_autonomy_policy);
        assert_eq!(
            evaluate_readiness_level(&evidence),
            GovernanceReadinessLevel::IndependentReviewReceipted
        );
        assert_eq!(
            missing_evidence_for_level(&evidence, GovernanceReadinessLevel::AutonomyReadyForScope),
            vec!["scoped_autonomy_policy"]
        );
    }

    #[test]
    fn generated_or_same_path_review_receipts_do_not_count_as_independent() {
        let receipts = vec![
            GovernanceIndependentReviewReceipt {
                receipt_id: "generated-philosopher-corpus".to_string(),
                subsystem: "philosopher_profiles".to_string(),
                reviewer: "corpus_builder".to_string(),
                authority: GovernanceIndependentReviewAuthority::GeneratedArtifact,
                verdict: GovernanceIndependentReviewVerdict::ApprovedForEvidence,
                evidence_uri: "data/governance/philosopher_corpus/combined.jsonl".to_string(),
                independent_from_primary_heuristic: true,
                notes: "generated corpus is source material, not independent review".to_string(),
            },
            GovernanceIndependentReviewReceipt {
                receipt_id: "same-path-triad".to_string(),
                subsystem: "triad_dispatch_policy".to_string(),
                reviewer: "triad_gate".to_string(),
                authority: GovernanceIndependentReviewAuthority::OperatorReviewed,
                verdict: GovernanceIndependentReviewVerdict::ApprovedForEvidence,
                evidence_uri: "audit/governance/same-path.json".to_string(),
                independent_from_primary_heuristic: false,
                notes: "same heuristic path cannot review itself".to_string(),
            },
        ];

        let philosopher_evidence = apply_independent_review_receipts(
            GovernanceReadinessEvidence::source_metadata_disclosed(),
            "philosopher_profiles",
            &receipts,
        );
        let triad_evidence = apply_independent_review_receipts(
            GovernanceReadinessEvidence::policy_enforced(),
            "triad_dispatch_policy",
            &receipts,
        );

        assert!(!philosopher_evidence.independent_review_receipts);
        assert!(!triad_evidence.independent_review_receipts);
    }

    #[test]
    fn default_report_does_not_claim_autonomy_ready_by_default() {
        let report = default_governance_readiness_report();
        assert!(!report.default_autonomy_ready);
        assert!(report
            .subsystems
            .iter()
            .any(|subsystem| subsystem.subsystem == "philosopher_profiles"
                && subsystem.current_level == GovernanceReadinessLevel::SourceMetadataDisclosed
                && !subsystem.autonomy_ready));
    }

    #[test]
    fn report_applies_valid_independent_reviews_without_scoped_autonomy() {
        let report = governance_readiness_report_with_independent_reviews(&[
            GovernanceIndependentReviewReceipt {
                receipt_id: "review-triad-dispatch-001".to_string(),
                subsystem: "triad_dispatch_policy".to_string(),
                reviewer: "hades".to_string(),
                authority: GovernanceIndependentReviewAuthority::ExternalAudit,
                verdict: GovernanceIndependentReviewVerdict::ApprovedForEvidence,
                evidence_uri: "audit/governance/triad-dispatch-independent-review.json".to_string(),
                independent_from_primary_heuristic: true,
                notes: "reviewed policy enforcement evidence".to_string(),
            },
            GovernanceIndependentReviewReceipt {
                receipt_id: "generated-philosopher-corpus".to_string(),
                subsystem: "philosopher_profiles".to_string(),
                reviewer: "corpus_builder".to_string(),
                authority: GovernanceIndependentReviewAuthority::GeneratedArtifact,
                verdict: GovernanceIndependentReviewVerdict::ApprovedForEvidence,
                evidence_uri: "data/governance/philosopher_corpus/combined.jsonl".to_string(),
                independent_from_primary_heuristic: true,
                notes: "generated artifact must not count as independent review".to_string(),
            },
        ]);

        let dispatch = report
            .subsystems
            .iter()
            .find(|subsystem| subsystem.subsystem == "triad_dispatch_policy")
            .expect("triad dispatch policy readiness is reported");
        assert_eq!(
            dispatch.current_level,
            GovernanceReadinessLevel::IndependentReviewReceipted
        );
        assert!(dispatch.evidence.independent_review_receipts);
        assert!(!dispatch.evidence.scoped_autonomy_policy);
        assert!(!dispatch.autonomy_ready);
        assert!(dispatch
            .receipts
            .iter()
            .any(|receipt| receipt == "review-triad-dispatch-001"));
        assert_eq!(
            dispatch.claimed_level,
            GovernanceReadinessLevel::PolicyEnforced
        );
        assert!(dispatch.missing_evidence.is_empty());

        let philosopher_profiles = report
            .subsystems
            .iter()
            .find(|subsystem| subsystem.subsystem == "philosopher_profiles")
            .expect("philosopher profile readiness is reported");
        assert!(!philosopher_profiles.evidence.independent_review_receipts);
        assert!(!report.default_autonomy_ready);
    }
}
