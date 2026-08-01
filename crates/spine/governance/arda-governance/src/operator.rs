//! Read-only operator projection joining readiness, ledger, and metrics.

use crate::{
    BaconLiteEvent, BaconLiteLedgerSummary, GovernanceMetricsSnapshot, GovernanceReadinessReport,
    GovernanceVetoReason, PhilosopherAction, PhilosopherLifecycleReceipt, TriadPhilosopherVerdict,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceDecisionConfidenceBand {
    #[default]
    NoData,
    Low,
    Medium,
    High,
}

impl GovernanceDecisionConfidenceBand {
    pub fn from_confidence(confidence: f64) -> Self {
        if !confidence.is_finite() || confidence <= 0.0 {
            Self::NoData
        } else if confidence < 0.50 {
            Self::Low
        } else if confidence < 0.75 {
            Self::Medium
        } else {
            Self::High
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::NoData => "no_data",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompactPhilosopherEvidence {
    pub action: PhilosopherAction,
    pub alignment_score: f64,
    pub reason: String,
    pub lifecycle: PhilosopherLifecycleReceipt,
}

impl From<TriadPhilosopherVerdict> for CompactPhilosopherEvidence {
    fn from(verdict: TriadPhilosopherVerdict) -> Self {
        Self {
            action: verdict.action,
            alignment_score: verdict.alignment_score,
            reason: verdict.reason,
            lifecycle: verdict.lifecycle,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GovernanceReadinessGap {
    pub subsystem: String,
    pub source_maturity: String,
    pub missing_evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GovernanceOperatorDecision {
    pub decision: String,
    pub evidence_source: String,
    pub policy_version: String,
    pub scorer_version: String,
    pub review_mode: String,
    pub source_maturity: String,
    pub reason: String,
    pub typed_veto: Option<GovernanceVetoReason>,
    pub confidence: f64,
    pub confidence_band: GovernanceDecisionConfidenceBand,
    pub philosopher_evidence: Option<CompactPhilosopherEvidence>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GovernanceStatusReport {
    pub schema_version: String,
    pub default_autonomy_ready: bool,
    pub autonomy_claim: String,
    pub latest_decision: Option<GovernanceOperatorDecision>,
    pub readiness_gaps: Vec<GovernanceReadinessGap>,
    pub readiness: GovernanceReadinessReport,
    pub recent_ledger: BaconLiteLedgerSummary,
    pub metrics: GovernanceMetricsSnapshot,
}

pub fn build_governance_status_report(
    readiness: GovernanceReadinessReport,
    recent_ledger: BaconLiteLedgerSummary,
    metrics: GovernanceMetricsSnapshot,
    latest_event: Option<BaconLiteEvent>,
) -> GovernanceStatusReport {
    let readiness_gaps = readiness
        .subsystems
        .iter()
        .filter(|subsystem| !subsystem.missing_evidence.is_empty())
        .map(|subsystem| GovernanceReadinessGap {
            subsystem: subsystem.subsystem.clone(),
            source_maturity: subsystem.maturity_label.clone(),
            missing_evidence: subsystem.missing_evidence.clone(),
        })
        .collect();
    let latest_decision = latest_event.map(|event| GovernanceOperatorDecision {
        decision: if event.passed { "pass" } else { "fail" }.to_string(),
        evidence_source: event
            .evidence_source
            .map(evidence_source_label)
            .unwrap_or("unavailable")
            .to_string(),
        policy_version: event.policy_version,
        scorer_version: event.scorer_version,
        review_mode: review_mode_label(event.review_mode).to_string(),
        source_maturity: event.source_maturity,
        reason: event.rationale,
        typed_veto: event.typed_veto,
        confidence: event.confidence,
        confidence_band: GovernanceDecisionConfidenceBand::from_confidence(event.confidence),
        philosopher_evidence: event.philosopher_evidence,
    });
    GovernanceStatusReport {
        schema_version: "arda.governance.status.v1".to_string(),
        default_autonomy_ready: readiness.default_autonomy_ready,
        autonomy_claim: if readiness.default_autonomy_ready {
            "scoped_readiness_requires_operator_review".to_string()
        } else {
            "not_autonomy_ready".to_string()
        },
        latest_decision,
        readiness_gaps,
        readiness,
        recent_ledger,
        metrics,
    }
}

pub fn render_governance_status_human(report: &GovernanceStatusReport) -> String {
    let mut output = format!(
        "Governance status: {} (default_autonomy_ready={})\n",
        report.autonomy_claim, report.default_autonomy_ready
    );
    if let Some(decision) = &report.latest_decision {
        output.push_str(&format!(
            "Latest decision: {} | evidence={} | policy={} | scorer={} | review={} | maturity={} | confidence={:.3} ({})\nReason: {}\n",
            decision.decision,
            decision.evidence_source,
            decision.policy_version,
            decision.scorer_version,
            decision.review_mode,
            decision.source_maturity,
            decision.confidence,
            decision.confidence_band.as_str(),
            decision.reason,
        ));
        if let Some(veto) = &decision.typed_veto {
            output.push_str(&format!(
                "Typed veto: {} (observed_passes={}, required_passes={})\n",
                veto.render_compatibility(),
                veto.observed_passes,
                veto.required_passes
            ));
        }
        if let Some(philosopher) = &decision.philosopher_evidence {
            output.push_str(&format!(
                "Philosopher: {:?} score={:.3} reason={} | source={} | revision={} | maturity={:?} | review={:?} | authority={}\n",
                philosopher.action,
                philosopher.alignment_score,
                philosopher.reason,
                philosopher.lifecycle.profile_source,
                philosopher.lifecycle.source_revision,
                philosopher.lifecycle.maturity,
                philosopher.lifecycle.review_mode,
                philosopher.lifecycle.review_authority,
            ));
        }
    } else {
        output.push_str("Latest decision: unavailable\n");
    }
    output.push_str(&format!(
        "Recent ledger: records={} malformed={}\nReadiness gaps: {}\n",
        report.recent_ledger.records,
        report.recent_ledger.malformed_records,
        report.readiness_gaps.len()
    ));
    for gap in &report.readiness_gaps {
        output.push_str(&format!(
            "- {} | maturity={} | missing={}\n",
            gap.subsystem,
            gap.source_maturity,
            gap.missing_evidence.join(",")
        ));
    }
    output
}

fn evidence_source_label(source: crate::GovernanceScoringSource) -> &'static str {
    match source {
        crate::GovernanceScoringSource::StructuredEvidence => "structured_evidence",
        crate::GovernanceScoringSource::LegacyResultMapping => "legacy_result_mapping",
        crate::GovernanceScoringSource::HeuristicFallback => "heuristic_fallback",
        crate::GovernanceScoringSource::MalformedStructuredFallback => {
            "malformed_structured_fallback"
        }
    }
}

fn review_mode_label(mode: crate::GovernanceReviewMode) -> &'static str {
    match mode {
        crate::GovernanceReviewMode::HeuristicLocal => "heuristic_local",
        crate::GovernanceReviewMode::IndependentAgent => "independent_agent",
        crate::GovernanceReviewMode::HumanReviewed => "human_reviewed",
        crate::GovernanceReviewMode::ConsensusReceipted => "consensus_receipted",
    }
}
