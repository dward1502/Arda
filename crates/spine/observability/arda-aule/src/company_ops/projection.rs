use super::{CompanyOpsEvent, CompanyOpsEventKind};
use arda_core::company_ops::{
    ClientEngagement, Commitment, EngagementState, Opportunity, OutcomeKind, OutcomeReceipt,
    ProposalDraft, RevenueExperiment,
};
use serde::Serialize;
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct CompanyOpsProjection {
    pub engagements: Vec<ClientEngagement>,
    pub opportunities: Vec<Opportunity>,
    pub drafts: Vec<ProposalDraft>,
    pub commitments: Vec<Commitment>,
    pub experiments: Vec<RevenueExperiment>,
    pub outcomes: Vec<OutcomeReceipt>,
    pub scored_opportunities: Vec<ScoredOpportunity>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ValueScore {
    pub urgency: f64,
    pub expected_value: f64,
    pub operator_time_cost: f64,
    pub strategic_fit: f64,
    pub family_time_fit: f64,
    pub reversibility: f64,
    pub evidence_quality: f64,
    pub commitment_risk: f64,
    pub reviewed_outcome_signal: f64,
    pub uncertainty: f64,
    pub total: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScoredOpportunity {
    pub opportunity: Opportunity,
    pub score: ValueScore,
}

pub fn score_opportunity(
    opportunity: &Opportunity,
    now: chrono::DateTime<chrono::Utc>,
) -> ValueScore {
    score_opportunity_with_outcomes(opportunity, &[], now)
}

fn score_opportunity_with_outcomes(
    opportunity: &Opportunity,
    reviewed_outcomes: &[&OutcomeReceipt],
    now: chrono::DateTime<chrono::Utc>,
) -> ValueScore {
    let days = opportunity
        .expires_at
        .signed_duration_since(now)
        .num_hours()
        .max(0) as f64
        / 24.0;
    let urgency = (1.0 / (1.0 + days)).clamp(0.0, 1.0);
    let expected_value = (opportunity.expected_value.range.expected / 10_000.0).clamp(0.0, 1.0);
    let operator_time_cost = (opportunity.operator_time.expected_hours.expected
        / opportunity.operator_time.maximum_hours.max(f64::EPSILON))
    .clamp(0.0, 1.0);
    let strategic_fit = 0.5;
    let family_time_fit = (1.0 - operator_time_cost).clamp(0.0, 1.0);
    let reversibility = if matches!(
        opportunity.stage,
        EngagementState::Lead | EngagementState::Qualified
    ) {
        0.9
    } else {
        0.5
    };
    let reviewed_outcome_signal = if reviewed_outcomes.is_empty() {
        0.0
    } else {
        reviewed_outcomes
            .iter()
            .map(|outcome| match outcome.kind {
                OutcomeKind::Paid | OutcomeKind::Sale => 1.0,
                OutcomeKind::Delivered | OutcomeKind::Invoiced => 0.8,
                OutcomeKind::Reply | OutcomeKind::Meeting | OutcomeKind::Trial => 0.6,
                OutcomeKind::Loss => 0.0,
            })
            .sum::<f64>()
            / reviewed_outcomes.len() as f64
    };
    let evidence_quality = if reviewed_outcomes.is_empty() {
        opportunity.expected_value.range.confidence
    } else {
        let outcome_evidence_quality = reviewed_outcomes
            .iter()
            .map(|outcome| {
                if outcome.evidence.is_empty() {
                    0.8
                } else {
                    1.0
                }
            })
            .sum::<f64>()
            / reviewed_outcomes.len() as f64;
        (opportunity.expected_value.range.confidence * 0.7 + outcome_evidence_quality * 0.3)
            .clamp(0.0, 1.0)
    };
    let commitment_risk = if matches!(
        opportunity.stage,
        EngagementState::Won | EngagementState::Delivered | EngagementState::Invoiced
    ) {
        0.8
    } else {
        0.3
    };
    let uncertainty = 1.0 - evidence_quality;
    let total = urgency * 0.15
        + expected_value * 0.25
        + strategic_fit * 0.15
        + family_time_fit * 0.15
        + reversibility * 0.1
        + evidence_quality * 0.2
        + reviewed_outcome_signal * 0.1
        - commitment_risk * 0.15;
    ValueScore {
        urgency,
        expected_value,
        operator_time_cost,
        strategic_fit,
        family_time_fit,
        reversibility,
        evidence_quality,
        commitment_risk,
        reviewed_outcome_signal,
        uncertainty,
        total,
    }
}

pub fn build_projection(
    events: &[CompanyOpsEvent],
    now: chrono::DateTime<chrono::Utc>,
) -> CompanyOpsProjection {
    let mut engagements: BTreeMap<Uuid, ClientEngagement> = BTreeMap::new();
    let mut opportunities: BTreeMap<Uuid, Opportunity> = BTreeMap::new();
    let mut drafts: BTreeMap<Uuid, ProposalDraft> = BTreeMap::new();
    let mut commitments: BTreeMap<Uuid, Commitment> = BTreeMap::new();
    let mut experiments: BTreeMap<Uuid, RevenueExperiment> = BTreeMap::new();
    let mut outcomes: BTreeMap<Uuid, OutcomeReceipt> = BTreeMap::new();
    let mut ordered = events.to_vec();
    ordered.sort_by_key(|event| (event.occurred_at, event.event_id));
    for event in ordered {
        match event.kind {
            CompanyOpsEventKind::OpportunityObserved(record) => {
                opportunities.insert(record.opportunity_id, record);
            }
            CompanyOpsEventKind::ProposalDrafted(record) => {
                drafts.insert(record.proposal_id, record);
            }
            CompanyOpsEventKind::CommitmentApproved(record) => {
                commitments.insert(record.commitment_id, record);
            }
            CompanyOpsEventKind::ExperimentProposed(record) => {
                experiments.insert(record.experiment_id, record);
            }
            CompanyOpsEventKind::OutcomeRecorded(record) => {
                outcomes.insert(record.receipt_id, record);
            }
            CompanyOpsEventKind::EngagementObserved(record) => {
                engagements.insert(record.engagement_id, record);
            }
        }
    }
    let opportunities: Vec<_> = opportunities.into_values().collect();
    let outcomes: Vec<_> = outcomes.into_values().collect();
    let mut scored_opportunities: Vec<_> = opportunities
        .iter()
        .cloned()
        .map(|opportunity| ScoredOpportunity {
            score: score_opportunity_with_outcomes(
                &opportunity,
                &outcomes
                    .iter()
                    .filter(|outcome| {
                        outcome.reviewed
                            && engagements
                                .get(&outcome.engagement_id)
                                .is_some_and(|engagement| {
                                    engagement.organization_id == opportunity.organization_id
                                })
                    })
                    .collect::<Vec<_>>(),
                now,
            ),
            opportunity,
        })
        .collect();
    scored_opportunities.sort_by(|a, b| {
        b.score.total.total_cmp(&a.score.total).then_with(|| {
            a.opportunity
                .opportunity_id
                .cmp(&b.opportunity.opportunity_id)
        })
    });
    CompanyOpsProjection {
        engagements: engagements.into_values().collect(),
        opportunities,
        drafts: drafts.into_values().collect(),
        commitments: commitments.into_values().collect(),
        experiments: experiments.into_values().collect(),
        outcomes,
        scored_opportunities,
    }
}

impl CompanyOpsProjection {
    pub fn write_canonical(&self, root: &std::path::Path) -> std::io::Result<()> {
        let directory = root.join("data/business");
        std::fs::create_dir_all(&directory)?;
        for (name, value) in [
            (
                "opportunities.json",
                serde_json::to_value(&self.opportunities).unwrap(),
            ),
            ("drafts.json", serde_json::to_value(&self.drafts).unwrap()),
            (
                "commitments.json",
                serde_json::to_value(&self.commitments).unwrap(),
            ),
            (
                "experiments.json",
                serde_json::to_value(&self.experiments).unwrap(),
            ),
            (
                "outcomes.json",
                serde_json::to_value(&self.outcomes).unwrap(),
            ),
            ("company-ops.json", serde_json::to_value(self).unwrap()),
        ] {
            std::fs::write(
                directory.join(name),
                serde_json::to_vec_pretty(&value).unwrap(),
            )?;
        }
        Ok(())
    }
}
