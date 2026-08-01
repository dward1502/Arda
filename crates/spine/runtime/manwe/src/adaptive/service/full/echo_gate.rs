// sigil: REPAIR
//! Echo Gate — pre-route governance hook.
//!
//! Evaluates an input prompt against regex-detected risk/evidence markers,
//! input-shape confidence (rho), and alignment (gamma), producing a governance
//! decision (Proceed / Pause / Abort) plus a delta weight used to penalize
//! non-local providers under elevated risk.
//!
//! Called from `CharonService::select_route_candidate` (full gating) and
//! `CharonService::proxy_openai_request` (Abort-only hard stop before
//! provider fallback).
use regex::RegexSet;
use serde::Serialize;
use serde_json::Value;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub(super) enum GateAction {
    Proceed,
    Pause,
    Abort,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct EchoGateDecision {
    pub(super) action: GateAction,
    pub(super) delta: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum GovernanceMethod {
    Single,
    Triad,
    Chain,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct PreRouteGovernanceSnapshot {
    pub(super) action: GateAction,
    pub(super) governance_method: GovernanceMethod,
    pub(super) philosopher_lens: String,
    pub(super) chain_id: Option<String>,
    pub(super) rho: f64,
    pub(super) gamma: f64,
    pub(super) delta: f64,
    pub(super) bacon_evidence_score: f64,
    pub(super) soterion_protocol_markers: usize,
    pub(super) trigger_reason: String,
}

pub(super) fn evaluate_echo_gate(input: &str) -> EchoGateDecision {
    if high_risk_regex().is_some_and(|regex| regex.is_match(input)) {
        return EchoGateDecision {
            action: GateAction::Abort,
            delta: 0.7,
        };
    }

    // Require a destructive verb with a sensitive object. Bare coding verbs are
    // common in normal tool work and should not force local-only routing.
    if sensitive_mutation_regex().is_some_and(|regex| regex.is_match(input)) {
        return EchoGateDecision {
            action: GateAction::Pause,
            delta: 0.5,
        };
    }

    EchoGateDecision {
        action: GateAction::Proceed,
        delta: 0.1,
    }
}

pub(super) fn estimate_rho(input: &str) -> f64 {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return 0.35;
    }

    let word_count = trimmed.split_whitespace().count();
    match word_count {
        0 => 0.35,
        1..=3 => 0.58,
        4..=12 => 0.76,
        _ => 0.86,
    }
}

pub(super) fn estimate_gamma(input: &str) -> f64 {
    if high_risk_regex().is_some_and(|regex| regex.is_match(input)) {
        return 0.25;
    }

    if sensitive_mutation_regex().is_some_and(|regex| regex.is_match(input)) {
        return 0.55;
    }

    let lowered = input.to_ascii_lowercase();
    if lowered.contains("handle it") || lowered.contains("do it") || lowered.contains("just do it")
    {
        return 0.62;
    }

    0.86
}

#[cfg(test)]
fn evaluate_pre_route_governance(input: &str) -> PreRouteGovernanceSnapshot {
    evaluate_pre_route_governance_with_options(input, &Value::Null)
}

pub(super) fn evaluate_pre_route_governance_with_options(
    input: &str,
    options: &Value,
) -> PreRouteGovernanceSnapshot {
    let governance_method = governance_method(options);
    let philosopher_lens = philosopher_lens(options);
    let chain_id = chain_id(options);
    let gate = evaluate_echo_gate(input);
    let rho = estimate_rho(input);
    let gamma = estimate_gamma(input);
    let bacon_evidence_score = estimate_bacon_evidence(input);
    let soterion_protocol_markers = soterion_protocol_markers(input);

    let (action, trigger_reason) = match gate.action {
        GateAction::Abort => (GateAction::Abort, "delta_abort_keyword".to_string()),
        GateAction::Pause => (GateAction::Pause, "delta_pause_keyword".to_string()),
        GateAction::Proceed => {
            if governance_method == GovernanceMethod::Single && philosopher_lens == "bacon" {
                if bacon_evidence_score < 0.35 {
                    (
                        GateAction::Pause,
                        "bacon_low_empirical_grounding".to_string(),
                    )
                } else {
                    (GateAction::Proceed, "bacon_regex_clear".to_string())
                }
            } else if rho < 0.50 {
                (GateAction::Pause, "rho_low_confidence".to_string())
            } else if gamma < 0.60 {
                (GateAction::Pause, "gamma_low_alignment".to_string())
            } else if gate.delta >= 0.50 {
                (GateAction::Pause, "delta_elevated".to_string())
            } else if governance_method == GovernanceMethod::Chain {
                (GateAction::Proceed, "chain_regex_clear".to_string())
            } else {
                (GateAction::Proceed, "triad_clear".to_string())
            }
        }
    };

    PreRouteGovernanceSnapshot {
        action,
        governance_method,
        philosopher_lens,
        chain_id,
        rho,
        gamma,
        delta: gate.delta,
        bacon_evidence_score,
        soterion_protocol_markers,
        trigger_reason,
    }
}

fn governance_method(options: &Value) -> GovernanceMethod {
    option_str(options, "governance_method")
        .or_else(|| option_str(options, "philosopher_method"))
        .map(|value| match value.to_ascii_lowercase().as_str() {
            "single" | "single_philosopher" | "philosopher" => GovernanceMethod::Single,
            "chain" | "governance_chain" => GovernanceMethod::Chain,
            _ => GovernanceMethod::Triad,
        })
        .unwrap_or(GovernanceMethod::Triad)
}

fn philosopher_lens(options: &Value) -> String {
    option_str(options, "governance_philosopher")
        .or_else(|| option_str(options, "philosopher_lens"))
        .or_else(|| option_str(options, "philosopher"))
        .unwrap_or("bacon")
        .to_ascii_lowercase()
}

fn chain_id(options: &Value) -> Option<String> {
    option_str(options, "governance_chain_id")
        .or_else(|| option_str(options, "chain_id"))
        .map(ToString::to_string)
}

fn option_str<'a>(options: &'a Value, key: &str) -> Option<&'a str> {
    options
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
}

fn estimate_bacon_evidence(input: &str) -> f64 {
    let mut score: f64 = 0.20;
    if evidence_anchor_regex().is_some_and(|regex| regex.is_match(input)) {
        score += 0.35;
    }
    if verification_regex().is_some_and(|regex| regex.is_match(input)) {
        score += 0.25;
    }
    if disconfirmation_regex().is_some_and(|regex| regex.is_match(input)) {
        score += 0.20;
    }
    score.clamp(0.0, 1.0)
}

fn soterion_protocol_markers(input: &str) -> usize {
    soterion_protocol_regex()
        .map(|regex| regex.matches(input).into_iter().count())
        .unwrap_or(0)
}

fn high_risk_regex() -> Option<&'static RegexSet> {
    static SET: OnceLock<Option<RegexSet>> = OnceLock::new();
    SET.get_or_init(|| {
        RegexSet::new([
            r"(?i)\b(modify|delete)\s+system\s+file\b",
            r"(?i)\brm\s+-rf\s+/",
            r"(?i)\bdisable\s+security\b",
            r"(?i)\bdrop\s+(database|table)\b",
            r"(?i)\bforce\s+push\s+to\s+(main|master)\b",
        ])
        .ok()
    })
    .as_ref()
}

fn sensitive_mutation_regex() -> Option<&'static RegexSet> {
    static SET: OnceLock<Option<RegexSet>> = OnceLock::new();
    SET.get_or_init(|| {
        RegexSet::new([
            r"(?i)\b(modify|delete|drop|wipe|erase|overwrite|purge)\b.{0,48}\b(production|prod|secrets?|credentials?|api\s+key|private\s+key|database|backup)\b",
            r"(?i)\b(modify|delete|drop|wipe|erase|overwrite|purge)\b.{0,48}(/etc/|\.env|passwd|shadow)",
        ])
        .ok()
    })
    .as_ref()
}

fn evidence_anchor_regex() -> Option<&'static RegexSet> {
    static SET: OnceLock<Option<RegexSet>> = OnceLock::new();
    SET.get_or_init(|| {
        RegexSet::new([
            r"https?://",
            r"(?i)\b(source|sources|receipt|receipts|evidence|proof|provenance)\b",
            r"\b[a-zA-Z0-9_\-./]+\.jsonl?\b",
            r"\b[a-zA-Z0-9_\-./]+\.md\b",
            r"\b[a-zA-Z0-9_\-./]+\.toml\b",
        ])
        .ok()
    })
    .as_ref()
}

fn verification_regex() -> Option<&'static RegexSet> {
    static SET: OnceLock<Option<RegexSet>> = OnceLock::new();
    SET.get_or_init(|| {
        RegexSet::new([
            r"(?i)\b(verify|verified|validation|validated|test|tests|cargo\s+test|cargo\s+check|audit|observed|command)\b",
        ])
        .ok()
    })
    .as_ref()
}

fn disconfirmation_regex() -> Option<&'static RegexSet> {
    static SET: OnceLock<Option<RegexSet>> = OnceLock::new();
    SET.get_or_init(|| {
        RegexSet::new([
            r"(?i)\b(disconfirm|falsif|counter[- ]?evidence|would\s+change|failure\s+mode|bias|idol)\b",
        ])
        .ok()
    })
    .as_ref()
}

fn soterion_protocol_regex() -> Option<&'static RegexSet> {
    static SET: OnceLock<Option<RegexSet>> = OnceLock::new();
    SET.get_or_init(|| {
        RegexSet::new([r"[\u{2696}\u{26A1}\u{267E}\u{1F4E6}\u{1F511}\u{1F441}]"]).ok()
    })
    .as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_aborts_on_modify_system_file() {
        let d = evaluate_echo_gate("please modify system file /etc/passwd");
        assert_eq!(d.action, GateAction::Abort);
        assert!((d.delta - 0.7).abs() < 1e-9);
    }

    #[test]
    fn gate_pauses_on_modify_plus_sensitive_noun() {
        let d = evaluate_echo_gate("modify production secrets");
        assert_eq!(d.action, GateAction::Pause);
        assert!((d.delta - 0.5).abs() < 1e-9);
    }

    #[test]
    fn gate_proceeds_on_bare_modify() {
        // Bare "modify" was the noisiest false-PAUSE trigger; it must now
        // proceed so coding agents aren't constantly forced to local-only.
        let d = evaluate_echo_gate("modify this for me");
        assert_eq!(d.action, GateAction::Proceed);
    }

    #[test]
    fn gate_proceeds_on_plain_input() {
        let d = evaluate_echo_gate("summarize the changelog for v0.4");
        assert_eq!(d.action, GateAction::Proceed);
        assert!((d.delta - 0.1).abs() < 1e-9);
    }

    #[test]
    fn rho_scales_with_word_count() {
        assert_eq!(estimate_rho(""), 0.35);
        assert_eq!(estimate_rho("   "), 0.35);
        assert_eq!(estimate_rho("run"), 0.58);
        assert_eq!(estimate_rho("run the thing"), 0.58);
        assert_eq!(estimate_rho("run the thing with care"), 0.76);
        assert_eq!(
            estimate_rho(
                "this sentence has well over twelve words so that rho falls into the highest confidence tier"
            ),
            0.86
        );
    }

    #[test]
    fn gamma_hits_high_risk_system_verbs() {
        assert_eq!(estimate_gamma("please modify system file now"), 0.25);
        assert_eq!(estimate_gamma("delete system file once"), 0.25);
        assert_eq!(estimate_gamma("disable security checks"), 0.25);
    }

    #[test]
    fn gamma_hits_medium_risk_verb_plus_sensitive_noun() {
        assert_eq!(estimate_gamma("delete the production database"), 0.55);
        assert_eq!(estimate_gamma("overwrite the .env file"), 0.55);
        assert_eq!(estimate_gamma("wipe credentials store"), 0.55);
    }

    #[test]
    fn gamma_does_not_trip_on_bare_coding_verbs() {
        // Bare verbs appear constantly in coding prompts and must not force
        // PAUSE → local-only routing on their own. Without a sensitive noun
        // they should fall through to the default-clear band.
        assert_eq!(estimate_gamma("modify the draft"), 0.86);
        assert_eq!(estimate_gamma("delete the note"), 0.86);
        assert_eq!(estimate_gamma("overwrite the row"), 0.86);
        assert_eq!(estimate_gamma("refactor the route handler"), 0.86);
    }

    #[test]
    fn gamma_hits_vague_directives() {
        assert_eq!(estimate_gamma("just do it"), 0.62);
        assert_eq!(estimate_gamma("handle it for me"), 0.62);
    }

    #[test]
    fn gamma_defaults_clear() {
        assert_eq!(estimate_gamma("summarize this document"), 0.86);
    }

    #[test]
    fn governance_aborts_when_gate_aborts() {
        let g = evaluate_pre_route_governance("modify system file /etc/hosts");
        assert_eq!(g.action, GateAction::Abort);
        assert_eq!(g.trigger_reason, "delta_abort_keyword");
    }

    #[test]
    fn governance_pauses_when_gate_pauses() {
        let g = evaluate_pre_route_governance("modify production secrets right now");
        assert_eq!(g.action, GateAction::Pause);
        assert_eq!(g.trigger_reason, "delta_pause_keyword");
    }

    #[test]
    fn governance_pauses_on_low_rho() {
        // Empty input: echo_gate returns Proceed, but rho=0.35 trips the rho<0.50 branch.
        let g = evaluate_pre_route_governance("");
        assert_eq!(g.action, GateAction::Pause);
        assert_eq!(g.trigger_reason, "rho_low_confidence");
    }

    #[test]
    fn governance_pauses_on_low_gamma() {
        // Verb-plus-sensitive-noun pairs now hit the regex prefilter first,
        // before the lower-priority gamma branch.
        let g = evaluate_pre_route_governance("delete the production database before lunch please");
        assert_eq!(g.action, GateAction::Pause);
        assert_eq!(g.trigger_reason, "delta_pause_keyword");
    }

    #[test]
    fn governance_proceeds_on_clear_path() {
        let g = evaluate_pre_route_governance(
            "summarize the release notes and identify any breaking API changes for users",
        );
        assert_eq!(g.action, GateAction::Proceed);
        assert_eq!(g.trigger_reason, "triad_clear");
    }

    #[test]
    fn single_bacon_method_uses_regex_evidence_without_llm() {
        let g = evaluate_pre_route_governance_with_options(
            "verify with cargo test and receipt core/state/queue_summary.json; disconfirm if the command fails",
            &serde_json::json!({
                "governance_method": "single",
                "governance_philosopher": "bacon"
            }),
        );

        assert_eq!(g.action, GateAction::Proceed);
        assert_eq!(g.governance_method, GovernanceMethod::Single);
        assert_eq!(g.philosopher_lens, "bacon");
        assert_eq!(g.trigger_reason, "bacon_regex_clear");
        assert!(g.bacon_evidence_score >= 0.75);
    }

    #[test]
    fn single_bacon_pauses_without_empirical_grounding() {
        let g = evaluate_pre_route_governance_with_options(
            "handle it",
            &serde_json::json!({
                "philosopher_method": "single_philosopher",
                "philosopher_lens": "bacon"
            }),
        );

        assert_eq!(g.action, GateAction::Pause);
        assert_eq!(g.trigger_reason, "bacon_low_empirical_grounding");
    }

    #[test]
    fn chain_method_projects_chain_metadata() {
        let g = evaluate_pre_route_governance_with_options(
            "verify route evidence from docs/contracts/SOTERION_SIGIL_METHOD.md ⚖",
            &serde_json::json!({
                "governance_method": "chain",
                "governance_chain_id": "default_triad"
            }),
        );

        assert_eq!(g.governance_method, GovernanceMethod::Chain);
        assert_eq!(g.chain_id.as_deref(), Some("default_triad"));
        assert_eq!(g.trigger_reason, "chain_regex_clear");
        assert_eq!(g.soterion_protocol_markers, 1);
    }
}
