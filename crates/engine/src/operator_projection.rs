use arda_aule::prometheus::autopilot::{
    QueueRecord, QueueRecordStatus, ScheduleLedger, ScheduleRecord, ScheduleState,
    TaskQueueAnalyzer,
};
use arda_core::operator_projection::{
    CapabilityProjection, CommunicationProjection, CouncilProjection, DependencyHealth,
    DependencyProjection, EvidenceProjection, JouleWorkProjection, MeasurementSource,
    NodeProjection, ObjectiveBudgetProjection, ObjectiveProjection, ObjectiveStatus,
    OperatorProjection, PersonalOperationsProjection, ProjectionAuthority, ProjectionFreshness,
    ReminderProjection, ReminderStatus, RunProjection, RunStatus, WorkerProjection,
};
use arda_core::personal_ops::{PersonalOpsRecord, ReminderDeliveryState};
use arda_core::run_graph::{CapabilityCompositionReceipt, NodeKind, NodeState, RunGraph};
use chrono::{DateTime, Utc};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::personal_ops::{build_projection, PersonalOpsLogStore};
use crate::runs::RunStore;

pub const OPERATOR_PROJECTION_PATH: &str = "core/state/operator_projection.json";
pub const CURRENT_RUNS_PATH: &str = "data/workbench/current-runs.json";
static OPERATOR_PROJECTION_WRITE: Mutex<()> = Mutex::new(());

pub fn publish_operator_projection(
    root: &Path,
    generated_at: DateTime<Utc>,
) -> Result<OperatorProjection, OperatorProjectionPublishError> {
    let run_root = root.join("data/runs");
    let entries = fs::read_dir(&run_root).map_err(|source| OperatorProjectionPublishError::Io {
        path: run_root.clone(),
        source,
    })?;
    let mut run_directories = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| OperatorProjectionPublishError::Io {
            path: run_root.clone(),
            source,
        })?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|source| OperatorProjectionPublishError::Io {
                path: path.clone(),
                source,
            })?;
        if kind.is_dir() {
            run_directories.push(path);
        }
    }
    run_directories.sort();

    let mut graphs = Vec::new();
    for directory in &run_directories {
        let checkpoint = directory.join("checkpoint.json");
        let raw = match fs::read_to_string(&checkpoint) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(source) => {
                return Err(OperatorProjectionPublishError::Io {
                    path: checkpoint,
                    source,
                })
            }
        };
        let graph = RunGraph::from_json_str(&raw).map_err(|error| {
            OperatorProjectionPublishError::InvalidCanonicalInput {
                path: checkpoint,
                error: error.to_string(),
            }
        })?;
        graphs.push(graph);
    }

    let total_run_count = graphs.len();
    let current_run_ids = load_current_run_ids(root)?;
    let mut current_directories = Vec::new();
    let mut current_graphs = Vec::new();
    for (directory, graph) in run_directories.iter().zip(graphs.iter()) {
        if current_run_ids.contains(graph.run_id.as_str())
            && !matches!(
                derive_run_status(graph),
                RunStatus::Succeeded | RunStatus::Failed | RunStatus::Cancelled
            )
        {
            current_directories.push(directory.clone());
            current_graphs.push(graph.clone());
        }
    }
    let historical_run_count = total_run_count.saturating_sub(current_graphs.len());
    let graphs = current_graphs;
    let run_directories = current_directories;

    let runs = graphs.iter().map(project_run).collect::<Vec<_>>();
    let queue_records = load_effective_queue_records(root)?;
    let schedules = load_effective_schedules(root)?;
    let objectives = project_objectives(&graphs, &runs, &queue_records, &schedules);
    let capabilities = project_capabilities(&run_directories)?;
    let councils = project_councils(&run_directories, &graphs)?;
    let personal_operations = project_personal_operations(root, generated_at)?;
    let (joulework, resource_configured) = project_joulework(root, &graphs)?;
    let evidence = project_evidence(root, &graphs)?;

    let projection = OperatorProjection {
        schema_version: OperatorProjection::SCHEMA_VERSION.to_string(),
        projection_id: format!("operator-projection-{}", generated_at.timestamp_millis()),
        generated_at,
        authority: ProjectionAuthority::ReadOnly,
        freshness: ProjectionFreshness::Fresh,
        objectives,
        runs,
        capabilities,
        // A run graph does not persist the canonical approval identity, scope,
        // or absolute approval expiry required by the shared contract.
        pending_approvals: Vec::new(),
        councils,
        personal_operations,
        joulework,
        evidence,
        communications: Vec::<CommunicationProjection>::new(),
        dependencies: vec![
            dependency(
                "run_store",
                DependencyHealth::Ready,
                format!(
                    "{} current validated checkpoint(s); {} historical checkpoint(s) retained outside the current agenda",
                    graphs.len(), historical_run_count
                ),
            ),
            dependency(
                "resource_ledger",
                if resource_configured {
                    DependencyHealth::Ready
                } else {
                    DependencyHealth::NotConfigured
                },
                if resource_configured {
                    "canonical resource ledger loaded"
                } else {
                    "canonical resource ledger is not configured"
                }
                .to_string(),
            ),
            dependency(
                "personal_ops_store",
                if root.join("data/personal/events.jsonl").is_file() {
                    DependencyHealth::Ready
                } else {
                    DependencyHealth::NotConfigured
                },
                "canonical personal-operations log inspected".to_string(),
            ),
            dependency(
                "approval_expiry_store",
                DependencyHealth::NotConfigured,
                "run graphs do not persist canonical approval identity, scope, and absolute expiry"
                    .to_string(),
            ),
        ],
    };
    projection
        .validate()
        .map_err(|error| OperatorProjectionPublishError::InvalidProjection(error.to_string()))?;
    atomic_write_projection(root, &projection)?;
    Ok(projection)
}

fn load_current_run_ids(root: &Path) -> Result<BTreeSet<String>, OperatorProjectionPublishError> {
    let path = root.join(CURRENT_RUNS_PATH);
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeSet::new()),
        Err(source) => return Err(OperatorProjectionPublishError::Io { path, source }),
    };
    let value: serde_json::Value = serde_json::from_str(&raw).map_err(|error| {
        OperatorProjectionPublishError::InvalidCanonicalInput {
            path: path.clone(),
            error: error.to_string(),
        }
    })?;
    if value["schema_version"] != "arda.workbench.current-runs.v1" {
        return Err(OperatorProjectionPublishError::InvalidCanonicalInput {
            path,
            error: "unsupported current-run registry version".to_string(),
        });
    }
    let ids = value["run_ids"].as_array().ok_or_else(|| {
        OperatorProjectionPublishError::InvalidCanonicalInput {
            path: root.join(CURRENT_RUNS_PATH),
            error: "current-run registry requires a run_ids array".to_string(),
        }
    })?;
    Ok(ids
        .iter()
        .filter_map(|value| value.as_str())
        .map(str::to_owned)
        .collect())
}

fn project_run(graph: &RunGraph) -> RunProjection {
    RunProjection {
        run_id: graph.run_id.as_str().to_string(),
        objective_id: graph.objective_id.as_str().to_string(),
        status: derive_run_status(graph),
        nodes: graph
            .nodes
            .iter()
            .map(|node| NodeProjection {
                node_id: node.id.as_str().to_string(),
                kind: node.kind,
                state: node.state,
            })
            .collect(),
        workers: graph
            .nodes
            .iter()
            .filter_map(|node| {
                node.worker.as_ref().map(|worker| WorkerProjection {
                    node_id: node.id.as_str().to_string(),
                    role: worker.role,
                    worker_id: worker.worker_id.clone(),
                    route_id: worker.route_id.clone(),
                    state: node.state,
                })
            })
            .collect(),
    }
}

fn derive_run_status(graph: &RunGraph) -> RunStatus {
    let states = graph
        .nodes
        .iter()
        .map(|node| node.state)
        .collect::<Vec<_>>();
    if states.contains(&NodeState::Running) {
        RunStatus::Running
    } else if graph.nodes.iter().any(|node| {
        node.kind == NodeKind::Approval
            && matches!(
                node.state,
                NodeState::Pending | NodeState::Ready | NodeState::Blocked
            )
    }) {
        RunStatus::AwaitingApproval
    } else if states.contains(&NodeState::Failed) {
        RunStatus::Failed
    } else if states.contains(&NodeState::Blocked) {
        RunStatus::Blocked
    } else if !states.is_empty() && states.iter().all(|state| *state == NodeState::Succeeded) {
        RunStatus::Succeeded
    } else if states.contains(&NodeState::Cancelled) {
        RunStatus::Cancelled
    } else {
        RunStatus::Pending
    }
}

fn project_objectives(
    graphs: &[RunGraph],
    runs: &[RunProjection],
    queue_records: &[QueueRecord],
    schedules: &BTreeMap<String, ScheduleRecord>,
) -> Vec<ObjectiveProjection> {
    let mut grouped = BTreeMap::<String, (Option<String>, Vec<RunStatus>, Vec<usize>)>::new();
    for (graph, run) in graphs.iter().zip(runs) {
        let project_id = graph
            .provenance
            .project_contract_digest
            .strip_prefix("project:")
            .map(ToOwned::to_owned);
        let entry = grouped
            .entry(graph.objective_id.as_str().to_string())
            .or_insert_with(|| (project_id, Vec::new(), Vec::new()));
        entry.1.push(run.status);
        entry.2.push(
            runs.iter()
                .position(|candidate| candidate.run_id == run.run_id)
                .expect("run projection belongs to current graph set"),
        );
    }
    let mut objectives = grouped
        .into_iter()
        .map(|(objective_id, (project_id, statuses, run_indexes))| {
            let preferred_queue =
                select_queue_control_for_runs(queue_records, &objective_id, runs, &run_indexes);
            let queue_bound_run_index =
                preferred_queue
                    .and_then(queue_workbench_run_id)
                    .and_then(|queue_run_id| {
                        run_indexes.iter().copied().find(|index| {
                            runs[*index].run_id == queue_run_id
                                && !matches!(
                                    runs[*index].status,
                                    RunStatus::Succeeded | RunStatus::Failed | RunStatus::Cancelled
                                )
                        })
                    });
            let run_index = queue_bound_run_index.or_else(|| {
                run_indexes
                    .iter()
                    .copied()
                    .find(|index| {
                        !matches!(
                            runs[*index].status,
                            RunStatus::Succeeded | RunStatus::Failed | RunStatus::Cancelled
                        )
                    })
                    .or_else(|| run_indexes.last().copied())
            });
            let graph = run_index.map(|index| &graphs[index]);
            let run = run_index.map(|index| &runs[index]);
            let queue = preferred_queue.filter(|record| match queue_workbench_run_id(record) {
                Some(queue_run_id) => run.is_some_and(|run| run.run_id == queue_run_id),
                None => true,
            });
            let current_node = graph.and_then(current_node);
            let mut evidence = graph
                .into_iter()
                .flat_map(|graph| graph.nodes.iter())
                .filter_map(|node| node.output_digest.clone())
                .collect::<Vec<_>>();
            let mut seen_evidence = BTreeSet::new();
            evidence.retain(|digest| seen_evidence.insert(digest.clone()));
            if let Some(digest) = queue.and_then(|record| {
                record
                    .extra
                    .get("execution_receipt_digest")
                    .and_then(serde_json::Value::as_str)
            }) {
                if !evidence.iter().any(|item| item == digest) {
                    evidence.push(digest.to_string());
                }
            }
            let schedule = queue
                .and_then(|record| schedules.get(&record.id))
                .filter(|schedule| schedule.objective_id == objective_id);
            ObjectiveProjection {
                title: queue
                    .and_then(|record| record.title.clone())
                    .unwrap_or_else(|| objective_id.clone()),
                objective_id,
                project_id,
                status: queue
                    .map(objective_status_from_queue)
                    .unwrap_or_else(|| derive_objective_status(&statuses)),
                current_task_id: queue.map(|record| record.id.clone()),
                current_run_id: run.map(|run| run.run_id.clone()),
                current_node_id: current_node.map(|node| node.id.as_str().to_string()),
                evidence,
                next_continuation: queue.and_then(|record| {
                    record
                        .extra
                        .get("continuation_decision")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned)
                }),
                next_wake_at: schedule.and_then(|schedule| {
                    (schedule.state == ScheduleState::Scheduled)
                        .then_some(schedule.not_before_utc)
                        .flatten()
                }),
                provider_route: current_node
                    .and_then(|node| node.worker.as_ref())
                    .map(|worker| worker.route_id.clone()),
                budget: current_node.map(|node| ObjectiveBudgetProjection {
                    max_joules: node.budget.max_joules,
                    max_cost_usd: node.budget.max_cost_usd,
                }),
                blocker: queue.and_then(|record| {
                    record
                        .extra
                        .get("detail")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned)
                }),
            }
        })
        .collect::<Vec<_>>();
    let projected_ids = objectives
        .iter()
        .map(|objective| objective.objective_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut queue_only = BTreeMap::<String, Vec<&QueueRecord>>::new();
    for record in queue_records {
        let Some(objective_id) = queue_objective_id(record) else {
            continue;
        };
        if projected_ids.contains(objective_id) || queue_status_is_terminal(record) {
            continue;
        }
        queue_only
            .entry(objective_id.to_string())
            .or_default()
            .push(record);
    }
    objectives.extend(
        queue_only
            .into_iter()
            .filter_map(|(objective_id, records)| {
                let record = records
                    .into_iter()
                    .min_by_key(|record| queue_control_priority(record))?;
                let schedule = schedules
                    .get(&record.id)
                    .filter(|schedule| schedule.objective_id == objective_id);
                let evidence = record
                    .extra
                    .get("execution_receipt_digest")
                    .and_then(serde_json::Value::as_str)
                    .map(|digest| vec![digest.to_string()])
                    .unwrap_or_default();
                Some(ObjectiveProjection {
                    objective_id,
                    project_id: queue_project_id(record).map(str::to_owned),
                    title: record.title.clone().unwrap_or_else(|| record.id.clone()),
                    status: objective_status_from_queue(record),
                    current_task_id: Some(record.id.clone()),
                    current_run_id: None,
                    current_node_id: None,
                    evidence,
                    next_continuation: record
                        .extra
                        .get("continuation_decision")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned),
                    next_wake_at: schedule.and_then(|schedule| {
                        (schedule.state == ScheduleState::Scheduled)
                            .then_some(schedule.not_before_utc)
                            .flatten()
                    }),
                    provider_route: None,
                    budget: None,
                    blocker: record
                        .extra
                        .get("detail")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned),
                })
            }),
    );
    objectives
}

fn load_effective_queue_records(
    root: &Path,
) -> Result<Vec<QueueRecord>, OperatorProjectionPublishError> {
    let path = root.join("core/projects/tasks/queue.jsonl");
    if !path.is_file() {
        return Ok(Vec::new());
    }
    TaskQueueAnalyzer::new(&path)
        .load()
        .map(TaskQueueAnalyzer::effective_records)
        .map_err(
            |error| OperatorProjectionPublishError::InvalidCanonicalInput {
                path,
                error: error.to_string(),
            },
        )
}

fn load_effective_schedules(
    root: &Path,
) -> Result<BTreeMap<String, ScheduleRecord>, OperatorProjectionPublishError> {
    let path = root.join("core/projects/tasks/schedules.jsonl");
    ScheduleLedger::new(&path).effective().map_err(|error| {
        OperatorProjectionPublishError::InvalidCanonicalInput {
            path,
            error: error.to_string(),
        }
    })
}

fn queue_objective_id(record: &QueueRecord) -> Option<&str> {
    record
        .extra
        .get("meta")
        .and_then(serde_json::Value::as_object)
        .and_then(|meta| meta.get("objective_id"))
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            record
                .extra
                .get("source_objective_packet_id")
                .and_then(serde_json::Value::as_str)
        })
}

fn queue_workbench_run_id(record: &QueueRecord) -> Option<&str> {
    record
        .extra
        .get("workbench_run_id")
        .and_then(serde_json::Value::as_str)
}

fn queue_project_id(record: &QueueRecord) -> Option<&str> {
    record
        .extra
        .get("meta")
        .and_then(serde_json::Value::as_object)
        .and_then(|meta| meta.get("project_id"))
        .and_then(serde_json::Value::as_str)
}

fn queue_status_is_terminal(record: &QueueRecord) -> bool {
    record.canonical_status().is_terminal()
}

fn queue_control_priority(record: &QueueRecord) -> u8 {
    match record.canonical_status() {
        QueueRecordStatus::InProgress => 0,
        QueueRecordStatus::Pending => 1,
        QueueRecordStatus::Blocked => 2,
        QueueRecordStatus::Completed
        | QueueRecordStatus::Failed
        | QueueRecordStatus::Cancelled
        | QueueRecordStatus::Other => 3,
    }
}

fn select_queue_control_for_runs<'a>(
    records: &'a [QueueRecord],
    objective_id: &str,
    runs: &[RunProjection],
    run_indexes: &[usize],
) -> Option<&'a QueueRecord> {
    records
        .iter()
        .filter(|record| {
            !queue_status_is_terminal(record)
                && queue_objective_id(record) == Some(objective_id)
                && queue_workbench_run_id(record).is_none_or(|queue_run_id| {
                    run_indexes.iter().copied().any(|index| {
                        runs[index].run_id == queue_run_id
                            && !matches!(
                                runs[index].status,
                                RunStatus::Succeeded | RunStatus::Failed | RunStatus::Cancelled
                            )
                    })
                })
        })
        .min_by_key(|record| {
            (
                queue_control_priority(record),
                u8::from(queue_workbench_run_id(record).is_none()),
            )
        })
}

fn objective_status_from_queue(record: &QueueRecord) -> ObjectiveStatus {
    match record.canonical_status() {
        QueueRecordStatus::InProgress => ObjectiveStatus::Active,
        QueueRecordStatus::Blocked => ObjectiveStatus::Blocked,
        QueueRecordStatus::Failed => ObjectiveStatus::Failed,
        QueueRecordStatus::Cancelled => ObjectiveStatus::Cancelled,
        QueueRecordStatus::Completed => ObjectiveStatus::Succeeded,
        QueueRecordStatus::Pending | QueueRecordStatus::Other => ObjectiveStatus::Pending,
    }
}

fn current_node(graph: &RunGraph) -> Option<&arda_core::run_graph::RunNode> {
    graph
        .nodes
        .iter()
        .find(|node| node.state == NodeState::Running)
        .or_else(|| {
            graph.nodes.iter().find(|node| {
                matches!(
                    node.state,
                    NodeState::Ready | NodeState::Blocked | NodeState::Pending
                )
            })
        })
        .or_else(|| graph.nodes.last())
}

fn derive_objective_status(statuses: &[RunStatus]) -> ObjectiveStatus {
    if statuses.iter().any(|status| {
        matches!(
            status,
            RunStatus::Running | RunStatus::AwaitingApproval | RunStatus::Pending
        )
    }) {
        ObjectiveStatus::Active
    } else if statuses.contains(&RunStatus::Blocked) {
        ObjectiveStatus::Blocked
    } else if !statuses.is_empty()
        && statuses
            .iter()
            .all(|status| *status == RunStatus::Succeeded)
    {
        ObjectiveStatus::Succeeded
    } else if statuses.contains(&RunStatus::Failed) {
        ObjectiveStatus::Failed
    } else if statuses.contains(&RunStatus::Cancelled) {
        ObjectiveStatus::Cancelled
    } else {
        ObjectiveStatus::Pending
    }
}

fn project_capabilities(
    run_directories: &[PathBuf],
) -> Result<Vec<CapabilityProjection>, OperatorProjectionPublishError> {
    let mut projected = BTreeMap::new();
    for directory in run_directories {
        let path = directory.join("capability-composition.json");
        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(source) => return Err(OperatorProjectionPublishError::Io { path, source }),
        };
        let receipt: CapabilityCompositionReceipt =
            serde_json::from_str(&raw).map_err(|error| {
                OperatorProjectionPublishError::InvalidCanonicalInput {
                    path: path.clone(),
                    error: error.to_string(),
                }
            })?;
        for decision in receipt.decisions {
            let optional = decision
                .reasons
                .iter()
                .any(|reason| reason == "not_required_by_signed_contract_or_role");
            let capability_id = decision.capability.id;
            let candidate = CapabilityProjection {
                capability_id: capability_id.clone(),
                version: decision.capability.version,
                health: if decision.selected {
                    DependencyHealth::Ready
                } else {
                    DependencyHealth::Unavailable
                },
                selected: decision.selected,
                optional,
                selection_reasons: decision.reasons,
            };
            match projected.entry(capability_id) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(candidate);
                }
                std::collections::btree_map::Entry::Occupied(mut entry)
                    if candidate.selected && !entry.get().selected =>
                {
                    entry.insert(candidate);
                }
                std::collections::btree_map::Entry::Occupied(_) => {}
            }
        }
    }
    Ok(projected.into_values().collect())
}

fn project_councils(
    run_directories: &[PathBuf],
    graphs: &[RunGraph],
) -> Result<Vec<CouncilProjection>, OperatorProjectionPublishError> {
    let graphs_by_id = graphs
        .iter()
        .map(|graph| (graph.run_id.as_str(), graph))
        .collect::<BTreeMap<_, _>>();
    let mut councils = Vec::new();
    for directory in run_directories {
        let path = directory.join("council-run.json");
        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(source) => return Err(OperatorProjectionPublishError::Io { path, source }),
        };
        let council: arda_core::council_run::CouncilRun =
            serde_json::from_str(&raw).map_err(|error| {
                OperatorProjectionPublishError::InvalidCanonicalInput {
                    path: path.clone(),
                    error: error.to_string(),
                }
            })?;
        let graph = graphs_by_id.get(council.run_id.as_str()).ok_or_else(|| {
            OperatorProjectionPublishError::InvalidCanonicalInput {
                path: path.clone(),
                error: format!("council references missing run {}", council.run_id),
            }
        })?;
        council.validate(graph).map_err(|error| {
            OperatorProjectionPublishError::InvalidCanonicalInput {
                path: path.clone(),
                error: error.to_string(),
            }
        })?;
        councils.push(CouncilProjection {
            council_id: council.council_id,
            run_id: council.run_id,
            state: format!("{:?}", council.state).to_ascii_lowercase(),
            synthesis: council.synthesis,
            material_tensions: council
                .material_tensions
                .into_iter()
                .map(|item| item.summary)
                .collect(),
            non_approval: council.non_approval,
        });
    }
    Ok(councils)
}

fn project_personal_operations(
    root: &Path,
    generated_at: DateTime<Utc>,
) -> Result<PersonalOperationsProjection, OperatorProjectionPublishError> {
    let store = PersonalOpsLogStore::new(root);
    let events = store.load_all().map_err(|error| {
        OperatorProjectionPublishError::InvalidCanonicalInput {
            path: store.events_path.clone(),
            error: error.to_string(),
        }
    })?;
    let captures = events
        .iter()
        .filter(|event| matches!(event.record, PersonalOpsRecord::CaptureRecorded(_)))
        .count();
    let projection = build_projection(&events, generated_at, generated_at.date_naive());
    let resumable_items =
        projection.today.len() + projection.waiting.len() + projection.scheduled.len();
    let reminders = projection
        .today
        .iter()
        .chain(&projection.waiting)
        .chain(&projection.scheduled)
        .filter_map(|item| {
            let reminder_id = item.reminder_id?;
            let state = item.reminder_state.as_ref()?;
            Some(ReminderProjection {
                reminder_id: reminder_id.to_string(),
                item_id: item.item_id.to_string(),
                status: match state.delivery_state {
                    ReminderDeliveryState::Attempted => ReminderStatus::Pending,
                    ReminderDeliveryState::Delivered => ReminderStatus::Delivered,
                    ReminderDeliveryState::Acknowledged => ReminderStatus::Acknowledged,
                    ReminderDeliveryState::Deferred => ReminderStatus::Deferred,
                    ReminderDeliveryState::Dismissed => ReminderStatus::Dismissed,
                    ReminderDeliveryState::Failed => ReminderStatus::Failed,
                },
                next_due_at: item
                    .due_at
                    .as_deref()
                    .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                    .map(|value| value.with_timezone(&Utc)),
            })
        })
        .collect();
    Ok(PersonalOperationsProjection {
        captures,
        resumable_items,
        reminders,
    })
}

fn project_joulework(
    root: &Path,
    graphs: &[RunGraph],
) -> Result<(JouleWorkProjection, bool), OperatorProjectionPublishError> {
    let budget_joules = graphs
        .iter()
        .flat_map(|graph| graph.nodes.iter())
        .map(|node| node.budget.max_joules)
        .sum::<f64>();
    let ledger_path = root.join("data/resource-ledger/events.jsonl");
    let configured = ledger_path.is_file();
    let rollup = if let Some(graph) = graphs.first() {
        let store = RunStore::open(root, graph.run_id.clone()).map_err(|error| {
            OperatorProjectionPublishError::InvalidCanonicalInput {
                path: ledger_path.clone(),
                error: error.to_string(),
            }
        })?;
        store.resource_rollup_since(0, None).map_err(|error| {
            OperatorProjectionPublishError::InvalidCanonicalInput {
                path: ledger_path.clone(),
                error: error.to_string(),
            }
        })?
    } else {
        Default::default()
    };
    let consumed_joules = rollup.local_joulework;
    let (source, confidence) = match (rollup.observed_entries, rollup.default_entries) {
        (0, 0) => (MeasurementSource::Unknown, 0.0),
        (_, 0) => (MeasurementSource::Observed, 1.0),
        (0, _) => (MeasurementSource::DefaultFallback, 0.5),
        _ => (MeasurementSource::Estimated, 0.75),
    };
    Ok((
        JouleWorkProjection {
            budget_joules,
            consumed_joules,
            remaining_joules: (budget_joules - consumed_joules).max(0.0),
            source,
            source_confidence: confidence,
        },
        configured,
    ))
}

fn project_evidence(
    root: &Path,
    graphs: &[RunGraph],
) -> Result<Vec<EvidenceProjection>, OperatorProjectionPublishError> {
    let mut evidence = Vec::new();
    let mut seen = BTreeSet::new();
    for graph in graphs {
        let store = RunStore::open(root, graph.run_id.clone()).map_err(|error| {
            OperatorProjectionPublishError::InvalidCanonicalInput {
                path: root.join("data/runs").join(graph.run_id.as_str()),
                error: error.to_string(),
            }
        })?;
        let recovered = store.recover().map_err(|error| {
            OperatorProjectionPublishError::InvalidCanonicalInput {
                path: store.events_path(),
                error: error.to_string(),
            }
        })?;
        for event in recovered.events {
            if let crate::runs::RunEventKind::EvidenceLinked {
                evidence_id,
                evidence_path,
                authority,
            } = event.kind
            {
                if seen.insert(evidence_id.clone()) {
                    evidence.push(EvidenceProjection {
                        evidence_id,
                        kind: authority,
                        uri: evidence_path,
                        observed_at: DateTime::<Utc>::from_timestamp_millis(
                            i64::try_from(event.recorded_at_unix_ms).unwrap_or(i64::MAX),
                        )
                        .unwrap_or(DateTime::<Utc>::UNIX_EPOCH),
                        freshness: ProjectionFreshness::Fresh,
                    });
                }
            }
        }
    }
    Ok(evidence)
}

fn dependency(id: &str, health: DependencyHealth, detail: String) -> DependencyProjection {
    DependencyProjection {
        dependency_id: id.to_string(),
        health,
        freshness: ProjectionFreshness::Fresh,
        detail,
    }
}

fn atomic_write_projection(
    root: &Path,
    projection: &OperatorProjection,
) -> Result<(), OperatorProjectionPublishError> {
    let _write = OPERATOR_PROJECTION_WRITE.lock().map_err(|_| {
        OperatorProjectionPublishError::InvalidProjection(
            "operator projection writer lock was poisoned".to_string(),
        )
    })?;
    let path = root.join(OPERATOR_PROJECTION_PATH);
    let parent = path.parent().expect("projection path has parent");
    fs::create_dir_all(parent).map_err(|source| OperatorProjectionPublishError::Io {
        path: parent.to_path_buf(),
        source,
    })?;
    let temporary = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(projection)
        .map_err(|error| OperatorProjectionPublishError::InvalidProjection(error.to_string()))?;
    let mut file =
        fs::File::create(&temporary).map_err(|source| OperatorProjectionPublishError::Io {
            path: temporary.clone(),
            source,
        })?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|source| OperatorProjectionPublishError::Io {
            path: temporary.clone(),
            source,
        })?;
    fs::rename(&temporary, &path)
        .map_err(|source| OperatorProjectionPublishError::Io { path, source })?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum OperatorProjectionPublishError {
    #[error("operator projection I/O failed at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid canonical operator input at {path}: {error}")]
    InvalidCanonicalInput { path: PathBuf, error: String },
    #[error("generated operator projection is invalid: {0}")]
    InvalidProjection(String),
}
