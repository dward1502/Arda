//! Authenticated Hermes Gateway operator-message ingress.
//!
//! Hermes owns transport credentials and allowlist authentication. This
//! loopback-only endpoint accepts the normalized event plus that authentication
//! assertion, records the canonical operator session through Oromë, and then
//! invokes the existing Workbench mutation surfaces with event-derived
//! idempotency keys.

use arda_orome::operator_bridge::{
    ApprovalBinding, ApprovalSingleUseState, Audience, BridgeApproval, BridgeLineage,
    BridgeOperation, BridgeRequest, ContentSensitivity, HermesMessageEvent, HermesPromptResponse,
    OperatorIdentity,
};
use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
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
    projects::{require_loopback, ApiError},
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
    Objective {
        project_id: String,
        text: String,
    },
    Context,
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
    Json(mut incoming): Json<GatewayOperatorMessage>,
) -> Result<Json<GatewayOperatorResponse>, ApiError> {
    require_loopback(peer)?;
    if !incoming.operator.authenticated
        || incoming.operator.authentication_method != "gateway_identity"
    {
        return Err(ApiError::forbidden(
            "operator message requires Hermes Gateway identity authentication",
        ));
    }
    if incoming.event.user_id.as_deref() != Some(incoming.operator.operator_id.as_str()) {
        return Err(ApiError::forbidden(
            "gateway operator identity does not match MessageEvent user_id",
        ));
    }

    let command = parse_command(&incoming.event.text)?;
    let audience = audience(&incoming.event);
    if matches!(
        command,
        Command::Capture(_)
            | Command::Objective { .. }
            | Command::Context
            | Command::Acknowledge { .. }
            | Command::Defer { .. }
    ) && !matches!(audience, Audience::Direct | Audience::OperatorPrivate)
    {
        return Err(ApiError::forbidden(
            "personal operator commands require a private conversation",
        ));
    }
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
            objective_id: None,
            project_id: command_project_id(&command).map(str::to_owned),
            task_id: None,
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
    let session = match pending.as_ref() {
        Some(binding) => runtime
            .ingest_approval(bridge_request, binding, now)
            .map_err(bridge_error)?,
        None => runtime.ingest(bridge_request, now).map_err(bridge_error)?,
    };

    let (summary, mut evidence_refs) = apply_command(&state, &incoming, &command).await?;
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
        Command::Objective { project_id, text } => {
            let projects = get_json(state, "/v1/projects").await?;
            let attached = projects["projects"].as_array().is_some_and(|projects| {
                projects.iter().any(|project| {
                    project["contract"]["identity"]["project_id"].as_str()
                        == Some(project_id.as_str())
                })
            });
            if !attached {
                return Err(ApiError::not_found(format!(
                    "project `{project_id}` is not attached"
                )));
            }
            let capture = post_json(
                state,
                "/v1/personal/captures",
                json!({
                    "operator_id": incoming.operator.operator_id,
                    "text": text,
                    "audio_reference": null,
                    "project_id": project_id,
                    "priority": null,
                    "due_at": null
                }),
                Some(message_id),
                Some(&incoming.operator.operator_id),
            )
            .await?;
            let capture_id = required_string(&capture, "capture_id")?;
            Ok((
                format!("Created objective capture {capture_id} for project {project_id}."),
                vec![
                    format!("arda://personal/captures/{capture_id}"),
                    format!("arda://projects/{project_id}"),
                ],
            ))
        }
        Command::Context => {
            let response = get_json(state, "/v1/personal/resume").await?;
            let summary = response["resume"]["summary"]
                .as_str()
                .unwrap_or("No personal resume context is available.")
                .to_owned();
            Ok((summary, vec!["arda://personal/resume".into()]))
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
        "objective" => {
            let (project_id, text) = take_arg(args, "objective project_id")?;
            if text.is_empty() {
                return Err(ApiError::bad_request("objective text cannot be empty"));
            }
            Ok(Command::Objective {
                project_id,
                text: text.to_owned(),
            })
        }
        "context" => require_no_args(args).map(|()| Command::Context),
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
            "unsupported operator command; use capture, objective, context, status, approve, reject, revise, cancel, acknowledge, defer, result, or council",
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

fn command_operation(command: &Command) -> BridgeOperation {
    match command {
        Command::Capture(_) | Command::Objective { .. } => BridgeOperation::Capture,
        Command::Context
        | Command::Status { .. }
        | Command::Result { .. }
        | Command::Council { .. } => BridgeOperation::Query,
        Command::Approve { .. } => BridgeOperation::Approve,
        Command::Reject { .. } => BridgeOperation::Reject,
        Command::Revise { .. } => BridgeOperation::Revise,
        Command::Cancel { .. } => BridgeOperation::Cancel,
        Command::Acknowledge { .. } => BridgeOperation::Acknowledge,
        Command::Defer { .. } => BridgeOperation::Defer,
    }
}

fn command_run_id(command: &Command) -> Option<&str> {
    match command {
        Command::Capture(_)
        | Command::Objective { .. }
        | Command::Context
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
        Command::Objective { project_id, .. } => Some(project_id),
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
