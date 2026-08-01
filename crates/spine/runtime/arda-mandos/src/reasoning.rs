// sigil: REPAIR
use arda_core::Ledger;
use arda_core::Task;
use arda_economics::LoveEquation;
use arda_governance::{bacon_lite_validate, triad_validate, BaconLiteResult, TriadResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;

use crate::context::{
    ReasoningContext, ReasoningContextError, ReasoningEdgeType, ReasoningNodeKind,
};
use crate::evidence::{
    EvidenceAssessment, EvidenceDisposition, EvidenceFreshness, EvidenceIndependence,
    EvidenceIntegrity, EvidenceKind, EvidenceRef, EvidenceSignal, EvidenceSignalKind,
    EvidenceStance,
};
use crate::pageindex::{IndexingReport, PageIndex};

pub const ORACLE_SCHEMA_VERSION: &str = "arda.mandos.v3";
pub const DEFAULT_ORACLE_POLICY_ID: &str = "arda.mandos.default";
pub const DEFAULT_ORACLE_POLICY_VERSION: &str = "1.1.0";
pub const MAX_QUERY_ID_BYTES: usize = 128;
pub const MAX_QUERY_TASK_BYTES: usize = 8 * 1024;
pub const MAX_QUERY_REQUESTER_BYTES: usize = 128;
pub const MAX_QUERY_CONTEXT_ITEMS: usize = 64;
pub const MAX_QUERY_CONTEXT_ITEM_BYTES: usize = 8 * 1024;

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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueryType {
    Market,
    Document,
    Financial,
    #[default]
    General,
}

pub struct OracleEngine {
    ledger: Option<Ledger>,
    history: Vec<Verdict>,
    pub history_queries: HashMap<String, OracleQuery>,
    history_request_digests: HashMap<String, String>,
    page_index: PageIndex,
    policy: OraclePolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OracleQuery {
    pub id: String,
    pub task: String,
    pub context: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<EvidenceRef>,
    pub requester: String,
    pub timestamp: DateTime<Utc>,
    #[serde(default)]
    pub query_type: QueryType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub causation_id: Option<String>,
}

impl OracleQuery {
    pub fn new(
        id: impl Into<String>,
        task: impl Into<String>,
        requester: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            task: task.into(),
            context: Vec::new(),
            evidence: Vec::new(),
            requester: requester.into(),
            timestamp: Utc::now(),
            query_type: QueryType::General,
            correlation_id: None,
            causation_id: None,
        }
    }

    pub fn validate(&self) -> Result<(), OracleQueryError> {
        validate_required("id", &self.id, MAX_QUERY_ID_BYTES)?;
        validate_required("task", &self.task, MAX_QUERY_TASK_BYTES)?;
        validate_required("requester", &self.requester, MAX_QUERY_REQUESTER_BYTES)?;
        let evidence_item_count = self.context.len() + self.evidence.len();
        if evidence_item_count > MAX_QUERY_CONTEXT_ITEMS {
            return Err(OracleQueryError::TooManyContextItems {
                actual: evidence_item_count,
                maximum: MAX_QUERY_CONTEXT_ITEMS,
            });
        }
        for (index, item) in self.context.iter().enumerate() {
            if item.len() > MAX_QUERY_CONTEXT_ITEM_BYTES {
                return Err(OracleQueryError::ContextItemTooLong {
                    index,
                    actual: item.len(),
                    maximum: MAX_QUERY_CONTEXT_ITEM_BYTES,
                });
            }
        }
        for (index, evidence) in self.evidence.iter().enumerate() {
            for (field, value) in [
                ("source_id", Some(evidence.source_id.as_str())),
                ("locator", Some(evidence.locator.as_str())),
                ("digest", Some(evidence.digest.as_str())),
                ("excerpt", evidence.excerpt.as_deref()),
                ("claim", evidence.claim.as_deref()),
            ] {
                if let Some(value) = value {
                    if value.len() > MAX_QUERY_CONTEXT_ITEM_BYTES {
                        return Err(OracleQueryError::EvidenceFieldTooLong {
                            index,
                            field,
                            actual: value.len(),
                            maximum: MAX_QUERY_CONTEXT_ITEM_BYTES,
                        });
                    }
                }
            }
        }
        for (field, value) in [
            ("correlation_id", self.correlation_id.as_deref()),
            ("causation_id", self.causation_id.as_deref()),
        ] {
            if let Some(value) = value {
                validate_required(field, value, MAX_QUERY_ID_BYTES)?;
            }
        }
        Ok(())
    }

    pub(crate) fn is_same_request(&self, other: &Self) -> bool {
        self.id == other.id
            && self.task == other.task
            && self.context == other.context
            && self.evidence.len() == other.evidence.len()
            && self
                .evidence
                .iter()
                .zip(&other.evidence)
                .all(|(left, right)| left.same_request_identity(right))
            && self.requester == other.requester
            && self.query_type == other.query_type
            && self.correlation_id == other.correlation_id
            && self.causation_id == other.causation_id
    }

    pub(crate) fn request_identity_digest(&self) -> String {
        let mut value =
            serde_json::to_value(self).expect("OracleQuery serialization is infallible");
        if let Some(fields) = value.as_object_mut() {
            fields.remove("timestamp");
            if let Some(evidence) = fields
                .get_mut("evidence")
                .and_then(|value| value.as_array_mut())
            {
                for item in evidence {
                    if let Some(item_fields) = item.as_object_mut() {
                        item_fields.remove("excerpt");
                        item_fields.remove("sensitive_excerpt");
                        item_fields.remove("integrity");
                    }
                }
            }
        }
        let encoded =
            serde_json::to_vec(&value).expect("request identity serialization is infallible");
        format!("sha256:{:x}", Sha256::digest(encoded))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OracleQueryError {
    EmptyField {
        field: &'static str,
    },
    FieldTooLong {
        field: &'static str,
        actual: usize,
        maximum: usize,
    },
    TooManyContextItems {
        actual: usize,
        maximum: usize,
    },
    ContextItemTooLong {
        index: usize,
        actual: usize,
        maximum: usize,
    },
    EvidenceFieldTooLong {
        index: usize,
        field: &'static str,
        actual: usize,
        maximum: usize,
    },
    DuplicateQueryId {
        id: String,
    },
    ReasoningContext {
        message: String,
    },
}

impl fmt::Display for OracleQueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyField { field } => write!(formatter, "query {field} must not be empty"),
            Self::FieldTooLong {
                field,
                actual,
                maximum,
            } => write!(
                formatter,
                "query {field} is {actual} bytes; maximum is {maximum}"
            ),
            Self::TooManyContextItems { actual, maximum } => write!(
                formatter,
                "query context has {actual} items; maximum is {maximum}"
            ),
            Self::ContextItemTooLong {
                index,
                actual,
                maximum,
            } => write!(
                formatter,
                "query context item {index} is {actual} bytes; maximum is {maximum}"
            ),
            Self::EvidenceFieldTooLong {
                index,
                field,
                actual,
                maximum,
            } => write!(
                formatter,
                "query evidence item {index} field {field} is {actual} bytes; maximum is {maximum}"
            ),
            Self::DuplicateQueryId { id } => {
                write!(
                    formatter,
                    "query id '{id}' already exists with different content"
                )
            }
            Self::ReasoningContext { message } => {
                write!(
                    formatter,
                    "could not construct bounded reasoning context: {message}"
                )
            }
        }
    }
}

impl std::error::Error for OracleQueryError {}

fn validate_required(
    field: &'static str,
    value: &str,
    maximum: usize,
) -> Result<(), OracleQueryError> {
    if value.trim().is_empty() {
        return Err(OracleQueryError::EmptyField { field });
    }
    if value.len() > maximum {
        return Err(OracleQueryError::FieldTooLong {
            field,
            actual: value.len(),
            maximum,
        });
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verdict {
    pub query_id: String,
    pub policy_id: String,
    pub policy_version: String,
    pub outcome: VerdictOutcome,
    pub gates: TriadGates,
    pub reasoning: ReasoningContext,
    pub resonance_score: f64,
    pub timestamp: DateTime<Utc>,
    pub query_timestamp: DateTime<Utc>,
    pub evaluated_at: DateTime<Utc>,
    #[serde(default)]
    pub conditions: Vec<VerdictCondition>,
    #[serde(default)]
    pub vetoes: Vec<PolicyVeto>,
    pub governance: VerdictGovernance,
}

impl Verdict {
    pub fn redacted_for_export(&self) -> Self {
        let mut redacted = self.clone();
        for gate in [
            &mut redacted.gates.aurelius,
            &mut redacted.gates.bacon,
            &mut redacted.gates.sun_tzu,
        ] {
            for assessment in &mut gate.evidence {
                assessment.evidence = assessment.evidence.clone().redacted_for_export();
            }
        }
        redacted.reasoning = redacted.reasoning.redacted_for_export();
        redacted
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VerdictConditionKind {
    ProvideEvidence,
    ClarifyLogic,
    ReviewTiming,
    Escalate,
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
#[serde(rename_all = "snake_case")]
pub enum VerdictOutcome {
    Pass,
    Fail,
    Conditional,
    Escalate,
}

impl VerdictOutcome {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Conditional => "conditional",
            Self::Escalate => "escalate",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GateKind {
    Aurelius,
    Bacon,
    SunTzu,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub kind: GateKind,
    pub passed: bool,
    pub score: f64,
    pub concerns: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<EvidenceAssessment>,
    #[serde(default)]
    pub evidence_signals: Vec<EvidenceSignal>,
    #[serde(default)]
    pub disposition: GateDisposition,
    #[serde(default)]
    pub scored: bool,
}

impl GateResult {
    pub fn seal(self, _kind: GateKind) -> Self {
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriadGates {
    pub aurelius: GateResult,
    pub bacon: GateResult,
    pub sun_tzu: GateResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum GateDisposition {
    Accepted,
    #[default]
    Rejected,
    Escalated,
}

impl GateDisposition {
    pub fn validate_score(&self, score: &mut f64, signals: &[EvidenceSignal]) {
        *score = normalize_score(*score);
        if signals
            .iter()
            .any(|signal| signal.kind == EvidenceSignalKind::Conflicting)
        {
            *score = (*score - 0.05).max(0.0);
        }
        if *score == 0.0 {
            *score = 0.0;
        }
    }
}

#[derive(Default)]
struct ClaimSources {
    supporting: BTreeSet<String>,
    independent_supporting: BTreeSet<String>,
    contradicting: BTreeSet<String>,
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
            history_queries: HashMap::new(),
            history_request_digests: HashMap::new(),
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

    pub fn evaluate(&mut self, query: OracleQuery) -> Result<Verdict, OracleQueryError> {
        query.validate()?;
        let request_digest = query.request_identity_digest();
        if let Some(existing_digest) = self.history_request_digests.get(&query.id) {
            if existing_digest == &request_digest {
                return self.find_cached_verdict_by_id(&query.id);
            }
            return Err(OracleQueryError::DuplicateQueryId {
                id: query.id.clone(),
            });
        }
        if let Some(existing_query) = self.history_queries.get(&query.id) {
            if existing_query.is_same_request(&query) {
                return self.find_cached_verdict_by_id(&query.id);
            }
            return Err(OracleQueryError::DuplicateQueryId {
                id: query.id.clone(),
            });
        }

        if self
            .history
            .iter()
            .any(|verdict| verdict.query_id == query.id)
        {
            return Err(OracleQueryError::DuplicateQueryId {
                id: query.id.clone(),
            });
        }

        let normalized_task = normalize_lexical_text(&query.task);
        let aurelius = self.evaluate_aurelius(&query, &normalized_task);
        let bacon = self.evaluate_bacon(&query, &normalized_task);
        let sun_tzu = self.evaluate_sun_tzu(&query, &normalized_task);

        let mut gates = TriadGates {
            aurelius: self.build_gate_result(
                GateKind::Aurelius,
                aurelius.score,
                aurelius.concerns.clone(),
                aurelius.evidence.clone(),
                aurelius.evidence_signals.clone(),
            ),
            bacon: self.build_gate_result(
                GateKind::Bacon,
                bacon.score,
                bacon.concerns.clone(),
                bacon.evidence.clone(),
                bacon.evidence_signals.clone(),
            ),
            sun_tzu: self.build_gate_result(
                GateKind::SunTzu,
                sun_tzu.score,
                sun_tzu.concerns.clone(),
                sun_tzu.evidence.clone(),
                sun_tzu.evidence_signals.clone(),
            ),
        };

        let vetoes = self.collect_vetoes(&normalized_task);
        let outcome = self.determine_outcome(&gates, &vetoes);
        if outcome == VerdictOutcome::Escalate {
            for gate in [&mut gates.aurelius, &mut gates.bacon, &mut gates.sun_tzu] {
                if !gate.passed {
                    gate.disposition = GateDisposition::Escalated;
                }
            }
        }
        let conditions = self.build_conditions(&gates, &outcome);
        let reasoning = self.build_reasoning(&outcome, &gates)?;
        let resonance_score = self.calculate_resonance(&outcome, &gates);
        let governance = self.evaluate_governance(&query, &outcome, resonance_score);

        let evaluated_at = Utc::now();
        let verdict = Verdict {
            query_id: query.id.clone(),
            policy_id: self.policy.policy_id.clone(),
            policy_version: self.policy.policy_version.clone(),
            outcome: outcome.clone(),
            gates,
            reasoning,
            resonance_score,
            timestamp: query.timestamp,
            query_timestamp: query.timestamp,
            evaluated_at,
            conditions,
            vetoes,
            governance,
        };

        self.history_request_digests
            .insert(query.id.clone(), request_digest);
        self.history_queries.insert(query.id.clone(), query);
        self.history.push(verdict.clone());
        Ok(verdict)
    }

    pub fn find_cached_verdict_by_id(&self, id: &str) -> Result<Verdict, OracleQueryError> {
        self.history
            .iter()
            .find(|verdict| verdict.query_id == id)
            .cloned()
            .ok_or_else(|| OracleQueryError::DuplicateQueryId { id: id.to_string() })
    }

    pub fn get_history(&self) -> &[Verdict] {
        &self.history
    }

    pub(crate) fn rollback_verdict(&mut self, query_id: &str) {
        if self
            .history
            .last()
            .is_some_and(|verdict| verdict.query_id == query_id)
        {
            self.history.pop();
            self.history_queries.remove(query_id);
            self.history_request_digests.remove(query_id);
        }
    }

    pub fn history(&self) -> &[Verdict] {
        self.history.as_slice()
    }

    pub fn history_queries(&self) -> &HashMap<String, OracleQuery> {
        &self.history_queries
    }

    fn build_gate_result(
        &self,
        kind: GateKind,
        score: f64,
        concerns: Vec<String>,
        evidence: Vec<EvidenceAssessment>,
        evidence_signals: Vec<EvidenceSignal>,
    ) -> GateResult {
        let gate_score = normalize_score(score);
        let passed = match kind {
            GateKind::Aurelius => {
                gate_score >= normalize_score(self.policy.aurelius_pass_threshold)
            }
            GateKind::Bacon => gate_score >= normalize_score(self.policy.bacon_pass_threshold),
            GateKind::SunTzu => gate_score >= normalize_score(self.policy.sun_tzu_pass_threshold),
        };
        let disposition = if passed {
            GateDisposition::Accepted
        } else {
            GateDisposition::Rejected
        };

        GateResult {
            kind,
            passed,
            score: gate_score,
            concerns,
            evidence,
            evidence_signals,
            disposition,
            scored: true,
        }
    }

    pub fn record_restart_verdict(
        &mut self,
        verdict: Verdict,
        request_digest: Option<String>,
    ) -> Result<(), OracleQueryError> {
        let key = verdict.query_id.clone();
        if self.history_queries.contains_key(&key) {
            return Err(OracleQueryError::DuplicateQueryId { id: key });
        }
        self.history_queries.insert(
            key.clone(),
            OracleQuery {
                id: verdict.query_id.clone(),
                task: String::new(),
                context: Vec::new(),
                evidence: Vec::new(),
                requester: String::new(),
                timestamp: verdict.query_timestamp,
                query_type: Default::default(),
                correlation_id: None,
                causation_id: None,
            },
        );
        if let Some(request_digest) = request_digest {
            self.history_request_digests.insert(key, request_digest);
        }
        self.history.push(verdict);
        Ok(())
    }

    fn evaluate_aurelius(&self, query: &OracleQuery, normalized_task: &str) -> GateResult {
        let mut concerns = Vec::new();
        let mut evidence = Vec::new();
        let mut evidence_signals = Vec::new();
        let mut score = 1.0;

        if (normalized_task.contains("should")
            || normalized_task.contains("must")
            || normalized_task.contains("need"))
            && query.context.is_empty()
            && query.evidence.is_empty()
        {
            concerns.push("Task requires justification but none provided".to_string());
            score -= 0.3;
            let unavailable = EvidenceRef::unavailable(
                "query-justification",
                "query.context",
                query.timestamp,
                "No justification evidence was supplied",
            );
            evidence.push(EvidenceAssessment {
                integrity: unavailable.integrity(),
                evidence: unavailable,
                disposition: EvidenceDisposition::Rejected,
                affected_score: true,
                score_effect: -0.3,
                rationale: "Missing justification reduced logical confidence".to_string(),
            });
            evidence_signals.push(EvidenceSignal {
                kind: EvidenceSignalKind::Missing,
                description: "Required justification evidence is unavailable".to_string(),
                source_ids: vec!["query-justification".to_string()],
            });
        }

        if Self::has_contradictions(normalized_task) {
            concerns.push("Logical contradiction detected in task or context".to_string());
            score = 0.0;
            let inferred = EvidenceRef::inferred(
                "oracle:aurelius",
                "query.task",
                query.timestamp,
                "Logical contradiction detected",
                EvidenceStance::Contradicting,
            );
            evidence.push(EvidenceAssessment {
                integrity: inferred.integrity(),
                evidence: inferred,
                disposition: EvidenceDisposition::Accepted,
                affected_score: true,
                score_effect: -1.0,
                rationale: "Deterministic contradiction analysis set the gate score to zero"
                    .to_string(),
            });
        } else {
            let inferred = EvidenceRef::inferred(
                "oracle:aurelius",
                "query.task",
                query.timestamp,
                "No configured contradiction pattern detected",
                EvidenceStance::Supporting,
            );
            evidence.push(EvidenceAssessment {
                integrity: inferred.integrity(),
                evidence: inferred,
                disposition: EvidenceDisposition::Accepted,
                affected_score: false,
                score_effect: 0.0,
                rationale: "Deterministic contradiction analysis found no score adjustment"
                    .to_string(),
            });
        }

        self.build_gate_result(
            GateKind::Aurelius,
            score,
            concerns,
            evidence,
            evidence_signals,
        )
    }

    fn evaluate_bacon(&mut self, query: &OracleQuery, normalized_task: &str) -> GateResult {
        let mut concerns = Vec::new();
        let mut evidence_signals = Vec::new();
        let mut score: f64 = 0.55;
        let mut evidence_refs: Vec<_> = query
            .evidence
            .iter()
            .cloned()
            .map(|evidence| {
                let source_quality = evidence.source_quality;
                evidence
                    .with_source_quality(source_quality)
                    .classify_freshness(query.timestamp)
            })
            .collect();
        evidence_refs.extend(query.context.iter().enumerate().map(|(index, context)| {
            EvidenceRef::supplied(
                format!("query:{}:context:{index}", query.id),
                format!("query.context[{index}]"),
                query.timestamp,
                context,
            )
            .with_sensitive_excerpt(true)
            .classify_freshness(query.timestamp)
        }));

        if evidence_refs.is_empty() {
            concerns.push("No explicit evidence provided - querying document index".to_string());

            let results = self.page_index.search_all(&query.task);
            if results.is_empty() {
                evidence_refs.push(EvidenceRef::unavailable(
                    "pageindex-search",
                    "pageindex://search",
                    query.timestamp,
                    "No supplied or retrieved evidence matched the query",
                ));
            } else {
                evidence_refs.extend(results.into_iter().map(|result| {
                    EvidenceRef::retrieved(
                        result.doc_id,
                        result.source_ref,
                        query.timestamp,
                        result.title.clone(),
                        result.relevance_score,
                    )
                    .with_claim(result.title, EvidenceStance::Supporting)
                }));
            }
        }

        let retrieved_count = evidence_refs
            .iter()
            .filter(|evidence| evidence.kind == EvidenceKind::Retrieved)
            .count()
            .max(1);
        let mut evidence = Vec::new();
        let mut positive_effect = 0.0;
        let mut negative_effect = 0.0;
        let mut stale_sources = Vec::new();
        let mut missing_sources = Vec::new();

        for evidence_ref in evidence_refs {
            let integrity = evidence_ref.integrity();
            let (disposition, effect, rationale) = if integrity == EvidenceIntegrity::Invalid {
                (
                    EvidenceDisposition::Rejected,
                    0.0,
                    "Evidence digest does not match its source content; retained for audit only"
                        .to_string(),
                )
            } else if matches!(
                integrity,
                EvidenceIntegrity::Redacted | EvidenceIntegrity::Unverifiable
            ) {
                (
                    EvidenceDisposition::Rejected,
                    0.0,
                    "Evidence content is unavailable for digest verification; retained for audit only"
                        .to_string(),
                )
            } else if evidence_ref.kind == EvidenceKind::Unavailable {
                missing_sources.push(evidence_ref.source_id.clone());
                (
                    EvidenceDisposition::Rejected,
                    0.0,
                    "Unavailable evidence cannot affect the score".to_string(),
                )
            } else if evidence_ref.freshness == EvidenceFreshness::Stale {
                stale_sources.push(evidence_ref.source_id.clone());
                (
                    EvidenceDisposition::Rejected,
                    0.0,
                    "Stale evidence is retained for audit but excluded from scoring".to_string(),
                )
            } else if evidence_ref.source_quality <= 0.0 {
                (
                    EvidenceDisposition::Rejected,
                    0.0,
                    "Zero-quality evidence is retained for audit but excluded from scoring"
                        .to_string(),
                )
            } else {
                let effect = match evidence_ref.stance {
                    EvidenceStance::Contradicting => -0.15 * evidence_ref.source_quality,
                    EvidenceStance::Supporting | EvidenceStance::Neutral => {
                        if evidence_ref.kind == EvidenceKind::Retrieved {
                            0.2 * evidence_ref.source_quality / retrieved_count as f64
                        } else {
                            self.policy.evidence_bonus_per_item.max(0.0)
                                * evidence_ref.source_quality
                        }
                    }
                };
                (
                    EvidenceDisposition::Accepted,
                    effect,
                    "Verified digest plus freshness, provenance, and caller-supplied quality permit bounded scoring; none proves truth".to_string(),
                )
            };
            if effect >= 0.0 {
                positive_effect += effect;
            } else {
                negative_effect += effect;
            }
            evidence.push(EvidenceAssessment {
                evidence: evidence_ref,
                integrity,
                disposition,
                affected_score: effect != 0.0,
                score_effect: effect,
                rationale,
            });
        }

        score += positive_effect.min(self.policy.maximum_evidence_bonus.max(0.0));
        score += negative_effect;
        if !missing_sources.is_empty() {
            evidence_signals.push(EvidenceSignal {
                kind: EvidenceSignalKind::Missing,
                description: "Evidence is unavailable; this is explicit uncertainty".to_string(),
                source_ids: missing_sources,
            });
        }
        if !stale_sources.is_empty() {
            evidence_signals.push(EvidenceSignal {
                kind: EvidenceSignalKind::Stale,
                description: "Stale evidence was retained for audit and rejected from scoring"
                    .to_string(),
                source_ids: stale_sources,
            });
        }
        evidence_signals.extend(Self::cross_source_signals(&evidence));

        let has_financial = query.task.contains('$')
            || normalized_task.contains("budget")
            || normalized_task.contains("cost");

        let accepted_evidence_count = evidence
            .iter()
            .filter(|assessment| assessment.disposition == EvidenceDisposition::Accepted)
            .count();
        if has_financial && accepted_evidence_count < 2 {
            concerns.push("Financial task requires stronger evidence base".to_string());
            score -= 0.2;
        }

        self.build_gate_result(GateKind::Bacon, score, concerns, evidence, evidence_signals)
    }

    fn evaluate_sun_tzu(&self, query: &OracleQuery, normalized_task: &str) -> GateResult {
        let mut concerns = Vec::new();
        let mut evidence = Vec::new();
        let mut score = 1.0;

        let urgent_keywords = ["urgent", "asap", "immediately", "emergency", "critical"];
        let has_urgency = urgent_keywords.iter().any(|k| normalized_task.contains(k));
        let has_dangerous_operation = Self::has_dangerous_operation(normalized_task);

        if has_urgency {
            concerns.push("Task marked urgent — verify timing is truly critical".to_string());
            score -= 0.15;
        }

        if has_dangerous_operation {
            concerns.push("Dangerous operation requires explicit human review".to_string());
            score = 0.0;
        }

        let strategic_claim = if has_dangerous_operation {
            "Configured dangerous-operation pattern detected"
        } else if has_urgency {
            "Urgency language requires timing review"
        } else {
            "No configured urgency or dangerous-operation pattern detected"
        };
        let inferred = EvidenceRef::inferred(
            "oracle:sun-tzu",
            "query.task",
            query.timestamp,
            strategic_claim,
            if has_dangerous_operation {
                EvidenceStance::Contradicting
            } else {
                EvidenceStance::Neutral
            },
        );
        evidence.push(EvidenceAssessment {
            integrity: inferred.integrity(),
            evidence: inferred,
            disposition: EvidenceDisposition::Accepted,
            affected_score: has_urgency || has_dangerous_operation,
            score_effect: if has_dangerous_operation {
                -1.0
            } else if has_urgency {
                -0.15
            } else {
                0.0
            },
            rationale: "Deterministic strategy analysis recorded as inferred evidence".to_string(),
        });

        self.build_gate_result(GateKind::SunTzu, score, concerns, evidence, Vec::new())
    }

    fn cross_source_signals(evidence: &[EvidenceAssessment]) -> Vec<EvidenceSignal> {
        let mut claims: BTreeMap<String, ClaimSources> = BTreeMap::new();
        for assessment in evidence.iter().filter(|assessment| {
            assessment.disposition == EvidenceDisposition::Accepted
                && assessment.evidence.claim.is_some()
        }) {
            let claim = assessment
                .evidence
                .claim
                .as_deref()
                .unwrap_or_default()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase();
            let sources = claims.entry(claim).or_default();
            match assessment.evidence.stance {
                EvidenceStance::Supporting => {
                    sources
                        .supporting
                        .insert(assessment.evidence.source_id.clone());
                    if assessment.evidence.independence == EvidenceIndependence::Independent {
                        sources
                            .independent_supporting
                            .insert(assessment.evidence.source_id.clone());
                    }
                }
                EvidenceStance::Contradicting => {
                    sources
                        .contradicting
                        .insert(assessment.evidence.source_id.clone());
                }
                EvidenceStance::Neutral => {}
            }
        }

        let mut signals = Vec::new();
        for (claim, sources) in claims {
            if !sources.supporting.is_empty() && !sources.contradicting.is_empty() {
                let source_ids = sources
                    .supporting
                    .iter()
                    .chain(sources.contradicting.iter())
                    .cloned()
                    .collect();
                signals.push(EvidenceSignal {
                    kind: EvidenceSignalKind::Conflicting,
                    description: format!(
                        "Accepted sources conflict about '{claim}'; uncertainty remains explicit"
                    ),
                    source_ids,
                });
            }
            if sources.independent_supporting.len() >= 2 {
                signals.push(EvidenceSignal {
                    kind: EvidenceSignalKind::Corroborating,
                    description: format!(
                        "Independent sources corroborate '{claim}', which raises support but does not prove truth"
                    ),
                    source_ids: sources.independent_supporting.into_iter().collect(),
                });
            }
        }
        signals
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
        } else if pass_count == 0 {
            VerdictOutcome::Escalate
        } else if pass_count >= self.policy.minimum_passed_gates_for_conditional.clamp(1, 3) {
            VerdictOutcome::Conditional
        } else {
            VerdictOutcome::Fail
        }
    }

    fn collect_vetoes(&self, normalized_task: &str) -> Vec<PolicyVeto> {
        let mut vetoes = Vec::new();
        if self.policy.contradiction_veto_enabled && Self::has_contradictions(normalized_task) {
            vetoes.push(PolicyVeto {
                kind: PolicyVetoKind::Contradiction,
                gate: "Aurelius".to_string(),
                reason: "Logical contradiction must be resolved before proceeding".to_string(),
            });
        }
        if self.policy.dangerous_operation_veto_enabled
            && Self::has_dangerous_operation(normalized_task)
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
        if *outcome == VerdictOutcome::Escalate {
            return vec![VerdictCondition {
                kind: VerdictConditionKind::Escalate,
                gate: "Triad".to_string(),
                description: "No policy gate passed; automated resolution is unavailable"
                    .to_string(),
                required_action: "Escalate to explicit human review before proceeding".to_string(),
            }];
        }
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
        outcome: &VerdictOutcome,
        gates: &TriadGates,
    ) -> Result<ReasoningContext, OracleQueryError> {
        let mut reasoning = ReasoningContext::default();
        let outcome_id = reasoning
            .add_node(
                ReasoningNodeKind::Claim,
                format!("Oracle outcome is {}", outcome_label(outcome)),
                None,
            )
            .map_err(reasoning_error)?;

        for (gate_name, gate, passed_rationale, failed_rationale) in [
            (
                "Aurelius",
                &gates.aurelius,
                "Task is logically consistent and well-formed",
                "Task fails logical consistency checks",
            ),
            (
                "Bacon",
                &gates.bacon,
                "Task has sufficient evidence grounding",
                "Task lacks sufficient evidence or context",
            ),
            (
                "Sun Tzu",
                &gates.sun_tzu,
                "Task timing and strategy are appropriate",
                "Task timing or strategic considerations are questionable",
            ),
        ] {
            let gate_id = reasoning
                .add_node(
                    ReasoningNodeKind::Claim,
                    format!(
                        "{gate_name}: {}",
                        if gate.passed {
                            passed_rationale
                        } else {
                            failed_rationale
                        }
                    ),
                    None,
                )
                .map_err(reasoning_error)?;
            reasoning
                .add_edge(&outcome_id, &gate_id, ReasoningEdgeType::DependsOn)
                .map_err(reasoning_error)?;

            for concern in &gate.concerns {
                let objection_id = reasoning
                    .add_node(
                        ReasoningNodeKind::Objection,
                        format!("{gate_name}: {concern}"),
                        None,
                    )
                    .map_err(reasoning_error)?;
                reasoning
                    .add_edge(&gate_id, &objection_id, ReasoningEdgeType::ObjectsTo)
                    .map_err(reasoning_error)?;
            }

            for assessment in &gate.evidence {
                let evidence_id = reasoning
                    .register_evidence(assessment.evidence.clone())
                    .map_err(reasoning_error)?;
                let evidence_node_id = reasoning
                    .add_node(
                        ReasoningNodeKind::Evidence,
                        format!(
                            "{gate_name}: source '{}' was {} — {}",
                            assessment.evidence.source_id,
                            disposition_label(assessment.disposition),
                            assessment.rationale
                        ),
                        Some(&evidence_id),
                    )
                    .map_err(reasoning_error)?;
                let edge_type = if assessment.disposition == EvidenceDisposition::Rejected
                    || assessment.evidence.stance == EvidenceStance::Contradicting
                {
                    ReasoningEdgeType::ObjectsTo
                } else {
                    ReasoningEdgeType::Supports
                };
                reasoning
                    .add_edge(&gate_id, &evidence_node_id, edge_type)
                    .map_err(reasoning_error)?;
            }

            for signal in &gate.evidence_signals {
                let (kind, edge_type) = match signal.kind {
                    EvidenceSignalKind::Corroborating => {
                        (ReasoningNodeKind::Claim, ReasoningEdgeType::Supports)
                    }
                    EvidenceSignalKind::Missing
                    | EvidenceSignalKind::Stale
                    | EvidenceSignalKind::Conflicting => {
                        (ReasoningNodeKind::Objection, ReasoningEdgeType::ObjectsTo)
                    }
                };
                let signal_id = reasoning
                    .add_node(kind, format!("{gate_name}: {}", signal.description), None)
                    .map_err(reasoning_error)?;
                reasoning
                    .add_edge(&gate_id, &signal_id, edge_type)
                    .map_err(reasoning_error)?;
            }
        }

        reasoning.validate().map_err(reasoning_error)?;
        Ok(reasoning)
    }

    fn calculate_resonance(&self, outcome: &VerdictOutcome, gates: &TriadGates) -> f64 {
        let base = 0.85;
        let accepted_evidence_count = gates
            .bacon
            .evidence
            .iter()
            .filter(|assessment| assessment.disposition == EvidenceDisposition::Accepted)
            .count();
        let context_bonus = (accepted_evidence_count as f64 * 0.02).min(0.1);
        let outcome_modifier = match outcome {
            VerdictOutcome::Pass => 0.05,
            VerdictOutcome::Conditional => 0.0,
            VerdictOutcome::Fail | VerdictOutcome::Escalate => -0.1,
        };

        (base + context_bonus + outcome_modifier).min(1.0)
    }

    fn has_contradictions(normalized_task: &str) -> bool {
        let contradictions = [
            ("always", "never"),
            ("must", "must not"),
            ("yes", "no"),
            ("increase", "decrease"),
        ];

        for (a, b) in contradictions {
            if normalized_task.contains(a) && normalized_task.contains(b) {
                return true;
            }
        }

        false
    }

    fn has_dangerous_operation(normalized_task: &str) -> bool {
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
        .any(|keyword| normalized_task.contains(keyword))
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
        let escalate_count = self
            .history
            .iter()
            .filter(|verdict| verdict.outcome == VerdictOutcome::Escalate)
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
        let disposition_counts = |select: fn(&Verdict) -> &GateResult| {
            let mut accepted = 0usize;
            let mut rejected = 0usize;
            let mut escalated = 0usize;
            for verdict in &self.history {
                match select(verdict).disposition {
                    GateDisposition::Accepted => accepted += 1,
                    GateDisposition::Rejected => rejected += 1,
                    GateDisposition::Escalated => escalated += 1,
                }
            }
            json!({
                "accepted": accepted,
                "rejected": rejected,
                "escalated": escalated,
            })
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
                "escalate": escalate_count,
            },
            "gate_dispositions": {
                "aurelius": disposition_counts(|verdict| &verdict.gates.aurelius),
                "bacon": disposition_counts(|verdict| &verdict.gates.bacon),
                "sun_tzu": disposition_counts(|verdict| &verdict.gates.sun_tzu),
            },
            "governance": {
                "bacon_lite_passed_total": bacon_lite_passed_total,
                "average_love_equation": average_love_equation,
                "average_triad": average_triad,
            },
            "recent_verdicts": self
                .history
                .iter()
                .rev()
                .take(10)
                .map(Verdict::redacted_for_export)
                .collect::<Vec<_>>(),
        })
    }

    pub fn format_verdict(&self, verdict: &Verdict) -> String {
        let outcome_str = match verdict.outcome {
            VerdictOutcome::Pass => "◈ PASS",
            VerdictOutcome::Fail => "∇ FAIL",
            VerdictOutcome::Conditional => "◈ CONDITIONAL",
            VerdictOutcome::Escalate => "△ ESCALATE",
        };

        let mut output = format!(
            "{} | Resonance: {:.2} | Policy: {}@{}\n",
            outcome_str, verdict.resonance_score, verdict.policy_id, verdict.policy_version
        );

        output.push_str("\nReasoning context:\n");
        for node in verdict.reasoning.traverse().unwrap_or_default() {
            output.push_str(&format!(
                "  - [{:?}] {}\n",
                node.kind, node.public_rationale
            ));
            if let Some(evidence_id) = &node.evidence_id {
                if let Some(evidence) = verdict.reasoning.evidence().get(evidence_id) {
                    output.push_str(&format!(
                        "    {:?} {} @ {} ({})\n",
                        evidence.kind, evidence.source_id, evidence.locator, evidence.digest
                    ));
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
    ) -> IndexingReport {
        self.page_index.index_document(doc_id, title, toc)
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
        VerdictOutcome::Fail | VerdictOutcome::Escalate => 0,
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

pub(crate) fn normalize_lexical_text(text: &str) -> String {
    text.to_lowercase()
}

fn reasoning_error(error: ReasoningContextError) -> OracleQueryError {
    OracleQueryError::ReasoningContext {
        message: error.to_string(),
    }
}

fn outcome_label(outcome: &VerdictOutcome) -> &'static str {
    match outcome {
        VerdictOutcome::Pass => "pass",
        VerdictOutcome::Conditional => "conditional",
        VerdictOutcome::Fail => "fail",
        VerdictOutcome::Escalate => "escalate",
    }
}

fn disposition_label(disposition: EvidenceDisposition) -> &'static str {
    match disposition {
        EvidenceDisposition::Accepted => "accepted",
        EvidenceDisposition::Rejected => "rejected",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(task: &str, context: Vec<&str>) -> OracleQuery {
        let mut query = OracleQuery::new("q1", task, "operator");
        query.context = context.into_iter().map(|item| item.to_string()).collect();
        query
    }

    fn evaluate(engine: &mut OracleEngine, query: OracleQuery) -> Verdict {
        engine.evaluate(query).expect("test query should be valid")
    }

    fn typed_evidence(
        source_id: &str,
        observed_at: DateTime<Utc>,
        claim: &str,
        stance: crate::evidence::EvidenceStance,
    ) -> crate::evidence::EvidenceRef {
        crate::evidence::EvidenceRef::supplied(
            source_id,
            format!("fixture://{source_id}"),
            observed_at,
            format!("sensitive excerpt from {source_id}"),
        )
        .with_claim(claim, stance)
        .with_independence(crate::evidence::EvidenceIndependence::Independent)
        .with_source_quality(0.9)
        .with_sensitive_excerpt(true)
    }

    #[test]
    fn evidence_digest_binds_observation_and_sensitive_serialization_is_redacted() {
        let observed_at = DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
            .expect("timestamp")
            .with_timezone(&Utc);
        let first = typed_evidence(
            "report-a",
            observed_at,
            "deployment is safe",
            crate::evidence::EvidenceStance::Supporting,
        );
        let second = typed_evidence(
            "report-a",
            observed_at + chrono::TimeDelta::days(1),
            "deployment is safe",
            crate::evidence::EvidenceStance::Supporting,
        );

        assert_ne!(first.digest, second.digest);
        assert!(first.digest.starts_with("sha256:"));
        let exported = serde_json::to_value(&first).expect("serialize evidence");
        assert_eq!(exported["excerpt"], "[REDACTED]");
        assert_eq!(exported["digest"], first.digest);
        assert_eq!(exported["source_id"], "report-a");
        assert_eq!(exported["locator"], "fixture://report-a");
    }

    #[test]
    fn redacted_cross_transport_retry_preserves_evidence_identity() {
        let mut original = OracleQuery::new("cross-transport", "review evidence", "operator");
        original.evidence = vec![EvidenceRef::supplied(
            "sensitive-report",
            "vault://report",
            original.timestamp,
            "private source excerpt",
        )];
        let mut transported: OracleQuery =
            serde_json::from_value(serde_json::to_value(&original).expect("serialize typed query"))
                .expect("deserialize typed query");
        transported.timestamp += chrono::TimeDelta::seconds(1);
        let mut engine = OracleEngine::new();

        let first = evaluate(&mut engine, original);
        let retry = evaluate(&mut engine, transported);

        assert_eq!(first.query_id, retry.query_id);
        assert_eq!(engine.get_history().len(), 1);
    }

    #[test]
    fn missing_evidence_is_an_unavailable_rejected_reference_and_explicit_signal() {
        let mut engine = OracleEngine::new();
        let verdict = evaluate(&mut engine, query("document market posture", vec![]));
        let bacon = &verdict.gates.bacon;

        assert!(bacon
            .evidence_signals
            .iter()
            .any(|signal| signal.kind == crate::evidence::EvidenceSignalKind::Missing));
        assert!(bacon.evidence.iter().any(|assessment| {
            assessment.evidence.kind == crate::evidence::EvidenceKind::Unavailable
                && assessment.disposition == crate::evidence::EvidenceDisposition::Rejected
                && !assessment.affected_score
        }));
    }

    #[test]
    fn stale_evidence_is_rejected_and_reported_separately_from_missing() {
        let query_time = DateTime::parse_from_rfc3339("2026-06-01T00:00:00Z")
            .expect("timestamp")
            .with_timezone(&Utc);
        let mut oracle_query = query("document market posture", vec![]);
        oracle_query.timestamp = query_time;
        oracle_query.evidence = vec![typed_evidence(
            "stale-report",
            query_time - chrono::TimeDelta::days(90),
            "market posture is stable",
            crate::evidence::EvidenceStance::Supporting,
        )];
        let mut engine = OracleEngine::new();
        let verdict = evaluate(&mut engine, oracle_query);
        let bacon = &verdict.gates.bacon;

        assert!(bacon
            .evidence_signals
            .iter()
            .any(|signal| signal.kind == crate::evidence::EvidenceSignalKind::Stale));
        assert!(!bacon
            .evidence_signals
            .iter()
            .any(|signal| signal.kind == crate::evidence::EvidenceSignalKind::Missing));
        assert_eq!(
            bacon.evidence[0].disposition,
            crate::evidence::EvidenceDisposition::Rejected
        );
        assert_eq!(
            bacon.evidence[0].evidence.freshness,
            crate::evidence::EvidenceFreshness::Stale
        );
    }

    #[test]
    fn stale_retrieved_evidence_is_also_rejected() {
        let query_time = DateTime::parse_from_rfc3339("2026-06-01T00:00:00Z")
            .expect("timestamp")
            .with_timezone(&Utc);
        let mut oracle_query = query("document market posture", vec![]);
        oracle_query.timestamp = query_time;
        oracle_query.evidence = vec![EvidenceRef::retrieved(
            "archive-report",
            "archive://report",
            query_time - chrono::TimeDelta::days(90),
            "archived market posture",
            0.9,
        )];
        let mut engine = OracleEngine::new();
        let verdict = evaluate(&mut engine, oracle_query);

        assert_eq!(
            verdict.gates.bacon.evidence[0].disposition,
            EvidenceDisposition::Rejected
        );
        assert_eq!(
            verdict.gates.bacon.evidence[0].evidence.freshness,
            EvidenceFreshness::Stale
        );
    }

    #[test]
    fn tampered_evidence_digest_is_retained_but_rejected_from_scoring() {
        let mut oracle_query = query("document market posture", vec![]);
        let mut evidence = EvidenceRef::supplied(
            "tampered-report",
            "fixture://tampered",
            oracle_query.timestamp,
            "source content",
        );
        evidence.digest = "sha256:0000".to_string();
        oracle_query.evidence = vec![evidence];
        let mut engine = OracleEngine::new();
        let verdict = evaluate(&mut engine, oracle_query);
        let assessment = &verdict.gates.bacon.evidence[0];

        assert_eq!(assessment.disposition, EvidenceDisposition::Rejected);
        assert!(!assessment.affected_score);
        assert!(assessment.rationale.contains("digest"));
        assert_eq!(assessment.evidence.digest, "sha256:0000");
    }

    #[test]
    fn rejected_evidence_does_not_increase_resonance_or_governance_inputs() {
        let query_time = DateTime::parse_from_rfc3339("2026-06-01T00:00:00Z")
            .expect("timestamp")
            .with_timezone(&Utc);
        let mut baseline_query = query("document market posture", vec![]);
        baseline_query.timestamp = query_time;
        let mut rejected_query = baseline_query.clone();
        rejected_query.id = "rejected-evidence".to_string();
        let mut tampered = EvidenceRef::supplied(
            "tampered-report",
            "fixture://tampered",
            query_time,
            "source content",
        );
        tampered.digest = "sha256:0000".to_string();
        rejected_query.evidence = vec![tampered];
        let mut baseline_engine = OracleEngine::new();
        let mut rejected_engine = OracleEngine::new();

        let baseline = evaluate(&mut baseline_engine, baseline_query);
        let rejected = evaluate(&mut rejected_engine, rejected_query);

        assert_eq!(baseline.resonance_score, rejected.resonance_score);
        assert_eq!(
            baseline.governance.love_equation_guard.score,
            rejected.governance.love_equation_guard.score
        );
    }

    #[test]
    fn conflicting_evidence_is_visible_and_both_sources_remain_auditable() {
        let query_time = DateTime::parse_from_rfc3339("2026-06-01T00:00:00Z")
            .expect("timestamp")
            .with_timezone(&Utc);
        let mut oracle_query = query("document market posture", vec![]);
        oracle_query.timestamp = query_time;
        oracle_query.evidence = vec![
            typed_evidence(
                "report-a",
                query_time,
                "market posture is stable",
                crate::evidence::EvidenceStance::Supporting,
            ),
            typed_evidence(
                "report-b",
                query_time,
                "market posture is stable",
                crate::evidence::EvidenceStance::Contradicting,
            ),
        ];
        let mut engine = OracleEngine::new();
        let verdict = evaluate(&mut engine, oracle_query);
        let bacon = &verdict.gates.bacon;

        assert!(bacon
            .evidence_signals
            .iter()
            .any(|signal| signal.kind == crate::evidence::EvidenceSignalKind::Conflicting));
        assert_eq!(bacon.evidence.len(), 2);
        assert!(bacon
            .evidence
            .iter()
            .all(|assessment| !assessment.rationale.is_empty()));
        assert!(bacon
            .evidence
            .iter()
            .all(|assessment| assessment.affected_score));
    }

    #[test]
    fn independent_supporting_sources_are_reported_as_corroborating_not_proof() {
        let query_time = DateTime::parse_from_rfc3339("2026-06-01T00:00:00Z")
            .expect("timestamp")
            .with_timezone(&Utc);
        let mut oracle_query = query("document market posture", vec![]);
        oracle_query.timestamp = query_time;
        oracle_query.evidence = vec![
            typed_evidence(
                "report-a",
                query_time,
                "market posture is stable",
                crate::evidence::EvidenceStance::Supporting,
            ),
            typed_evidence(
                "report-b",
                query_time,
                "market posture is stable",
                crate::evidence::EvidenceStance::Supporting,
            ),
        ];
        let mut engine = OracleEngine::new();
        let verdict = evaluate(&mut engine, oracle_query);
        let bacon = &verdict.gates.bacon;

        let corroborating = bacon
            .evidence_signals
            .iter()
            .find(|signal| signal.kind == crate::evidence::EvidenceSignalKind::Corroborating)
            .expect("corroboration signal");
        assert!(corroborating.description.contains("does not prove"));
        assert_eq!(corroborating.source_ids, vec!["report-a", "report-b"]);
        assert!(bacon.evidence.iter().all(|assessment| {
            assessment.disposition == crate::evidence::EvidenceDisposition::Accepted
        }));
    }

    #[test]
    fn bacon_searches_all_documents_and_emits_stable_source_references() {
        fn toc(id: &str) -> Vec<crate::pageindex::TocEntry> {
            vec![crate::pageindex::TocEntry {
                id: id.to_string(),
                title: "Shared Evidence".to_string(),
                level: 1,
                page: Some(7),
            }]
        }

        let mut engine = OracleEngine::new();
        engine.index_document("zeta".to_string(), "Zeta".to_string(), toc("z"));
        engine.index_document("alpha".to_string(), "Alpha".to_string(), toc("a"));
        let first = evaluate(&mut engine, query("shared evidence", vec![]));
        let mut first_refs: Vec<_> = first
            .reasoning
            .evidence()
            .values()
            .filter(|evidence| evidence.locator.starts_with("pageindex://"))
            .map(|evidence| evidence.locator.clone())
            .collect();
        first_refs.sort();

        engine.index_document("zeta".to_string(), "Renamed Zeta".to_string(), toc("z"));
        engine.index_document("alpha".to_string(), "Renamed Alpha".to_string(), toc("a"));
        let mut second_query = query("shared evidence", vec![]);
        second_query.id = "q2".to_string();
        let second = evaluate(&mut engine, second_query);
        let mut second_refs: Vec<_> = second
            .reasoning
            .evidence()
            .values()
            .filter(|evidence| evidence.locator.starts_with("pageindex://"))
            .map(|evidence| evidence.locator.clone())
            .collect();
        second_refs.sort();

        assert_eq!(first_refs.len(), 2);
        assert!(first_refs[0].starts_with("pageindex://alpha/"));
        assert!(first_refs[1].starts_with("pageindex://zeta/"));
        assert_eq!(first_refs, second_refs);
    }

    #[test]
    fn contradictory_query_fails_aurelius_gate() {
        let mut engine = OracleEngine::new();
        let verdict = evaluate(
            &mut engine,
            query("we must increase and decrease access", vec![]),
        );

        assert_eq!(verdict.outcome, VerdictOutcome::Fail);
        assert!(!verdict.gates.aurelius.passed);
        assert_eq!(verdict.vetoes.len(), 1);
        assert_eq!(verdict.vetoes[0].kind, PolicyVetoKind::Contradiction);
        assert!(!engine.get_history().is_empty());
    }

    #[test]
    fn oracle_verdict_uses_a_bounded_reasoning_graph_instead_of_parallel_vectors() {
        let query_time = DateTime::parse_from_rfc3339("2026-06-01T00:00:00Z")
            .expect("timestamp")
            .with_timezone(&Utc);
        let mut oracle_query = query("review deployment posture", vec![]);
        oracle_query.timestamp = query_time;
        oracle_query.evidence = vec![typed_evidence(
            "deployment-report",
            query_time,
            "deployment posture is acceptable",
            EvidenceStance::Supporting,
        )];
        let mut engine = OracleEngine::new();

        let verdict = evaluate(&mut engine, oracle_query);
        let summary = verdict.reasoning.summary().expect("valid reasoning graph");
        let traversal = verdict
            .reasoning
            .traverse()
            .expect("deterministic traversal");

        assert!(summary.node_count >= 4, "outcome plus three gate claims");
        assert!(summary.max_depth <= verdict.reasoning.limits().max_depth);
        assert!(traversal
            .iter()
            .any(|node| node.kind == crate::context::ReasoningNodeKind::Evidence));
        assert!(traversal
            .iter()
            .any(|node| node.kind == crate::context::ReasoningNodeKind::Claim));
        assert!(!verdict.reasoning.edges().is_empty());
        assert_eq!(verdict.reasoning.validate(), Ok(()));
    }

    #[test]
    fn dangerous_operation_vetoes_otherwise_passing_gates() {
        let mut engine = OracleEngine::new();
        let verdict = evaluate(
            &mut engine,
            query(
                "perform a destructive database wipe immediately",
                vec!["operator requested maintenance", "backup completed"],
            ),
        );

        assert_eq!(verdict.outcome, VerdictOutcome::Fail);
        assert_eq!(verdict.vetoes.len(), 1);
        assert_eq!(verdict.vetoes[0].kind, PolicyVetoKind::DangerousOperation);
    }

    #[test]
    fn additional_relevant_evidence_never_lowers_bacon_score() {
        let mut with_one_item = OracleEngine::new();
        let baseline = evaluate(
            &mut with_one_item,
            query("document market posture", vec!["recent report"]),
        );
        let mut with_two_items = OracleEngine::new();
        let supported = evaluate(
            &mut with_two_items,
            query(
                "document market posture",
                vec!["recent report", "operator note"],
            ),
        );

        assert!(supported.gates.bacon.score >= baseline.gates.bacon.score);
    }

    #[test]
    fn verdict_and_status_identify_the_active_policy() {
        let policy = OraclePolicy {
            policy_id: "arda.mandos.test".to_string(),
            policy_version: "9.8.7".to_string(),
            ..OraclePolicy::default()
        };
        let mut engine = OracleEngine::new().with_policy(policy);

        let verdict = evaluate(
            &mut engine,
            query(
                "document market posture",
                vec!["recent report", "operator note"],
            ),
        );
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
            let verdict = evaluate(&mut engine, oracle_query);
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

        let verdict = evaluate(
            &mut engine,
            query(
                "we must increase and decrease access",
                vec!["operator rationale", "review note"],
            ),
        );

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
            let baseline = evaluate(&mut baseline_engine, oracle_query.clone());
            let score = match gate {
                GateUnderTest::Aurelius => baseline.gates.aurelius.score,
                GateUnderTest::Bacon => baseline.gates.bacon.score,
                GateUnderTest::SunTzu => baseline.gates.sun_tzu.score,
            };

            let mut policy = OraclePolicy::default();
            match gate {
                GateUnderTest::Aurelius => policy.aurelius_pass_threshold = score,
                GateUnderTest::Bacon => policy.bacon_pass_threshold = score,
                GateUnderTest::SunTzu => policy.sun_tzu_pass_threshold = score,
            }
            let aurelius_threshold = policy.aurelius_pass_threshold;
            let bacon_threshold = policy.bacon_pass_threshold;
            let sun_tzu_threshold = policy.sun_tzu_pass_threshold;
            let mut engine = OracleEngine::new().with_policy(policy);
            let verdict = evaluate(&mut engine, oracle_query.clone());
            let (passed, gate_score, threshold) = match gate {
                GateUnderTest::Aurelius => (
                    verdict.gates.aurelius.passed,
                    verdict.gates.aurelius.score,
                    aurelius_threshold,
                ),
                GateUnderTest::Bacon => (
                    verdict.gates.bacon.passed,
                    verdict.gates.bacon.score,
                    bacon_threshold,
                ),
                GateUnderTest::SunTzu => (
                    verdict.gates.sun_tzu.passed,
                    verdict.gates.sun_tzu.score,
                    sun_tzu_threshold,
                ),
            };
            assert!(passed, "score {gate_score} must meet threshold {threshold}");

            if (0.0..1.0).contains(&score) {
                let mut policy = OraclePolicy::default();
                let above = (score + 0.001).clamp(0.0, 1.0);
                match gate {
                    GateUnderTest::Aurelius => policy.aurelius_pass_threshold = above,
                    GateUnderTest::Bacon => policy.bacon_pass_threshold = above,
                    GateUnderTest::SunTzu => policy.sun_tzu_pass_threshold = above,
                }
                let mut engine = OracleEngine::new().with_policy(policy);
                let verdict = evaluate(&mut engine, oracle_query.clone());
                let passed = match gate {
                    GateUnderTest::Aurelius => verdict.gates.aurelius.passed,
                    GateUnderTest::Bacon => verdict.gates.bacon.passed,
                    GateUnderTest::SunTzu => verdict.gates.sun_tzu.passed,
                };
                assert!(
                    !passed,
                    "threshold above score must still fail for score {score}"
                );
            }
        }
    }

    #[test]
    fn outcome_policy_table_covers_pass_conditional_fail_and_veto() {
        fn gate(engine: &OracleEngine, passed: bool, kind: GateKind) -> GateResult {
            engine.build_gate_result(
                kind,
                if passed { 1.0 } else { 0.0 },
                Vec::new(),
                Vec::new(),
                Vec::new(),
            )
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
            ([false, false, false], Vec::new(), VerdictOutcome::Escalate),
            ([true, true, true], vec![veto], VerdictOutcome::Fail),
        ];

        for (passes, vetoes, expected) in cases {
            let gates = TriadGates {
                aurelius: gate(&engine, passes[0], GateKind::Aurelius),
                bacon: gate(&engine, passes[1], GateKind::Bacon),
                sun_tzu: gate(&engine, passes[2], GateKind::SunTzu),
            };
            assert_eq!(engine.determine_outcome(&gates, &vetoes), expected);
        }

        let escalate_gates = TriadGates {
            aurelius: gate(&engine, false, GateKind::Aurelius),
            bacon: gate(&engine, false, GateKind::Bacon),
            sun_tzu: gate(&engine, false, GateKind::SunTzu),
        };
        assert_eq!(
            engine.determine_outcome(&escalate_gates, &[]),
            VerdictOutcome::Escalate
        );
    }

    #[test]
    fn escalation_is_typed_serialized_and_operator_actionable() {
        let policy = OraclePolicy {
            aurelius_pass_threshold: 0.8,
            bacon_pass_threshold: 0.6,
            sun_tzu_pass_threshold: 0.9,
            ..OraclePolicy::default()
        };
        let mut engine = OracleEngine::new().with_policy(policy);
        let verdict = evaluate(
            &mut engine,
            query("URGENT budget should proceed", Vec::new()),
        );

        assert_eq!(verdict.outcome, VerdictOutcome::Escalate);
        assert!(verdict.vetoes.is_empty());
        assert!(verdict.conditions.iter().any(|condition| {
            condition.kind == VerdictConditionKind::Escalate
                && condition.required_action.contains("human review")
        }));
        for gate in [
            &verdict.gates.aurelius,
            &verdict.gates.bacon,
            &verdict.gates.sun_tzu,
        ] {
            assert_eq!(gate.disposition, GateDisposition::Escalated);
        }
        let exported = serde_json::to_value(&verdict).expect("serialize escalation verdict");
        assert_eq!(exported["outcome"], "escalate");
        assert_eq!(exported["conditions"][0]["kind"], "escalate");
        assert_eq!(engine.status_snapshot()["verdict_counts"]["escalate"], 1);
    }

    #[test]
    fn status_projects_bounded_per_gate_disposition_counters() {
        let mut engine = OracleEngine::new();
        let verdict = engine
            .evaluate(query(
                "gate-disposition-metrics",
                vec!["review evidence", "confirm rollback"],
            ))
            .expect("verdict");

        let status = engine.status_snapshot();
        for (name, gate) in [
            ("aurelius", &verdict.gates.aurelius),
            ("bacon", &verdict.gates.bacon),
            ("sun_tzu", &verdict.gates.sun_tzu),
        ] {
            let counters = &status["gate_dispositions"][name];
            let accepted = counters["accepted"].as_u64().expect("accepted count");
            let rejected = counters["rejected"].as_u64().expect("rejected count");
            let escalated = counters["escalated"].as_u64().expect("escalated count");
            assert_eq!(accepted + rejected + escalated, 1);
            match gate.disposition {
                GateDisposition::Accepted => assert_eq!(accepted, 1),
                GateDisposition::Rejected => assert_eq!(rejected, 1),
                GateDisposition::Escalated => assert_eq!(escalated, 1),
            }
        }
        assert!(!status["gate_dispositions"]
            .to_string()
            .contains("gate-disposition-metrics"));
    }

    #[test]
    fn policy_lexical_matching_is_case_insensitive() {
        let mut engine = OracleEngine::new();
        let contradiction = evaluate(
            &mut engine,
            query("We MUST increase and DECREASE access", vec!["reviewed"]),
        );
        assert!(contradiction
            .vetoes
            .iter()
            .any(|veto| veto.kind == PolicyVetoKind::Contradiction));

        let mut engine = OracleEngine::new();
        let dangerous = evaluate(
            &mut engine,
            query("DISABLE SAFEGUARDS after review", vec!["reviewed"]),
        );
        assert!(dangerous
            .vetoes
            .iter()
            .any(|veto| veto.kind == PolicyVetoKind::DangerousOperation));
    }

    #[test]
    fn financial_query_without_evidence_trips_bacon_concerns() {
        let mut engine = OracleEngine::new();
        let verdict = evaluate(&mut engine, query("budget should increase by $500", vec![]));

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
        let verdict = evaluate(
            &mut engine,
            query(
                "document market posture",
                vec!["recent report", "operator note"],
            ),
        );

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

    #[test]
    fn direct_engine_rejects_invalid_query_without_history_mutation() {
        let mut engine = OracleEngine::new();
        let invalid = OracleQuery::new("valid-id", "   ", "operator");

        let error = engine.evaluate(invalid).expect_err("blank task must fail");

        assert!(matches!(
            error,
            OracleQueryError::EmptyField { field: "task" }
        ));
        assert!(engine.get_history().is_empty());
    }

    #[test]
    fn query_validation_enforces_all_contract_bounds() {
        let mut too_many_context = OracleQuery::new("q", "task", "operator");
        too_many_context.context = vec!["evidence".to_string(); MAX_QUERY_CONTEXT_ITEMS + 1];
        assert!(matches!(
            too_many_context.validate(),
            Err(OracleQueryError::TooManyContextItems { .. })
        ));

        let mut oversized_context = OracleQuery::new("q", "task", "operator");
        oversized_context.context = vec!["x".repeat(MAX_QUERY_CONTEXT_ITEM_BYTES + 1)];
        assert!(matches!(
            oversized_context.validate(),
            Err(OracleQueryError::ContextItemTooLong { index: 0, .. })
        ));

        let mut oversized_evidence = OracleQuery::new("q-evidence", "task", "operator");
        oversized_evidence.evidence = vec![EvidenceRef::supplied(
            "report",
            "fixture://report",
            Utc::now(),
            "x".repeat(MAX_QUERY_CONTEXT_ITEM_BYTES + 1),
        )];
        assert!(matches!(
            oversized_evidence.validate(),
            Err(OracleQueryError::EvidenceFieldTooLong {
                index: 0,
                field: "excerpt",
                ..
            })
        ));

        for (mut invalid, expected_field) in [
            (
                OracleQuery::new("x".repeat(MAX_QUERY_ID_BYTES + 1), "task", "operator"),
                "id",
            ),
            (
                OracleQuery::new("q", "x".repeat(MAX_QUERY_TASK_BYTES + 1), "operator"),
                "task",
            ),
            (
                OracleQuery::new("q", "task", "x".repeat(MAX_QUERY_REQUESTER_BYTES + 1)),
                "requester",
            ),
        ] {
            assert!(matches!(
                invalid.validate(),
                Err(OracleQueryError::FieldTooLong { field, .. }) if field == expected_field
            ));
            invalid.task = "unused".to_string();
        }
    }

    #[test]
    fn query_contract_has_stable_snake_case_json_round_trip() {
        let timestamp = DateTime::parse_from_rfc3339("2025-01-02T03:04:05Z")
            .expect("timestamp")
            .with_timezone(&Utc);
        let mut original = OracleQuery::new("query-1", "review evidence", "operator");
        original.context = vec!["report:42".to_string()];
        original.timestamp = timestamp;
        original.query_type = QueryType::Financial;
        original.correlation_id = Some("objective-7".to_string());

        let encoded = serde_json::to_value(&original).expect("serialize query");

        assert_eq!(
            encoded,
            serde_json::json!({
                "id": "query-1",
                "task": "review evidence",
                "context": ["report:42"],
                "requester": "operator",
                "timestamp": "2025-01-02T03:04:05Z",
                "query_type": "financial",
                "correlation_id": "objective-7"
            })
        );
        let decoded: OracleQuery = serde_json::from_value(encoded).expect("deserialize query");
        assert_eq!(decoded, original);

        let legacy: OracleQuery = serde_json::from_value(serde_json::json!({
            "id": "legacy-query",
            "task": "review evidence",
            "context": [],
            "requester": "operator",
            "timestamp": "2025-01-02T03:04:05Z"
        }))
        .expect("legacy query without optional typed fields");
        assert_eq!(legacy.query_type, QueryType::General);
    }
}
