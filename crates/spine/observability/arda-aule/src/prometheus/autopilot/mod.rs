#![cfg(feature = "full-cli")]
// sigil: REPAIR
//! CEO autopilot — autonomous loop covering Phase 1-3 of CEO_LOOP.md plus
//! Arda Core execution, Oracle governance, and A2H human-in-the-loop hooks.

pub mod a2h;
pub mod bootstrap;
pub mod core_executor_bridge;
pub mod dashboard;
pub mod decomposer;
pub mod delegation;
pub mod evidence_registry;
pub mod governance_policy;
pub mod knowledge_triage;
pub mod learning;
pub mod learning_consumer;
pub mod oracle_gate;
pub mod outcomes;
pub mod pipeline_bridge;
pub mod planner;
pub mod queue_operation;
pub mod queue_writer;
pub mod reporting;
pub mod runner;
pub mod service_health;
pub mod source_registry;
pub mod task_queue;
pub mod taxonomy;
pub mod validator;

pub use a2h::{
    H2AProcessReport, HumanApprovedObjective, PendingAuthorization, PendingAuthorizationStatus,
    append_pending_authorization, authorize_for_escalation, authorize_for_escalation_with_id,
    process_h2a_responses, write_message,
};
pub use bootstrap::{LoadedDefaults, load_defaults, load_registry_from_world};
pub use core_executor_bridge::{
    Dispatch, ExecutionStatus, dispatch as executor_dispatch,
    dispatch_with_conditions as executor_dispatch_with_conditions,
};
pub use dashboard::{Alert, AlertSeverity, DashboardSnapshot};
pub use decomposer::{Objective, ObjectiveDecomposer, PlannedTask, Priority};
pub use delegation::{
    AgentCapabilities, AgentRegistry, Delegation, DelegationReport, delegate_plan,
};
pub use evidence_registry::{EVIDENCE_REGISTRY_CONTRACT, EvidenceRecord, EvidenceRegistry};
pub use knowledge_triage::{
    AutonomyLane, KNOWLEDGE_ACTIONABLE_REVIEW_GATE, KNOWLEDGE_ACTIONABLE_REVIEW_SCHEMA,
    KNOWLEDGE_SAFE_LOCAL_PROMOTION_GATE, KNOWLEDGE_TASK_EXECUTION_RECEIPT_SCHEMA,
    KNOWLEDGE_TASK_PROMOTION_RECEIPT_SCHEMA, KnowledgeActionableReviewRecord,
    KnowledgeClassification, KnowledgeExecutionDecision, KnowledgeTaskExecutionReceipt,
    KnowledgeTaskExecutionReport, KnowledgeTaskPromotionReceipt, KnowledgeTaskPromotionReport,
    KnowledgeTriageConfig, KnowledgeTriageRecord, KnowledgeTriageReport, PromotionDecision,
    classify_knowledge_source, execute_knowledge_task_queue, promote_knowledge_tasks,
    run_knowledge_triage,
};
pub use learning::{LearningState, LearningStore, OutcomeStats};
pub use learning_consumer::{
    LearningConsumerError, LearningConsumptionReceipt, LearningCycleInput, LearningCycleMetrics,
    LearningCycleReceipt, LearningCycleReport, LearningDisposition, LearningLoopPolicy,
    LearningLoopSwitches, consume_approved_delta, emit_research_suggestion, run_learning_cycle,
};
pub use oracle_gate::{GateDecision, OracleGate};
pub use outcomes::{ObservedCursor, OutcomeObserver};
pub use pipeline_bridge::submit_plan as submit_plan_to_pipeline;
pub use queue_operation::QueueOperationStatus;
pub use queue_writer::{
    append_apollo_dispatch_attempt_to_queue, append_apollo_dispatch_to_queue, append_plan_to_queue,
    append_plan_to_queue_with_conditions, task_id_for,
};
pub use reporting::write_daily_report;
pub use runner::{
    AutonomyPreflightReport, AutonomyPreflightSummary, AutopilotConfig, CeoAutopilot, CycleReport,
    PlanCycle, ceo_loop, inspect_autonomy_preflight, write_autonomy_preflight,
};
pub use service_health::{
    ServiceHealth, ServiceHealthMonitor, ServiceHealthReport, SystemdQuery, UserSystemd,
};
pub use source_registry::{SOURCE_REGISTRY_CONTRACT, SourceDescriptor, SourceRegistry};
pub use task_queue::{QueueRecord, TaskQueueAnalyzer, TaskQueueMetrics};
pub use taxonomy::{CANONICAL_TYPES, canonical, is_apollo_dispatchable};
pub use validator::{PlanValidator, ValidationResult};
