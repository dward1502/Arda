#![cfg(feature = "full-cli")]
// sigil: REPAIR
//! CEO_LOOP governance policy binding for autonomy action classes.
//!
//! This module is intentionally conservative: unknown or lifecycle/destructive
//! actions never gain mutation rights by default. They route to review unless a
//! later phase wires explicit, audited approval evidence.

use super::decomposer::{Objective, PlannedTask, Priority};
use arda_governance::{PhilosopherAction, TriadPhilosopherVerdict};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

pub const ACTION_CLASS_CONTRACT: &str = "arda.action_classification.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceGate {
    SafeAutonomous,
    TriadQuorumRequired,
    TriadQuorumApproved,
    HumanRequired,
    HadesReviewRequired,
    ReviewRequired,
    ReadOnlyBenchmarkRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriadGateScore {
    pub gate: String,
    pub passed: bool,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriadQuorumEvidence {
    pub source: String,
    pub query_id: String,
    pub outcome: String,
    pub resonance: f64,
    pub passed_gates: usize,
    pub total_gates: usize,
    pub quorum_ratio: f64,
    pub required_quorum_ratio: f64,
    pub required_pass_rate: f64,
    pub gate_scores: Vec<TriadGateScore>,
    pub concerns: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triad_philosopher: Option<TriadPhilosopherVerdict>,
}

impl TriadQuorumEvidence {
    pub fn satisfies(&self) -> bool {
        self.total_gates > 0
            && self.quorum_ratio >= self.required_quorum_ratio
            && self.resonance >= self.required_pass_rate
            && self.outcome == "pass"
            && self.triad_philosopher_allows_delegation()
    }

    pub fn triad_philosopher_allows_delegation(&self) -> bool {
        self.triad_philosopher
            .as_ref()
            .map(|verdict| verdict.action == PhilosopherAction::Proceed)
            .unwrap_or(true)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceDecision {
    pub contract: String,
    pub objective_id: String,
    pub action_class: String,
    pub gate: GovernanceGate,
    pub requires_human: bool,
    pub requires_triad: bool,
    pub requires_hades_review: bool,
    pub confidence: String,
    pub allowed_to_delegate: bool,
    pub reasons: Vec<String>,
    pub evidence: Vec<String>,
    pub triad_quorum: Option<TriadQuorumEvidence>,
}

impl GovernanceDecision {
    pub fn blocks_delegation(&self) -> bool {
        !self.allowed_to_delegate
    }

    pub fn requires_escalation(&self) -> bool {
        self.requires_human || matches!(self.gate, GovernanceGate::ReviewRequired)
    }
}

#[derive(Debug, Clone)]
pub struct GovernancePolicy {
    pub autonomy_score_threshold: f64,
    pub triad_quorum_ratio: f64,
    pub triad_required_pass_rate: f64,
    pub autonomous_classes: BTreeSet<String>,
    pub human_required_classes: BTreeSet<String>,
    pub triad_quorum_classes: BTreeSet<String>,
    pub hades_lifecycle_classes: BTreeSet<String>,
    pub read_only_benchmark_required: bool,
    pub evidence: Vec<String>,
}

impl Default for GovernancePolicy {
    fn default() -> Self {
        Self {
            autonomy_score_threshold: 0.65,
            triad_quorum_ratio: 0.66,
            triad_required_pass_rate: 0.45,
            autonomous_classes: set([
                "read_only_audit",
                "bounded_research",
                "documentation_indexing",
                "safe_exports",
                "routine_status_reporting",
                "non_destructive_benchmarking",
                "local_refactors",
                "routine_maintenance",
                "provider_status_check",
            ]),
            human_required_classes: set([
                "funds_movement",
                "legal_commitment",
                "human_identity_or_access_change",
                "destructive_delete",
                "fleet_reimage",
                "credential_rotation_or_disclosure",
                "external_customer_commitment_without_prior_scope",
                "service_disable",
            ]),
            triad_quorum_classes: set([
                "strategy_change",
                "provider_reroute",
                "pricing_change",
                "customer_commitment",
                "data_retention_change",
                "governance_policy_change",
                "autonomy_level_increase",
            ]),
            hades_lifecycle_classes: set([
                "archive_or_retention",
                "disposal_review",
                "quarantine_release",
                "supersession_marking",
                "generated_artifact_cleanup",
                "task_completion_disposal_boundary",
            ]),
            read_only_benchmark_required: true,
            evidence: vec!["default_governance_policy".into()],
        }
    }
}

impl GovernancePolicy {
    pub fn load_from_root(root: &Path) -> Self {
        let mut policy = Self::default();
        let path = root.join("core/state/governance_runtime.json");
        let Ok(raw) = std::fs::read_to_string(&path) else {
            policy
                .evidence
                .push(format!("missing:{}", display_path(&path)));
            return policy;
        };
        let Ok(value) = serde_json::from_str::<Value>(&raw) else {
            policy
                .evidence
                .push(format!("invalid_json:{}", display_path(&path)));
            return policy;
        };

        policy.evidence = vec![display_path(&path)];
        let active_policy = value
            .pointer("/contracts/active_ruleset/policy")
            .unwrap_or(&Value::Null);
        if let Some(v) = active_policy
            .get("autonomy_score_threshold")
            .and_then(Value::as_f64)
        {
            policy.autonomy_score_threshold = v;
        }
        if let Some(v) = active_policy
            .get("triad_required_pass_rate")
            .and_then(Value::as_f64)
        {
            policy.triad_required_pass_rate = v;
        }
        if let Some(v) = active_policy
            .pointer("/human_augmentation/consensus/triad_quorum_ratio")
            .and_then(Value::as_f64)
        {
            policy.triad_quorum_ratio = v;
        }
        if let Some(classes) = active_policy
            .pointer("/human_augmentation/critical_decision_routing/autonomous_classes")
            .and_then(Value::as_array)
        {
            policy.autonomous_classes.extend(strings(classes));
        }
        if let Some(classes) = active_policy
            .pointer("/human_augmentation/critical_decision_routing/human_required_classes")
            .and_then(Value::as_array)
        {
            policy.human_required_classes.extend(strings(classes));
        }
        if let Some(classes) = active_policy
            .pointer("/human_augmentation/critical_decision_routing/triad_quorum_classes")
            .and_then(Value::as_array)
        {
            policy.triad_quorum_classes.extend(strings(classes));
        }
        if active_policy
            .pointer("/task_lifecycle/rules/hades_controls_final_lifecycle_boundary")
            .and_then(Value::as_bool)
            == Some(true)
        {
            policy
                .hades_lifecycle_classes
                .insert("task_completion_disposal_boundary".into());
        }
        policy
    }

    pub fn classify_objective(
        &self,
        objective: &Objective,
        plan: &[PlannedTask],
    ) -> GovernanceDecision {
        self.classify_objective_with_triad_evidence(objective, plan, None)
    }

    pub fn classify_objective_with_triad_evidence(
        &self,
        objective: &Objective,
        plan: &[PlannedTask],
        triad_evidence: Option<TriadQuorumEvidence>,
    ) -> GovernanceDecision {
        let action_class = classify_action_class(objective, plan);
        self.decision_for_class(objective, action_class, plan, triad_evidence)
    }

    fn decision_for_class(
        &self,
        objective: &Objective,
        action_class: String,
        plan: &[PlannedTask],
        triad_evidence: Option<TriadQuorumEvidence>,
    ) -> GovernanceDecision {
        let mut reasons = Vec::new();
        let mut gate = GovernanceGate::ReviewRequired;
        let mut allowed_to_delegate = false;
        let mut requires_human = false;
        let mut requires_triad = false;
        let mut requires_hades_review = false;
        let confidence = if action_class == "review_required" {
            "low"
        } else {
            "high"
        }
        .to_string();

        if self.human_required_classes.contains(&action_class) {
            gate = GovernanceGate::HumanRequired;
            requires_human = true;
            reasons.push(format!(
                "action_class '{action_class}' requires human sovereign approval"
            ));
        } else if self.triad_quorum_classes.contains(&action_class) {
            requires_triad = true;
            if let Some(quorum) = triad_evidence.as_ref().filter(|quorum| quorum.satisfies()) {
                gate = GovernanceGate::TriadQuorumApproved;
                allowed_to_delegate = true;
                reasons.push(format!(
                    "action_class '{action_class}' has ORACLE triad quorum evidence ({}/{}, resonance {:.2})",
                    quorum.passed_gates, quorum.total_gates, quorum.resonance
                ));
            } else {
                gate = GovernanceGate::TriadQuorumRequired;
                allowed_to_delegate = false;
                if let Some(quorum) = triad_evidence.as_ref() {
                    reasons.push(format!(
                        "action_class '{action_class}' has insufficient ORACLE triad quorum ({}/{}, resonance {:.2})",
                        quorum.passed_gates, quorum.total_gates, quorum.resonance
                    ));
                    if let Some(verdict) = quorum
                        .triad_philosopher
                        .as_ref()
                        .filter(|verdict| verdict.action != PhilosopherAction::Proceed)
                    {
                        reasons.push(format!(
                            "Triad Philosopher requires {} before delegation: {}",
                            philosopher_action_label(verdict.action),
                            verdict.reason
                        ));
                    }
                    reasons.extend(quorum.concerns.iter().cloned());
                } else {
                    reasons.push(format!(
                        "action_class '{action_class}' requires ORACLE triad quorum evidence before delegation"
                    ));
                }
            }
        } else if self.hades_lifecycle_classes.contains(&action_class) {
            gate = GovernanceGate::HadesReviewRequired;
            requires_hades_review = true;
            reasons.push(format!(
                "action_class '{action_class}' is HADES-controlled and remains audit-only"
            ));
        } else if self.autonomous_classes.contains(&action_class) {
            gate = GovernanceGate::SafeAutonomous;
            allowed_to_delegate = true;
            reasons.push(format!(
                "action_class '{action_class}' is safe within configured bounds"
            ));
        } else {
            requires_human = true;
            reasons.push(format!(
                "action_class '{action_class}' is unknown or unconfigured; defaulting to review_required"
            ));
        }

        if requires_read_only_benchmark(plan) && !matches!(gate, GovernanceGate::HumanRequired) {
            gate = GovernanceGate::ReadOnlyBenchmarkRequired;
            allowed_to_delegate = false;
            reasons.push(
                "benchmark or audit objective must run through read-only benchmark gate first"
                    .into(),
            );
        }

        let mut evidence = self.evidence.clone();
        if let Some(quorum) = triad_evidence.as_ref() {
            evidence.push(format!(
                "oracle_quorum:{}:{}/{}:{:.2}",
                quorum.query_id, quorum.passed_gates, quorum.total_gates, quorum.resonance
            ));
            if let Some(verdict) = quorum.triad_philosopher.as_ref() {
                evidence.push(format!(
                    "triad_philosopher:{}:{:.2}",
                    philosopher_action_label(verdict.action),
                    verdict.alignment_score
                ));
            }
        }

        GovernanceDecision {
            contract: ACTION_CLASS_CONTRACT.into(),
            objective_id: objective.id.clone(),
            action_class,
            gate,
            requires_human,
            requires_triad,
            requires_hades_review,
            confidence,
            allowed_to_delegate,
            reasons,
            evidence,
            triad_quorum: triad_evidence,
        }
    }
}

pub fn classify_action_class(objective: &Objective, plan: &[PlannedTask]) -> String {
    for tag in &objective.tags {
        if let Some(class) = tag.strip_prefix("action_class:") {
            return class.trim().to_string();
        }
    }
    let text = searchable_text(objective, plan);
    let contains = |needles: &[&str]| needles.iter().any(|needle| text.contains(needle));

    if contains(&[
        "funds movement",
        "transfer funds",
        "send payment",
        "wire ",
        "pay invoice",
    ]) {
        "funds_movement".into()
    } else if contains(&[
        "legal commitment",
        "sign contract",
        "contractual",
        "terms of service",
    ]) {
        "legal_commitment".into()
    } else if contains(&[
        "credential",
        "rotate secret",
        "api key",
        "access change",
        "identity",
    ]) {
        "credential_rotation_or_disclosure".into()
    } else if contains(&["delete", "destructive", "remove permanently", "wipe "]) {
        "destructive_delete".into()
    } else if contains(&["reimage", "wipe host", "factory reset"]) {
        "fleet_reimage".into()
    } else if contains(&[
        "reroute provider",
        "provider reroute",
        "change provider routing",
    ]) {
        "provider_reroute".into()
    } else if contains(&[
        "strategy",
        "pricing",
        "customer commitment",
        "data retention",
        "governance policy",
        "increase autonomy",
    ]) {
        if text.contains("pricing") {
            "pricing_change".into()
        } else if text.contains("customer commitment") {
            "customer_commitment".into()
        } else if text.contains("data retention") {
            "data_retention_change".into()
        } else if text.contains("governance policy") {
            "governance_policy_change".into()
        } else if text.contains("increase autonomy") {
            "autonomy_level_increase".into()
        } else {
            "strategy_change".into()
        }
    } else if contains(&[
        "archive",
        "retention",
        "disposal",
        "quarantine",
        "supersede",
        "cleanup generated artifact",
    ]) {
        "archive_or_retention".into()
    } else if contains(&["benchmark", "read-only cycle", "read only cycle"]) {
        "non_destructive_benchmarking".into()
    } else if contains(&["status", "health", "provider status"]) {
        "provider_status_check".into()
    } else if contains(&["research", "investigate", "audit"]) {
        "bounded_research".into()
    } else if contains(&["document", "index", "summary"]) {
        "documentation_indexing".into()
    } else if contains(&["refactor", "test", "build", "local change"]) {
        "local_refactors".into()
    } else {
        "review_required".into()
    }
}

fn philosopher_action_label(action: PhilosopherAction) -> &'static str {
    match action {
        PhilosopherAction::Proceed => "proceed",
        PhilosopherAction::Revise => "revise",
        PhilosopherAction::Hold => "hold",
        PhilosopherAction::Reject => "reject",
    }
}

fn requires_read_only_benchmark(plan: &[PlannedTask]) -> bool {
    plan.iter().any(|task| {
        let haystack = format!("{} {}", task.task_type, task.title).to_lowercase();
        haystack.contains("benchmark")
            || haystack.contains("read-only")
            || haystack.contains("read only")
    })
}

fn searchable_text(objective: &Objective, plan: &[PlannedTask]) -> String {
    let mut parts = vec![objective.statement.clone()];
    parts.extend(objective.constraints.clone());
    parts.extend(objective.success_criteria.clone());
    parts.extend(objective.tags.clone());
    for task in plan {
        parts.push(task.title.clone());
        parts.push(task.task_type.clone());
        parts.extend(task.depends_on.clone());
        if task.priority == Priority::Critical {
            parts.push("critical".into());
        }
    }
    parts.join(" ").to_lowercase()
}

fn strings(values: &[Value]) -> impl Iterator<Item = String> + '_ {
    values.iter().filter_map(Value::as_str).map(str::to_string)
}

fn set<const N: usize>(items: [&str; N]) -> BTreeSet<String> {
    items.into_iter().map(str::to_string).collect()
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn obj(statement: &str) -> Objective {
        Objective {
            id: "obj1".into(),
            statement: statement.into(),
            constraints: Vec::new(),
            deadline: None,
            success_criteria: Vec::new(),
            tags: Vec::new(),
        }
    }

    fn tagged_obj(class: &str) -> Objective {
        let mut objective = obj("explicit class");
        objective.tags = vec![format!("action_class:{class}")];
        objective
    }

    fn task(task_type: &str, title: &str) -> PlannedTask {
        PlannedTask {
            key: "t1".into(),
            title: title.into(),
            task_type: task_type.into(),
            depends_on: Vec::new(),
            priority: Priority::Medium,
            joule_cost: 1.0,
            eta_seconds: 1,
            assigned_agent: Some("prometheus".into()),
        }
    }

    #[test]
    fn loads_policy_thresholds_and_classes_from_governance_runtime() {
        let dir = tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
        let state = dir.path().join("core/state");
        std::fs::create_dir_all(&state).unwrap_or_else(|err| panic!("mkdir failed: {err}"));
        std::fs::write(
            state.join("governance_runtime.json"),
            r#"{
              "contracts":{"active_ruleset":{"policy":{
                "autonomy_score_threshold":0.77,
                "triad_required_pass_rate":0.51,
                "human_augmentation":{"consensus":{"triad_quorum_ratio":0.67},
                  "critical_decision_routing":{
                    "autonomous_classes":["safe_exports"],
                    "human_required_classes":["legal_commitment"],
                    "triad_quorum_classes":["provider_reroute"]
                  }}
              }}}
            }"#,
        )
        .unwrap_or_else(|err| panic!("write policy failed: {err}"));
        let policy = GovernancePolicy::load_from_root(dir.path());
        assert_eq!(policy.autonomy_score_threshold, 0.77);
        assert_eq!(policy.triad_required_pass_rate, 0.51);
        assert!(policy.human_required_classes.contains("legal_commitment"));
        assert!(policy.triad_quorum_classes.contains("provider_reroute"));
    }

    #[test]
    fn human_required_class_blocks_delegation_and_routes_human() {
        let policy = GovernancePolicy::default();
        let decision = policy.classify_objective(&obj("transfer funds to vendor"), &[]);
        assert_eq!(decision.action_class, "funds_movement");
        assert_eq!(decision.gate, GovernanceGate::HumanRequired);
        assert!(decision.requires_human);
        assert!(!decision.allowed_to_delegate);
    }

    #[test]
    fn triad_class_requires_quorum_and_blocks_delegation_without_evidence() {
        let policy = GovernancePolicy::default();
        let decision = policy.classify_objective(&obj("reroute provider traffic"), &[]);
        assert_eq!(decision.action_class, "provider_reroute");
        assert_eq!(decision.gate, GovernanceGate::TriadQuorumRequired);
        assert!(decision.requires_triad);
        assert!(!decision.allowed_to_delegate);
        assert!(decision.triad_quorum.is_none());
    }

    #[test]
    fn triad_class_allows_delegation_with_oracle_quorum_evidence() {
        let policy = GovernancePolicy::default();
        let evidence = TriadQuorumEvidence {
            source: "oracle_gate".into(),
            query_id: "autopilot::obj1".into(),
            outcome: "pass".into(),
            resonance: 0.9,
            passed_gates: 2,
            total_gates: 3,
            quorum_ratio: 2.0 / 3.0,
            required_quorum_ratio: policy.triad_quorum_ratio,
            required_pass_rate: policy.triad_required_pass_rate,
            gate_scores: vec![
                TriadGateScore {
                    gate: "aurelius".into(),
                    passed: true,
                    score: 0.8,
                },
                TriadGateScore {
                    gate: "bacon".into(),
                    passed: true,
                    score: 0.7,
                },
                TriadGateScore {
                    gate: "sun_tzu".into(),
                    passed: false,
                    score: 0.4,
                },
            ],
            concerns: Vec::new(),
            triad_philosopher: None,
        };
        let decision = policy.classify_objective_with_triad_evidence(
            &obj("reroute provider traffic"),
            &[],
            Some(evidence),
        );
        assert_eq!(decision.action_class, "provider_reroute");
        assert_eq!(decision.gate, GovernanceGate::TriadQuorumApproved);
        assert!(decision.requires_triad);
        assert!(decision.allowed_to_delegate);
        assert!(decision
            .evidence
            .iter()
            .any(|item| item.contains("oracle_quorum:autopilot::obj1")));
        assert_eq!(
            decision
                .triad_quorum
                .as_ref()
                .map(|quorum| quorum.passed_gates),
            Some(2)
        );
    }

    #[test]
    fn triad_class_blocks_delegation_when_oracle_quorum_is_below_threshold() {
        let policy = GovernancePolicy::default();
        let evidence = TriadQuorumEvidence {
            source: "oracle_gate".into(),
            query_id: "autopilot::obj1".into(),
            outcome: "conditional".into(),
            resonance: 0.4,
            passed_gates: 1,
            total_gates: 3,
            quorum_ratio: 1.0 / 3.0,
            required_quorum_ratio: policy.triad_quorum_ratio,
            required_pass_rate: policy.triad_required_pass_rate,
            gate_scores: vec![
                TriadGateScore {
                    gate: "aurelius".into(),
                    passed: true,
                    score: 0.8,
                },
                TriadGateScore {
                    gate: "bacon".into(),
                    passed: false,
                    score: 0.2,
                },
                TriadGateScore {
                    gate: "sun_tzu".into(),
                    passed: false,
                    score: 0.3,
                },
            ],
            concerns: vec!["insufficient quorum".into()],
            triad_philosopher: None,
        };
        let decision = policy.classify_objective_with_triad_evidence(
            &obj("reroute provider traffic"),
            &[],
            Some(evidence),
        );
        assert_eq!(decision.gate, GovernanceGate::TriadQuorumRequired);
        assert!(decision.requires_triad);
        assert!(!decision.allowed_to_delegate);
        assert!(decision
            .reasons
            .iter()
            .any(|reason| reason.contains("insufficient ORACLE triad quorum")));
    }

    #[test]
    fn triad_class_blocks_delegation_when_philosopher_does_not_proceed() {
        let policy = GovernancePolicy::default();
        let evidence = TriadQuorumEvidence {
            source: "oracle_gate".into(),
            query_id: "autopilot::obj1".into(),
            outcome: "pass".into(),
            resonance: 0.9,
            passed_gates: 3,
            total_gates: 3,
            quorum_ratio: 1.0,
            required_quorum_ratio: policy.triad_quorum_ratio,
            required_pass_rate: policy.triad_required_pass_rate,
            gate_scores: vec![
                TriadGateScore {
                    gate: "aurelius".into(),
                    passed: true,
                    score: 0.8,
                },
                TriadGateScore {
                    gate: "bacon".into(),
                    passed: true,
                    score: 0.8,
                },
                TriadGateScore {
                    gate: "sun_tzu".into(),
                    passed: true,
                    score: 0.8,
                },
            ],
            concerns: Vec::new(),
            triad_philosopher: Some(arda_governance::TriadPhilosopherVerdict {
                action: arda_governance::PhilosopherAction::Hold,
                reason: "evidence grounding is insufficient for confident action".into(),
                alignment_score: 0.42,
                lifecycle: Default::default(),
            }),
        };

        let decision = policy.classify_objective_with_triad_evidence(
            &obj("reroute provider traffic"),
            &[],
            Some(evidence),
        );

        assert_eq!(decision.gate, GovernanceGate::TriadQuorumRequired);
        assert!(decision.requires_triad);
        assert!(!decision.allowed_to_delegate);
        assert!(decision.reasons.iter().any(|reason| {
            reason.contains("Triad Philosopher requires hold before delegation")
        }));
    }

    #[test]
    fn hades_lifecycle_class_remains_audit_only() {
        let policy = GovernancePolicy::default();
        let decision = policy.classify_objective(&obj("archive old human notes"), &[]);
        assert_eq!(decision.action_class, "archive_or_retention");
        assert_eq!(decision.gate, GovernanceGate::HadesReviewRequired);
        assert!(decision.requires_hades_review);
        assert!(!decision.allowed_to_delegate);
    }

    #[test]
    fn unknown_class_defaults_to_review_required() {
        let policy = GovernancePolicy::default();
        let decision = policy.classify_objective(&tagged_obj("mystery_action"), &[]);
        assert_eq!(decision.gate, GovernanceGate::ReviewRequired);
        assert!(decision.requires_human);
        assert!(!decision.allowed_to_delegate);
    }

    #[test]
    fn benchmark_task_requires_read_only_gate() {
        let policy = GovernancePolicy::default();
        let decision = policy.classify_objective(
            &obj("run autonomy benchmark"),
            &[task("benchmark", "CEO_LOOP read-only cycle benchmark")],
        );
        assert_eq!(decision.gate, GovernanceGate::ReadOnlyBenchmarkRequired);
        assert!(!decision.allowed_to_delegate);
    }
}
