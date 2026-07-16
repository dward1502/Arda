// sigil: REPAIR
//
// Deep-analysis helpers: scholarly-metadata title/summary projection,
// implementation-brief templates (scholarly and repo-based), and the
// shallow-metadata recovery path used before a deep analysis is scored.

use arda_core::error::Result;

use super::scholarly;
use super::source::{extract_url, infer_tags};
use super::{AthenaStore, BookEntry, ShallowAnalysis, SourceType};

impl AthenaStore {
    pub(super) fn recover_shallow_analysis(
        &self,
        source_id: &str,
        shallow: BookEntry,
    ) -> Result<BookEntry> {
        if shallow.data.scholarly_metadata.is_some() {
            return Ok(shallow);
        }
        let url = self
            .latest_ingest_record(source_id)?
            .and_then(|record| record.url)
            .or_else(|| extract_url(&shallow.data.title));
        let Some(url) = url else {
            return Ok(shallow);
        };
        let Some(metadata) = scholarly::fetch_scholarly_metadata(&url) else {
            return Ok(shallow);
        };

        let mut recovered = shallow;
        recovered.data.title = metadata.paper_title.clone();
        recovered.data.summary = metadata.abstract_text.clone();
        recovered.data.relevance_tags =
            infer_tags(&url, &SourceType::ScholarlyLink, Some(&metadata));
        recovered.data.scholarly_metadata = Some(metadata);
        Ok(recovered)
    }
}

pub(super) fn scholarly_title_for_deep(shallow: &ShallowAnalysis) -> String {
    shallow
        .scholarly_metadata
        .as_ref()
        .map(|meta| meta.paper_title.clone())
        .unwrap_or_else(|| shallow.title.clone())
}

pub(super) fn deep_summary_for_source(shallow: &ShallowAnalysis) -> String {
    if let Some(meta) = &shallow.scholarly_metadata {
        let mut out = format!(
            "{} Authors: {}.",
            meta.abstract_text,
            meta.authors.join(", ")
        );
        if !meta.subjects.is_empty() {
            out.push_str(&format!(" Subjects: {}.", meta.subjects.join(", ")));
        }
        if let Some(comments) = &meta.comments {
            out.push_str(&format!(" Comments: {}.", comments));
        }
        return out;
    }
    format!(
        "{} Deep synthesis generated from deterministic governance scaffold.",
        shallow.summary
    )
}

pub(super) fn implementation_brief_for_source(
    shallow: &ShallowAnalysis,
) -> Option<serde_json::Value> {
    scholarly_implementation_brief(shallow).or_else(|| repo_implementation_brief(shallow))
}

fn scholarly_implementation_brief(shallow: &ShallowAnalysis) -> Option<serde_json::Value> {
    let meta = shallow.scholarly_metadata.as_ref()?;
    let abstract_lower = meta.abstract_text.to_ascii_lowercase();
    let implications = vec![
        if abstract_lower.contains("routing") {
            Some("Add workload-specialized model routing between planning and execution lanes.")
        } else {
            None
        },
        if abstract_lower.contains("memory") {
            Some("Persist project-specific memory across sessions with retrieval hooks during execution.")
        } else {
            None
        },
        if abstract_lower.contains("context") {
            Some("Use context compaction and explicit reminder mechanisms to limit context bloat.")
        } else {
            None
        },
        if abstract_lower.contains("harness") || abstract_lower.contains("safety") {
            Some("Keep command execution behind a harness with explicit safety phases and verification checkpoints.")
        } else {
            None
        },
    ]
    .into_iter()
    .flatten()
    .map(str::to_string)
    .collect::<Vec<_>>();
    let method = if abstract_lower.contains("dual-agent") {
        "Dual-agent separation between planning and execution"
    } else if abstract_lower.contains("compound ai system") {
        "Compound AI system architecture with specialized routing"
    } else {
        "Terminal-native coding-agent architecture with context controls"
    };
    let risks = vec![
        "Context bloat can degrade reasoning quality over long-horizon tasks.",
        "Autonomy without explicit harness controls can create unsafe command execution.",
        "Instruction fade-out across long sessions can erode policy adherence.",
    ];
    Some(serde_json::json!({
        "method_summary": method,
        "risks": risks,
        "implementation_implications": implications,
        "source_url": meta.source_url,
    }))
}

fn repo_implementation_brief(shallow: &ShallowAnalysis) -> Option<serde_json::Value> {
    let title = shallow.title.to_ascii_lowercase();
    let identifier = shallow
        .github_metadata
        .as_ref()
        .map(|m| m.full_name.to_ascii_lowercase())
        .unwrap_or_else(|| title.clone());
    let source_url = shallow
        .github_metadata
        .as_ref()
        .map(|m| m.source_url.clone())
        .unwrap_or_else(|| shallow.title.clone());
    let is_github = shallow.github_metadata.is_some() || title.contains("github.com/");
    if !is_github {
        return None;
    }

    let title = identifier;

    let (method_summary, implications, risks) = if title.contains("d4vinci/scrapling") {
        (
            "Provider-directed sovereign crawl ingestion with a bounded alternative fetch stack",
            vec![
                "Promote Scrapling from shim-backed fetch path into a bounded runtime contract with explicit env and install requirements.",
                "Define provider-order policy so ATHENA can prefer Scrapling without silently bypassing the live crawl4ai lane.",
                "Capture stable receipts and markdown artifacts through the same ATHENA crawl surface used by other providers.",
            ],
            vec![
                "Scrapling cannot become the default until browser and fetcher dependencies are bounded in sovereign runtime surfaces.",
                "Provider-order drift can create inconsistent captures if Scrapling and crawl4ai are not governed by one policy surface.",
            ],
        )
    } else if title.contains("unclecode/crawl4ai") {
        (
            "Containerized sovereign crawl service for continuous ATHENA ingest",
            vec![
                "Keep crawl4ai as the continuously available primary ingest runtime until Scrapling has a bounded runtime contract.",
                "Verify repeated ATHENA crawl captures against external sources and persist receipts as activation evidence.",
                "Route crawl service lifecycle through package runtime activation and steward surfaces instead of ad hoc shell state.",
            ],
            vec![
                "Container health alone is insufficient; activation requires verified ATHENA crawl captures.",
                "Large markdown captures can inflate storage pressure if receipts and artifact retention are not tracked.",
            ],
        )
    } else if title.contains("berriai/litellm") {
        (
            "Local gateway-based provider normalization for CHARON and downstream consumers",
            vec![
                "Keep the LiteLLM gateway live as the normalized routing layer for local and edge model access.",
                "Bind provider-health and model-policy checks to the same package activation surface used by ATHENA and CHARON.",
                "Prefer explicit gateway runtime wrappers over manual process launches so restart posture remains deterministic.",
            ],
            vec![
                "Gateway runtime drift can silently break routing if wrapper dependencies and backend availability diverge.",
                "Unbounded proxy configuration can obscure which sovereign providers are actually serving requests.",
            ],
        )
    } else if title.contains("alexsjones/llmfit") {
        (
            "Decision-support model selection integrated as an active signal rather than a daemon runtime",
            vec![
                "Use llmfit recommendations to tune route heuristics and model-profile policy instead of spawning a separate long-lived service.",
                "Project fit recommendations into CHARON and governor state so routing changes remain auditable.",
            ],
            vec![
                "Treating llmfit as authoritative runtime policy would bypass sovereign routing controls.",
            ],
        )
    } else if title.contains("qwibitai/nanoclaw") {
        (
            "Governed-on-demand edge runtime with bounded transport and auth posture",
            vec![
                "Keep NanoClaw behind explicit edge-target, transport, and auth contracts rather than treating channel assumptions as readiness law.",
                "Promote live edge visibility before changing doctrine or broadening runtime scope.",
            ],
            vec![
                "Channel/auth assumptions can create false readiness if edge visibility and target reachability are not verified.",
            ],
        )
    } else {
        (
            "Repo-backed implementation candidate requiring sovereign contract mapping",
            vec![
                "Promote the source from evidence into a bounded runtime, package, or workflow contract before activation.",
                "Attach implementation work to sovereign state exports so UI and agents consume the same source of truth.",
            ],
            vec![
                "Digesting a GitHub repo is not sufficient for runtime adoption without bounded activation surfaces.",
            ],
        )
    };

    Some(serde_json::json!({
        "method_summary": method_summary,
        "risks": risks,
        "implementation_implications": implications,
        "source_url": source_url,
    }))
}
