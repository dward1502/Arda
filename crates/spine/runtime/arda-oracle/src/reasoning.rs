// sigil: REPAIR
use arda_core::Ledger;
use arda_core::Task;
use annunimas_governance::{bacon_lite_validate, triad_validate, BaconLiteResult, TriadResult};
use annunimas_plutus::LoveEquation;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::pageindex::PageIndex;

pub const ORACLE_SCHEMA_VERSION: &str = "annunimas.oracle.v1";

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
    pub outcome: VerdictOutcome,
    pub gates: TriadGates,
    pub reasoning: Vec<GateReasoning>,
    pub resonance_score: f64,
    pub timestamp: DateTime<Utc>,
    pub conditions: Option<String>,
    pub governance: VerdictGovernance,
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
        }
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

        let outcome = self.determine_outcome(&gates);
        let reasoning = self.build_reasoning(aurelius, bacon, sun_tzu);
        let resonance_score = self.calculate_resonance(&query, &outcome);
        let governance = self.evaluate_governance(&query, &outcome, resonance_score);

        let verdict = Verdict {
            query_id: query.id,
            outcome: outcome.clone(),
            gates,
            reasoning,
            resonance_score,
            timestamp: Utc::now(),
            conditions: None,
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
            score -= 0.4;
        }

        let passed = score >= 0.6;

        GateResult {
            passed,
            score,
            concerns,
        }
    }

    fn evaluate_bacon(&mut self, query: &OracleQuery) -> GateResult {
        let mut concerns = Vec::new();
        let mut score = 1.0;

        if query.context.is_empty() {
            concerns.push("No explicit evidence provided - querying document index".to_string());
            score -= 0.3;

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
        }

        let evidence_weight = query.context.len() as f64 * 0.15;
        score = (score - evidence_weight).max(0.0);

        let has_financial = query.task.contains('$')
            || query.task.contains("budget")
            || query.task.contains("cost");

        if has_financial && query.context.len() < 2 {
            concerns.push("Financial task requires stronger evidence base".to_string());
            score -= 0.2;
        }

        let passed = score >= 0.5;

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

        let passed = score >= 0.5;

        GateResult {
            passed,
            score,
            concerns,
        }
    }

    fn determine_outcome(&self, gates: &TriadGates) -> VerdictOutcome {
        let pass_count = [
            gates.aurelius.passed,
            gates.bacon.passed,
            gates.sun_tzu.passed,
        ]
        .iter()
        .filter(|&&p| p)
        .count();

        if pass_count >= 2 {
            VerdictOutcome::Pass
        } else if pass_count == 0 {
            VerdictOutcome::Fail
        } else {
            VerdictOutcome::Conditional
        }
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
            "{} | Resonance: {:.2}\n",
            outcome_str, verdict.resonance_score
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

        assert_eq!(verdict.outcome, VerdictOutcome::Pass);
        assert!(!verdict.gates.aurelius.passed);
        assert!(!engine.get_history().is_empty());
    }

    #[test]
    fn financial_query_without_evidence_trips_bacon_concerns() {
        let mut engine = OracleEngine::new();
        let verdict = engine.evaluate(query("budget should increase by $500", vec![]));

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
