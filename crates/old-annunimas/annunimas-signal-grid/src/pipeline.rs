// sigil: ANKH
use serde::{Deserialize, Serialize};

pub const GOVERNANCE_VALIDATORS: [&str; 5] = [
    "triad",
    "bacon_lite",
    "joulework",
    "love_equation",
    "soterion_trace",
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommentSignal {
    ProductFeedback,
    SalesSignal,
    RepeatedQuestion,
    NegativeSentiment,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RouteDecision {
    RouteToProductOrOperations,
    RouteToBusinessIntelligenceOrCrm,
    RouteToContentGapBrief,
    PauseAndAlertHuman,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CrisisTier {
    Normal,
    Pause,
    FullStop,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignalGridPolicy {
    pub negative_spike: bool,
    pub legal_threat: bool,
    pub export_state: bool,
}

impl SignalGridPolicy {
    pub const fn normal() -> Self {
        Self {
            negative_spike: false,
            legal_threat: false,
            export_state: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct SignalRoutePlan {
    pub signal: CommentSignal,
    pub decision: RouteDecision,
    pub crisis_tier: CrisisTier,
    pub human_review_required: bool,
    pub state_export_required: bool,
    pub governance_validators: [&'static str; 5],
}

impl CommentSignal {
    pub fn route(self) -> RouteDecision {
        match self {
            Self::ProductFeedback => RouteDecision::RouteToProductOrOperations,
            Self::SalesSignal => RouteDecision::RouteToBusinessIntelligenceOrCrm,
            Self::RepeatedQuestion => RouteDecision::RouteToContentGapBrief,
            Self::NegativeSentiment => RouteDecision::PauseAndAlertHuman,
        }
    }
}

pub fn crisis_tier_for(policy: SignalGridPolicy) -> CrisisTier {
    if policy.legal_threat {
        CrisisTier::FullStop
    } else if policy.negative_spike {
        CrisisTier::Pause
    } else {
        CrisisTier::Normal
    }
}

pub fn route_plan(signal: CommentSignal, policy: SignalGridPolicy) -> SignalRoutePlan {
    let crisis_tier = crisis_tier_for(policy);
    SignalRoutePlan {
        signal,
        decision: signal.route(),
        crisis_tier,
        human_review_required: matches!(signal, CommentSignal::NegativeSentiment)
            || !matches!(crisis_tier, CrisisTier::Normal),
        state_export_required: policy.export_state,
        governance_validators: GOVERNANCE_VALIDATORS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_questions_route_to_content_gap_brief_without_human_review() {
        let plan = route_plan(CommentSignal::RepeatedQuestion, SignalGridPolicy::normal());

        assert_eq!(plan.decision, RouteDecision::RouteToContentGapBrief);
        assert_eq!(plan.crisis_tier, CrisisTier::Normal);
        assert!(!plan.human_review_required);
        assert!(plan.state_export_required);
    }

    #[test]
    fn legal_threat_forces_full_stop_and_human_review() {
        let plan = route_plan(
            CommentSignal::SalesSignal,
            SignalGridPolicy {
                negative_spike: false,
                legal_threat: true,
                export_state: true,
            },
        );

        assert_eq!(
            plan.decision,
            RouteDecision::RouteToBusinessIntelligenceOrCrm
        );
        assert_eq!(plan.crisis_tier, CrisisTier::FullStop);
        assert!(plan.human_review_required);
    }
}
