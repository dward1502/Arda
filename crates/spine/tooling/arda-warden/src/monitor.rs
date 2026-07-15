// sigil: REPAIR
use arda_core::config::Config;
use arda_core::task::Task;
use arda_governance::bacon_lite_validate;
use arda_plutus::LoveEquation;
use serde::{Deserialize, Serialize};
use tracing::info;

const DENY_PATTERNS: [&str; 5] = ["rm -rf", "mkfs", "dd if=", "git reset --hard", ":(){"];
const BASE_VERIFICATION_STEPS: [&str; 3] =
    ["capture_pre_state", "record_intent", "record_post_state"];

pub async fn audit_container(container_name: &str, config: &Config) -> anyhow::Result<()> {
    // Mock Podman health check (expand with podman-rs crate later)
    info!("Checking health of {}", container_name);

    // JouleWork usage check
    let usage = 50; // Mock
    if usage > config.joulework.threshold as usize {
        tracing::warn!("High joule usage in {}: {}", container_name, usage);
    }

    // Love resonance mock
    let resonance = 92.0;
    info!("Resonance for {}: {:.1}%", container_name, resonance);

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionHarnessPolicy {
    pub risk_level: String,
    pub sandbox_required: bool,
    pub approval_required: bool,
    pub network_access: String,
    pub verification_steps: Vec<String>,
    pub deny_patterns: Vec<String>,
    pub governance: HarnessGovernance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessLoveEquationGuard {
    pub resonance: f64,
    pub attention: f64,
    pub reciprocity: f64,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessGovernance {
    pub bacon_lite_passed: bool,
    pub bacon_lite_confidence: f64,
    pub love_equation_guard: HarnessLoveEquationGuard,
}

pub fn evaluate_execution_harness(
    payload: &serde_json::Value,
    priority: Option<&str>,
) -> ExecutionHarnessPolicy {
    let priority = priority.unwrap_or("normal").to_ascii_lowercase();
    let payload_text = payload.to_string().to_ascii_lowercase();
    let destructive = DENY_PATTERNS
        .iter()
        .any(|pattern| payload_text.contains(pattern));
    let network_requested = payload_text.contains("http://")
        || payload_text.contains("https://")
        || payload_text.contains("curl ")
        || payload_text.contains("wget ")
        || payload
            .get("network")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
    let command_requested = payload_text.contains("command")
        || payload_text.contains("shell")
        || payload_text.contains("script")
        || payload_text.contains("bash")
        || payload_text.contains("cargo ");
    let approval_required =
        destructive || matches!(priority.as_str(), "high" | "critical") || network_requested;
    let risk_level = if destructive {
        "critical"
    } else if approval_required || command_requested {
        "elevated"
    } else {
        "guarded"
    };
    let mut verification_steps = owned_steps(BASE_VERIFICATION_STEPS);
    if command_requested {
        verification_steps.insert(1, String::from("dry_run_or_simulate"));
    }
    if approval_required {
        verification_steps.push(String::from("human_approval"));
    }
    let task = Task::new(
        format!(
            "evaluate harness for priority={} because payload requires bounded execution",
            priority
        ),
        "monitor",
    );
    let bacon_lite = bacon_lite_validate(&task);
    let resonance = if approval_required { 0.74 } else { 0.58 };
    let attention = if network_requested || command_requested {
        0.8
    } else {
        0.62
    };
    let reciprocity = if destructive { 0.32 } else { 0.72 };
    let love_score = LoveEquation::new().calculate(
        "warden",
        "execution_harness",
        resonance,
        attention,
        reciprocity,
    );

    ExecutionHarnessPolicy {
        risk_level: String::from(risk_level),
        sandbox_required: command_requested || destructive,
        approval_required,
        network_access: if network_requested {
            String::from("restricted")
        } else {
            String::from("none")
        },
        verification_steps,
        deny_patterns: owned_steps(DENY_PATTERNS),
        governance: HarnessGovernance {
            bacon_lite_passed: bacon_lite.passed,
            bacon_lite_confidence: bacon_lite.confidence,
            love_equation_guard: HarnessLoveEquationGuard {
                resonance,
                attention,
                reciprocity,
                score: love_score,
            },
        },
    }
}

fn owned_steps<const N: usize>(steps: [&str; N]) -> Vec<String> {
    steps.into_iter().map(String::from).collect()
}

#[cfg(test)]
mod tests {
    use super::evaluate_execution_harness;

    #[test]
    fn harness_requires_approval_for_destructive_or_network_payloads() {
        let policy = evaluate_execution_harness(
            &serde_json::json!({
                "command": "curl https://example.com && rm -rf /tmp/example"
            }),
            Some("high"),
        );
        assert_eq!(policy.risk_level, "critical");
        assert!(policy.sandbox_required);
        assert!(policy.approval_required);
        assert_eq!(policy.network_access, "restricted");
        assert!(policy
            .verification_steps
            .contains(&String::from("human_approval")));
        assert!(policy.governance.bacon_lite_confidence >= 0.0);
        assert!(policy.governance.love_equation_guard.score > 0.0);
    }

    #[test]
    fn harness_keeps_low_risk_payload_guarded_without_approval() {
        let policy = evaluate_execution_harness(
            &serde_json::json!({
                "intent": "read local runtime posture"
            }),
            Some("normal"),
        );

        assert_eq!(policy.risk_level, "guarded");
        assert!(!policy.sandbox_required);
        assert!(!policy.approval_required);
        assert_eq!(policy.network_access, "none");
        assert!(!policy
            .verification_steps
            .contains(&String::from("human_approval")));
    }
}
