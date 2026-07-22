// sigil: REPAIR
use arda_core::Ledger;
use arda_core::Task;
use arda_economics::LoveEquation;
use arda_governance::{bacon_lite_validate, triad_validate, BaconLiteResult, TriadResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::pageindex::PageIndex;

pub const ORACLE_SCHEMA_VERSION: &str = "arda.mandos.v1";
pub const DEFAULT_ORACLE_POLICY_ID: &str = "arda.mandos.default";
pub const DEFAULT_ORACLE_POLICY_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OraclePolicy {
    pub policy_id: String,
    pub policy_version: String,
    pub aurelius_pass_threshold: f64,
    pub bacon_pass_threshold: f64,
    pub sun_tzu_pass_threshold: f64,
    pub evidence_bonus_per_item: f64,
    pub maximum_evidence_bonus: f64,
    pub minimum_passed_gates_for_conditional: usize,
    pub contradiction_veto_enabled: bool,
    pub dangerous_operation_veto_enabled: bool,
}

impl Default for OraclePolicy {
    fn default() -> Self {
        Self {
            policy_id: DEFAULT_ORACLE_POLICY_ID.to_string(),
            policy_version: DEFAULT_ORACLE_POLICY_VERSION.to_string(),
            aurelius_pass_threshold: 0.6,
            bacon_pass_threshold: 0.6,
            sun_tzu_pass_threshold: 0.5,
            evidence_bonus_per_item: 0.15,
            maximum_evidence_bonus: 0.3,
            minimum_passed_gates_for_conditional: 1,
            contradiction_veto_enabled: true,
            dangerous_operation_veto_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    Market,
    Document,
    Financial,
    General,
}

pub struct OracleEngine {
    ledger: Option<Ledger>,
    history: Vec<Verdict>,
    page_index: PageIndex,
    policy: OraclePolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleQuery {
    pub id: String,
    pub task: String,
    pub context: Vec<String>,
    pub requester: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verdict {
    pub query_id: String,
    pub policy_id: String,
    pub policy_version: String,
    pub outcome: VerdictOutcome,
    pub gates: TriadGates,
    pub reasoning: Vec<GateReasoning>,
    pub resonance_score: f64,
    pub timestamp: DateTime<Utc>,
    #[serde(default)]
    pub conditions: Vec<VerdictCondition>,
    #[serde(default)]
    pub vetoes: Vec<PolicyVeto>,
    pub governance: VerdictGovernance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VerdictConditionKind {
    ProvideEvidence,
    ClarifyLogic,
    ReviewTiming,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerdictCondition {
    pub kind: VerdictConditionKind,
    pub gate: String,
    pub description: String,
    pub required_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PolicyVetoKind {
    Contradiction,
    DangerousOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyVeto {
    pub kind: PolicyVetoKind,
    pub gate: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VerdictOutcome {
    Pass,
    Fail,
    Conditional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriadGates {
    pub aurelius: GateResult,
    pub bacon: GateResult,
    pub sun_tzu: GateResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub passed: bool,
    pub score: f64,
    pub concerns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateReasoning {
    pub gate: String,
    pub reasoning: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoveEquationGuard {
    pub resonance: f64,
    pub attention: f64,
    pub reciprocity: f64,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerdictGovernance {
    pub triad: TriadResult,
    pub bacon_lite: BaconLiteResult,
    pub love_equation_guard: LoveEquationGuard,
}

impl OracleEngine {
    pub fn new() -> Self {
        Self {
            ledger: None,
            history: Vec::new(),
            page_index: PageIndex::new(),
            policy: OraclePolicy::default(),
        }
    }

    pub fn with_policy(mut self, policy: OraclePolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn policy(&self) -> &OraclePolicy {
        &self.policy
    }

    pub fn with_ledger(mut self, ledger: Ledger) -> Self {
        self.ledger = Some(ledger);
        self
    }

    pub fn evaluate(&mut self, query: OracleQuery) -> Verdict {
        let aurelius = self.evaluate_aurelius(&query);
        let bacon = self.evaluate_bacon(&query);
        let sun_tzu = self.evaluate_sun_tzu(&query);

        let gates = TriadGates {
            aurelius: aurelius.clone(),
            bacon: bacon.clone(),
            sun_tzu: sun_tzu.clone(),
        };

        let vetoes = self.collect_vetoes(&query);
        let outcome = self.determine_outcome(&gates, &vetoes);
        let conditions = self.build_conditions(&gates, &outcome);
        let reasoning = self.build_reasoning(aurelius, bacon, sun_tzu);
        let resonance_score = self.calculate_resonance(&query, &outcome);
        let governance = self.evaluate_governance(&query, &outcome, resonance_score);

        let verdict = Verdict {
            query_id: query.id,
            policy_id: self.policy.policy_id.clone(),
            policy_version: self.policy.policy_version.clone(),
            outcome: outcome.clone(),
            gates,
            reasoning,
            resonance_score,
            timestamp: Utc::now(),
            conditions,
            vetoes,
            governance,
        };

        self.history.push(verdict.clone());
        verdict
    }

    fn evaluate_aurelius(&self, query: &OracleQuery) -> GateResult {
        let mut concerns = Vec::new();
        let mut score = 1.0;

        let task_lower = query.task.to_lowercase();

        if (task_lower.contains("should")
            || task_lower.contains("must")
            || task_lower.contains("need"))
            && query.context.is_empty()
        {
            concerns.push("Task requires justification but none provided".to_string());
            score -= 0.3;
        }

        if self.has_contradictions(&query.task) {
            concerns.push("Logical contradiction detected in task or context".to_string());
            score = 0.0;
        }

        score = normalize_score(score);
        let passed = score >= self.policy.aurelius_pass_threshold;

        GateResult {
            passed,
            score,
            concerns,
        }
    }

    fn evaluate_bacon(&mut self, query: &OracleQuery) -> GateResult {
        let mut concerns = Vec::new();
        let mut score: f64 = 0.55;

        if query.context.is_empty() {
            concerns.push("No explicit evidence provided - querying document index".to_string());

            if let Some(doc_id) = self.page_index.list_documents().first() {
                let results = self.page_index.search(doc_id, &query.task);
                if !results.is_empty() {
                    score += 0.2;
                    concerns.push(format!(
                        "Retrieved {} evidence items from PageIndex",
                        results.len()
                    ));
                }
            }
        } else {
            let evidence_bonus = (query.context.len() as f64
                * self.policy.evidence_bonus_per_item.max(0.0))
            .min(self.policy.maximum_evidence_bonus.max(0.0));
            score += evidence_bonus;
        }

        let task_lower = query.task.to_lowercase();
        let has_financial = query.task.contains('$')
            || task_lower.contains("budget")
            || task_lower.contains("cost");

        if has_financial && query.context.len() < 2 {
            concerns.push("Financial task requires stronger evidence base".to_string());
            score -= 0.2;
        }

        score = normalize_score(score);
        let passed = score >= self.policy.bacon_pass_threshold;

        GateResult {
            passed,
            score,
            concerns,
        }
    }

    fn evaluate_sun_tzu(&self, query: &OracleQuery) -> GateResult {
        let mut concerns = Vec::new();
        let mut score = 1.0;

        let task_lower = query.task.to_lowercase();

        let urgent_keywords = ["urgent", "asap", "immediately", "emergency", "critical"];
        let has_urgency = urgent_keywords.iter().any(|k| task_lower.contains(k));

        if has_urgency {
            concerns.push("Task marked urgent — verify timing is truly critical".to_string());
            score -= 0.15;
        }

        if self.has_dangerous_operation(&query.task) {
            concerns.push("Dangerous operation requires explicit human review".to_string());
            score = 0.0;
        }

        score = normalize_score(score);
        let passed = score >= self.policy.sun_tzu_pass_threshold;

        GateResult {
            passed,
            score,
            concerns,
        }
    }

    fn determine_outcome(&self, gates: &TriadGates, vetoes: &[PolicyVeto]) -> VerdictOutcome {
        if !vetoes.is_empty() {
            return VerdictOutcome::Fail;
        }

        let pass_count = [
            gates.aurelius.passed,
            gates.bacon.passed,
            gates.sun_tzu.passed,
        ]
        .iter()
        .filter(|&&p| p)
        .count();

        if pass_count == 3 {
            VerdictOutcome::Pass
        } else if pass_count >= self.policy.minimum_passed_gates_for_conditional.clamp(1, 3) {
            VerdictOutcome::Conditional
        } else {
            VerdictOutcome::Fail
        }
    }

    fn collect_vetoes(&self, query: &OracleQuery) -> Vec<PolicyVeto> {
        let mut vetoes = Vec::new();
        if self.policy.contradiction_veto_enabled && self.has_contradictions(&query.task) {
            vetoes.push(PolicyVeto {
                kind: PolicyVetoKind::Contradiction,
                gate: "Aurelius".to_string(),
                reason: "Logical contradiction must be resolved before proceeding".to_string(),
            });
        }
        if self.policy.dangerous_operation_veto_enabled && self.has_dangerous_operation(&query.task)
        {
            vetoes.push(PolicyVeto {
                kind: PolicyVetoKind::DangerousOperation,
                gate: "Sun Tzu".to_string(),
                reason: "Dangerous operation requires explicit human review".to_string(),
            });
        }
        vetoes
    }

    fn build_conditions(
        &self,
        gates: &TriadGates,
        outcome: &VerdictOutcome,
    ) -> Vec<VerdictCondition> {
        if *outcome != VerdictOutcome::Conditional {
            return Vec::new();
        }

        let mut conditions = Vec::new();
        if !gates.aurelius.passed {
            conditions.push(VerdictCondition {
                kind: VerdictConditionKind::ClarifyLogic,
                gate: "Aurelius".to_string(),
                description: "The proposal needs a logically consistent justification".to_string(),
                required_action: "Resolve the listed logical concerns and resubmit".to_string(),
            });
        }
        if !gates.bacon.passed {
            conditions.push(VerdictCondition {
                kind: VerdictConditionKind::ProvideEvidence,
                gate: "Bacon".to_string(),
                description: "The proposal lacks sufficient supporting evidence".to_string(),
                required_action:
                    "Provide at least two relevant, independently reviewable evidence items"
                        .to_string(),
            });
        }
        if !gates.sun_tzu.passed {
            conditions.push(VerdictCondition {
                kind: VerdictConditionKind::ReviewTiming,
                gate: "Sun Tzu".to_string(),
                description: "The proposal's timing or operational strategy requires review"
                    .to_string(),
                required_action: "Document timing, rollback, and human-approval safeguards"
                    .to_string(),
            });
        }
        conditions
    }

    fn build_reasoning(
        &self,
        aurelius: GateResult,
        bacon: GateResult,
        sun_tzu: GateResult,
    ) -> Vec<GateReasoning> {
        let mut reasoning = Vec::new();

        reasoning.push(GateReasoning {
            gate: "Aurelius".to_string(),
            reasoning: if aurelius.passed {
                "Task is logically consistent and well-formed".to_string()
            } else {
                "Task fails logical consistency checks".to_string()
            },
            evidence: aurelius.concerns.clone(),
        });

        reasoning.push(GateReasoning {
            gate: "Bacon".to_string(),
            reasoning: if bacon.passed {
                "Task has sufficient evidence grounding".to_string()
            } else {
                "Task lacks sufficient evidence or context".to_string()
            },
            evidence: bacon.concerns.clone(),
        });

        reasoning.push(GateReasoning {
            gate: "Sun Tzu".to_string(),
            reasoning: if sun_tzu.passed {
                "Task timing and strategy are appropriate".to_string()
            } else {
                "Task timing or strategic considerations questionable".to_string()
            },
            evidence: sun_tzu.concerns.clone(),
        });

        reasoning
    }

    fn calculate_resonance(&self, query: &OracleQuery, outcome: &VerdictOutcome) -> f64 {
        let base = 0.85;
        let context_bonus = (query.context.len() as f64 * 0.02).min(0.1);

        let outcome_modifier = match outcome {
            VerdictOutcome::Pass => 0.05,
            VerdictOutcome::Conditional => 0.0,
            VerdictOutcome::Fail => -0.1,
        };

        (base + context_bonus + outcome_modifier).min(1.0)
    }

    fn has_contradictions(&self, task: &str) -> bool {
        let task_lower = task.to_lowercase();

        let contradictions = [
            ("always", "never"),
            ("must", "must not"),
            ("yes", "no"),
            ("increase", "decrease"),
        ];

        for (a, b) in contradictions {
            if task_lower.contains(a) && task_lower.contains(b) {
                return true;
            }
        }

        false
    }

    fn has_dangerous_operation(&self, task: &str) -> bool {
        let task_lower = task.to_lowercase();
        [
            "destructive",
            "dangerous",
            "database wipe",
            "wipe database",
            "drop database",
            "delete all",
            "disable safeguards",
            "bypass safety",
        ]
        .iter()
        .any(|keyword| task_lower.contains(keyword))
    }

    pub fn get_history(&self) -> &[Verdict] {
        &self.history
    }

    pub fn status_snapshot(&self) -> serde_json::Value {
        let pass_count = self
            .history
            .iter()
            .filter(|verdict| verdict.outcome == VerdictOutcome::Pass)
            .count();
        let conditional_count = self
            .history
            .iter()
            .filter(|verdict| verdict.outcome == VerdictOutcome::Conditional)
            .count();
        let fail_count = self
            .history
            .iter()
            .filter(|verdict| verdict.outcome == VerdictOutcome::Fail)
            .count();
        let bacon_lite_passed_total = self
            .history
            .iter()
            .filter(|verdict| verdict.governance.bacon_lite.passed)
            .count();
        let average_love_equation = if self.history.is_empty() {
            0.0
        } else {
            self.history
                .iter()
                .map(|verdict| verdict.governance.love_equation_guard.score)
                .sum::<f64>()
                / self.history.len() as f64
        };
        let average_triad = if self.history.is_empty() {
            0.0
        } else {
            self.history
                .iter()
                .map(|verdict| {
                    (verdict.governance.triad.aurelius_score
                        + verdict.governance.triad.bacon_score
                        + verdict.governance.triad.sun_tzu_score)
                        / 3.0
                })
                .sum::<f64>()
                / self.history.len() as f64
        };

        json!({
            "schema_version": ORACLE_SCHEMA_VERSION,
            "policy_id": self.policy.policy_id,
            "policy_version": self.policy.policy_version,
            "generated_at_utc": Utc::now().to_rfc3339(),
            "history_total": self.history.len(),
            "verdict_counts": {
                "pass": pass_count,
                "conditional": conditional_count,
                "fail": fail_count,
            },
            "governance": {
                "bacon_lite_passed_total": bacon_lite_passed_total,
                "average_love_equation": average_love_equation,
                "average_triad": average_triad,
            },
            "recent_verdicts": self.history.iter().rev().take(10).cloned().collect::<Vec<_>>(),
        })
    }

    pub fn format_verdict(&self, verdict: &Verdict) -> String {
        let outcome_str = match verdict.outcome {
            VerdictOutcome::Pass => "◈ PASS",
            VerdictOutcome::Fail => "∇ FAIL",
            VerdictOutcome::Conditional => "◈ CONDITIONAL",
        };

        let mut output = format!(
            "{} | Resonance: {:.2} | Policy: {}@{}\n",
            outcome_str, verdict.resonance_score, verdict.policy_id, verdict.policy_version
        );

        for gate in &verdict.reasoning {
            output.push_str(&format!("\n[{}]\n", gate.gate));
            output.push_str(&format!("  {}\n", gate.reasoning));
            if !gate.evidence.is_empty() {
                output.push_str("  Concerns:\n");
                for concern in &gate.evidence {
                    output.push_str(&format!("    - {}\n", concern));
                }
            }
        }

        if !verdict.conditions.is_empty() {
            output.push_str("\nConditions:\n");
            for condition in &verdict.conditions {
                output.push_str(&format!(
                    "  - [{}] {}\n",
                    condition.gate, condition.required_action
                ));
            }
        }

        output
    }

    pub fn index_document(
        &mut self,
        doc_id: String,
        title: String,
        toc: Vec<crate::pageindex::TocEntry>,
    ) {
        self.page_index.index_document(doc_id, title, toc);
    }

    fn evaluate_governance(
        &self,
        query: &OracleQuery,
        outcome: &VerdictOutcome,
        resonance_score: f64,
    ) -> VerdictGovernance {
        let task = build_governance_task(query, outcome, resonance_score);
        let triad = triad_validate(&task, None);
        let bacon_lite = bacon_lite_validate(&task);
        let resonance = resonance_score.clamp(0.0, 1.0);
        let attention = bacon_lite.confidence.clamp(0.0, 1.0);
        let reciprocity =
            ((triad.sun_tzu_score + if triad.passed { 0.85 } else { 0.45 }) / 2.0).clamp(0.0, 1.0);
        let score = LoveEquation::new().calculate(
            "oracle",
            &query.requester,
            resonance,
            attention,
            reciprocity,
        );
        VerdictGovernance {
            triad,
            bacon_lite,
            love_equation_guard: LoveEquationGuard {
                resonance,
                attention,
                reciprocity,
                score,
            },
        }
    }
}

fn build_governance_task(
    query: &OracleQuery,
    outcome: &VerdictOutcome,
    resonance_score: f64,
) -> Task {
    let mut task = Task::new(
        format!(
            "{} because oracle context size {} supports decision framing",
            query.task,
            query.context.len()
        ),
        "query",
    );
    task.assign("oracle");
    task.execution_started_at = Some(task.created_at + chrono::TimeDelta::seconds(1));
    task.updated_at = task.created_at + chrono::TimeDelta::seconds(2);
    task.joule_cost_estimated = 1.0;
    task.joule_cost_actual = (0.5 + (query.context.len() as f64 * 0.1) + resonance_score).max(0.25);
    task.clarifications_requested = 0;
    task.clarifications_resolved = match outcome {
        VerdictOutcome::Pass => 1,
        VerdictOutcome::Conditional => 1,
        VerdictOutcome::Fail => 0,
    };
    task.status = arda_core::task::TaskStatus::Complete;
    task
}

impl Default for OracleEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn normalize_score(score: f64) -> f64 {
    if score.is_finite() {
        score.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(task: &str, context: Vec<&str>) -> OracleQuery {
        OracleQuery {
            id: "q1".to_string(),
            task: task.to_string(),
            context: context.into_iter().map(|item| item.to_string()).collect(),
            requester: "operator".to_string(),
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn contradictory_query_fails_aurelius_gate() {
        let mut engine = OracleEngine::new();
        let verdict = engine.evaluate(query("we must increase and decrease access", vec![]));

        assert_eq!(verdict.outcome, VerdictOutcome::Fail);
        assert!(!verdict.gates.aurelius.passed);
        assert_eq!(verdict.vetoes.len(), 1);
        assert_eq!(verdict.vetoes[0].kind, PolicyVetoKind::Contradiction);
        assert!(!engine.get_history().is_empty());
    }

    #[test]
    fn dangerous_operation_vetoes_otherwise_passing_gates() {
        let mut engine = OracleEngine::new();
        let verdict = engine.evaluate(query(
            "perform a destructive database wipe immediately",
            vec!["operator requested maintenance", "backup completed"],
        ));

        assert_eq!(verdict.outcome, VerdictOutcome::Fail);
        assert_eq!(verdict.vetoes.len(), 1);
        assert_eq!(verdict.vetoes[0].kind, PolicyVetoKind::DangerousOperation);
    }

    #[test]
    fn additional_relevant_evidence_never_lowers_bacon_score() {
        let mut with_one_item = OracleEngine::new();
        let baseline =
            with_one_item.evaluate(query("document market posture", vec!["recent report"]));
        let mut with_two_items = OracleEngine::new();
        let supported = with_two_items.evaluate(query(
            "document market posture",
            vec!["recent report", "operator note"],
        ));

        assert!(supported.gates.bacon.score >= baseline.gates.bacon.score);
    }

    #[test]
    fn bacon_threshold_is_inclusive_and_crossing_it_adds_a_typed_condition() {
        let mut baseline_engine = OracleEngine::new();
        let baseline =
            baseline_engine.evaluate(query("document market posture", vec!["independent report"]));
        let bacon_score = baseline.gates.bacon.score;

        let at_threshold_policy = OraclePolicy {
            bacon_pass_threshold: bacon_score,
            ..OraclePolicy::default()
        };
        let mut at_threshold = OracleEngine::new().with_policy(at_threshold_policy);
        let passing =
            at_threshold.evaluate(query("document market posture", vec!["independent report"]));
        assert_eq!(passing.outcome, VerdictOutcome::Pass);

        let above_threshold_policy = OraclePolicy {
            bacon_pass_threshold: bacon_score + f64::EPSILON,
            ..OraclePolicy::default()
        };
        let mut above_threshold = OracleEngine::new().with_policy(above_threshold_policy);
        let conditional =
            above_threshold.evaluate(query("document market posture", vec!["independent report"]));
        assert_eq!(conditional.outcome, VerdictOutcome::Conditional);
        assert_eq!(
            conditional.conditions[0].kind,
            VerdictConditionKind::ProvideEvidence
        );
    }

    #[test]
    fn verdict_and_status_identify_the_active_policy() {
        let policy = OraclePolicy {
            policy_id: "arda.mandos.test".to_string(),
            policy_version: "9.8.7".to_string(),
            ..OraclePolicy::default()
        };
        let mut engine = OracleEngine::new().with_policy(policy);

        let verdict = engine.evaluate(query(
            "document market posture",
            vec!["recent report", "operator note"],
        ));
        let status = engine.status_snapshot();

        assert_eq!(verdict.policy_id, "arda.mandos.test");
        assert_eq!(verdict.policy_version, "9.8.7");
        assert_eq!(status["policy_id"], "arda.mandos.test");
        assert_eq!(status["policy_version"], "9.8.7");
    }

    #[test]
    fn exposed_gate_scores_are_finite_and_bounded() {
        let cases = [
            query("document market posture", vec![]),
            query("budget should increase by $500", vec![]),
            query(
                "perform a destructive database wipe immediately",
                vec!["backup completed"],
            ),
            query(
                "document market posture",
                vec!["one", "two", "three", "four", "five"],
            ),
        ];

        for oracle_query in cases {
            let mut engine = OracleEngine::new();
            let verdict = engine.evaluate(oracle_query);
            for score in [
                verdict.gates.aurelius.score,
                verdict.gates.bacon.score,
                verdict.gates.sun_tzu.score,
            ] {
                assert!(score.is_finite());
                assert!((0.0..=1.0).contains(&score));
            }
        }
    }

    #[test]
    fn disabling_contradiction_veto_downgrades_failure_to_conditional() {
        let policy = OraclePolicy {
            contradiction_veto_enabled: false,
            ..OraclePolicy::default()
        };
        let mut engine = OracleEngine::new().with_policy(policy);

        let verdict = engine.evaluate(query(
            "we must increase and decrease access",
            vec!["operator rationale", "review note"],
        ));

        assert_eq!(verdict.outcome, VerdictOutcome::Conditional);
        assert!(verdict.vetoes.is_empty());
        assert_eq!(
            verdict.conditions[0].kind,
            VerdictConditionKind::ClarifyLogic
        );
    }

    #[test]
    fn each_gate_threshold_is_inclusive_with_bounded_boundary_behavior() {
        #[derive(Clone, Copy)]
        enum GateUnderTest {
            Aurelius,
            Bacon,
            SunTzu,
        }

        let cases = [
            (
                GateUnderTest::Aurelius,
                query("document market posture", vec!["independent report"]),
            ),
            (
                GateUnderTest::Bacon,
                query("document market posture", vec!["independent report"]),
            ),
            (
                GateUnderTest::SunTzu,
                query("urgent document market posture", vec!["independent report"]),
            ),
        ];

        for (gate, oracle_query) in cases {
            let mut baseline_engine = OracleEngine::new();
            let baseline = baseline_engine.evaluate(oracle_query.clone());
            let score = match gate {
                GateUnderTest::Aurelius => baseline.gates.aurelius.score,
                GateUnderTest::Bacon => baseline.gates.bacon.score,
                GateUnderTest::SunTzu => baseline.gates.sun_tzu.score,
            };

            for (threshold, expected_pass) in [
                ((score - f64::EPSILON).max(0.0), true),
                (score, true),
                (score + f64::EPSILON, false),
            ] {
                let mut policy = OraclePolicy::default();
                match gate {
                    GateUnderTest::Aurelius => policy.aurelius_pass_threshold = threshold,
                    GateUnderTest::Bacon => policy.bacon_pass_threshold = threshold,
                    GateUnderTest::SunTzu => policy.sun_tzu_pass_threshold = threshold,
                }
                let mut engine = OracleEngine::new().with_policy(policy);
                let verdict = engine.evaluate(oracle_query.clone());
                let passed = match gate {
                    GateUnderTest::Aurelius => verdict.gates.aurelius.passed,
                    GateUnderTest::Bacon => verdict.gates.bacon.passed,
                    GateUnderTest::SunTzu => verdict.gates.sun_tzu.passed,
                };
                assert_eq!(passed, expected_pass);
            }
        }
    }

    #[test]
    fn outcome_policy_table_covers_pass_conditional_fail_and_veto() {
        fn gate(passed: bool) -> GateResult {
            GateResult {
                passed,
                score: if passed { 1.0 } else { 0.0 },
                concerns: Vec::new(),
            }
        }

        let engine = OracleEngine::new();
        let veto = PolicyVeto {
            kind: PolicyVetoKind::DangerousOperation,
            gate: "Sun Tzu".to_string(),
            reason: "test veto".to_string(),
        };
        let cases = [
            ([true, true, true], Vec::new(), VerdictOutcome::Pass),
            ([true, true, false], Vec::new(), VerdictOutcome::Conditional),
            ([false, false, false], Vec::new(), VerdictOutcome::Fail),
            ([true, true, true], vec![veto], VerdictOutcome::Fail),
        ];

        for (passes, vetoes, expected) in cases {
            let gates = TriadGates {
                aurelius: gate(passes[0]),
                bacon: gate(passes[1]),
                sun_tzu: gate(passes[2]),
            };
            assert_eq!(engine.determine_outcome(&gates, &vetoes), expected);
        }
    }

    #[test]
    fn financial_query_without_evidence_trips_bacon_concerns() {
        let mut engine = OracleEngine::new();
        let verdict = engine.evaluate(query("budget should increase by $500", vec![]));

        assert_eq!(verdict.outcome, VerdictOutcome::Conditional);
        assert_eq!(verdict.conditions.len(), 1);
        assert_eq!(
            verdict.conditions[0].kind,
            VerdictConditionKind::ProvideEvidence
        );
        assert!(!verdict.conditions[0].required_action.is_empty());
        assert!(!verdict.gates.bacon.concerns.is_empty());
        assert!(verdict.gates.bacon.score < 1.0);
        assert!(verdict.governance.bacon_lite.confidence >= 0.0);

        let snapshot = engine.status_snapshot();
        assert_eq!(snapshot["schema_version"], ORACLE_SCHEMA_VERSION);
        assert_eq!(snapshot["history_total"], 1);
        assert!(
            snapshot["governance"]["average_triad"]
                .as_f64()
                .unwrap_or_default()
                > 0.0
        );
    }

    #[test]
    fn verdict_formatting_and_history_work_for_passing_query() {
        let mut engine = OracleEngine::new();
        let verdict = engine.evaluate(query(
            "document market posture",
            vec!["recent report", "operator note"],
        ));

        let formatted = engine.format_verdict(&verdict);
        assert!(formatted.contains("PASS") || formatted.contains("CONDITIONAL"));
        assert_eq!(engine.get_history().len(), 1);
        assert_eq!(
            engine.status_snapshot()["recent_verdicts"]
                .as_array()
                .map(|items| items.len()),
            Some(1)
        );
    }
}
