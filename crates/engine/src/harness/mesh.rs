use super::HarnessState;
use arda_orome::a2a_mesh::{
    A2aHandoffReceipt, A2aMeshError, CapabilityObservation, MeshRegistry, NodeEnrollment,
    WorkEnvelope, HANDOFF_RECEIPT_SCHEMA_VERSION,
};
use axum::extract::Path;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use serde_json::{json, Value};
use uuid::Uuid;

const REGISTRY_PATH: &str = "core/state/a2a_mesh_registry.jsonl";
const NODE_CONFIG_PATH: &str = "config/a2a-node.toml";

#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct A2aNodeConfig {
    schema_version: String,
    node_id: String,
    agent_id: String,
    trust_domain: String,
    capabilities: Vec<String>,
    allowed_data_domains: Vec<String>,
    inbound_bearer_env: String,
}

#[derive(serde::Deserialize)]
pub(super) struct RevokeRequest {
    reason: String,
}

pub(super) async fn enroll(
    State(state): State<HarnessState>,
    headers: HeaderMap,
    Json(enrollment): Json<NodeEnrollment>,
) -> Result<StatusCode, Response> {
    require_operator(&state, &headers)?;
    mutate_registry(&state, move |registry, now| {
        registry.enroll(enrollment, now)
    })
    .await?;
    Ok(StatusCode::CREATED)
}

pub(super) async fn publish_observation(
    State(state): State<HarnessState>,
    headers: HeaderMap,
    Json(observation): Json<CapabilityObservation>,
) -> Result<StatusCode, Response> {
    require_operator(&state, &headers)?;
    mutate_registry(&state, move |registry, now| {
        registry.publish_observation(observation, now)
    })
    .await?;
    Ok(StatusCode::CREATED)
}

pub(super) async fn revoke(
    State(state): State<HarnessState>,
    headers: HeaderMap,
    Path(node_id): Path<String>,
    Json(request): Json<RevokeRequest>,
) -> Result<StatusCode, Response> {
    require_operator(&state, &headers)?;
    mutate_registry(&state, move |registry, now| {
        registry.revoke(&node_id, &request.reason, now)
    })
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn mutate_registry(
    state: &HarnessState,
    mutation: impl FnOnce(&mut MeshRegistry, chrono::DateTime<Utc>) -> Result<(), A2aMeshError>
        + Send
        + 'static,
) -> Result<(), Response> {
    let path = state.workbench_root.join(REGISTRY_PATH);
    tokio::task::spawn_blocking(move || {
        let mut registry = MeshRegistry::open(path)?;
        mutation(&mut registry, Utc::now())
    })
    .await
    .map_err(|_| internal("mesh registry worker failed"))?
    .map_err(map_registry_error)
}

pub(super) async fn get_projection(
    State(state): State<HarnessState>,
    headers: HeaderMap,
) -> Result<Json<Value>, Response> {
    require_operator(&state, &headers)?;
    let path = state.workbench_root.join(REGISTRY_PATH);
    let projection = tokio::task::spawn_blocking(move || {
        MeshRegistry::open(path).map(|registry| registry.projection(Utc::now()))
    })
    .await
    .map_err(|_| internal("mesh projection worker failed"))?
    .map_err(map_registry_error)?;
    serde_json::to_value(projection)
        .map(Json)
        .map_err(|_| internal("mesh projection serialization failed"))
}

pub(super) async fn agent_card(
    State(state): State<HarnessState>,
    headers: HeaderMap,
) -> Result<Json<Value>, Response> {
    let config = load_node_config(&state)?;
    require_inbound_bearer(&config, &headers)?;
    let host = headers
        .get("host")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| bad_request("A2A Agent Card request has no host"))?;
    let skills = config
        .capabilities
        .iter()
        .map(|id| {
            json!({
                "id": id,
                "name": "Arda typed capability",
                "description": "Executes a bounded typed Arda work envelope",
                "tags": ["arda", "typed-work"]
            })
        })
        .collect::<Vec<_>>();
    Ok(Json(json!({
        "name": config.agent_id,
        "description": "Enrolled Arda node exposing bounded typed work over A2A",
        "url": format!("http://{host}/v1/a2a"),
        "protocolVersion": "1.0",
        "version": "1",
        "capabilities": {"streaming": false},
        "skills": skills,
        "metadata": {
            "ardaNodeId": config.node_id,
            "ardaTrustDomain": config.trust_domain,
        }
    })))
}

pub(super) async fn receive(
    State(state): State<HarnessState>,
    headers: HeaderMap,
    Json(request): Json<Value>,
) -> Result<Json<Value>, Response> {
    let config = load_node_config(&state)?;
    require_inbound_bearer(&config, &headers)?;
    let envelope = parse_inbound_request(&request, &config)?;
    envelope
        .validate_at(Utc::now())
        .map_err(map_registry_error)?;

    let received_at = Utc::now();
    let task_id = format!("task:{}", envelope.envelope_id);
    let receipt = A2aHandoffReceipt {
        schema_version: HANDOFF_RECEIPT_SCHEMA_VERSION.to_owned(),
        receipt_id: format!("receipt:{}", Uuid::new_v4()),
        envelope_id: envelope.envelope_id.clone(),
        objective_id: envelope.objective_id.clone(),
        run_id: envelope.run_id.clone(),
        worker_id: envelope.worker_id.clone(),
        source_node_id: envelope
            .route_trace
            .last()
            .cloned()
            .unwrap_or_else(|| "unknown".to_owned()),
        target_node_id: config.node_id.clone(),
        a2a_task_id: task_id.clone(),
        a2a_context_id: envelope.run_id.clone(),
        status: "completed".to_owned(),
        dispatched_at: received_at,
        completed_at: Utc::now(),
    };
    let envelope_to_claim = envelope.clone();
    let receipt_to_store = receipt.clone();
    mutate_registry(&state, move |registry, now| {
        registry.claim_dispatch(&envelope_to_claim, now)?;
        registry.record_receipt(receipt_to_store, now)
    })
    .await?;

    Ok(Json(json!({
        "jsonrpc": "2.0",
        "id": envelope.envelope_id,
        "result": {"task": {
            "id": task_id,
            "contextId": envelope.run_id,
            "status": {"state": "TASK_STATE_COMPLETED"},
            "artifacts": [{
                "artifactId": format!("artifact:{}", envelope.envelope_id),
                "parts": [{
                    "data": {"payload": envelope.payload, "receipt": receipt},
                    "mediaType": "application/vnd.arda.typed-result.v1+json"
                }]
            }]
        }}
    })))
}

pub(super) async fn dispatch(
    State(state): State<HarnessState>,
    headers: HeaderMap,
    Json(envelope): Json<WorkEnvelope>,
) -> Result<Json<Value>, Response> {
    require_operator(&state, &headers)?;
    let path = state.workbench_root.join(REGISTRY_PATH);
    let now = Utc::now();
    let (peer, bearer, envelope) = tokio::task::spawn_blocking(move || {
        let mut registry = MeshRegistry::open(path)?;
        let peer = registry.route(&envelope, now)?;
        registry.claim_dispatch(&envelope, now)?;
        let bearer = std::env::var(&peer.enrollment.bearer_env)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .ok_or(A2aMeshError::AuthenticationUnavailable)?;
        Ok::<_, A2aMeshError>((peer, bearer, envelope))
    })
    .await
    .map_err(|_| internal("mesh dispatch worker failed"))?
    .map_err(map_registry_error)?;

    let card_response = state
        .client
        .get(&peer.enrollment.agent_card_url)
        .bearer_auth(&bearer)
        .timeout(state.manwe_proxy_timeout)
        .send()
        .await
        .map_err(|_| bad_gateway("enrolled peer Agent Card is unreachable"))?;
    if !card_response.status().is_success() {
        return Err(bad_gateway("enrolled peer rejected Agent Card discovery"));
    }
    let card: Value = card_response
        .json()
        .await
        .map_err(|_| bad_gateway("enrolled peer returned an invalid Agent Card"))?;
    let rpc_url =
        validate_agent_card(&card, &peer.enrollment.agent_card_url, &envelope.capability)?;
    let request = envelope
        .to_a2a_send_message(&peer.enrollment.identity.node_id)
        .map_err(map_registry_error)?;
    let dispatched_at = Utc::now();
    let response = state
        .client
        .post(rpc_url)
        .bearer_auth(&bearer)
        .timeout(state.manwe_proxy_timeout)
        .json(&request)
        .send()
        .await
        .map_err(|_| bad_gateway("enrolled peer A2A endpoint is unreachable"))?;
    if !response.status().is_success() {
        return Err(bad_gateway(
            "enrolled peer rejected the authenticated A2A task",
        ));
    }
    let body: Value = response
        .json()
        .await
        .map_err(|_| bad_gateway("enrolled peer returned invalid A2A JSON"))?;
    let result = validate_completion(&body, &envelope)?;
    let completed_at = Utc::now();
    let receipt = A2aHandoffReceipt {
        schema_version: HANDOFF_RECEIPT_SCHEMA_VERSION.to_owned(),
        receipt_id: format!("receipt:{}", Uuid::new_v4()),
        envelope_id: envelope.envelope_id.clone(),
        objective_id: envelope.objective_id.clone(),
        run_id: envelope.run_id.clone(),
        worker_id: envelope.worker_id.clone(),
        source_node_id: envelope
            .route_trace
            .last()
            .cloned()
            .unwrap_or_else(|| "node-root".to_owned()),
        target_node_id: peer.enrollment.identity.node_id.clone(),
        a2a_task_id: result["id"].as_str().unwrap_or_default().to_owned(),
        a2a_context_id: result["contextId"].as_str().unwrap_or_default().to_owned(),
        status: "completed".to_owned(),
        dispatched_at,
        completed_at,
    };
    let receipt_to_store = receipt.clone();
    let registry_path = state.workbench_root.join(REGISTRY_PATH);
    tokio::task::spawn_blocking(move || {
        let mut registry = MeshRegistry::open(registry_path)?;
        registry.record_receipt(receipt_to_store, completed_at)
    })
    .await
    .map_err(|_| internal("mesh receipt worker failed"))?
    .map_err(map_registry_error)?;

    Ok(Json(json!({
        "schema_version": "arda.a2a-dispatch-response.v1",
        "receipt": receipt,
        "result": result,
    })))
}

fn require_operator(state: &HarnessState, headers: &HeaderMap) -> Result<(), Response> {
    let supplied = headers
        .get("x-arda-operator-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim);
    if supplied == Some(state.operator_id.as_str()) {
        Ok(())
    } else {
        Err(error_response(
            StatusCode::FORBIDDEN,
            "forbidden",
            "mesh access requires configured operator identity",
        ))
    }
}

fn validate_agent_card(
    card: &Value,
    enrolled_card_url: &str,
    capability: &str,
) -> Result<String, Response> {
    if card["protocolVersion"].as_str() != Some("1.0") {
        return Err(bad_gateway("enrolled peer does not advertise A2A v1.0"));
    }
    let advertised = card["skills"].as_array().is_some_and(|skills| {
        skills
            .iter()
            .any(|skill| skill["id"].as_str() == Some(capability))
    });
    if !advertised {
        return Err(bad_gateway(
            "Agent Card does not advertise the routed capability",
        ));
    }
    let rpc = card["url"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| bad_gateway("Agent Card omits its A2A endpoint"))?;
    let enrolled = reqwest::Url::parse(enrolled_card_url)
        .map_err(|_| bad_gateway("enrollment contains an invalid Agent Card URL"))?;
    let advertised = reqwest::Url::parse(rpc)
        .map_err(|_| bad_gateway("Agent Card contains an invalid A2A endpoint"))?;
    if enrolled.scheme() != advertised.scheme()
        || enrolled.host_str() != advertised.host_str()
        || enrolled.port_or_known_default() != advertised.port_or_known_default()
    {
        return Err(bad_gateway(
            "Agent Card endpoint escaped the enrolled origin",
        ));
    }
    Ok(advertised.to_string())
}

fn validate_completion<'a>(
    body: &'a Value,
    envelope: &WorkEnvelope,
) -> Result<&'a Value, Response> {
    if body["jsonrpc"].as_str() != Some("2.0")
        || body["id"].as_str() != Some(envelope.envelope_id.as_str())
    {
        return Err(bad_gateway("A2A completion response correlation is forged"));
    }
    let wrapper = body
        .get("result")
        .ok_or_else(|| bad_gateway("A2A completion response has no result"))?;
    // A2A v1.0 wraps SendMessage results in exactly one `task` or
    // `message` member. Accept a legacy bare Task only for enrolled peers.
    let result = wrapper.get("task").unwrap_or(wrapper);
    if result["id"].as_str().is_none_or(str::is_empty)
        || result["contextId"].as_str() != Some(envelope.run_id.as_str())
        || result["status"]["state"].as_str() != Some("TASK_STATE_COMPLETED")
    {
        return Err(bad_gateway("A2A completion response correlation is forged"));
    }
    Ok(result)
}

fn load_node_config(state: &HarnessState) -> Result<A2aNodeConfig, Response> {
    let text = std::fs::read_to_string(state.workbench_root.join(NODE_CONFIG_PATH))
        .map_err(|_| service_unavailable("A2A node configuration is unavailable"))?;
    let config: A2aNodeConfig = toml::from_str(&text)
        .map_err(|_| service_unavailable("A2A node configuration is invalid"))?;
    if config.schema_version != "arda.a2a-node-config.v1"
        || config.node_id.trim().is_empty()
        || config.agent_id.trim().is_empty()
        || config.trust_domain.trim().is_empty()
        || config.capabilities.is_empty()
        || config.allowed_data_domains.is_empty()
        || config.inbound_bearer_env.trim().is_empty()
    {
        return Err(service_unavailable("A2A node configuration is invalid"));
    }
    Ok(config)
}

fn require_inbound_bearer(config: &A2aNodeConfig, headers: &HeaderMap) -> Result<(), Response> {
    let expected = std::env::var(&config.inbound_bearer_env)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| service_unavailable("A2A inbound authentication is unavailable"))?;
    let supplied = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    if supplied == Some(expected.as_str()) {
        Ok(())
    } else {
        Err(error_response(
            StatusCode::UNAUTHORIZED,
            "a2a_authentication_failed",
            "A2A request authentication failed",
        ))
    }
}

fn parse_inbound_request(
    request: &Value,
    config: &A2aNodeConfig,
) -> Result<WorkEnvelope, Response> {
    if request["jsonrpc"].as_str() != Some("2.0")
        || request["method"].as_str() != Some("SendMessage")
    {
        return Err(bad_request("A2A request method is invalid"));
    }
    let message = &request["params"]["message"];
    let data = message["parts"]
        .as_array()
        .and_then(|parts| {
            parts.iter().find(|part| {
                part["mediaType"].as_str() == Some("application/vnd.arda.work-envelope.v1+json")
            })
        })
        .and_then(|part| part.get("data"))
        .cloned()
        .ok_or_else(|| bad_request("A2A request has no typed Arda work envelope"))?;
    let envelope: WorkEnvelope = serde_json::from_value(data)
        .map_err(|_| bad_request("A2A typed work envelope is invalid"))?;
    if request["id"].as_str() != Some(envelope.envelope_id.as_str())
        || message["messageId"].as_str() != Some(envelope.envelope_id.as_str())
        || message["contextId"].as_str() != Some(envelope.run_id.as_str())
        || message["metadata"]["ardaTargetNode"].as_str() != Some(config.node_id.as_str())
        || !config.capabilities.contains(&envelope.capability)
        || !config.allowed_data_domains.contains(&envelope.data_domain)
        || envelope.route_trace.contains(&config.node_id)
    {
        return Err(bad_request(
            "A2A request correlation or node policy is invalid",
        ));
    }
    Ok(envelope)
}

fn map_registry_error(error: A2aMeshError) -> Response {
    let status = match error {
        A2aMeshError::ReplayDetected => StatusCode::CONFLICT,
        A2aMeshError::AuthenticationUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        A2aMeshError::RegistryIo | A2aMeshError::InvalidRegistryRow => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
        _ => StatusCode::UNPROCESSABLE_ENTITY,
    };
    error_response(status, "mesh_contract_rejected", &error.to_string())
}

fn bad_request(message: &str) -> Response {
    error_response(StatusCode::BAD_REQUEST, "invalid_a2a_request", message)
}

fn service_unavailable(message: &str) -> Response {
    error_response(StatusCode::SERVICE_UNAVAILABLE, "a2a_unavailable", message)
}

fn bad_gateway(message: &str) -> Response {
    error_response(StatusCode::BAD_GATEWAY, "a2a_peer_rejected", message)
}

fn internal(message: &str) -> Response {
    error_response(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", message)
}

fn error_response(status: StatusCode, code: &str, message: &str) -> Response {
    (
        status,
        Json(json!({
            "schema_version": "arda.hud.error.v1",
            "status": "failed",
            "code": code,
            "message": message,
        })),
    )
        .into_response()
}
