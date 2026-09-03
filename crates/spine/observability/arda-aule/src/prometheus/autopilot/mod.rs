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
pub mod execution_outcome;
pub mod executive_cycle;
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
pub mod schedule;
pub mod service_health;
pub mod source_registry;
pub mod task_queue;
pub mod taxonomy;
pub mod validator;
pub mod workbench_executor;

pub use a2h::{
    append_pending_authorization, authorize_for_escalation, authorize_for_escalation_with_id,
    process_h2a_responses, review_arandur_recommendation, write_message, H2AProcessReport,
    HumanApprovedObjective, PendingAuthorization, PendingAuthorizationStatus,
    RecommendationReviewReceipt,
};
pub use bootstrap::{load_defaults, load_registry_from_world, LoadedDefaults};
pub use core_executor_bridge::{
    dispatch as executor_dispatch, dispatch_with_conditions as executor_dispatch_with_conditions,
    Dispatch, ExecutionStatus,
};
pub use dashboard::{Alert, AlertSeverity, DashboardSnapshot};
pub use decomposer::{
    Objective, ObjectiveContextSource, ObjectiveDecomposer, ObjectivePlan, PlannedTask, Priority,
};
pub use delegation::{
    delegate_plan, AgentCapabilities, AgentRegistry, Delegation, DelegationReport,
};
pub use evidence_registry::{EvidenceRecord, EvidenceRegistry, EVIDENCE_REGISTRY_CONTRACT};
pub use execution_outcome::{
    project_terminal_outcome, ExecutionOutcomeProjectionReceipt, EXECUTION_OUTCOME_CONTRACT,
};
pub use executive_cycle::{
    CouncilMode, ExecutiveCycleError, ExecutiveCycleInput, ExecutiveCycleReceipt,
    ExecutiveCycleResult, ExecutiveCycleStore, ExecutiveDisposition, ExecutivePhase,
    ExecutiveResourceBudget, RoleRequest, EXECUTIVE_CYCLE_CONTRACT, EXECUTIVE_CYCLE_LEDGER,
};
pub use knowledge_triage::{
    classify_knowledge_source, execute_knowledge_task_queue, promote_knowledge_tasks,
    run_knowledge_triage, AutonomyLane, KnowledgeActionableReviewRecord, KnowledgeClassification,
    KnowledgeExecutionDecision, KnowledgeTaskExecutionReceipt, KnowledgeTaskExecutionReport,
    KnowledgeTaskPromotionReceipt, KnowledgeTaskPromotionReport, KnowledgeTriageConfig,
    KnowledgeTriageRecord, KnowledgeTriageReport, PromotionDecision,
    KNOWLEDGE_ACTIONABLE_REVIEW_GATE, KNOWLEDGE_ACTIONABLE_REVIEW_SCHEMA,
    KNOWLEDGE_SAFE_LOCAL_PROMOTION_GATE, KNOWLEDGE_TASK_EXECUTION_RECEIPT_SCHEMA,
    KNOWLEDGE_TASK_PROMOTION_RECEIPT_SCHEMA,
};
pub use learning::{LearningState, LearningStore, OutcomeStats};
pub use learning_consumer::{
    consume_approved_delta, emit_research_suggestion, run_learning_cycle, LearningConsumerError,
    LearningConsumptionReceipt, LearningCycleInput, LearningCycleMetrics, LearningCycleReceipt,
    LearningCycleReport, LearningDisposition, LearningLoopPolicy, LearningLoopSwitches,
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
    ceo_loop, inspect_autonomy_preflight, write_autonomy_preflight, AutonomyPreflightReport,
    AutonomyPreflightSummary, AutopilotConfig, CeoAutopilot, CycleReport, PlanCycle,
};
pub use schedule::{ScheduleLedger, ScheduleMode, ScheduleRecord, ScheduleState};
pub use service_health::{
    ServiceHealth, ServiceHealthMonitor, ServiceHealthReport, SystemdQuery, UserSystemd,
};
pub use source_registry::{SourceDescriptor, SourceRegistry, SOURCE_REGISTRY_CONTRACT};
pub use task_queue::{QueueRecord, QueueRecordStatus, TaskQueueAnalyzer, TaskQueueMetrics};
pub use taxonomy::{canonical, is_apollo_dispatchable, CANONICAL_TYPES};
pub use validator::{PlanValidator, ValidationResult};
pub use workbench_executor::{
    ExplicitExecutionOutcome, ExplicitReceiptReference, ExplicitWorkbenchWorkItem,
    QueueExecutionReceipt, WorkbenchExecutionAdapter,
};
