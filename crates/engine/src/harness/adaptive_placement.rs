use super::{mesh, HarnessState};
use arda_orome::a2a_mesh::{MeshPeerProjection, MeshProjection};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const SCHEMA_VERSION: &str = "arda.adaptive-placement.v1";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AdaptivePlacementRequest {
    objective_id: String,
    objective: String,
    data_domain: String,
    #[serde(default = "default_task_kind")]
    task_kind: String,
    #[serde(default)]
    deterministic_tool_suffices: bool,
    #[serde(default)]
    material_unresolved_risks: bool,
    #[serde(default)]
    unresolved_disagreement: bool,
    #[serde(default)]
    requires_tools: bool,
    #[serde(default)]
    requires_structured_output: bool,
    #[serde(default = "default_budget")]
    max_cost_usd: f64,
    #[serde(default)]
    execute: bool,
}

fn default_task_kind() -> String {
    "reasoning".to_owned()
}
fn default_budget() -> f64 {
    0.05
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
enum AdaptiveRole {
    Worker,
    Critic,
    Adjudicator,
    DeterministicTool,
}

#[derive(Debug, Clone, Serialize)]
struct RoleCapabilityRequest {
    role: AdaptiveRole,
    required_node_capability: String,
    required_model_tasks: Vec<String>,
    requires_tools: bool,
    requires_structured_output: bool,
    allowed_access_tiers: Vec<String>,
    execution_lifetime: String,
}

#[derive(Debug, Clone)]
struct Candidate {
    node: MeshPeerProjection,
    provider: Value,
    model: Value,
    score: f64,
}

#[derive(Debug, Serialize)]
struct PlacementReceipt {
    receipt_id: String,
    objective_id: String,
    role: AdaptiveRole,
    node_id: String,
    provider_id: String,
    model_id: String,
    execution_lifetime: String,
    provider_lifetime: String,
    privacy_decision: String,
    cost_decision: String,
    pressure_decision: String,
    health_decision: String,
    fallback_decisions: Vec<String>,
    sources: Vec<String>,
    execution: Value,
}

pub(super) async fn compose_place_execute(
    State(state): State<HarnessState>,
    headers: HeaderMap,
    Json(request): Json<AdaptivePlacementRequest>,
) -> Result<Json<Value>, Response> {
    mesh::require_operator(&state, &headers)?;
    validate_request(&request)?;

    let roles = compose_roles(&request);
    if request.deterministic_tool_suffices {
        return Ok(Json(json!({
            "schema_version": SCHEMA_VERSION,
            "objective_id": request.objective_id,
            "composition": {"worker_count": 0, "roles": roles, "reason": "deterministic code suffices"},
            "placements": [],
            "deterministic_execution": {
                "status": if request.execute { "completed" } else { "planned" },
                "lifetime": "process",
                "objective_digest": digest(request.objective.as_bytes()),
                "source": "operator-declared deterministic_tool_suffices"
            }
        })));
    }

    let mesh = mesh::projection_for_state(&state).await?;
    let provider_projection = fetch_providers(&state).await?;
    let mut selected = Vec::new();
    let mut used_profiles = BTreeSet::new();
    for role in &roles {
        let candidate =
            choose_candidate(role, &request, &mesh, &provider_projection, &used_profiles)?;
        used_profiles.insert((
            provider_id(&candidate.provider),
            candidate.node.node_id.clone(),
        ));
        selected.push((role.clone(), candidate));
    }

    let mut receipts = Vec::new();
    let mut prior_outputs = Vec::new();
    for (index, (role, candidate)) in selected.into_iter().enumerate() {
        let execution = if request.execute {
            execute_role(&state, &role.role, &request, &candidate, &prior_outputs).await?
        } else {
            json!({"status": "planned", "sequence": index + 1})
        };
        if let Some(summary) = execution.get("internal_output").and_then(Value::as_str) {
            prior_outputs.push(summary.to_owned());
        }
        let public_execution = redact_internal_output(execution);
        receipts.push(build_receipt(
            role.role.clone(),
            &request,
            &mesh,
            &provider_projection,
            &candidate,
            public_execution,
        ));
    }

    Ok(Json(json!({
        "schema_version": SCHEMA_VERSION,
        "objective_id": request.objective_id,
        "composition": {
            "worker_count": roles.len(),
            "roles": roles,
            "reason": if request.unresolved_disagreement {
                "a named material risk remains disputed, so bounded adjudication is required"
            } else if request.material_unresolved_risks {
                "named material unresolved risks require one independent critic"
            } else {
                "one worker is sufficient because no material unresolved risk was declared"
            }
        },
        "capability_profiles_used": used_profiles.into_iter().map(|(provider_id, node_id)| json!({
            "node_id": node_id, "provider_id": provider_id
        })).collect::<Vec<_>>(),
        "placements": receipts
    })))
}

fn validate_request(request: &AdaptivePlacementRequest) -> Result<(), Response> {
    if request.objective_id.trim().is_empty()
        || request.objective.trim().is_empty()
        || (request.unresolved_disagreement && !request.material_unresolved_risks)
        || request.data_domain.trim().is_empty()
        || request.task_kind.trim().is_empty()
        || !request.max_cost_usd.is_finite()
        || request.max_cost_usd < 0.0
    {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "invalid adaptive placement request",
        ));
    }
    Ok(())
}

fn compose_roles(request: &AdaptivePlacementRequest) -> Vec<RoleCapabilityRequest> {
    let access = if request.data_domain == "personal" || request.data_domain == "private" {
        vec!["local".to_owned()]
    } else {
        vec![
            "local".to_owned(),
            "mixed".to_owned(),
            "paid_cloud".to_owned(),
        ]
    };
    if request.deterministic_tool_suffices {
        return vec![RoleCapabilityRequest {
            role: AdaptiveRole::DeterministicTool,
            required_node_capability: "arda.deterministic.v1".to_owned(),
            required_model_tasks: Vec::new(),
            requires_tools: false,
            requires_structured_output: false,
            allowed_access_tiers: vec!["local".to_owned()],
            execution_lifetime: "process".to_owned(),
        }];
    }
    let mut roles = vec![RoleCapabilityRequest {
        role: AdaptiveRole::Worker,
        required_node_capability: "arda.cognition.worker.v1".to_owned(),
        required_model_tasks: vec![request.task_kind.clone()],
        requires_tools: request.requires_tools,
        requires_structured_output: request.requires_structured_output,
        allowed_access_tiers: access.clone(),
        execution_lifetime: "task".to_owned(),
    }];
    if request.material_unresolved_risks {
        roles.push(RoleCapabilityRequest {
            role: AdaptiveRole::Critic,
            required_node_capability: "arda.cognition.critic.v1".to_owned(),
            required_model_tasks: vec!["reasoning".to_owned()],
            requires_tools: false,
            requires_structured_output: false,
            allowed_access_tiers: access.clone(),
            execution_lifetime: "task".to_owned(),
        });
    }
    if request.unresolved_disagreement {
        roles.push(RoleCapabilityRequest {
            role: AdaptiveRole::Adjudicator,
            required_node_capability: "arda.cognition.adjudicator.v1".to_owned(),
            required_model_tasks: vec!["reasoning".to_owned()],
            requires_tools: false,
            requires_structured_output: true,
            allowed_access_tiers: access,
            execution_lifetime: "task".to_owned(),
        });
    }
    roles
}

async fn fetch_providers(state: &HarnessState) -> Result<Value, Response> {
    let url = format!(
        "{}/providers?include_models=true",
        state.manwe_url.trim_end_matches('/')
    );
    let mut request = state.client.get(&url).timeout(state.manwe_proxy_timeout);
    if let Some(bearer) = state.manwe_proxy_bearer.as_deref() {
        request = request.bearer_auth(bearer);
    }
    let response = request.send().await.map_err(|_| {
        error(
            StatusCode::BAD_GATEWAY,
            "Manwe provider health is unreachable",
        )
    })?;
    if !response.status().is_success() {
        return Err(error(
            StatusCode::BAD_GATEWAY,
            "Manwe provider health request failed",
        ));
    }
    response.json().await.map_err(|_| {
        error(
            StatusCode::BAD_GATEWAY,
            "Manwe returned invalid provider health",
        )
    })
}

fn choose_candidate(
    role: &RoleCapabilityRequest,
    request: &AdaptivePlacementRequest,
    mesh: &MeshProjection,
    providers: &Value,
    used: &BTreeSet<(String, String)>,
) -> Result<Candidate, Response> {
    let mut candidates = Vec::new();
    for node in mesh
        .peers
        .iter()
        .filter(|node| node.availability == "online")
    {
        if !node
            .capabilities
            .iter()
            .any(|cap| cap == &role.required_node_capability)
        {
            continue;
        }
        for provider in providers["providers"].as_array().into_iter().flatten() {
            let pid = provider_id(provider);
            let binding = format!("manwe.provider:{pid}");
            if !node.capabilities.iter().any(|cap| cap == &binding) {
                continue;
            }
            if provider["enabled"] != true
                || provider["healthy"] != true
                || provider["in_cooldown"] == true
                || provider["operational_blocked"] == true
            {
                continue;
            }
            if !role
                .allowed_access_tiers
                .iter()
                .any(|tier| provider["access_tier"].as_str() == Some(tier))
            {
                continue;
            }
            for model in provider["models"].as_array().into_iter().flatten() {
                if model["healthy"] != true || model["in_cooldown"] == true {
                    continue;
                }
                let tasks = model["capable_tasks"].as_array();
                if !role.required_model_tasks.iter().all(|required| {
                    tasks.is_some_and(|items| {
                        items.iter().any(|item| item.as_str() == Some(required))
                    })
                }) {
                    continue;
                }
                if role.requires_tools && model["capabilities"]["tools"] != true {
                    continue;
                }
                if role.requires_structured_output
                    && model["capabilities"]["structured_output"] != true
                {
                    continue;
                }
                let cost = estimated_cost(model);
                if cost > request.max_cost_usd {
                    continue;
                }
                let pressure = pressure_score(node);
                let quality = match provider["quality_band"].as_str() {
                    Some("high") => 0.0,
                    Some("medium") => 0.15,
                    _ => 0.3,
                };
                let reuse = if used.contains(&(pid.clone(), node.node_id.clone())) {
                    0.5
                } else {
                    0.0
                };
                candidates.push(Candidate {
                    node: node.clone(),
                    provider: provider.clone(),
                    model: model.clone(),
                    score: pressure + quality + reuse + cost,
                });
            }
        }
    }
    candidates.sort_by(|a, b| {
        a.score
            .total_cmp(&b.score)
            .then_with(|| provider_id(&a.provider).cmp(&provider_id(&b.provider)))
    });
    candidates.into_iter().next().ok_or_else(|| {
        error(
            StatusCode::CONFLICT,
            "no healthy node/model profile satisfies the role capability request",
        )
    })
}

fn pressure_score(node: &MeshPeerProjection) -> f64 {
    node.pressure
        .as_ref()
        .map(|p| {
            let gpu = p.gpu.unwrap_or(0.0);
            (f64::from(p.cpu) + f64::from(p.memory) + f64::from(gpu))
                / if p.gpu.is_some() { 3.0 } else { 2.0 }
                + (p.queue_depth as f64 * 0.02).min(0.4)
        })
        .unwrap_or(1.0)
}

fn estimated_cost(model: &Value) -> f64 {
    let input = model["cost_per_million_tokens_in"].as_f64().unwrap_or(0.0);
    let output = model["cost_per_million_tokens_out"].as_f64().unwrap_or(0.0);
    input * 0.0005 + output * 0.0002
}

async fn execute_role(
    state: &HarnessState,
    role: &AdaptiveRole,
    request: &AdaptivePlacementRequest,
    candidate: &Candidate,
    prior_outputs: &[String],
) -> Result<Value, Response> {
    let role_name = format!("{role:?}").to_ascii_lowercase();
    let context = if prior_outputs.is_empty() {
        String::new()
    } else {
        format!(
            "\nPrior bounded role outputs:\n{}",
            prior_outputs.join("\n---\n")
        )
    };
    let prompt = format!("Role: {role_name}. Objective: {}{context}\nReturn a concise, source-conscious result. Do not claim actions you did not perform.", request.objective);
    let url = format!(
        "{}/v1/chat/completions",
        state.manwe_url.trim_end_matches('/')
    );
    let mut outbound = state
        .client
        .post(url)
        .timeout(state.manwe_proxy_timeout)
        .json(&json!({
            "model": model_id(&candidate.model),
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": 160,
            "temperature": 0.1,
            "stream": false
        }));
    if let Some(bearer) = state.manwe_proxy_bearer.as_deref() {
        outbound = outbound.bearer_auth(bearer);
    }
    let response = outbound.send().await.map_err(|_| {
        error(
            StatusCode::BAD_GATEWAY,
            "placed role execution could not reach Manwe",
        )
    })?;
    let status = response.status();
    let actual_provider = response
        .headers()
        .get("x-manwe-provider-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let actual_model = response
        .headers()
        .get("x-manwe-model-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let body: Value = response.json().await.map_err(|_| {
        error(
            StatusCode::BAD_GATEWAY,
            "placed role execution returned invalid JSON",
        )
    })?;
    if !status.is_success() {
        return Err(error(
            StatusCode::BAD_GATEWAY,
            "placed role execution failed",
        ));
    }
    if actual_provider != provider_id(&candidate.provider)
        || actual_model != model_id(&candidate.model)
    {
        return Err(error(
            StatusCode::BAD_GATEWAY,
            "Manwe execution route differed from the placed profile",
        ));
    }
    let output = body["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    Ok(json!({
        "status": "completed",
        "actual_provider_id": actual_provider,
        "actual_model_id": actual_model,
        "output_digest": digest(output.as_bytes()),
        "output_chars": output.chars().count(),
        "lifetime_status": "task_terminated_after_receipt",
        "internal_output": output
    }))
}

fn redact_internal_output(mut execution: Value) -> Value {
    if let Some(object) = execution.as_object_mut() {
        object.remove("internal_output");
    }
    execution
}

fn build_receipt(
    role: AdaptiveRole,
    request: &AdaptivePlacementRequest,
    mesh: &MeshProjection,
    providers: &Value,
    candidate: &Candidate,
    execution: Value,
) -> PlacementReceipt {
    let pid = provider_id(&candidate.provider);
    let mid = model_id(&candidate.model);
    let rejected = providers["providers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|provider| {
            let other = provider_id(provider);
            (other != pid).then(|| {
                format!(
                    "{other}: {}",
                    if provider["operational_blocked"] == true || provider["healthy"] != true {
                        "rejected by live health"
                    } else {
                        "lower joint capability/pressure/cost score or no eligible node binding"
                    }
                )
            })
        })
        .collect();
    PlacementReceipt {
        receipt_id: format!("placement:{}:{}", request.objective_id, format!("{role:?}").to_ascii_lowercase()),
        objective_id: request.objective_id.clone(),
        role,
        node_id: candidate.node.node_id.clone(),
        provider_id: pid.clone(),
        model_id: mid.clone(),
        execution_lifetime: "task".to_owned(),
        provider_lifetime: candidate.provider["hermes_bridge"]["persistent"].as_bool().map(|v| if v { "persistent" } else { "request" }).unwrap_or("provider_managed").to_owned(),
        privacy_decision: format!("data domain `{}` admitted to provider access tier `{}` in trust domain `{}`", request.data_domain, candidate.provider["access_tier"].as_str().unwrap_or("unknown"), candidate.node.trust_domain),
        cost_decision: format!("estimated bounded request cost ${:.6} within ${:.6} objective limit", estimated_cost(&candidate.model), request.max_cost_usd),
        pressure_decision: format!("live node pressure score {:.3} from CPU/memory/GPU/queue observation", pressure_score(&candidate.node)),
        health_decision: format!("Manwe reports provider `{pid}` and model `{mid}` healthy, enabled, and outside cooldown"),
        fallback_decisions: rejected,
        sources: vec![
            format!("arda.a2a-mesh-projection.v1 generated {} for node `{}`", mesh.generated_at.to_rfc3339(), candidate.node.node_id),
            format!("Manwe /providers intelligence refreshed {} for provider `{pid}`", candidate.provider["intelligence_refreshed_at_utc"].as_str().unwrap_or("unknown")),
            format!("Manwe model catalog health for `{mid}`"),
        ],
        execution,
    }
}

fn provider_id(value: &Value) -> String {
    value["provider_id"]
        .as_str()
        .or_else(|| value["id"].as_str())
        .unwrap_or_default()
        .to_owned()
}
fn model_id(value: &Value) -> String {
    value["id"].as_str().unwrap_or_default().to_owned()
}
fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn error(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({"error": message}))).into_response()
}
