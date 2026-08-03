//! Explicit-question Warden → Varda research briefs linked to Workbench runs.
//!
//! Warden results are discovery previews only. This boundary fetches canonical
//! source content, records Varda crawl and evaluation receipts, and links the
//! resulting advisory brief to an existing run without changing node state.

use arda_core::run_graph::{NodeId, RunId};
use arda_outpost_protocol::{inspect_untrusted_content, ResearchBetaPolicy};
use arda_varda::ingest::{AthenaStore, CrawlMarkdownResult};
use axum::{
    extract::{ConnectInfo, State},
    Json,
};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    net::SocketAddr,
    path::Path,
};

use crate::runs::{RunEventDraft, RunEventKind, RunStore};

use super::{
    projects::{require_loopback, ApiError, MutationEnvelope, WORKBENCH_MUTATIONS},
    HarnessState,
};

const BRIEF_SCHEMA: &str = "arda.workbench.research-brief.v1";
const MAX_SOURCES: usize = 5;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchBriefRequest {
    run_id: String,
    node_id: String,
    question: String,
    #[serde(default = "default_source_limit")]
    source_limit: usize,
    envelope: MutationEnvelope,
}

fn default_source_limit() -> usize {
    3
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefCitation {
    citation_id: String,
    title: String,
    discovered_url: String,
    canonical_url: String,
    content_sha256: String,
    excerpt: String,
    stance: String,
    varda_source_id: String,
    varda_pipeline_id: String,
    crawl_receipt_path: String,
    policy_readiness: String,
    confidence: f64,
    #[serde(default)]
    normalized_source_id: String,
    #[serde(default)]
    fetched_at_utc: String,
    #[serde(default)]
    expires_at_utc: String,
    #[serde(default)]
    freshness_status: String,
    #[serde(default)]
    evaluation_digest: String,
    #[serde(default)]
    expiry_digest: String,
    #[serde(default)]
    receipt_references: Vec<String>,
    #[serde(default)]
    evidence_boundary: String,
    #[serde(default)]
    prompt_injection_detected: bool,
    #[serde(default)]
    prompt_injection_signals: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchClaim {
    claim_id: String,
    claim: String,
    evidence_citation_ids: Vec<String>,
    stance: String,
    uncertainty: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ResearchScope {
    source_policy: String,
    max_sources: usize,
    max_source_bytes: usize,
    expires_at_utc: String,
    max_results: usize,
    max_fetch_bytes: usize,
    max_tokens: usize,
    max_attempts: usize,
    cooldown_ms: u64,
    max_sources_per_domain: usize,
    retained_preview_volume: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchSourceQuality {
    citation_id: String,
    source_id: String,
    canonical_url: String,
    policy_readiness: String,
    confidence: f64,
    freshness_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchNextStep {
    kind: String,
    action: String,
    authority: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchBrief {
    schema_version: String,
    brief_id: String,
    run_id: String,
    node_id: String,
    question: String,
    generated_at_utc: String,
    authority: String,
    execution_authorized: bool,
    warden_provider: String,
    warden_memory_receipt: Option<String>,
    summary: String,
    contradiction_status: String,
    contradictions: Vec<String>,
    citations: Vec<BriefCitation>,
    source_failures: Vec<String>,
    workbench_run_link: String,
    #[serde(default)]
    scope: ResearchScope,
    #[serde(default)]
    claims: Vec<ResearchClaim>,
    #[serde(default)]
    supporting_citation_ids: Vec<String>,
    #[serde(default)]
    opposing_citation_ids: Vec<String>,
    #[serde(default)]
    source_quality: Vec<ResearchSourceQuality>,
    #[serde(default)]
    uncertainty: Vec<String>,
    #[serde(default)]
    missing_evidence: Vec<String>,
    #[serde(default)]
    next_research_or_proposal: Vec<ResearchNextStep>,
    #[serde(default)]
    receipt_references: Vec<String>,
    #[serde(default)]
    material_fingerprint: String,
    #[serde(default)]
    change_status: String,
    #[serde(default)]
    no_change_receipt_path: Option<String>,
    #[serde(default)]
    evidence_boundaries: Vec<String>,
    #[serde(default)]
    prompt_injection_detected: bool,
}

#[derive(Debug, Deserialize)]
struct WardenSearchResponse {
    report: WardenReport,
    memory: WardenMemory,
}

#[derive(Debug, Deserialize)]
struct WardenReport {
    provider: String,
    results: Vec<WardenResult>,
}

#[derive(Debug, Deserialize)]
struct WardenResult {
    title: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct WardenMemory {
    memory_id: Option<String>,
}

pub(super) async fn create_brief(
    State(state): State<HarnessState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(request): Json<ResearchBriefRequest>,
) -> Result<Json<ResearchBrief>, ApiError> {
    require_loopback(peer)?;
    request.envelope.validate()?;
    validate_request(&request)?;
    let policy = ResearchBetaPolicy::default();
    policy
        .validate()
        .map_err(|field| ApiError::internal(format!("invalid research beta policy: {field}")))?;
    let _guard = WORKBENCH_MUTATIONS.lock().await;

    let run_id = RunId::new(&request.run_id)
        .map_err(|error| ApiError::bad_request(format!("invalid run id: {error}")))?;
    let node_id = NodeId::new(&request.node_id)
        .map_err(|error| ApiError::bad_request(format!("invalid node id: {error}")))?;
    let store = RunStore::open(&state.workbench_root, run_id).map_err(store_error)?;
    let recovered = store.recover().map_err(store_error)?;
    let graph = recovered
        .checkpoint
        .ok_or_else(|| ApiError::not_found(format!("run `{}` was not found", request.run_id)))?;
    if !graph.nodes.iter().any(|node| node.id == node_id) {
        return Err(ApiError::not_found(format!(
            "node `{}` was not found in run `{}`",
            request.node_id, request.run_id
        )));
    }

    let brief_id = stable_brief_id(&request.run_id, &request.node_id, &request.question);
    let brief_path = brief_path(&state.workbench_root, &request.run_id, &brief_id);
    if recovered
        .applied_idempotency_keys
        .contains_key(&request.envelope.idempotency_key)
    {
        let raw = fs::read(&brief_path).map_err(|error| {
            ApiError::internal(format!("failed to read existing research brief: {error}"))
        })?;
        let brief = serde_json::from_slice(&raw).map_err(|error| {
            ApiError::internal(format!("failed to parse existing research brief: {error}"))
        })?;
        return Ok(Json(brief));
    }

    let previous_brief = fs::read(&brief_path)
        .ok()
        .and_then(|raw| serde_json::from_slice::<ResearchBrief>(&raw).ok());
    let generated_at = Utc::now();
    let expires_at = generated_at + ChronoDuration::minutes(15);

    let scout_url = state
        .warden_scout_url
        .as_deref()
        .ok_or_else(|| ApiError::internal("Warden scout is not configured"))?;
    let mut warden = None;
    let mut last_search_error = None;
    for attempt in 0..policy.max_attempts {
        let response = state
            .client
            .post(format!("{}/search", scout_url.trim_end_matches('/')))
            .timeout(state.warden_scout_timeout)
            .json(&serde_json::json!({
                "query": request.question,
                "limit": request.source_limit.min(MAX_SOURCES).min(policy.max_results),
                "source_policy": "allowlisted_public_web",
                "expires_at": expires_at.to_rfc3339(),
            }))
            .send()
            .await;
        match response {
            Ok(response)
                if response.status().is_server_error()
                    || response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS =>
            {
                last_search_error = Some(format!(
                    "Warden search returned transient status {}",
                    response.status()
                ));
            }
            Ok(response) => {
                warden = Some(
                    response
                        .error_for_status()
                        .map_err(|error| {
                            ApiError::internal(format!("Warden search returned failure: {error}"))
                        })?
                        .json::<WardenSearchResponse>()
                        .await
                        .map_err(|error| {
                            ApiError::internal(format!("invalid Warden search response: {error}"))
                        })?,
                );
                break;
            }
            Err(error) => {
                last_search_error = Some(format!("Warden search failed: {error}"));
            }
        }
        if attempt + 1 < policy.max_attempts {
            tokio::time::sleep(std::time::Duration::from_millis(policy.cooldown_ms)).await;
        }
    }
    let warden = warden.ok_or_else(|| {
        ApiError::internal(
            last_search_error.unwrap_or_else(|| "Warden search exhausted retry budget".to_string()),
        )
    })?;

    let athena_root = state.workbench_root.join("data/athena");
    let mut citations = Vec::new();
    let mut source_failures = Vec::new();
    let mut domain_counts = BTreeMap::<String, usize>::new();
    for result in warden.report.results.into_iter().take(
        request
            .source_limit
            .min(MAX_SOURCES)
            .min(policy.max_results),
    ) {
        if let Ok(url) = reqwest::Url::parse(&result.url) {
            if let Some(domain) = url.host_str().map(str::to_ascii_lowercase) {
                let count = domain_counts.entry(domain.clone()).or_default();
                if *count >= policy.max_sources_per_domain {
                    source_failures.push(format!(
                        "{}: source-domain rate bound exceeded for `{domain}`",
                        result.url
                    ));
                    continue;
                }
                *count += 1;
            }
        }
        match fetch_and_evaluate(&state, &athena_root, &request, result, expires_at, &policy).await
        {
            Ok(citation) => citations.push(citation),
            Err(error) => source_failures.push(error),
        }
    }

    let (contradiction_status, contradictions) = contradiction_assessment(&citations);
    let claims = claims_from_citations(&citations);
    let supporting_citation_ids = citation_ids_with_stance(&citations, "supporting_or_contextual");
    let opposing_citation_ids = citation_ids_with_stance(&citations, "opposing_or_cautionary");
    let source_quality = citations
        .iter()
        .map(|citation| ResearchSourceQuality {
            citation_id: citation.citation_id.clone(),
            source_id: citation.normalized_source_id.clone(),
            canonical_url: citation.canonical_url.clone(),
            policy_readiness: citation.policy_readiness.clone(),
            confidence: citation.confidence,
            freshness_status: citation.freshness_status.clone(),
        })
        .collect::<Vec<_>>();
    let uncertainty = uncertainty_items(&citations, &source_failures, &contradictions);
    let missing_evidence = missing_evidence_items(&citations, &source_failures);
    let next_research_or_proposal = next_steps(&citations, &source_failures, &contradictions);
    let receipt_references = receipt_references(&citations, warden.memory.memory_id.as_deref());
    let material_fingerprint =
        material_fingerprint(&citations, &source_failures, &contradiction_status);
    let scope = ResearchScope {
        source_policy: "allowlisted_public_web".to_string(),
        max_sources: request
            .source_limit
            .min(MAX_SOURCES)
            .min(policy.max_results),
        max_source_bytes: policy.max_fetch_bytes,
        expires_at_utc: expires_at.to_rfc3339(),
        max_results: policy.max_results,
        max_fetch_bytes: policy.max_fetch_bytes,
        max_tokens: policy.max_tokens,
        max_attempts: policy.max_attempts,
        cooldown_ms: policy.cooldown_ms,
        max_sources_per_domain: policy.max_sources_per_domain,
        retained_preview_volume: policy.retained_preview_volume,
    };
    let evidence_boundaries = citations
        .iter()
        .map(|citation| citation.evidence_boundary.clone())
        .collect::<Vec<_>>();
    let prompt_injection_detected = citations
        .iter()
        .any(|citation| citation.prompt_injection_detected);
    let brief = ResearchBrief {
        schema_version: BRIEF_SCHEMA.to_string(),
        brief_id: brief_id.clone(),
        run_id: request.run_id.clone(),
        node_id: request.node_id.clone(),
        question: request.question.clone(),
        generated_at_utc: generated_at.to_rfc3339(),
        authority: "advisory_research_evidence".to_string(),
        execution_authorized: false,
        warden_provider: warden.report.provider,
        warden_memory_receipt: warden.memory.memory_id,
        summary: summarize(&request.question, &citations, &source_failures),
        contradiction_status,
        contradictions,
        citations,
        source_failures,
        workbench_run_link: format!(
            "arda://workbench/runs/{}?evidence={brief_id}",
            request.run_id
        ),
        scope,
        claims,
        supporting_citation_ids,
        opposing_citation_ids,
        source_quality,
        uncertainty,
        missing_evidence,
        next_research_or_proposal,
        receipt_references,
        material_fingerprint: material_fingerprint.clone(),
        change_status: "material_change".to_string(),
        no_change_receipt_path: None,
        evidence_boundaries,
        prompt_injection_detected,
    };

    if let Some(previous) = previous_brief {
        if previous.material_fingerprint == material_fingerprint
            && !previous_brief_expired(&previous, generated_at)
        {
            let receipt_path =
                no_change_receipt_path(&state.workbench_root, &request.run_id, &brief_id);
            let receipt = NoChangeReceipt {
                schema_version: "arda.workbench.research-no-change.v1".to_string(),
                brief_id: brief_id.clone(),
                run_id: request.run_id.clone(),
                node_id: request.node_id.clone(),
                recorded_at_utc: generated_at.to_rfc3339(),
                material_fingerprint,
                reason:
                    "evidence, evaluation, contradiction state, and freshness state are unchanged"
                        .to_string(),
                evidence_references: brief.receipt_references.clone(),
            };
            write_json_atomic(&receipt_path, &receipt)?;
            let mut unchanged = previous;
            unchanged.change_status = "no_material_change".to_string();
            unchanged.no_change_receipt_path = Some(
                receipt_path
                    .strip_prefix(&state.workbench_root)
                    .unwrap_or(&receipt_path)
                    .to_string_lossy()
                    .to_string(),
            );
            append_evidence_event(
                &store,
                node_id,
                &request.envelope.idempotency_key,
                format!("{brief_id}:no-change"),
                &receipt_path,
                &receipt,
                &state.workbench_root,
            )?;
            return Ok(Json(unchanged));
        }
    }
    write_json_atomic(&brief_path, &brief)?;
    append_evidence_event(
        &store,
        node_id,
        &request.envelope.idempotency_key,
        brief_id,
        &brief_path,
        &brief,
        &state.workbench_root,
    )?;
    Ok(Json(brief))
}

fn validate_request(request: &ResearchBriefRequest) -> Result<(), ApiError> {
    if request.question.trim().is_empty() {
        return Err(ApiError::bad_request("research question cannot be empty"));
    }
    if request.question.len() > 512 {
        return Err(ApiError::bad_request("research question exceeds 512 bytes"));
    }
    if request.source_limit == 0 || request.source_limit > MAX_SOURCES {
        return Err(ApiError::bad_request(format!(
            "source_limit must be between 1 and {MAX_SOURCES}"
        )));
    }
    Ok(())
}

async fn fetch_and_evaluate(
    state: &HarnessState,
    athena_root: &Path,
    request: &ResearchBriefRequest,
    result: WardenResult,
    expires_at: DateTime<Utc>,
    policy: &ResearchBetaPolicy,
) -> Result<BriefCitation, String> {
    let discovered = reqwest::Url::parse(&result.url)
        .map_err(|error| format!("{}: invalid URL: {error}", result.url))?;
    validate_public_url(&discovered)?;
    let response = canonical_fetch(&discovered, state.warden_scout_timeout, policy).await?;
    let canonical = response.url().clone();
    validate_public_url(&canonical)?;
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("{}: canonical body failed: {error}", result.url))?;
    if bytes.len() > policy.max_fetch_bytes {
        return Err(format!(
            "{}: source exceeds {} bytes",
            result.url, policy.max_fetch_bytes
        ));
    }
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let normalized = visible_text(&text);
    if normalized.trim().is_empty() {
        return Err(format!(
            "{}: canonical source has no readable text",
            result.url
        ));
    }
    if normalized.len() > policy.max_tokens.saturating_mul(4) {
        return Err(format!(
            "{}: source exceeds approximate token budget of {}",
            result.url, policy.max_tokens
        ));
    }
    let inspection = inspect_untrusted_content(&normalized);
    let evidence_boundary = inspection.boundary.clone();
    let prompt_injection_detected = inspection.prompt_injection_detected;
    let prompt_injection_signals = inspection.signals;
    let content_sha256 = format!("sha256:{:x}", Sha256::digest(&bytes));
    let canonical_url = normalize_source_identity(&canonical.to_string());
    let fetched_at_utc = Utc::now().to_rfc3339();
    let expires_at_utc = expires_at.to_rfc3339();
    let freshness_status = if expires_at > Utc::now() {
        "fresh"
    } else {
        "expired"
    }
    .to_string();
    let question = request.question.clone();
    let node_context = format!(
        "Workbench run {} node {} explicit research question: {}",
        request.run_id, request.node_id, request.question
    );
    let title = result.title;
    let crawl = CrawlMarkdownResult {
        pipeline_id: String::new(),
        url: canonical_url.clone(),
        filter: "canonical_http_text".to_string(),
        query: Some(question.clone()),
        markdown: normalized.clone(),
        success: true,
        provider: "workbench_canonical_fetch".to_string(),
    };
    let athena_root = athena_root.to_path_buf();
    let discovered_url = discovered.to_string();
    let normalized_source_id = format!("source-{:x}", Sha256::digest(canonical_url.as_bytes()));
    tokio::task::spawn_blocking(move || {
        let store = AthenaStore::new(&athena_root).map_err(|error| error.to_string())?;
        let record = store
            .ingest(&canonical_url, "workbench_research", &node_context)
            .map_err(|error| error.to_string())?;
        let receipt = store
            .record_crawl_capture(
                &canonical_url,
                "workbench_research",
                &node_context,
                "workbench://canonical-http-fetch",
                &CrawlMarkdownResult {
                    pipeline_id: record.pipeline_id.clone(),
                    ..crawl
                },
            )
            .map_err(|error| error.to_string())?;
        let deep = store
            .deep_analyze(&record.id)
            .map_err(|error| error.to_string())?;
        let excerpt = citation_excerpt(&normalized, &question);
        let evaluation_digest = digest_value(&serde_json::json!({
            "policy_readiness": deep.data.policy_readiness,
            "confidence": deep.data.confidence,
            "triad_passed": deep.data.triad_analysis.passed,
            "extraction_status": deep.data.extraction_status,
        }));
        let expiry_digest = digest_value(&serde_json::json!({
            "expires_at_utc": expires_at_utc,
            "freshness_status": freshness_status,
        }));
        Ok(BriefCitation {
            citation_id: format!("cite-{}", &record.id[..record.id.len().min(12)]),
            title,
            discovered_url,
            canonical_url,
            content_sha256,
            stance: classify_stance(&excerpt),
            excerpt,
            varda_source_id: record.id.clone(),
            varda_pipeline_id: record.pipeline_id.clone(),
            crawl_receipt_path: receipt.artifact_path.clone(),
            policy_readiness: deep.data.policy_readiness,
            confidence: deep.data.confidence,
            normalized_source_id,
            fetched_at_utc,
            expires_at_utc,
            freshness_status,
            evaluation_digest,
            expiry_digest,
            receipt_references: vec![
                format!("varda:source:{}", record.id),
                format!("varda:pipeline:{}", record.pipeline_id),
                format!("varda:crawl:{}", receipt.artifact_path),
            ],
            evidence_boundary,
            prompt_injection_detected,
            prompt_injection_signals,
        })
    })
    .await
    .map_err(|error| format!("{}: Varda worker failed: {error}", result.url))?
    .map_err(|error: String| format!("{}: Varda evaluation failed: {error}", result.url))
}

async fn canonical_fetch(
    initial: &reqwest::Url,
    timeout: std::time::Duration,
    policy: &ResearchBetaPolicy,
) -> Result<reqwest::Response, String> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| format!("failed to build canonical fetch client: {error}"))?;
    let mut url = initial.clone();
    for _ in 0..=5 {
        validate_public_url(&url)?;
        validate_public_resolution(&url).await?;
        let mut response = None;
        let mut last_error = None;
        for attempt in 0..policy.max_attempts {
            match client
                .get(url.clone())
                .timeout(timeout)
                .header(
                    reqwest::header::ACCEPT,
                    "text/html,text/plain,application/xhtml+xml",
                )
                .send()
                .await
            {
                Ok(candidate)
                    if candidate.status().is_server_error()
                        || candidate.status() == reqwest::StatusCode::TOO_MANY_REQUESTS =>
                {
                    last_error = Some(format!("{}: transient status {}", url, candidate.status()));
                    if attempt + 1 < policy.max_attempts {
                        tokio::time::sleep(std::time::Duration::from_millis(policy.cooldown_ms))
                            .await;
                    }
                }
                Ok(candidate) => {
                    response = Some(candidate);
                    break;
                }
                Err(error) => {
                    last_error = Some(format!("{url}: canonical fetch failed: {error}"));
                    if attempt + 1 < policy.max_attempts {
                        tokio::time::sleep(std::time::Duration::from_millis(policy.cooldown_ms))
                            .await;
                    }
                }
            }
        }
        let response = response.ok_or_else(|| {
            last_error.unwrap_or_else(|| format!("{url}: canonical fetch exhausted retry budget"))
        })?;
        if response.status().is_redirection() {
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| format!("{url}: redirect omitted a valid Location header"))?;
            url = url
                .join(location)
                .map_err(|error| format!("{url}: invalid redirect target: {error}"))?;
            continue;
        }
        return response
            .error_for_status()
            .map_err(|error| format!("{url}: canonical fetch returned failure: {error}"));
    }
    Err(format!("{initial}: canonical fetch exceeded 5 redirects"))
}

async fn validate_public_resolution(url: &reqwest::Url) -> Result<(), String> {
    let host = url
        .host_str()
        .ok_or_else(|| format!("{url}: URL has no host"))?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| format!("{url}: URL has no resolvable port"))?;
    let addresses: Vec<_> = tokio::net::lookup_host((host, port))
        .await
        .map_err(|error| format!("{url}: DNS resolution failed: {error}"))?
        .collect();
    if addresses.is_empty() {
        return Err(format!("{url}: DNS resolution returned no addresses"));
    }
    if addresses.iter().any(|address| {
        let ip = address.ip();
        ip.is_loopback() || ip.is_unspecified() || ip.is_multicast() || is_private_ip(ip)
    }) {
        return Err(format!("{url}: DNS resolved to a private/local address"));
    }
    Ok(())
}

fn validate_public_url(url: &reqwest::Url) -> Result<(), String> {
    if !matches!(url.scheme(), "http" | "https") {
        return Err(format!("{}: unsupported URL scheme", url));
    }
    let host = url
        .host_str()
        .ok_or_else(|| format!("{}: URL has no host", url))?;
    let blocked_name = host.eq_ignore_ascii_case("localhost")
        || host.ends_with(".local")
        || host.ends_with(".internal");
    let blocked_ip = host.parse::<std::net::IpAddr>().is_ok_and(|ip| {
        ip.is_loopback() || ip.is_unspecified() || ip.is_multicast() || is_private_ip(ip)
    });
    if blocked_name || blocked_ip {
        return Err(format!("{}: private/local canonical target rejected", url));
    }
    Ok(())
}

fn is_private_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(ip) => ip.is_private() || ip.is_link_local(),
        std::net::IpAddr::V6(ip) => ip.is_unique_local() || ip.is_unicast_link_local(),
    }
}

fn visible_text(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut in_tag = false;
    for ch in raw.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn citation_excerpt(text: &str, question: &str) -> String {
    let lowered = text.to_ascii_lowercase();
    let start = question
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|token| token.len() >= 5)
        .find_map(|token| lowered.find(&token.to_ascii_lowercase()))
        .unwrap_or(0)
        .saturating_sub(120);
    let end = (start + 480).min(text.len());
    let mut start = start;
    while start < end && !text.is_char_boundary(start) {
        start += 1;
    }
    let mut end = end;
    while end > start && !text.is_char_boundary(end) {
        end -= 1;
    }
    text[start..end].trim().to_string()
}

fn classify_stance(excerpt: &str) -> String {
    let lower = excerpt.to_ascii_lowercase();
    if [
        " however ",
        " contrary ",
        " does not ",
        " cannot ",
        " failed ",
        " risk ",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    {
        "opposing_or_cautionary".to_string()
    } else {
        "supporting_or_contextual".to_string()
    }
}

fn contradiction_assessment(citations: &[BriefCitation]) -> (String, Vec<String>) {
    let supporting = citations
        .iter()
        .filter(|citation| citation.stance == "supporting_or_contextual")
        .count();
    let cautionary = citations.len().saturating_sub(supporting);
    if citations.is_empty() {
        (
            "no_reliable_evidence_found".to_string(),
            vec![
                "No fetched source completed canonical retrieval and Varda evaluation.".to_string(),
            ],
        )
    } else if supporting > 0 && cautionary > 0 {
        (
            "contradictory_or_cautionary_evidence_requires_operator_review".to_string(),
            vec![format!(
                "The bounded source set contains {supporting} supporting/contextual and {cautionary} opposing/cautionary excerpt(s); no claim was flattened into execution authority."
            )],
        )
    } else {
        (
            "no_contradiction_detected_in_bounded_evidence".to_string(),
            Vec::new(),
        )
    }
}

fn summarize(question: &str, citations: &[BriefCitation], failures: &[String]) -> String {
    if citations.is_empty() {
        return format!(
            "No reliable evidence was found for the explicit question “{question}”. {} bounded source attempt(s) failed before a cited claim could be made; do not infer a factual answer from search previews.",
            failures.len()
        );
    }
    format!(
        "For the explicit question “{}”, Warden discovered {} source(s); cited evidence IDs [{}] link every factual claim below to fetched canonical content and Varda evaluation. {} source attempt(s) remained partial or failed. Review quality, freshness, contradictions, and uncertainty before changing the Workbench plan.",
        question,
        citations.len(),
        citations
            .iter()
            .map(|citation| citation.citation_id.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        failures.len()
    )
}

fn claims_from_citations(citations: &[BriefCitation]) -> Vec<ResearchClaim> {
    citations
        .iter()
        .enumerate()
        .map(|(index, citation)| ResearchClaim {
            claim_id: format!("claim-{}", index + 1),
            claim: citation.excerpt.clone(),
            evidence_citation_ids: vec![citation.citation_id.clone()],
            stance: citation.stance.clone(),
            uncertainty: if citation.confidence < 0.7 {
                "lower_confidence_requires_operator_review".to_string()
            } else {
                "bounded_to_cited_excerpt_and_evaluation".to_string()
            },
        })
        .collect()
}

fn citation_ids_with_stance(citations: &[BriefCitation], stance: &str) -> Vec<String> {
    citations
        .iter()
        .filter(|citation| citation.stance == stance)
        .map(|citation| citation.citation_id.clone())
        .collect()
}

fn uncertainty_items(
    citations: &[BriefCitation],
    failures: &[String],
    contradictions: &[String],
) -> Vec<String> {
    let mut items = Vec::new();
    if citations.iter().any(|citation| citation.confidence < 0.7) {
        items.push("At least one Varda evaluation has confidence below 0.7.".to_string());
    }
    if !failures.is_empty() {
        items.push(format!(
            "{} bounded source attempt(s) failed or were rejected.",
            failures.len()
        ));
    }
    if !contradictions.is_empty() {
        items.push("Contradiction or missing-evidence review remains unresolved.".to_string());
    }
    if items.is_empty() {
        items.push(
            "No additional uncertainty signal was detected within the bounded evidence set."
                .to_string(),
        );
    }
    items
}

fn missing_evidence_items(citations: &[BriefCitation], failures: &[String]) -> Vec<String> {
    let mut items = Vec::new();
    if citations.is_empty() {
        items.push(
            "At least one fetched canonical source is required before asserting a factual claim."
                .to_string(),
        );
    }
    if !failures.is_empty() {
        items.push(
            "Retry or replace the disclosed failed sources before treating the brief as complete."
                .to_string(),
        );
    }
    items
}

fn next_steps(
    citations: &[BriefCitation],
    failures: &[String],
    contradictions: &[String],
) -> Vec<ResearchNextStep> {
    let mut steps = Vec::new();
    if !failures.is_empty() || citations.is_empty() {
        steps.push(ResearchNextStep {
            kind: "next_research".to_string(),
            action: "Retry disclosed failures or refine the question/source policy; no proposal is authorized by this brief.".to_string(),
            authority: "operator_advisory_only".to_string(),
        });
    }
    if !contradictions.is_empty() {
        steps.push(ResearchNextStep {
            kind: "operator_review".to_string(),
            action:
                "Compare supporting and opposing citations before requesting any governed proposal."
                    .to_string(),
            authority: "operator_advisory_only".to_string(),
        });
    }
    if steps.is_empty() {
        steps.push(ResearchNextStep {
            kind: "proposal_candidate".to_string(),
            action: "Aulë may consider a separate governed proposal; this brief creates no executable work.".to_string(),
            authority: "governed_backend_only".to_string(),
        });
    }
    steps
}

fn receipt_references(citations: &[BriefCitation], warden_memory: Option<&str>) -> Vec<String> {
    let mut refs = BTreeSet::new();
    if let Some(memory) = warden_memory {
        refs.insert(format!("warden:memory:{memory}"));
    }
    for citation in citations {
        refs.extend(citation.receipt_references.iter().cloned());
        refs.insert(format!("varda:content:{}", citation.content_sha256));
        refs.insert(format!("varda:evaluation:{}", citation.evaluation_digest));
        refs.insert(format!("varda:expiry:{}", citation.expiry_digest));
    }
    refs.into_iter().collect()
}

fn material_fingerprint(
    citations: &[BriefCitation],
    failures: &[String],
    contradiction_status: &str,
) -> String {
    let mut evidence = citations
        .iter()
        .map(|citation| {
            serde_json::json!({
                "source_id": citation.normalized_source_id,
                "content_sha256": citation.content_sha256,
                "evaluation_digest": citation.evaluation_digest,
                "freshness_status": citation.freshness_status,
                "stance": citation.stance,
            })
        })
        .collect::<Vec<_>>();
    evidence.sort_by_key(|value| value.to_string());
    let mut failures = failures.to_vec();
    failures.sort();
    digest_value(&serde_json::json!({
        "evidence": evidence,
        "failures": failures,
        "contradiction_status": contradiction_status,
    }))
}

fn normalize_source_identity(raw: &str) -> String {
    let Ok(mut url) = reqwest::Url::parse(raw) else {
        return raw.to_string();
    };
    url.set_fragment(None);
    let mut query = url.query_pairs().into_owned().collect::<Vec<_>>();
    query.sort();
    if query.is_empty() {
        url.set_query(None);
    } else {
        url.query_pairs_mut().clear().extend_pairs(
            query
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str())),
        );
    }
    url.to_string()
}

fn digest_value(value: &serde_json::Value) -> String {
    format!("sha256:{:x}", Sha256::digest(value.to_string().as_bytes()))
}

#[derive(Debug, Serialize)]
struct NoChangeReceipt {
    schema_version: String,
    brief_id: String,
    run_id: String,
    node_id: String,
    recorded_at_utc: String,
    material_fingerprint: String,
    reason: String,
    evidence_references: Vec<String>,
}

fn previous_brief_expired(brief: &ResearchBrief, now: DateTime<Utc>) -> bool {
    brief.citations.iter().any(|citation| {
        DateTime::parse_from_rfc3339(&citation.expires_at_utc)
            .map(|expires| expires.with_timezone(&Utc) <= now)
            .unwrap_or(true)
    })
}

fn no_change_receipt_path(root: &Path, run_id: &str, brief_id: &str) -> std::path::PathBuf {
    root.join("data/runs")
        .join(run_id)
        .join("evidence")
        .join(format!("{brief_id}.no-change.json"))
}

fn append_evidence_event(
    store: &RunStore,
    node_id: NodeId,
    idempotency_key: &str,
    evidence_id: String,
    evidence_path: &Path,
    value: &impl Serialize,
    root: &Path,
) -> Result<(), ApiError> {
    store
        .append(RunEventDraft {
            node_id,
            idempotency_key: idempotency_key.to_string(),
            kind: RunEventKind::EvidenceLinked {
                evidence_id,
                evidence_path: evidence_path
                    .strip_prefix(root)
                    .unwrap_or(evidence_path)
                    .to_string_lossy()
                    .to_string(),
                authority: "advisory_research_evidence".to_string(),
            },
            receipt_digest: Some(digest_json(value)?),
        })
        .map(|_| ())
        .map_err(store_error)
}

fn stable_brief_id(run_id: &str, node_id: &str, question: &str) -> String {
    let digest = Sha256::digest(format!("{run_id}\0{node_id}\0{}", question.trim()).as_bytes());
    format!("research-{:x}", digest)[..25].to_string()
}

fn brief_path(root: &Path, run_id: &str, brief_id: &str) -> std::path::PathBuf {
    root.join("data/runs")
        .join(run_id)
        .join("evidence")
        .join(format!("{brief_id}.json"))
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), ApiError> {
    let parent = path
        .parent()
        .ok_or_else(|| ApiError::internal("research brief path has no parent"))?;
    fs::create_dir_all(parent).map_err(|error| {
        ApiError::internal(format!("failed to create brief directory: {error}"))
    })?;
    let temporary = path.with_extension(format!("tmp.{}", std::process::id()));
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| ApiError::internal(format!("failed to serialize brief: {error}")))?;
    fs::write(&temporary, bytes)
        .and_then(|_| fs::rename(&temporary, path))
        .map_err(|error| ApiError::internal(format!("failed to write research brief: {error}")))
}

fn digest_json(value: &impl Serialize) -> Result<String, ApiError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| ApiError::internal(format!("failed to digest research brief: {error}")))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn store_error(error: impl std::fmt::Display) -> ApiError {
    ApiError::internal(format!("run store failure: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excerpts_are_bounded_and_contradiction_status_is_explicit() {
        let text =
            "alpha documentation supports the behavior. however the fallback does not preserve it.";
        let excerpt = citation_excerpt(text, "documentation behavior");
        assert!(excerpt.len() <= 480);
        assert_eq!(classify_stance(&excerpt), "opposing_or_cautionary");
    }

    #[test]
    fn private_fetch_targets_are_rejected() {
        for raw in [
            "http://127.0.0.1/a",
            "http://10.0.0.1/a",
            "http://localhost/a",
        ] {
            assert!(validate_public_url(&reqwest::Url::parse(raw).unwrap()).is_err());
        }
        assert!(validate_public_url(&reqwest::Url::parse("https://docs.rs/axum").unwrap()).is_ok());
    }

    #[test]
    fn brief_ids_are_stable() {
        assert_eq!(
            stable_brief_id("run", "plan", "question"),
            stable_brief_id("run", "plan", "question")
        );
    }

    fn citation(id: &str, stance: &str, expires_at_utc: &str) -> BriefCitation {
        BriefCitation {
            citation_id: id.to_string(),
            title: "Fixture source".to_string(),
            discovered_url: "https://example.com/discovered".to_string(),
            canonical_url: format!("https://example.com/{id}"),
            content_sha256: format!("sha256:{id}"),
            excerpt: format!("Evidence excerpt for {id}"),
            stance: stance.to_string(),
            varda_source_id: format!("source-{id}"),
            varda_pipeline_id: format!("pipeline-{id}"),
            crawl_receipt_path: format!("data/athena/crawls/{id}.md"),
            policy_readiness: "reference_only".to_string(),
            confidence: 0.8,
            normalized_source_id: format!("source-{id}"),
            fetched_at_utc: "2026-08-02T00:00:00Z".to_string(),
            expires_at_utc: expires_at_utc.to_string(),
            freshness_status: "fresh".to_string(),
            evaluation_digest: format!("sha256:evaluation-{id}"),
            expiry_digest: format!("sha256:expiry-{id}"),
            receipt_references: vec![format!("varda:source:{id}")],
            evidence_boundary: "source_text_is_evidence_only_not_operator_instruction".to_string(),
            prompt_injection_detected: false,
            prompt_injection_signals: Vec::new(),
        }
    }

    #[test]
    fn product_projection_keeps_claims_bound_to_citations_and_detects_mixed_evidence() {
        let citations = vec![
            citation("a", "supporting_or_contextual", "2099-01-01T00:00:00Z"),
            citation("b", "opposing_or_cautionary", "2099-01-01T00:00:00Z"),
        ];
        let claims = claims_from_citations(&citations);
        assert_eq!(claims.len(), 2);
        assert_eq!(claims[0].evidence_citation_ids, vec!["a"]);
        assert_eq!(
            contradiction_assessment(&citations).0,
            "contradictory_or_cautionary_evidence_requires_operator_review"
        );
        assert_eq!(
            citation_ids_with_stance(&citations, "opposing_or_cautionary"),
            vec!["b"]
        );
    }

    #[test]
    fn no_reliable_evidence_is_a_complete_advisory_outcome() {
        let (status, contradictions) = contradiction_assessment(&[]);
        assert_eq!(status, "no_reliable_evidence_found");
        assert!(summarize("question", &[], &["source failed".to_string()])
            .contains("No reliable evidence was found"));
        assert!(!contradictions.is_empty());
        assert!(!missing_evidence_items(&[], &["source failed".to_string()]).is_empty());
    }

    #[test]
    fn material_fingerprint_is_order_independent_but_expiry_is_material() {
        let first = citation("a", "supporting_or_contextual", "2099-01-01T00:00:00Z");
        let second = citation("b", "opposing_or_cautionary", "2099-01-01T00:00:00Z");
        assert_eq!(
            material_fingerprint(&[first.clone(), second.clone()], &[], "mixed"),
            material_fingerprint(&[second, first], &[], "mixed")
        );
        let old = ResearchBrief {
            schema_version: BRIEF_SCHEMA.to_string(),
            brief_id: "brief".to_string(),
            run_id: "run".to_string(),
            node_id: "node".to_string(),
            question: "question".to_string(),
            generated_at_utc: "2026-08-02T00:00:00Z".to_string(),
            authority: "advisory_research_evidence".to_string(),
            execution_authorized: false,
            warden_provider: "fixture".to_string(),
            warden_memory_receipt: None,
            summary: "summary".to_string(),
            contradiction_status: "mixed".to_string(),
            contradictions: Vec::new(),
            citations: vec![citation(
                "old",
                "supporting_or_contextual",
                "2020-01-01T00:00:00Z",
            )],
            source_failures: Vec::new(),
            workbench_run_link: "arda://fixture".to_string(),
            scope: ResearchScope::default(),
            claims: Vec::new(),
            supporting_citation_ids: Vec::new(),
            opposing_citation_ids: Vec::new(),
            source_quality: Vec::new(),
            uncertainty: Vec::new(),
            missing_evidence: Vec::new(),
            next_research_or_proposal: Vec::new(),
            receipt_references: Vec::new(),
            material_fingerprint: "fingerprint".to_string(),
            change_status: "material_change".to_string(),
            no_change_receipt_path: None,
            evidence_boundaries: Vec::new(),
            prompt_injection_detected: false,
        };
        assert!(previous_brief_expired(&old, Utc::now()));

        let mut legacy = serde_json::to_value(&old).expect("serialize fixture brief");
        for field in [
            "scope",
            "claims",
            "supporting_citation_ids",
            "opposing_citation_ids",
            "source_quality",
            "uncertainty",
            "missing_evidence",
            "next_research_or_proposal",
            "receipt_references",
            "material_fingerprint",
            "change_status",
            "no_change_receipt_path",
        ] {
            legacy.as_object_mut().expect("brief object").remove(field);
        }
        let decoded: ResearchBrief =
            serde_json::from_value(legacy).expect("legacy brief remains readable");
        assert!(decoded.claims.is_empty());
    }
}
