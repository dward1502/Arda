//! Authenticated Hermes Gateway operator-message ingress.
//!
//! Hermes owns transport credentials and allowlist authentication. This
//! loopback-only endpoint accepts the normalized event plus that authentication
//! assertion, records the canonical operator session through Oromë, and then
//! invokes the existing Workbench mutation surfaces with event-derived
//! idempotency keys.

use crate::objectives::{
    ControlAction, LeafExecutionSpec, NewLeaf, NewObjective, ObjectiveState, ObjectiveStore,
    ProjectAuthority,
};
use arda_orome::operator_bridge::{
    ApprovalBinding, ApprovalSingleUseState, Audience, BridgeApproval, BridgeLineage,
    BridgeOperation, BridgeRequest, ContentSensitivity, HermesMessageEvent, HermesPromptResponse,
    OperatorIdentity,
};
use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::net::SocketAddr;

use crate::orome::OromeOperatorRuntime;
use crate::{council::CouncilOperatorProjection, runs::RunStore};
use arda_core::run_graph::RunId;

use super::{
    projects::{contract_digest, find_attached_project, require_loopback, ApiError},
    HarnessState,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayOperatorMessage {
    pub operator: OperatorIdentity,
    pub adapter_id: String,
    pub event: HermesMessageEvent,
}

#[derive(Debug, Serialize)]
pub struct GatewayOperatorResponse {
    schema_version: &'static str,
    summary: String,
    evidence_refs: Vec<String>,
    session_id: String,
    run_id: Option<String>,
}

#[derive(Debug)]
enum Command {
    Capture(String),
    Research(String),
    Objective {
        project_ids: Vec<String>,
        text: String,
    },
    Context,
    Objectives,
    PauseTask {
        task_id: String,
        objective_id: String,
        reason: String,
    },
    ResumeTask {
        task_id: String,
        objective_id: String,
        reason: String,
    },
    ReprioritizeTask {
        task_id: String,
        objective_id: String,
        priority: String,
        reason: String,
    },
    ReviseObjective {
        task_id: String,
        objective_id: String,
        revised_objective: String,
        reason: String,
    },
    ApproveObjective {
        task_id: String,
        objective_id: String,
        reason: String,
    },
    CancelTask {
        task_id: String,
        objective_id: String,
        reason: String,
    },
    Status {
        run_id: Option<String>,
    },
    Approve {
        run_id: String,
        node_id: String,
    },
    Reject {
        run_id: String,
        node_id: String,
        reason: String,
    },
    Revise {
        run_id: String,
        node_id: String,
        instruction: String,
    },
    Cancel {
        run_id: String,
        reason: String,
    },
    Acknowledge {
        reminder_id: String,
    },
    Defer {
        reminder_id: String,
    },
    Result {
        run_id: String,
    },
    Council {
        run_id: String,
    },
}

pub(super) async fn ingest_operator_message(
    State(state): State<HarnessState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(mut incoming): Json<GatewayOperatorMessage>,
) -> Result<Json<GatewayOperatorResponse>, ApiError> {
    require_loopback(peer)?;
    require_gateway_capability(&headers)?;
    if !incoming.operator.authenticated
        || incoming.operator.authentication_method != "gateway_identity"
    {
        return Err(ApiError::forbidden(
            "operator message requires Hermes Gateway identity authentication",
        ));
    }
    if incoming.operator.operator_id != state.operator_id {
        return Err(ApiError::forbidden(
            "gateway operator identity does not match configured Arda operator",
        ));
    }
    let authenticated_at =
        chrono::DateTime::parse_from_rfc3339(&incoming.operator.authenticated_at)
            .map(|value| value.with_timezone(&Utc))
            .map_err(|_| ApiError::forbidden("gateway authentication timestamp is invalid"))?;
    let authentication_age = Utc::now().signed_duration_since(authenticated_at);
    if authentication_age > Duration::minutes(5) || authentication_age < Duration::minutes(-1) {
        return Err(ApiError::forbidden(
            "gateway authentication assertion is stale or from the future",
        ));
    }
    if incoming.event.user_id.as_deref() != Some(incoming.operator.operator_id.as_str()) {
        return Err(ApiError::forbidden(
            "gateway operator identity does not match MessageEvent user_id",
        ));
    }

    let raw_message_id = incoming
        .event
        .message_id
        .as_deref()
        .or(incoming.event.source.message_id.as_deref())
        .ok_or_else(|| ApiError::bad_request("MessageEvent message_id is required"))?;
    let message_id = gateway_event_id(&incoming, raw_message_id);
    incoming.event.message_id = Some(message_id.clone());
    incoming.event.source.message_id = Some(message_id);

    let command = parse_command(&incoming.event.text)?;
    let audience = audience(&incoming.event);
    if matches!(
        command,
        Command::Capture(_)
            | Command::Research(_)
            | Command::Objective { .. }
            | Command::Context
            | Command::Objectives
            | Command::PauseTask { .. }
            | Command::ResumeTask { .. }
            | Command::ReprioritizeTask { .. }
            | Command::ReviseObjective { .. }
            | Command::ApproveObjective { .. }
            | Command::CancelTask { .. }
            | Command::Acknowledge { .. }
            | Command::Defer { .. }
    ) && !matches!(audience, Audience::Direct | Audience::OperatorPrivate)
    {
        return Err(ApiError::forbidden(
            "personal operator commands require a private conversation",
        ));
    }
    preflight_canonical_control(&state, &command)?;
    let session_id = session_id(&incoming.event);
    let run_id = command_run_id(&command).map(str::to_owned);
    let operation = command_operation(&command);
    let now = Utc::now();

    let approval_parts = match &command {
        Command::Approve { run_id, node_id } => Some((run_id, node_id, "approve", "Approve")),
        Command::Reject {
            run_id, node_id, ..
        } => Some((run_id, node_id, "reject", "Reject")),
        Command::Revise {
            run_id, node_id, ..
        } => Some((run_id, node_id, "revise", "Revise")),
        _ => None,
    };
    let (approval, pending) = if let Some((run_id, node_id, option_id, label)) = approval_parts {
        let run = get_json(&state, &format!("/v1/runs/{run_id}")).await?;
        require_pending_approval_node(&run, node_id)?;
        let prompt_id = format!("arda:{run_id}:{node_id}");
        let action_digest = approval_digest(
            run_id,
            node_id,
            &incoming.operator.operator_id,
            &session_id,
            &incoming.event.source.chat_id,
        );
        incoming.event.prompt_response = Some(HermesPromptResponse {
            prompt_id: prompt_id.clone(),
            option_id: option_id.into(),
            label: Some(label.into()),
            prompt_message_id: None,
        });
        let scope = vec![format!("run:{run_id}:node:{node_id}:{option_id}")];
        (
            Some(BridgeApproval {
                scope: scope.clone(),
                action_digest: action_digest.clone(),
                expires_at: (now + Duration::minutes(15)).to_rfc3339(),
                single_use_state: ApprovalSingleUseState::Available,
                consumed_by_event_id: None,
            }),
            Some(ApprovalBinding {
                prompt_id,
                operator_id: incoming.operator.operator_id.clone(),
                action_digest,
                scope,
                session_id: session_id.clone(),
                task_id: None,
                run_id: Some(run_id.clone()),
                conversation_id: incoming.event.source.chat_id.clone(),
                thread_id: incoming.event.source.thread_id.clone(),
            }),
        )
    } else {
        (None, None)
    };

    let bridge_request = BridgeRequest {
        operator: incoming.operator.clone(),
        lineage: BridgeLineage {
            session_id: session_id.clone(),
            objective_id: command_objective_id(&command).map(str::to_owned),
            project_id: command_project_id(&command).map(str::to_owned),
            task_id: command_task_id(&command).map(str::to_owned),
            run_id: run_id.clone(),
        },
        adapter_id: incoming.adapter_id.clone(),
        audience,
        sensitivity: ContentSensitivity::Private,
        operation,
        event: incoming.event.clone(),
        attachments: Vec::new(),
        approval,
    };
    let runtime = OromeOperatorRuntime::new(
        state
            .workbench_root
            .join("core/state/orome/operator-session"),
    )
    .map_err(bridge_error)?;
    // Resident objective mutations are durable and idempotent by gateway event ID.
    // Apply them before appending the operator-session event so a rejected control
    // cannot become an unretryable duplicate without changing objective state.
    let applied = if is_resident_objective_mutation(&command) {
        Some(apply_command(&state, &incoming, &command).await?)
    } else {
        None
    };
    let session = match pending.as_ref() {
        Some(binding) => runtime
            .ingest_approval(bridge_request, binding, now)
            .map_err(bridge_error)?,
        None => runtime.ingest(bridge_request, now).map_err(bridge_error)?,
    };

    let (summary, mut evidence_refs) = match applied {
        Some(result) => result,
        None => apply_command(&state, &incoming, &command).await?,
    };
    evidence_refs.insert(
        0,
        format!("arda://operator-events/{}", session.incoming.event_id),
    );
    Ok(Json(GatewayOperatorResponse {
        schema_version: "arda.gateway-operator-response.v1",
        summary,
        evidence_refs,
        session_id,
        run_id,
    }))
}

fn require_gateway_capability(headers: &HeaderMap) -> Result<(), ApiError> {
    let expected = gateway_capability()
        .ok_or_else(|| ApiError::internal("Hermes Gateway capability is not configured"))?;
    let presented = headers
        .get("x-arda-gateway-capability")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if !constant_time_eq(expected.as_bytes(), presented.as_bytes()) {
        return Err(ApiError::forbidden(
            "Hermes Gateway capability is missing or invalid",
        ));
    }
    Ok(())
}

fn gateway_capability() -> Option<String> {
    if let Ok(value) = std::env::var("ARDA_HERMES_GATEWAY_CAPABILITY") {
        let value = value.trim();
        if !value.is_empty() {
            return Some(value.to_owned());
        }
    }
    let directory = std::env::var_os("CREDENTIALS_DIRECTORY")?;
    let value = std::fs::read_to_string(
        std::path::Path::new(&directory).join("arda-hermes-gateway-capability"),
    )
    .ok()?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn constant_time_eq(expected: &[u8], presented: &[u8]) -> bool {
    let mut difference = expected.len() ^ presented.len();
    let width = expected.len().max(presented.len());
    for index in 0..width {
        let left = expected.get(index).copied().unwrap_or_default();
        let right = presented.get(index).copied().unwrap_or_default();
        difference |= usize::from(left ^ right);
    }
    difference == 0
}

fn preflight_canonical_control(state: &HarnessState, command: &Command) -> Result<(), ApiError> {
    match command {
        Command::PauseTask {
            task_id,
            objective_id,
            ..
        }
        | Command::ResumeTask {
            task_id,
            objective_id,
            ..
        }
        | Command::ReprioritizeTask {
            task_id,
            objective_id,
            ..
        }
        | Command::ReviseObjective {
            task_id,
            objective_id,
            ..
        }
        | Command::ApproveObjective {
            task_id,
            objective_id,
            ..
        }
        | Command::CancelTask {
            task_id,
            objective_id,
            ..
        } => {
            let store = objective_store(state)?;
            let leaf = store
                .leaf(task_id)
                .map_err(objective_store_error)?
                .ok_or_else(|| {
                    ApiError::not_found(format!("objective leaf `{task_id}` was not found"))
                })?;
            if leaf.objective_id != *objective_id {
                return Err(ApiError::forbidden(
                    "operator control objective does not match canonical leaf lineage",
                ));
            }
        }
        _ => {}
    }
    Ok(())
}

async fn apply_command(
    state: &HarnessState,
    incoming: &GatewayOperatorMessage,
    command: &Command,
) -> Result<(String, Vec<String>), ApiError> {
    let message_id = incoming
        .event
        .message_id
        .as_deref()
        .or(incoming.event.source.message_id.as_deref())
        .ok_or_else(|| ApiError::bad_request("MessageEvent message_id is required"))?;
    let envelope = mutation_envelope(message_id, &incoming.event.timestamp);
    match command {
        Command::Capture(text) => {
            let capture = post_json(
                state,
                "/v1/personal/captures",
                json!({
                    "operator_id": incoming.operator.operator_id,
                    "text": text,
                    "audio_reference": null,
                    "project_id": null,
                    "priority": null,
                    "due_at": null
                }),
                Some(message_id),
                Some(&incoming.operator.operator_id),
            )
            .await?;
            let capture_id = required_string(&capture, "capture_id")?;
            Ok((
                format!("Captured inbox item {capture_id}."),
                vec![format!("arda://personal/captures/{capture_id}")],
            ))
        }
        Command::Research(question) => {
            let digest = format!("{:x}", Sha256::digest(message_id.as_bytes()));
            let question_id = format!("operator-question-{}", &digest[..16]);
            let response = post_json(
                state,
                "/v1/research/questions",
                json!({
                    "question": {
                        "schema_version": "arda.warden.watchlist.v1",
                        "question_id": question_id,
                        "owner": incoming.operator.operator_id,
                        "question": question,
                        "rationale": "Explicit operator research request from Hermes Gateway",
                        "tags": ["operator-authored"],
                        "cadence": {"kind": "manual"},
                        "expires_at_utc": (Utc::now() + Duration::days(7)).to_rfc3339(),
                        "source_policy": {
                            "policy_id": "public-web",
                            "allowed_sources": ["https://"],
                            "max_sources_per_run": 5,
                            "allow_private_targets": false
                        },
                        "evidence_requirements": {
                            "minimum_canonical_sources": 1,
                            "require_canonical_fetch": true,
                            "max_source_age_seconds": 604800
                        },
                        "contradiction_policy": "require_disclosure",
                        "budgets": {
                            "max_results": 10,
                            "max_fetch_bytes": 2000000,
                            "max_tokens": 4000,
                            "max_attempts": 2
                        },
                        "notification_policy": {"enabled": false, "destination": null},
                        "state": "enabled",
                        "backend_suggestion_ids": []
                    },
                    "read_only": false,
                    "envelope": envelope
                }),
                None,
                Some(&incoming.operator.operator_id),
            )
            .await?;
            let backend_status = response["backend_status"].as_str().unwrap_or("registered");
            Ok((
                format!(
                    "Research question {question_id} registered; backend status: {backend_status}. No commitment was created."
                ),
                vec![format!("arda://research/questions/{question_id}")],
            ))
        }
        Command::Objective { project_ids, text } => {
            let objective = create_operator_objective(
                state,
                project_ids,
                text,
                &incoming.operator.operator_id,
                message_id,
                &incoming.event.timestamp,
            )?;
            let primary_project_id = project_ids
                .first()
                .expect("objective parser requires at least one project");
            let capture = post_json(
                state,
                "/v1/personal/captures",
                json!({
                    "operator_id": incoming.operator.operator_id,
                    "text": text,
                    "audio_reference": null,
                    "project_id": primary_project_id,
                    "priority": null,
                    "due_at": null
                }),
                Some(message_id),
                Some(&incoming.operator.operator_id),
            )
            .await?;
            let capture_id = required_string(&capture, "capture_id")?;
            let leaf_id = objective
                .leaves
                .first()
                .map(|leaf| leaf.id.as_str())
                .ok_or_else(|| ApiError::internal("objective omitted execution leaves"))?;
            Ok((
                format!(
                    "Created objective capture {capture_id} and resident objective {} for attached project(s) {}. Execution still requires review.",
                    objective.id,
                    project_ids.join(", ")
                ),
                std::iter::once(format!("arda://personal/captures/{capture_id}"))
                    .chain(
                        project_ids
                            .iter()
                            .map(|project_id| format!("arda://projects/{project_id}")),
                    )
                    .chain(std::iter::once(format!(
                        "arda://objectives/{}",
                        objective.id
                    )))
                    .chain(std::iter::once(format!(
                        "arda://objectives/{}/leaves/{leaf_id}",
                        objective.id
                    )))
                    .collect(),
            ))
        }
        Command::Context => {
            let objectives = objective_store(state)?
                .list_objectives()
                .map_err(objective_store_error)?;
            let selected = objectives.iter().find(|objective| {
                !matches!(
                    objective.state,
                    ObjectiveState::Completed | ObjectiveState::Cancelled | ObjectiveState::Failed
                )
            });
            let Some(selected) = selected else {
                return Ok((
                    "No current resident objective is available.".to_owned(),
                    vec!["arda://objectives".into()],
                ));
            };
            Ok((
                format!(
                    "Next resident objective: {} [{}]. Operator step: {}",
                    selected.text,
                    selected.state.as_str(),
                    if selected.state == ObjectiveState::PendingApproval {
                        "review and approve the authenticated objective"
                    } else {
                        "monitor resident execution"
                    }
                ),
                vec![format!("arda://objectives/{}", selected.id)],
            ))
        }
        Command::Objectives => {
            let objectives = objective_store(state)?
                .list_objectives()
                .map_err(objective_store_error)?;
            let mut lines = vec![format!(
                "Objectives: {} (authority=resident_objective_store, freshness=live).",
                objectives.len()
            )];
            let mut evidence_refs = vec!["arda://objectives".into()];
            for objective in objectives {
                lines.push(format!(
                    "{} [{}] priority={} revision={} projects={} text={}",
                    objective.id,
                    objective.state.as_str(),
                    objective.priority,
                    objective.revision,
                    objective.project_ids.join(","),
                    objective.text,
                ));
                evidence_refs.push(format!("arda://objectives/{}", objective.id));
            }
            Ok((lines.join("\n"), evidence_refs))
        }
        Command::PauseTask {
            task_id,
            objective_id,
            reason,
        } => {
            apply_objective_control(
                state,
                objective_id,
                ControlAction::Pause,
                message_id,
                &incoming.operator.operator_id,
            )?;
            Ok((
                format!("Paused resident objective {objective_id}: {reason}"),
                vec![format!("arda://objectives/{objective_id}/leaves/{task_id}")],
            ))
        }
        Command::ResumeTask {
            task_id,
            objective_id,
            reason,
        } => {
            apply_objective_control(
                state,
                objective_id,
                ControlAction::Resume,
                message_id,
                &incoming.operator.operator_id,
            )?;
            Ok((
                format!("Resumed resident objective {objective_id}: {reason}"),
                vec![format!("arda://objectives/{objective_id}/leaves/{task_id}")],
            ))
        }
        Command::ReprioritizeTask {
            task_id,
            objective_id,
            priority,
            reason,
        } => {
            let priority = objective_priority(priority)?;
            apply_objective_control(
                state,
                objective_id,
                ControlAction::Reprioritize { priority },
                message_id,
                &incoming.operator.operator_id,
            )?;
            Ok((
                format!("Reprioritized {task_id} to {priority}: {reason}"),
                vec![format!("arda://objectives/{objective_id}/leaves/{task_id}")],
            ))
        }
        Command::ReviseObjective {
            task_id,
            objective_id,
            revised_objective,
            reason,
        } => {
            apply_objective_control(
                state,
                objective_id,
                ControlAction::Revise {
                    text: revised_objective.clone(),
                },
                message_id,
                &incoming.operator.operator_id,
            )?;
            Ok((
                format!(
                    "Revised resident objective {objective_id}; fresh approval is required: {reason}"
                ),
                vec![format!("arda://objectives/{objective_id}/leaves/{task_id}")],
            ))
        }
        Command::ApproveObjective {
            task_id,
            objective_id,
            reason,
        } => {
            let store = objective_store(state)?;
            let revision = store
                .objective(objective_id)
                .map_err(objective_store_error)?
                .ok_or_else(|| {
                    ApiError::not_found(format!("objective `{objective_id}` was not found"))
                })?
                .revision;
            apply_objective_control(
                state,
                objective_id,
                ControlAction::Approve { revision },
                message_id,
                &incoming.operator.operator_id,
            )?;
            Ok((
                format!("Approved resident objective {objective_id}: {reason}"),
                vec![format!("arda://objectives/{objective_id}/leaves/{task_id}")],
            ))
        }
        Command::CancelTask {
            task_id,
            objective_id,
            reason,
        } => {
            apply_objective_control(
                state,
                objective_id,
                ControlAction::Cancel,
                message_id,
                &incoming.operator.operator_id,
            )?;
            Ok((
                format!("Cancelled resident objective {objective_id}: {reason}"),
                vec![format!("arda://objectives/{objective_id}/leaves/{task_id}")],
            ))
        }
        Command::Status { run_id: None } => {
            let response = get_json(state, "/v1/runs").await?;
            let mut active = 0_usize;
            let mut blocked = 0_usize;
            let mut awaiting_approval = 0_usize;
            let mut run_count = 0_usize;
            if let Some(runs) = response["runs"].as_array() {
                run_count = runs.len();
                for run in runs {
                    if let Some(nodes) = run["graph"]["nodes"].as_array() {
                        for node in nodes {
                            match node["state"].as_str() {
                                Some("ready" | "running") => active += 1,
                                Some("blocked") => blocked += 1,
                                _ => {}
                            }
                            if node["kind"] == "approval"
                                && matches!(
                                    node["state"].as_str(),
                                    Some("pending" | "ready" | "blocked" | "failed")
                                )
                            {
                                awaiting_approval += 1;
                            }
                        }
                    }
                }
            }
            Ok((
                format!(
                    "Runs: {run_count}; active nodes: {active}; blocked nodes: {blocked}; awaiting approval: {awaiting_approval}."
                ),
                vec!["arda://runs".into()],
            ))
        }
        Command::Status {
            run_id: Some(run_id),
        } => run_status(state, run_id).await,
        Command::Approve { run_id, node_id } => {
            post_json(
                state,
                &format!("/v1/runs/{run_id}/approve"),
                json!({"node_id": node_id, "envelope": envelope}),
                None,
                None,
            )
            .await?;
            Ok((
                format!("Approved {run_id}/{node_id}."),
                vec![format!("arda://runs/{run_id}/nodes/{node_id}")],
            ))
        }
        Command::Reject {
            run_id,
            node_id,
            reason,
        } => {
            post_json(
                state,
                &format!("/v1/runs/{run_id}/cancel"),
                json!({
                    "reason": format!("approval {node_id} rejected: {reason}"),
                    "envelope": envelope
                }),
                None,
                None,
            )
            .await?;
            Ok((
                format!("Rejected {run_id}/{node_id}."),
                vec![format!("arda://runs/{run_id}/nodes/{node_id}")],
            ))
        }
        Command::Revise {
            run_id,
            node_id,
            instruction,
        } => {
            post_json(
                state,
                &format!("/v1/runs/{run_id}/cancel"),
                json!({
                    "reason": format!("approval {node_id} requires revision: {instruction}"),
                    "envelope": envelope
                }),
                None,
                None,
            )
            .await?;
            Ok((
                format!("Requested revision for {run_id}/{node_id}."),
                vec![format!("arda://runs/{run_id}/nodes/{node_id}")],
            ))
        }
        Command::Cancel { run_id, reason } => {
            post_json(
                state,
                &format!("/v1/runs/{run_id}/cancel"),
                json!({"reason": reason, "envelope": envelope}),
                None,
                None,
            )
            .await?;
            Ok((
                format!("Cancelled run {run_id}."),
                vec![format!("arda://runs/{run_id}")],
            ))
        }
        Command::Acknowledge { reminder_id } | Command::Defer { reminder_id } => {
            let state_name = if matches!(command, Command::Acknowledge { .. }) {
                "acknowledged"
            } else {
                "deferred"
            };
            post_json(
                state,
                &format!("/v1/personal/reminders/{reminder_id}/acknowledge"),
                json!({
                    "operator_id": incoming.operator.operator_id,
                    "state": state_name,
                    "receipt_reference": format!("gateway:{message_id}")
                }),
                Some(message_id),
                Some(&incoming.operator.operator_id),
            )
            .await?;
            Ok((
                format!("Reminder {reminder_id} {state_name}."),
                vec![format!("arda://personal/reminders/{reminder_id}")],
            ))
        }
        Command::Result { run_id } => {
            let run = get_json(state, &format!("/v1/runs/{run_id}")).await?;
            let review = &run["review"];
            let provider_summary = review["provider_receipt"]["summary"]
                .as_str()
                .unwrap_or("No verified provider result is recorded.");
            let tests = review["tests"].as_array().map(Vec::as_slice).unwrap_or(&[]);
            let passed = tests
                .iter()
                .filter(|test| test["status"] == "passed")
                .count();
            let mut refs = vec![format!("arda://runs/{run_id}/result")];
            if let Some(changes) = review["changes"].as_array() {
                refs.extend(changes.iter().filter_map(|change| {
                    change["path"]
                        .as_str()
                        .map(|path| format!("arda://runs/{run_id}/files/{path}"))
                }));
            }
            Ok((
                format!(
                    "Run {run_id} result: {provider_summary} Verified tests: {passed}/{}.",
                    tests.len()
                ),
                refs,
            ))
        }
        Command::Council { run_id } => {
            let run_id_value = RunId::new(run_id).map_err(|error| {
                ApiError::bad_request(format!("invalid council run id: {error}"))
            })?;
            let store = RunStore::open(&state.workbench_root, run_id_value)
                .map_err(|error| ApiError::internal(format!("open council run: {error}")))?;
            let council = store
                .read_council_run()
                .map_err(|error| ApiError::internal(format!("read council run: {error}")))?
                .ok_or_else(|| ApiError::not_found(format!("council run `{run_id}` not found")))?;
            let projection = CouncilOperatorProjection::from_run(&council);
            Ok((
                projection.concise_message(),
                vec![format!("arda://runs/{run_id}/council")],
            ))
        }
    }
}

async fn run_status(state: &HarnessState, run_id: &str) -> Result<(String, Vec<String>), ApiError> {
    let run = get_json(state, &format!("/v1/runs/{run_id}")).await?;
    let states = run["graph"]["nodes"]
        .as_array()
        .map(|nodes| {
            nodes
                .iter()
                .map(|node| {
                    format!(
                        "{}={}",
                        node["id"].as_str().unwrap_or("unknown"),
                        node["state"].as_str().unwrap_or("unknown")
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|| "no nodes".into());
    Ok((
        format!("Run {run_id}: {states}."),
        vec![format!("arda://runs/{run_id}")],
    ))
}

fn create_operator_objective(
    state: &HarnessState,
    project_ids: &[String],
    text: &str,
    operator_id: &str,
    message_id: &str,
    timestamp: &str,
) -> Result<NewObjective, ApiError> {
    let mut hasher = Sha256::new();
    for project_id in project_ids {
        hasher.update(project_id.as_bytes());
        hasher.update([0]);
    }
    hasher.update(message_id.as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    let objective_id = format!("operator-objective-{}", &digest[..16]);
    let approval_envelope = json!({
        "approval": {
            "schema_version": "arda.orome.task_approval.v1",
            "proposal_id": format!("gateway-proposal:{message_id}"),
            "approval_id": format!("gateway-approval:{message_id}"),
            "ledger_writes": ["data/arda/objectives.sqlite3", "data/runs"],
            "decision": "policy_safe",
            "created_at_utc": timestamp
        },
        "idempotency_key": message_id
    });
    let objective_plan_receipt = format!(
        "sha256:{:x}",
        Sha256::digest(
            serde_json::to_vec(&json!({
                "objective_id": objective_id,
                "project_ids": project_ids,
                "text": text,
                "source_message_id": message_id,
            }))
            .map_err(|error| ApiError::internal(format!("serialize objective plan: {error}")))?
        )
    );

    let mut projects = Vec::with_capacity(project_ids.len());
    let mut leaves = Vec::with_capacity(project_ids.len().saturating_add(1));
    for (index, project_id) in project_ids.iter().enumerate() {
        let attached = find_attached_project(&state.workbench_root, project_id)?;
        let project_digest = contract_digest(&attached.contract)?;
        let workspace_root = state
            .workbench_root
            .join(attached.contract.workspace.root.as_str())
            .to_string_lossy()
            .into_owned();
        projects.push(ProjectAuthority {
            project_id: project_id.clone(),
            contract_digest: project_digest,
        });
        leaves.push(NewLeaf {
            id: format!("{objective_id}-project-{}", index + 1),
            project_id: Some(project_id.clone()),
            workspace_root,
            authority: "operator_approved_workbench".into(),
            dependencies: Vec::new(),
            execution: Some(LeafExecutionSpec {
                objective: text.to_owned(),
                execution_prompt: format!(
                    "Execute the approved objective for exact project {project_id}: {text}"
                ),
                verification_prompt: format!(
                    "Verify the project-local result for exact project {project_id}."
                ),
                review_prompt: "Review correctness, scope, and receipt evidence.".into(),
                approval_envelope: approval_envelope.clone(),
                objective_plan_receipt: objective_plan_receipt.clone(),
            }),
        });
    }
    if leaves.len() > 1 {
        let primary = leaves[0].clone();
        leaves.push(NewLeaf {
            id: format!("{objective_id}-join"),
            project_id: primary.project_id.clone(),
            workspace_root: primary.workspace_root,
            authority: "operator_approved_workbench".into(),
            dependencies: leaves.iter().map(|leaf| leaf.id.clone()).collect(),
            execution: Some(LeafExecutionSpec {
                objective: text.to_owned(),
                execution_prompt:
                    "Synthesize the completed project leaves into one objective result.".into(),
                verification_prompt:
                    "Verify every project leaf has canonical close-receipt lineage.".into(),
                review_prompt: "Review the joined result against the full approved objective."
                    .into(),
                approval_envelope,
                objective_plan_receipt,
            }),
        });
    }
    let objective = NewObjective {
        id: objective_id,
        source_id: format!("gateway:{message_id}"),
        idempotency_key: message_id.to_owned(),
        operator_id: operator_id.to_owned(),
        text: text.to_owned(),
        priority: 50,
        projects,
        leaves,
    };
    objective_store(state)?
        .create_authenticated_objective(objective.clone(), Utc::now().timestamp_millis())
        .map_err(objective_store_error)?;
    Ok(objective)
}

fn objective_store(state: &HarnessState) -> Result<ObjectiveStore, ApiError> {
    ObjectiveStore::open(state.workbench_root.join("data/arda/objectives.sqlite3"))
        .map_err(objective_store_error)
}

fn objective_store_error(error: anyhow::Error) -> ApiError {
    ApiError::conflict(format!(
        "resident objective store rejected mutation: {error}"
    ))
}

fn apply_objective_control(
    state: &HarnessState,
    objective_id: &str,
    action: ControlAction,
    message_id: &str,
    operator_id: &str,
) -> Result<(), ApiError> {
    objective_store(state)?
        .apply_control(
            objective_id,
            action,
            message_id,
            operator_id,
            Utc::now().timestamp_millis(),
        )
        .map_err(objective_store_error)?;
    Ok(())
}

fn objective_priority(priority: &str) -> Result<i64, ApiError> {
    match priority.to_ascii_lowercase().as_str() {
        "critical" => Ok(100),
        "high" => Ok(75),
        "normal" | "medium" => Ok(50),
        "low" => Ok(25),
        _ => priority.parse::<i64>().map_err(|_| {
            ApiError::bad_request("priority must be critical, high, normal, low, or an integer")
        }),
    }
}

fn parse_command(text: &str) -> Result<Command, ApiError> {
    let mut parts = text.trim().splitn(3, char::is_whitespace);
    if parts
        .next()
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
        != Some("arda")
    {
        return Err(ApiError::bad_request(
            "operator command must start with `arda`",
        ));
    }
    let verb = parts
        .next()
        .map(|value| value.to_ascii_lowercase())
        .ok_or_else(|| ApiError::bad_request("missing operator command"))?;
    let args = parts.next().unwrap_or("").trim();
    match verb.as_str() {
        "capture" => {
            if args.is_empty() {
                Err(ApiError::bad_request("capture text cannot be empty"))
            } else {
                Ok(Command::Capture(args.to_owned()))
            }
        }
        "research" => {
            if args.is_empty() {
                Err(ApiError::bad_request("research question cannot be empty"))
            } else {
                Ok(Command::Research(args.to_owned()))
            }
        }
        "objective" => {
            let (project_ids, text) = take_arg(args, "objective project_ids")?;
            if text.is_empty() {
                return Err(ApiError::bad_request("objective text cannot be empty"));
            }
            let project_ids = project_ids
                .split(',')
                .map(str::trim)
                .map(str::to_owned)
                .collect::<Vec<_>>();
            if project_ids.iter().any(String::is_empty) {
                return Err(ApiError::bad_request(
                    "objective project_ids must be a comma-separated list without empty entries",
                ));
            }
            let mut unique = std::collections::HashSet::new();
            if !project_ids
                .iter()
                .all(|project_id| unique.insert(project_id))
            {
                return Err(ApiError::bad_request(
                    "objective project_ids must not contain duplicates",
                ));
            }
            Ok(Command::Objective {
                project_ids,
                text: text.to_owned(),
            })
        }
        "context" => require_no_args(args).map(|()| Command::Context),
        "objectives" => require_no_args(args).map(|()| Command::Objectives),
        "pause-task" | "resume-task" => {
            let (task_id, rest) = take_arg(args, "task_id")?;
            let (objective_id, reason) = take_arg(rest, "objective_id")?;
            if reason.is_empty() {
                return Err(ApiError::bad_request("missing operator reason"));
            }
            if verb == "pause-task" {
                Ok(Command::PauseTask {
                    task_id,
                    objective_id,
                    reason: reason.to_owned(),
                })
            } else {
                Ok(Command::ResumeTask {
                    task_id,
                    objective_id,
                    reason: reason.to_owned(),
                })
            }
        }
        "reprioritize" => {
            let (task_id, rest) = take_arg(args, "task_id")?;
            let (objective_id, rest) = take_arg(rest, "objective_id")?;
            let (priority, reason) = take_arg(rest, "priority")?;
            if reason.is_empty() {
                return Err(ApiError::bad_request("missing reprioritization reason"));
            }
            Ok(Command::ReprioritizeTask {
                task_id,
                objective_id,
                priority,
                reason: reason.to_owned(),
            })
        }
        "revise-objective" => {
            let (task_id, rest) = take_arg(args, "task_id")?;
            let (objective_id, revision) = take_arg(rest, "objective_id")?;
            let (revised_objective, reason) = split_reason(revision)?;
            Ok(Command::ReviseObjective {
                task_id,
                objective_id,
                revised_objective,
                reason,
            })
        }
        "approve-objective" => {
            let (task_id, rest) = take_arg(args, "task_id")?;
            let (objective_id, reason) = take_arg(rest, "objective_id")?;
            if reason.is_empty() {
                return Err(ApiError::bad_request("missing approval reason"));
            }
            Ok(Command::ApproveObjective {
                task_id,
                objective_id,
                reason: reason.to_owned(),
            })
        }
        "cancel-task" => {
            let (task_id, rest) = take_arg(args, "task_id")?;
            let (objective_id, reason) = take_arg(rest, "objective_id")?;
            if reason.is_empty() {
                return Err(ApiError::bad_request("missing cancellation reason"));
            }
            Ok(Command::CancelTask {
                task_id,
                objective_id,
                reason: reason.to_owned(),
            })
        }
        "approve" => {
            let (run_id, rest) = take_arg(args, "approve run_id")?;
            let node_id = only_arg(rest, "approve node_id")?;
            Ok(Command::Approve { run_id, node_id })
        }
        "reject" => {
            let (run_id, rest) = take_arg(args, "reject run_id")?;
            let (node_id, reason) = take_arg(rest, "reject node_id")?;
            if reason.is_empty() {
                return Err(ApiError::bad_request("missing reject reason"));
            }
            Ok(Command::Reject {
                run_id,
                node_id,
                reason: reason.to_owned(),
            })
        }
        "revise" => {
            let (run_id, rest) = take_arg(args, "revise run_id")?;
            let (node_id, instruction) = take_arg(rest, "revise node_id")?;
            if instruction.is_empty() {
                return Err(ApiError::bad_request("missing revise instruction"));
            }
            Ok(Command::Revise {
                run_id,
                node_id,
                instruction: instruction.to_owned(),
            })
        }
        "cancel" => {
            let (run_id, reason) = take_arg(args, "cancel run_id")?;
            if reason.is_empty() {
                return Err(ApiError::bad_request("missing cancel reason"));
            }
            Ok(Command::Cancel {
                run_id,
                reason: reason.to_owned(),
            })
        }
        "resume" if args.is_empty() => Ok(Command::Context),
        "resume" | "status" => Ok(Command::Status {
            run_id: if args.is_empty() {
                None
            } else {
                Some(only_arg(args, "status run_id")?)
            },
        }),
        "ack" | "acknowledge" => Ok(Command::Acknowledge {
            reminder_id: only_arg(args, "acknowledge reminder_id")?,
        }),
        "defer" => Ok(Command::Defer {
            reminder_id: only_arg(args, "defer reminder_id")?,
        }),
        "result" => Ok(Command::Result {
            run_id: only_arg(args, "result run_id")?,
        }),
        "council" => Ok(Command::Council {
            run_id: only_arg(args, "council run_id")?,
        }),
        _ => Err(ApiError::bad_request(
            "unsupported operator command; use capture, research, objective, objectives, context, status, pause-task, resume-task, reprioritize, revise-objective, approve-objective, cancel-task, approve, reject, revise, cancel, acknowledge, defer, result, or council",
        )),
    }
}

fn take_arg<'a>(input: &'a str, name: &str) -> Result<(String, &'a str), ApiError> {
    let mut parts = input.trim().splitn(2, char::is_whitespace);
    let value = parts
        .next()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::bad_request(format!("missing {name}")))?;
    Ok((value.to_owned(), parts.next().unwrap_or("").trim()))
}

fn only_arg(input: &str, name: &str) -> Result<String, ApiError> {
    let (value, rest) = take_arg(input, name)?;
    if !rest.is_empty() {
        return Err(ApiError::bad_request(format!(
            "unexpected trailing arguments after {name}"
        )));
    }
    Ok(value)
}

fn require_no_args(input: &str) -> Result<(), ApiError> {
    if input.is_empty() {
        Ok(())
    } else {
        Err(ApiError::bad_request("command does not accept arguments"))
    }
}

fn split_reason(input: &str) -> Result<(String, String), ApiError> {
    let (value, reason) = input
        .split_once(" --reason ")
        .ok_or_else(|| ApiError::bad_request("objective revision requires `--reason`"))?;
    let value = value.trim();
    let reason = reason.trim();
    if value.is_empty() || reason.is_empty() {
        return Err(ApiError::bad_request(
            "objective revision and reason must be non-empty",
        ));
    }
    Ok((value.to_owned(), reason.to_owned()))
}

fn command_operation(command: &Command) -> BridgeOperation {
    match command {
        Command::Capture(_) | Command::Research(_) | Command::Objective { .. } => {
            BridgeOperation::Capture
        }
        Command::Context
        | Command::Objectives
        | Command::Status { .. }
        | Command::Result { .. }
        | Command::Council { .. } => BridgeOperation::Query,
        Command::Approve { .. } => BridgeOperation::Approve,
        Command::Reject { .. } => BridgeOperation::Reject,
        Command::Revise { .. } => BridgeOperation::Revise,
        Command::PauseTask { .. }
        | Command::ResumeTask { .. }
        | Command::ReprioritizeTask { .. }
        | Command::ReviseObjective { .. }
        | Command::ApproveObjective { .. } => BridgeOperation::Control,
        Command::Cancel { .. } | Command::CancelTask { .. } => BridgeOperation::Cancel,
        Command::Acknowledge { .. } => BridgeOperation::Acknowledge,
        Command::Defer { .. } => BridgeOperation::Defer,
    }
}

fn is_resident_objective_mutation(command: &Command) -> bool {
    matches!(
        command,
        Command::Objective { .. }
            | Command::PauseTask { .. }
            | Command::ResumeTask { .. }
            | Command::ReprioritizeTask { .. }
            | Command::ReviseObjective { .. }
            | Command::ApproveObjective { .. }
            | Command::CancelTask { .. }
    )
}

fn command_run_id(command: &Command) -> Option<&str> {
    match command {
        Command::Capture(_)
        | Command::Research(_)
        | Command::Objective { .. }
        | Command::Context
        | Command::Objectives
        | Command::PauseTask { .. }
        | Command::ResumeTask { .. }
        | Command::ReprioritizeTask { .. }
        | Command::ReviseObjective { .. }
        | Command::ApproveObjective { .. }
        | Command::CancelTask { .. }
        | Command::Status { run_id: None }
        | Command::Acknowledge { .. }
        | Command::Defer { .. } => None,
        Command::Approve { run_id, .. }
        | Command::Reject { run_id, .. }
        | Command::Revise { run_id, .. }
        | Command::Cancel { run_id, .. }
        | Command::Status {
            run_id: Some(run_id),
        }
        | Command::Result { run_id }
        | Command::Council { run_id } => Some(run_id),
    }
}

fn command_project_id(command: &Command) -> Option<&str> {
    match command {
        Command::Objective { project_ids, .. } => project_ids.first().map(String::as_str),
        _ => None,
    }
}

fn command_task_id(command: &Command) -> Option<&str> {
    match command {
        Command::PauseTask { task_id, .. }
        | Command::ResumeTask { task_id, .. }
        | Command::ReprioritizeTask { task_id, .. }
        | Command::ReviseObjective { task_id, .. }
        | Command::ApproveObjective { task_id, .. }
        | Command::CancelTask { task_id, .. } => Some(task_id),
        _ => None,
    }
}

fn command_objective_id(command: &Command) -> Option<&str> {
    match command {
        Command::PauseTask { objective_id, .. }
        | Command::ResumeTask { objective_id, .. }
        | Command::ReprioritizeTask { objective_id, .. }
        | Command::ReviseObjective { objective_id, .. }
        | Command::ApproveObjective { objective_id, .. }
        | Command::CancelTask { objective_id, .. } => Some(objective_id),
        _ => None,
    }
}

fn session_id(event: &HermesMessageEvent) -> String {
    format!(
        "{}:{}:{}",
        event.source.platform,
        event.source.chat_id,
        event.source.thread_id.as_deref().unwrap_or("root")
    )
}

fn gateway_event_id(incoming: &GatewayOperatorMessage, message_id: &str) -> String {
    let identity = serde_json::json!({
        "adapter_id": incoming.adapter_id,
        "platform": incoming.event.source.platform,
        "chat_id": incoming.event.source.chat_id,
        "thread_id": incoming.event.source.thread_id,
        "operator_id": incoming.operator.operator_id,
        "message_id": message_id,
    });
    let digest = Sha256::digest(
        serde_json::to_vec(&identity).expect("gateway event identity serialization cannot fail"),
    );
    format!("gateway-event:{digest:x}")
}

fn audience(event: &HermesMessageEvent) -> Audience {
    match event.source.chat_type.as_str() {
        "dm" | "private" => Audience::Direct,
        "group" | "guild" | "channel" => Audience::Group,
        _ => Audience::OperatorPrivate,
    }
}

fn approval_digest(
    run_id: &str,
    node_id: &str,
    operator_id: &str,
    session_id: &str,
    conversation_id: &str,
) -> String {
    let mut hasher = Sha256::new();
    for value in [run_id, node_id, operator_id, session_id, conversation_id] {
        hasher.update((value.len() as u64).to_be_bytes());
        hasher.update(value.as_bytes());
    }
    format!("sha256:{:x}", hasher.finalize())
}

fn require_pending_approval_node(run: &Value, node_id: &str) -> Result<(), ApiError> {
    let node = run["graph"]["nodes"]
        .as_array()
        .and_then(|nodes| nodes.iter().find(|node| node["id"] == node_id))
        .ok_or_else(|| ApiError::not_found(format!("approval node `{node_id}` was not found")))?;
    if node["kind"] != "approval" {
        return Err(ApiError::conflict(format!(
            "node `{node_id}` is not an approval node"
        )));
    }
    if !matches!(
        node["state"].as_str(),
        Some("pending" | "ready" | "blocked" | "failed")
    ) {
        return Err(ApiError::conflict(format!(
            "approval node `{node_id}` is not pending"
        )));
    }
    Ok(())
}

fn mutation_envelope(message_id: &str, timestamp: &str) -> Value {
    json!({
        "approval": {
            "schema_version": "arda.orome.task_approval.v1",
            "proposal_id": format!("operator-message:{message_id}"),
            "approval_id": format!("gateway:{message_id}"),
            "ledger_writes": ["operator_sessions.jsonl"],
            "decision": "policy_safe",
            "created_at_utc": timestamp
        },
        "idempotency_key": format!("gateway:{message_id}")
    })
}

async fn get_json(state: &HarnessState, path: &str) -> Result<Value, ApiError> {
    proxy_json(state.client.get(url(state, path))).await
}

async fn post_json(
    state: &HarnessState,
    path: &str,
    body: Value,
    idempotency_key: Option<&str>,
    operator_id: Option<&str>,
) -> Result<Value, ApiError> {
    let mut request = state.client.post(url(state, path)).json(&body);
    if let Some(key) = idempotency_key {
        request = request.header("idempotency-key", key);
    }
    if let Some(operator_id) = operator_id {
        request = request.header("x-arda-operator-id", operator_id);
    }
    proxy_json(request).await
}

async fn proxy_json(request: reqwest::RequestBuilder) -> Result<Value, ApiError> {
    let response = request
        .send()
        .await
        .map_err(|error| ApiError::internal(format!("canonical operation failed: {error}")))?;
    let status = response.status();
    let value = response
        .json::<Value>()
        .await
        .map_err(|error| ApiError::internal(format!("canonical response was invalid: {error}")))?;
    if status.is_success() {
        Ok(value)
    } else {
        let message = value["error"]
            .as_str()
            .unwrap_or("canonical operation failed")
            .to_owned();
        match status {
            StatusCode::BAD_REQUEST => Err(ApiError::bad_request(message)),
            StatusCode::NOT_FOUND => Err(ApiError::not_found(message)),
            StatusCode::CONFLICT => Err(ApiError::conflict(message)),
            StatusCode::FORBIDDEN => Err(ApiError::forbidden(message)),
            _ => Err(ApiError::internal(message)),
        }
    }
}

fn url(state: &HarnessState, path: &str) -> String {
    format!("http://{}{}", state.harness_addr, path)
}

fn required_string<'a>(value: &'a Value, field: &str) -> Result<&'a str, ApiError> {
    value[field]
        .as_str()
        .ok_or_else(|| ApiError::internal(format!("canonical response omitted `{field}`")))
}

fn bridge_error(error: arda_orome::operator_bridge::BridgeError) -> ApiError {
    use arda_orome::operator_bridge::BridgeError;
    match error {
        BridgeError::DuplicateEvent(_) | BridgeError::ApprovalAlreadyConsumed(_) => {
            ApiError::conflict(error.to_string())
        }
        BridgeError::Persistence(_) => ApiError::internal(error.to_string()),
        _ => ApiError::bad_request(error.to_string()),
    }
}
