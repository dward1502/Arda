mod hygiene;

use anyhow::{Context, Result};
use arda_core::next_action::{
    select_next_action, NextActionAuthorityState, NextActionCandidate, NextActionFreshness,
    NextActionProjection, NextActionSourceKind,
};
use arda_core::personal_ops::EvidenceClass;
use arda_core::run_graph::{NodeKind, NodeState, RunGraph};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::objectives::{
    agenda::{read_agenda, AgendaObjective},
    ObjectiveState,
};
use crate::personal_ops::{build_projection, PersonalOpsLogStore};

pub const NEXT_ACTION_PROJECTION_PATH: &str = "core/state/next_action.json";

pub fn publish_next_action_projection(
    root: &Path,
    operator_id: &str,
    generated_at: DateTime<Utc>,
) -> Result<NextActionProjection> {
    let agenda = read_agenda(root)?
        .unwrap_or_default()
        .into_iter()
        .filter(|objective| objective.operator_id == operator_id)
        .collect::<Vec<_>>();
    let mut candidates = objective_candidates(&agenda);
    candidates.extend(personal_operations_candidates(
        root,
        operator_id,
        generated_at,
    )?);
    candidates.extend(workbench_candidates(root, &agenda)?);
    candidates.extend(research_candidates(root, operator_id, generated_at)?);
    candidates.extend(hygiene::candidate(root, generated_at));
    let projection = select_next_action(candidates, generated_at);
    write_projection(root, &projection)?;
    Ok(projection)
}

fn objective_candidates(agenda: &[AgendaObjective]) -> Vec<NextActionCandidate> {
    agenda
        .iter()
        .map(|objective| NextActionCandidate {
            source_ref: format!("data/arda/objectives.sqlite3#{}", objective.id),
            id: objective.id.clone(),
            title: objective.text.clone(),
            source_kind: NextActionSourceKind::Objective,
            reason: "Current operator objective from resident ObjectiveStore.".to_owned(),
            freshness: NextActionFreshness::Fresh,
            authority_state: match objective.state {
                ObjectiveState::PendingApproval => NextActionAuthorityState::ReviewRequired,
                ObjectiveState::Paused => NextActionAuthorityState::Blocked,
                _ => NextActionAuthorityState::Ready,
            },
            next_operator_action: "Inspect or control this objective through Hermes.".to_owned(),
            priority: objective.priority.clamp(0, 100) as u8,
            operator_authored: true,
            terminal: false,
            future_gated: false,
            inferred_without_review: false,
        })
        .collect()
}

fn personal_operations_candidates(
    root: &Path,
    operator_id: &str,
    generated_at: DateTime<Utc>,
) -> Result<Vec<NextActionCandidate>> {
    let store = PersonalOpsLogStore::new(root);
    let events = store
        .load_all()
        .map_err(|error| anyhow::anyhow!("load personal operations: {error}"))?
        .into_iter()
        .filter(|event| event.record.operator_id() == operator_id)
        .collect::<Vec<_>>();
    let projection = build_projection(&events, generated_at, generated_at.date_naive());
    let mut seen = BTreeSet::new();
    let mut candidates = Vec::new();
    for (item, priority, reason) in projection
        .today
        .iter()
        .map(|item| {
            (
                item,
                80,
                "Operator-authored Personal Operations item for today.",
            )
        })
        .chain(projection.waiting.iter().map(|item| {
            (
                item,
                75,
                "Personal Operations reminder is waiting for operator attention.",
            )
        }))
    {
        if !seen.insert(item.item_id) {
            continue;
        }
        let inferred_without_review = matches!(
            item.evidence_class,
            EvidenceClass::Inferred | EvidenceClass::Unavailable
        );
        candidates.push(NextActionCandidate {
            id: item.item_id.to_string(),
            title: item.content.clone(),
            source_kind: NextActionSourceKind::PersonalOperations,
            source_ref: format!("data/personal/events.jsonl#{}", item.item_id),
            reason: reason.to_string(),
            freshness: NextActionFreshness::Fresh,
            authority_state: if inferred_without_review {
                NextActionAuthorityState::ReviewRequired
            } else {
                NextActionAuthorityState::Ready
            },
            next_operator_action: if inferred_without_review {
                "Review and correct this inferred classification before acting.".to_string()
            } else {
                "Open this item and complete or reschedule its next step.".to_string()
            },
            priority,
            operator_authored: item.evidence_class == EvidenceClass::OperatorAuthored,
            terminal: item.completed_at.is_some(),
            future_gated: false,
            inferred_without_review,
        });
    }
    Ok(candidates)
}

fn workbench_candidates(
    root: &Path,
    agenda: &[AgendaObjective],
) -> Result<Vec<NextActionCandidate>> {
    let mut candidates = Vec::new();
    for objective in agenda.iter().filter(|objective| {
        matches!(
            objective.state,
            ObjectiveState::Approved | ObjectiveState::Running
        )
    }) {
        for (_, run_id) in &objective.runs {
            let mut components = Path::new(run_id).components();
            anyhow::ensure!(
                matches!(components.next(), Some(std::path::Component::Normal(_)))
                    && components.next().is_none(),
                "invalid resident run identity"
            );
            let checkpoint_path = root.join("data/runs").join(run_id).join("checkpoint.json");
            let raw = match fs::read_to_string(&checkpoint_path) {
                Ok(raw) => raw,
                // Admission can persist a run identity before its first checkpoint.
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("read {}", checkpoint_path.display()))
                }
            };
            let graph = RunGraph::from_json_str(&raw)
                .map_err(|error| anyhow::anyhow!("parse {}: {error}", checkpoint_path.display()))?;
            anyhow::ensure!(
                graph.run_id.as_str() == run_id && graph.objective_id.as_str() == objective.id,
                "checkpoint disagrees with resident run/objective binding"
            );
            candidates.push(workbench_candidate(&graph, &checkpoint_path));
        }
    }
    Ok(candidates)
}

fn workbench_candidate(graph: &RunGraph, checkpoint_path: &Path) -> NextActionCandidate {
    let awaiting_approval = graph.nodes.iter().any(|node| {
        node.kind == NodeKind::Approval
            && matches!(
                node.state,
                NodeState::Pending | NodeState::Ready | NodeState::Blocked
            )
    });
    let running = graph
        .nodes
        .iter()
        .any(|node| node.state == NodeState::Running);
    let blocked = graph
        .nodes
        .iter()
        .any(|node| matches!(node.state, NodeState::Blocked | NodeState::Failed));
    let terminal = !graph.nodes.is_empty()
        && graph
            .nodes
            .iter()
            .all(|node| matches!(node.state, NodeState::Succeeded | NodeState::Cancelled));
    let (priority, authority_state, reason, next_operator_action) = if awaiting_approval {
        (
            100,
            NextActionAuthorityState::ReviewRequired,
            "Current Workbench run is awaiting an exact operator decision.",
            "Review the pending authority and approve, reject, or revise the run.",
        )
    } else if blocked {
        (
            95,
            NextActionAuthorityState::Blocked,
            "Current Workbench run is blocked or failed.",
            "Inspect the recorded failure and choose recovery, revision, or cancellation.",
        )
    } else if running {
        (
            85,
            NextActionAuthorityState::Ready,
            "Current Workbench run has active execution.",
            "Inspect current progress and receipts before intervening.",
        )
    } else {
        (
            70,
            NextActionAuthorityState::Ready,
            "Current Workbench run has an unfinished step.",
            "Open the run and continue its next governed node.",
        )
    };
    NextActionCandidate {
        id: graph.run_id.as_str().to_string(),
        title: graph.objective_id.as_str().to_string(),
        source_kind: NextActionSourceKind::Workbench,
        source_ref: relative_path(checkpoint_path),
        reason: reason.to_string(),
        freshness: NextActionFreshness::Fresh,
        authority_state,
        next_operator_action: next_operator_action.to_string(),
        priority,
        operator_authored: true,
        terminal,
        future_gated: false,
        inferred_without_review: false,
    }
}

fn research_candidates(
    root: &Path,
    operator_id: &str,
    generated_at: DateTime<Utc>,
) -> Result<Vec<NextActionCandidate>> {
    let path = root.join("data/workbench/research/questions.json");
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error).with_context(|| format!("read {}", path.display())),
    };
    let registry: Value =
        serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
    let mut candidates = Vec::new();
    for question in registry["records"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|question| question["owner"].as_str() == Some(operator_id))
    {
        let Some(id) = question["question_id"].as_str() else {
            continue;
        };
        let state = question["state"].as_str().unwrap_or("unknown");
        let expires_at = question["expires_at_utc"]
            .as_str()
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc));
        candidates.push(NextActionCandidate {
            id: id.to_string(),
            title: question["question"].as_str().unwrap_or(id).to_string(),
            source_kind: NextActionSourceKind::Research,
            source_ref: format!("{}#{id}", relative_path(&path)),
            reason: if state == "paused" {
                "Operator research question is paused and needs a lifecycle decision."
            } else {
                "Current operator research question has no reviewed answer yet."
            }
            .to_string(),
            freshness: if expires_at.is_some_and(|expiry| expiry <= generated_at) {
                NextActionFreshness::Stale
            } else {
                NextActionFreshness::Fresh
            },
            authority_state: if state == "paused" {
                NextActionAuthorityState::Blocked
            } else {
                NextActionAuthorityState::Advisory
            },
            next_operator_action: if state == "paused" {
                "Review whether to resume or retire this research question."
            } else {
                "Run or review bounded research without creating a commitment automatically."
            }
            .to_string(),
            priority: if state == "paused" { 60 } else { 40 },
            operator_authored: true,
            terminal: state == "retired",
            future_gated: false,
            inferred_without_review: false,
        });
    }
    Ok(candidates)
}

fn relative_path(path: &Path) -> String {
    path.to_string_lossy()
        .split_once("/core/")
        .map(|(_, suffix)| format!("core/{suffix}"))
        .or_else(|| {
            path.to_string_lossy()
                .split_once("/data/")
                .map(|(_, suffix)| format!("data/{suffix}"))
        })
        .unwrap_or_else(|| path.display().to_string())
}

fn write_projection(root: &Path, projection: &NextActionProjection) -> Result<()> {
    let path = root.join(NEXT_ACTION_PROJECTION_PATH);
    let parent = path.parent().context("next-action path has no parent")?;
    fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    let temporary = path.with_extension(format!("tmp.{}", std::process::id()));
    let mut bytes = serde_json::to_vec_pretty(projection)?;
    bytes.push(b'\n');
    let mut file =
        fs::File::create(&temporary).with_context(|| format!("create {}", temporary.display()))?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .with_context(|| format!("write {}", temporary.display()))?;
    fs::rename(&temporary, &path).with_context(|| format!("publish {}", path.display()))
}
