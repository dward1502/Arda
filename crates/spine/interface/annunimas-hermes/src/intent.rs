// sigil: REPAIR
use crate::types::{InboundMessage, IntentClass, IntentResult, IntentRoute};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassificationTier {
    Tier1Rule,
    Tier2Heuristic,
    Tier3Fallback,
}

pub fn classify_message(msg: &InboundMessage) -> IntentResult {
    if let Some(result) = classify_tier1(msg) {
        return result;
    }
    let tier2 = classify_tier2(msg);
    if tier2.confidence >= 0.75 {
        return tier2;
    }
    classify_tier3(msg, tier2)
}

fn classify_tier1(msg: &InboundMessage) -> Option<IntentResult> {
    let text = msg.content.trim().to_lowercase();
    if text.is_empty() {
        return None;
    }
    if text.starts_with('!') {
        return Some(IntentResult {
            intent: IntentClass::TaskRequest,
            priority: "urgent".to_string(),
            route_to: IntentRoute::Prometheus,
            joulework: 0.2,
            love_eq: 0.8,
            triad_passed: None,
            triad_score: None,
            confidence: 1.0,
            tier: "tier1_rule".to_string(),
            reason: "urgent prefix detected".to_string(),
        });
    }
    if text == "status" || text == "report" || text == "queue" {
        return Some(IntentResult {
            intent: IntentClass::StatusCheck,
            priority: "normal".to_string(),
            route_to: IntentRoute::Prometheus,
            joulework: 0.1,
            love_eq: 0.6,
            triad_passed: None,
            triad_score: None,
            confidence: 0.98,
            tier: "tier1_rule".to_string(),
            reason: "matched known status/report command".to_string(),
        });
    }
    if text.starts_with("schedule ") {
        return Some(IntentResult {
            intent: IntentClass::TaskRequest,
            priority: "normal".to_string(),
            route_to: IntentRoute::Calendar,
            joulework: 0.2,
            love_eq: 0.5,
            triad_passed: None,
            triad_score: None,
            confidence: 0.95,
            tier: "tier1_rule".to_string(),
            reason: "matched schedule command".to_string(),
        });
    }
    if text.starts_with('@') {
        return Some(IntentResult {
            intent: IntentClass::Redirect,
            priority: "normal".to_string(),
            route_to: IntentRoute::Boardroom,
            joulework: 0.1,
            love_eq: 0.5,
            triad_passed: None,
            triad_score: None,
            confidence: 0.95,
            tier: "tier1_rule".to_string(),
            reason: "agent mention redirect".to_string(),
        });
    }
    None
}

fn classify_tier2(msg: &InboundMessage) -> IntentResult {
    let text = msg.content.to_lowercase();
    let mut route = IntentRoute::Hermes;
    let mut intent = IntentClass::Unknown;
    let mut confidence: f64 = 0.62;
    let mut priority = "normal".to_string();
    let mut joulework: f64 = 0.3;
    let mut love_eq: f64 = 0.4;

    if text.contains('?') {
        intent = IntentClass::Question;
        route = IntentRoute::Athena;
        confidence += 0.12;
        love_eq += 0.1;
    }
    if text.contains("help") || text.contains("review") || text.contains("analyze") {
        intent = IntentClass::TaskRequest;
        route = IntentRoute::Prometheus;
        confidence += 0.1;
        joulework += 0.2;
    }
    if text.contains("meeting")
        || text.contains("calendar")
        || text.contains("tomorrow")
        || text.contains("schedule")
    {
        intent = IntentClass::TaskRequest;
        route = IntentRoute::Calendar;
        confidence += 0.08;
    }
    if text.contains("thanks") || text.contains("hello") || text.contains("hi ") {
        intent = IntentClass::Social;
        route = IntentRoute::Hermes;
        confidence += 0.05;
        joulework -= 0.1;
    }
    if msg.is_illuvatar {
        priority = "urgent".to_string();
        confidence += 0.06;
        love_eq += 0.2;
    }

    IntentResult {
        intent,
        priority,
        route_to: route,
        joulework: joulework.clamp(0.0, 1.0),
        love_eq: love_eq.clamp(0.0, 1.0),
        triad_passed: None,
        triad_score: None,
        confidence: confidence.clamp(0.0, 1.0),
        tier: "tier2_heuristic".to_string(),
        reason: "heuristic classification from content features".to_string(),
    }
}

fn classify_tier3(msg: &InboundMessage, mut prior: IntentResult) -> IntentResult {
    let text = msg.content.to_lowercase();
    prior.tier = "tier3_fallback".to_string();
    prior.reason = "fallback classification selected due low tier2 confidence".to_string();
    prior.confidence = 0.78;

    if text.contains("why") || text.contains("explain") || text.contains("what") {
        prior.intent = IntentClass::Question;
        prior.route_to = IntentRoute::Athena;
    } else if text.contains("do ") || text.contains("run ") || text.contains("fix ") {
        prior.intent = IntentClass::TaskRequest;
        prior.route_to = IntentRoute::Prometheus;
    } else {
        prior.intent = IntentClass::Unknown;
        prior.route_to = IntentRoute::Prometheus;
    }

    prior
}

#[cfg(test)]
mod tests {
    use super::classify_message;
    use crate::types::{InboundMessage, IntentRoute};

    #[test]
    fn urgent_prefix_routes_to_prometheus() {
        let msg = InboundMessage::new("discord", "illuvatar", "!stop hades");
        let out = classify_message(&msg);
        assert!(matches!(out.route_to, IntentRoute::Prometheus));
        assert_eq!(out.tier, "tier1_rule");
    }

    #[test]
    fn schedule_routes_to_calendar() {
        let msg = InboundMessage::new("discord", "illuvatar", "schedule sync tomorrow 9am");
        let out = classify_message(&msg);
        assert!(matches!(out.route_to, IntentRoute::Calendar));
    }
}
