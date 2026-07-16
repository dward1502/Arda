use arda_core::task::Task;
use arda_governance::{bacon_lite_validate, triad_validate};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignificanceResult {
    pub joulework: f64,
    pub love_eq: f64,
    pub triad: bool,
    pub bacon_lite_confidence: f64,
    pub significance: f64,
    pub sigil: String,
    pub class: String,
}

pub fn evaluate_significance(
    content: &str,
    event_type: Option<&str>,
    tags: &[String],
    confidence_hint: Option<f64>,
) -> SignificanceResult {
    let lower = content.to_ascii_lowercase();
    let event_lower = event_type.unwrap_or_default().to_ascii_lowercase();
    let tag_lowers = tags
        .iter()
        .map(|tag| tag.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let joulework = confidence_hint.unwrap_or(0.5).clamp(0.0, 1.0);
    let mut love_eq: f64 = if lower.contains("illuvatar")
        || lower.contains("mission")
        || lower.contains("arda")
        || lower.contains("sovereign")
    {
        0.9
    } else if lower.contains("security") || lower.contains("governance") {
        0.8
    } else {
        0.5
    };
    if event_lower.contains("interrupt")
        || event_lower.contains("boardroom")
        || tag_lowers.iter().any(|tag| {
            matches!(
                tag.as_str(),
                "boardroom"
                    | "interrupt"
                    | "decision"
                    | "continuity"
                    | "ceo_observability"
                    | "governance"
            )
        })
    {
        love_eq = love_eq.max(0.78);
    }

    let task = Task::new(content, "memory_encode");
    let triad = triad_validate(&task, None).passed;
    let bacon_lite = bacon_lite_validate(&task);
    let triad_score = if triad { 1.0 } else { 0.0 };
    let context_bonus = if tag_lowers.iter().any(|tag| {
        matches!(
            tag.as_str(),
            "boardroom" | "interrupt" | "decision" | "ceo_observability" | "tier1_rule"
        )
    }) || (event_lower == "inbound_classified"
        && tag_lowers
            .iter()
            .any(|tag| matches!(tag.as_str(), "checkpoint" | "decision" | "operator_review")))
    {
        0.08
    } else {
        0.0
    };
    let event_bonus = checkpoint_bonus(&event_lower, &tag_lowers, &lower);
    let penalty = low_signal_penalty(&event_lower, &tag_lowers, &lower);
    let significance = ((joulework * 0.35)
        + (love_eq * 0.35)
        + (triad_score * 0.15)
        + (bacon_lite.confidence * 0.15)
        + context_bonus
        + event_bonus
        - penalty)
        .clamp(0.0, 1.0);

    let (class, sigil) = classify_significance(significance);

    SignificanceResult {
        joulework,
        love_eq,
        triad,
        bacon_lite_confidence: bacon_lite.confidence,
        significance,
        sigil: sigil.to_owned(),
        class: class.to_owned(),
    }
}

fn checkpoint_bonus(event_type: &str, tags: &[String], content: &str) -> f64 {
    let high_value_event = matches!(
        event_type,
        "boardroom_posted"
            | "interruption_captured"
            | "task_delegated"
            | "task_completed"
            | "task_failed"
            | "routing_failure"
            | "decision_completed"
            | "council_gate"
            | "illuvatar_fanout"
    );
    let high_value_tag = tags.iter().any(|tag| {
        matches!(
            tag.as_str(),
            "checkpoint"
                | "decision"
                | "boardroom"
                | "interrupt"
                | "delegation"
                | "completion"
                | "failure"
                | "routing"
                | "continuity"
                | "governance"
        )
    });
    let content_bonus = if content.contains("because") || content.contains("blocked") {
        0.04
    } else {
        0.0
    };
    let base: f64 = if high_value_event {
        0.18
    } else if high_value_tag {
        0.10
    } else {
        0.0
    };
    let total: f64 = base + content_bonus;
    total.clamp(0.0, 0.24)
}

fn low_signal_penalty(event_type: &str, tags: &[String], content: &str) -> f64 {
    let tier3_fallback = tags.iter().any(|tag| tag == "tier3_fallback");
    let generic_classifier = matches!(event_type, "inbound_classified" | "outbound_queued")
        && !tags.iter().any(|tag| {
            matches!(
                tag.as_str(),
                "checkpoint" | "decision" | "boardroom" | "interrupt" | "operator_review"
            )
        });
    let generic_content = content == "hermes classified inbound message intent"
        || content == "prometheus received task"
        || content.len() < 48;
    if tier3_fallback && generic_classifier {
        0.24
    } else if generic_classifier && generic_content {
        0.16
    } else if generic_content {
        0.08
    } else {
        0.0
    }
}

pub fn classify_significance(significance: f64) -> (&'static str, &'static str) {
    if significance >= 0.8 {
        ("core", "MNEME_CORE")
    } else if significance >= 0.6 {
        ("active", "MNEME_ACTIVE")
    } else if significance >= 0.4 {
        ("peripheral", "MNEME_PERIPHERAL")
    } else if significance >= 0.2 {
        ("transient", "MNEME_TRANSIENT")
    } else {
        ("noise", "MNEME_RELEASED")
    }
}

#[cfg(test)]
mod tests {
    use super::evaluate_significance;

    #[test]
    fn high_relevance_is_core_or_active() {
        let res = evaluate_significance(
            "Illuvatar mission for ARDA",
            Some("executive_directive"),
            &["continuity".to_string()],
            Some(0.95),
        );
        assert!(res.significance >= 0.6);
        assert!(res.bacon_lite_confidence > 0.0);
    }

    #[test]
    fn boardroom_and_interrupt_context_gets_promoted() {
        let res = evaluate_significance(
            "HERMES captured async interruption while preserving in-flight execution",
            Some("interrupt_captured"),
            &[
                "hermes".to_string(),
                "interrupt".to_string(),
                "ceo_observability".to_string(),
            ],
            Some(0.84),
        );
        assert!(res.significance >= 0.45);
        assert_ne!(res.class, "noise");
    }

    #[test]
    fn generic_tier3_classification_stays_low_signal() {
        let res = evaluate_significance(
            "HERMES classified inbound message intent",
            Some("inbound_classified"),
            &["hermes".to_string(), "tier3_fallback".to_string()],
            Some(0.78),
        );
        assert!(res.significance < 0.3);
    }
}
