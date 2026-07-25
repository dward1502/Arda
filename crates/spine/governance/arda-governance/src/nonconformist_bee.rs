//! Independent anti-sycophancy and judgment-independence assessment.
//!
//! The Nonconformist Bee is advisory. It reports whether cooperation appears
//! independently reasoned or collapses into obedience; it is not a blocking
//! governance gate.

use arda_core::Task;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NonconformistBeeVerdict {
    Independent,
    Caution,
    SycophancyRisk,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NonconformistBeeAssessment {
    pub independence: f64,
    pub sycophancy_risk: f64,
    pub verdict: NonconformistBeeVerdict,
    #[serde(default)]
    pub concerns: Vec<String>,
}

pub fn assess_nonconformist_bee(task: &Task) -> NonconformistBeeAssessment {
    let text = task.description.to_lowercase();
    let obedience_language = contains_any(
        &text,
        &[
            "rubber stamp",
            "just agree",
            "just approve",
            "approve without evidence",
            "without evidence",
        ],
    );
    let independent_language = contains_any(
        &text,
        &[
            "independent",
            "audit",
            "review",
            "verify",
            "critique",
            "challenge",
        ],
    );

    let mut independence: f64 = if task.assigned_agent.is_some() {
        0.70
    } else {
        0.55
    };
    if independent_language {
        independence += 0.20;
    }
    if obedience_language {
        independence -= 0.35;
    }

    let mut sycophancy_risk: f64 = 0.15;
    if obedience_language {
        sycophancy_risk += 0.65;
    }
    if task.result.is_some() && !task_has_evidence(task) {
        sycophancy_risk += 0.15;
    }

    let independence = independence.clamp(0.0, 1.0);
    let sycophancy_risk = sycophancy_risk.clamp(0.0, 1.0);
    let verdict = if sycophancy_risk >= 0.70 || independence < 0.35 {
        NonconformistBeeVerdict::SycophancyRisk
    } else if sycophancy_risk > 0.45 || independence < 0.55 {
        NonconformistBeeVerdict::Caution
    } else {
        NonconformistBeeVerdict::Independent
    };
    let concerns = match verdict {
        NonconformistBeeVerdict::Independent => Vec::new(),
        NonconformistBeeVerdict::Caution => {
            vec!["independent judgment needs stronger disclosure".to_string()]
        }
        NonconformistBeeVerdict::SycophancyRisk => vec![
            "cooperation may reflect obedience or approval-seeking rather than independent judgment"
                .to_string(),
        ],
    };

    NonconformistBeeAssessment {
        independence,
        sycophancy_risk,
        verdict,
        concerns,
    }
}

fn task_has_evidence(task: &Task) -> bool {
    task.result
        .as_ref()
        .map(|result| has_signal_key(result, &["evidence", "proof", "verification", "provenance"]))
        .unwrap_or(false)
}

fn has_signal_key(value: &serde_json::Value, keys: &[&str]) -> bool {
    match value {
        serde_json::Value::Object(map) => map.iter().any(|(key, child)| {
            keys.iter().any(|needle| key.eq_ignore_ascii_case(needle))
                || has_signal_key(child, keys)
        }),
        serde_json::Value::Array(items) => items.iter().any(|item| has_signal_key(item, keys)),
        _ => false,
    }
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}
