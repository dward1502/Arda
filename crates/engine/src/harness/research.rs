//! Explicit-question Warden → Varda research briefs linked to Workbench runs.
//!
//! Warden results are discovery previews only. This boundary fetches canonical
//! source content, records Varda crawl and evaluation receipts, and links the
//! resulting advisory brief to an existing run without changing node state.

use arda_core::run_graph::{NodeId, RunId};
use arda_varda::ingest::{AthenaStore, CrawlMarkdownResult};
use axum::{
    extract::{ConnectInfo, State},
    Json,
};
use chrono::{Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, net::SocketAddr, path::Path};

use crate::runs::{RunEventDraft, RunEventKind, RunStore};

use super::{
    projects::{require_loopback, ApiError, MutationEnvelope, WORKBENCH_MUTATIONS},
    HarnessState,
};

const BRIEF_SCHEMA: &str = "arda.workbench.research-brief.v1";
const MAX_SOURCES: usize = 5;
const MAX_SOURCE_BYTES: usize = 512 * 1024;

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

    let scout_url = state
        .warden_scout_url
        .as_deref()
        .ok_or_else(|| ApiError::internal("Warden scout is not configured"))?;
    let warden: WardenSearchResponse = state
        .client
        .post(format!("{}/search", scout_url.trim_end_matches('/')))
        .timeout(state.warden_scout_timeout)
        .json(&serde_json::json!({
            "query": request.question,
            "limit": request.source_limit.min(MAX_SOURCES),
            "source_policy": "allowlisted_public_web",
            "expires_at": (Utc::now() + ChronoDuration::minutes(15)).to_rfc3339(),
        }))
        .send()
        .await
        .map_err(|error| ApiError::internal(format!("Warden search failed: {error}")))?
        .error_for_status()
        .map_err(|error| ApiError::internal(format!("Warden search returned failure: {error}")))?
        .json()
        .await
        .map_err(|error| ApiError::internal(format!("invalid Warden search response: {error}")))?;

    let athena_root = state.workbench_root.join("data/athena");
    let mut citations = Vec::new();
    let mut source_failures = Vec::new();
    for result in warden
        .report
        .results
        .into_iter()
        .take(request.source_limit.min(MAX_SOURCES))
    {
        match fetch_and_evaluate(&state, &athena_root, &request, result).await {
            Ok(citation) => citations.push(citation),
            Err(error) => source_failures.push(error),
        }
    }
    if citations.is_empty() {
        return Err(ApiError::internal(format!(
            "no Warden result completed canonical fetch and Varda evaluation: {}",
            source_failures.join("; ")
        )));
    }

    let (contradiction_status, contradictions) = contradiction_assessment(&citations);
    let brief = ResearchBrief {
        schema_version: BRIEF_SCHEMA.to_string(),
        brief_id: brief_id.clone(),
        run_id: request.run_id.clone(),
        node_id: request.node_id.clone(),
        question: request.question.clone(),
        generated_at_utc: Utc::now().to_rfc3339(),
        authority: "advisory_research_evidence".to_string(),
        execution_authorized: false,
        warden_provider: warden.report.provider,
        warden_memory_receipt: warden.memory.memory_id,
        summary: summarize(&request.question, &citations),
        contradiction_status,
        contradictions,
        citations,
        source_failures,
        workbench_run_link: format!(
            "arda://workbench/runs/{}?evidence={brief_id}",
            request.run_id
        ),
    };
    write_json_atomic(&brief_path, &brief)?;
    let receipt_digest = digest_json(&brief)?;
    store
        .append(RunEventDraft {
            node_id,
            idempotency_key: request.envelope.idempotency_key,
            kind: RunEventKind::EvidenceLinked {
                evidence_id: brief_id,
                evidence_path: brief_path
                    .strip_prefix(&state.workbench_root)
                    .unwrap_or(&brief_path)
                    .to_string_lossy()
                    .to_string(),
                authority: "advisory_research_evidence".to_string(),
            },
            receipt_digest: Some(receipt_digest),
        })
        .map_err(store_error)?;
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
) -> Result<BriefCitation, String> {
    let discovered = reqwest::Url::parse(&result.url)
        .map_err(|error| format!("{}: invalid URL: {error}", result.url))?;
    validate_public_url(&discovered)?;
    let response = canonical_fetch(&discovered, state.warden_scout_timeout).await?;
    let canonical = response.url().clone();
    validate_public_url(&canonical)?;
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("{}: canonical body failed: {error}", result.url))?;
    if bytes.len() > MAX_SOURCE_BYTES {
        return Err(format!(
            "{}: source exceeds {} bytes",
            result.url, MAX_SOURCE_BYTES
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
    let content_sha256 = format!("sha256:{:x}", Sha256::digest(&bytes));
    let canonical_url = canonical.to_string();
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
        Ok(BriefCitation {
            citation_id: format!("cite-{}", &record.id[..record.id.len().min(12)]),
            title,
            discovered_url,
            canonical_url,
            content_sha256,
            stance: classify_stance(&excerpt),
            excerpt,
            varda_source_id: record.id,
            varda_pipeline_id: record.pipeline_id,
            crawl_receipt_path: receipt.artifact_path,
            policy_readiness: deep.data.policy_readiness,
            confidence: deep.data.confidence,
        })
    })
    .await
    .map_err(|error| format!("{}: Varda worker failed: {error}", result.url))?
    .map_err(|error: String| format!("{}: Varda evaluation failed: {error}", result.url))
}

async fn canonical_fetch(
    initial: &reqwest::Url,
    timeout: std::time::Duration,
) -> Result<reqwest::Response, String> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| format!("failed to build canonical fetch client: {error}"))?;
    let mut url = initial.clone();
    for _ in 0..=5 {
        validate_public_url(&url)?;
        validate_public_resolution(&url).await?;
        let response = client
            .get(url.clone())
            .timeout(timeout)
            .header(
                reqwest::header::ACCEPT,
                "text/html,text/plain,application/xhtml+xml",
            )
            .send()
            .await
            .map_err(|error| format!("{url}: canonical fetch failed: {error}"))?;
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
    let mut out = String::with_capacity(raw.len().min(MAX_SOURCE_BYTES));
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
    if supporting > 0 && cautionary > 0 {
        (
            "mixed_evidence_requires_operator_review".to_string(),
            vec![format!(
                "The bounded source set contains {supporting} supporting/contextual and {cautionary} opposing/cautionary excerpt(s); no claim was flattened into execution authority."
            )],
        )
    } else {
        (
            "no_lexical_contradiction_detected_in_bounded_excerpts".to_string(),
            Vec::new(),
        )
    }
}

fn summarize(question: &str, citations: &[BriefCitation]) -> String {
    format!(
        "For the explicit question “{}”, Warden discovered {} source(s); canonical content was fetched and evaluated by Varda. Review the cited excerpts and Varda readiness fields before changing the Workbench plan.",
        question,
        citations.len()
    )
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
}
