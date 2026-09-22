use crate::objectives::agenda::{read_agenda, AgendaObjective, OBJECTIVE_STORE_PATH};
use crate::objectives::ObjectiveState;
use arda_core::operator_projection::{
    ApprovalStatus, CapabilityProjection, CouncilProjection, DependencyHealth,
    DependencyProjection, EvidenceProjection, JouleWorkProjection, MeasurementSource,
    NodeProjection, ObjectiveBudgetProjection, ObjectiveProjection, ObjectiveStatus,
    OperatorProjection, PendingApprovalProjection, PersonalOperationsProjection,
    ProjectionAuthority, ProjectionFreshness, ReminderProjection, ReminderStatus, RunProjection,
    RunStatus, WorkerProjection,
};
use arda_core::personal_ops::{PersonalOpsRecord, ReminderDeliveryState};
use arda_core::run_graph::{CapabilityCompositionReceipt, NodeKind, NodeState, RunGraph};
use chrono::{DateTime, Utc};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::personal_ops::{build_projection, PersonalOpsLogStore, ProactiveCycleStore};
use crate::runs::RunStore;

pub const OPERATOR_PROJECTION_PATH: &str = "core/state/operator_projection.json";
pub const CURRENT_RUNS_PATH: &str = "data/workbench/current-runs.json";
static OPERATOR_PROJECTION_WRITE: Mutex<()> = Mutex::new(());

pub fn publish_operator_projection(
    root: &Path,
    generated_at: DateTime<Utc>,
) -> Result<OperatorProjection, OperatorProjectionPublishError> {
    let agenda = read_agenda(root).map_err(|error| {
        OperatorProjectionPublishError::InvalidCanonicalInput {
            path: root.join(OBJECTIVE_STORE_PATH),
            error: error.to_string(),
        }
    })?;
    let run_root = root.join("data/runs");
    let entries = match fs::read_dir(&run_root) {
        Ok(entries) => Some(entries),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(source) => {
            return Err(OperatorProjectionPublishError::Io {
                path: run_root,
                source,
            })
        }
    };
    let current_run_ids = agenda
        .iter()
        .flatten()
        .flat_map(|objective| objective.runs.iter().map(|(_, run)| run.clone()))
        .collect::<BTreeSet<_>>();
    let mut historical_run_count = 0;
    let mut run_directories = Vec::new();
    for entry in entries.into_iter().flatten() {
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
            if current_run_ids.contains(&entry.file_name().to_string_lossy().into_owned()) {
                run_directories.push(path);
            } else {
                historical_run_count += 1;
            }
        }
    }
    run_directories.sort();

    let mut graphs = Vec::new();
    for directory in &run_directories {
        let checkpoint = directory.join("checkpoint.json");
        let raw = match fs::read_to_string(&checkpoint) {
            Ok(raw) => raw,
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
    historical_run_count += graphs.len().saturating_sub(current_graphs.len());
    let graphs = current_graphs;
    let run_directories = current_directories;

    let runs = graphs
        .iter()
        .map(|graph| {
            let owner = agenda
                .iter()
                .flatten()
                .find(|objective| {
                    objective.runs.iter().any(|(leaf, run)| {
                        run == graph.run_id.as_str()
                            && (graph.objective_id.as_str() == objective.id
                                || graph.objective_id.as_str() == leaf)
                    })
                })
                .ok_or_else(|| OperatorProjectionPublishError::InvalidCanonicalInput {
                    path: root.join(OBJECTIVE_STORE_PATH),
                    error: format!(
                        "run {} has no matching canonical objective/leaf binding",
                        graph.run_id.as_str()
                    ),
                })?;
            Ok(project_run(graph, &owner.id))
        })
        .collect::<Result<Vec<_>, OperatorProjectionPublishError>>()?;
    let objectives = project_objectives(&graphs, &runs, agenda.as_deref().unwrap_or_default());
    let capabilities = project_capabilities(&run_directories)?;
    let councils = project_councils(&run_directories, &graphs)?;
    let personal_operations = project_personal_operations(root, generated_at)?;
    let (joulework, resource_configured) = project_joulework(root, &graphs)?;
    let evidence = project_evidence(root, &graphs)?;

    // Build pending approvals from run graph approval receipts.
    // Each receipt carries canonical approval identity, scope, and absolute expiry.
    let now = Utc::now();
    let pending_approvals: Vec<PendingApprovalProjection> = graphs
        .iter()
        .flat_map(|graph| {
            graph
                .approval_receipts
                .iter()
                .map(move |receipt| PendingApprovalProjection {
                    approval_id: receipt.receipt_id.to_string(),
                    run_id: graph.run_id.as_str().to_owned(),
                    node_id: None,
                    scope: receipt.approved_scope.clone(),
                    action_digest: receipt.proposal_id.to_string(),
                    expires_at: receipt.expires_at,
                    status: if receipt.expires_at < now {
                        ApprovalStatus::Expired
                    } else {
                        ApprovalStatus::Pending
                    },
                })
        })
        .collect();

    let approval_receipts_present = graphs.iter().any(|g| !g.approval_receipts.is_empty());

    let proactive_store = ProactiveCycleStore::new(root);
    let communications = proactive_store.project_communications().unwrap_or_default();

    let projection = OperatorProjection {
        schema_version: OperatorProjection::SCHEMA_VERSION.to_string(),
        projection_id: format!("operator-projection-{}", generated_at.timestamp_millis()),
        generated_at,
        authority: ProjectionAuthority::ReadOnly,
        freshness: ProjectionFreshness::Fresh,
        objectives,
        runs,
        capabilities,
        pending_approvals,
        councils,
        personal_operations,
        joulework,
        evidence,
        communications,
        dependencies: vec![
            dependency(
                "objective_store",
                if agenda.is_some() { DependencyHealth::Ready } else { DependencyHealth::NotConfigured },
                if agenda.is_some() { "current resident objectives; legacy queue is not read" } else { "resident objective store unavailable; no legacy fallback" }.to_string(),
            ),
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
                if approval_receipts_present {
                    DependencyHealth::Ready
                } else {
                    DependencyHealth::NotConfigured
                },
                if approval_receipts_present {
                    "canonical approval receipts persisted in run graphs"
                } else {
                    "no run graph approval receipts found; approval identity, scope, and expiry not yet persisted"
                }
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

fn project_run(graph: &RunGraph, objective_id: &str) -> RunProjection {
    RunProjection {
        run_id: graph.run_id.as_str().to_string(),
        objective_id: objective_id.to_string(),
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
    agenda: &[AgendaObjective],
) -> Vec<ObjectiveProjection> {
    agenda
        .iter()
        .map(|objective| {
            let graph = graphs.iter().find(|graph| {
                objective.runs.iter().any(|(leaf, run)| {
                    run == graph.run_id.as_str()
                        && (graph.objective_id.as_str() == objective.id
                            || graph.objective_id.as_str() == leaf)
                })
            });
            let run =
                graph.and_then(|graph| runs.iter().find(|run| run.run_id == graph.run_id.as_str()));
            let node = graph.and_then(current_node);
            let evidence = graph
                .into_iter()
                .flat_map(|graph| graph.nodes.iter())
                .filter_map(|node| node.output_digest.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            ObjectiveProjection {
                objective_id: objective.id.clone(),
                project_id: objective.project_id.clone(),
                title: objective.text.clone(),
                status: match objective.state {
                    ObjectiveState::PendingApproval | ObjectiveState::Paused => {
                        ObjectiveStatus::Blocked
                    }
                    ObjectiveState::Approved => ObjectiveStatus::Pending,
                    ObjectiveState::Running => ObjectiveStatus::Active,
                    ObjectiveState::Completed => ObjectiveStatus::Succeeded,
                    ObjectiveState::Cancelled => ObjectiveStatus::Cancelled,
                    ObjectiveState::Failed => ObjectiveStatus::Failed,
                },
                current_task_id: graph.and_then(|graph| {
                    objective
                        .runs
                        .iter()
                        .find(|(_, run)| run == graph.run_id.as_str())
                        .map(|(leaf, _)| leaf.clone())
                }),
                current_run_id: run.map(|run| run.run_id.clone()),
                current_node_id: node.map(|node| node.id.as_str().to_owned()),
                evidence,
                next_continuation: None,
                next_wake_at: objective
                    .next_wake_ms
                    .and_then(DateTime::from_timestamp_millis),
                provider_route: node
                    .and_then(|node| node.worker.as_ref())
                    .map(|worker| worker.route_id.clone()),
                budget: node.map(|node| ObjectiveBudgetProjection {
                    max_joules: node.budget.max_joules,
                    max_cost_usd: node.budget.max_cost_usd,
                }),
                blocker: match objective.state {
                    ObjectiveState::PendingApproval => Some("pending operator approval".to_owned()),
                    ObjectiveState::Paused => Some("paused by operator".to_owned()),
                    _ => None,
                },
            }
        })
        .collect()
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
