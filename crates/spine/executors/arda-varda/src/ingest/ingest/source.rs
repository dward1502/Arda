// sigil: REPAIR
//
// Source-analysis helpers: canonical input normalization, source typing,
// shallow-analysis construction, graph-token normalization, and the small
// deterministic scoring helpers used during ingest and deep analysis.

use sha2::{Digest, Sha256};

use super::{github, scholarly, GithubMetadata, ScholarlyMetadata, ShallowAnalysis, SourceType};

pub(super) fn source_id_from_input(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let hash = hasher.finalize();
    let hex = hash.iter().map(|b| format!("{b:02x}")).collect::<String>();
    format!("src_{}", &hex[..8])
}

pub(super) fn canonicalize_ingest_input(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return canonicalize_url(trimmed);
    }
    trimmed.to_string()
}

fn canonicalize_url(raw: &str) -> String {
    let mut value = raw.trim().to_string();
    if let Some(hash_idx) = value.find('#') {
        value.truncate(hash_idx);
    }
    while value.ends_with('/') {
        value.pop();
    }

    let Some((scheme, rest)) = value.split_once("://") else {
        return value;
    };

    let mut host_end = rest.len();
    for sep in ['/', '?'] {
        if let Some(idx) = rest.find(sep) {
            host_end = host_end.min(idx);
        }
    }
    let host = rest[..host_end].to_ascii_lowercase();
    let tail = &rest[host_end..];
    let scheme = scheme.to_ascii_lowercase();

    let mut canonical = format!("{scheme}://{host}{tail}");
    if host == "arxiv.org" {
        canonical = canonical
            .replace("/pdf/", "/abs/")
            .trim_end_matches(".pdf")
            .to_string();
    }
    canonical
}

pub(super) fn normalize_graph_token(input: &str) -> String {
    input
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

pub(super) fn classify_source(input: &str) -> SourceType {
    let lower = input.to_ascii_lowercase();
    if lower.starts_with("x-bookmark:") {
        return SourceType::XBookmark;
    }
    if is_x_post(&lower) {
        return SourceType::XPost;
    }
    if is_chat_export(&lower) {
        return SourceType::ChatExport;
    }
    if lower.contains("github.com") && lower.split('/').count() >= 5 {
        return SourceType::GithubRepo;
    }
    if lower.contains("raw.githubusercontent.com") {
        return SourceType::GithubFile;
    }
    if lower.contains("arxiv.org") || lower.contains("doi.org") {
        return SourceType::ScholarlyLink;
    }
    if lower.contains("docs.") || lower.contains("rfc") || lower.contains("readthedocs") {
        return SourceType::Documentation;
    }
    if lower.contains(".gov") {
        return SourceType::GovernmentDoc;
    }
    if lower.ends_with(".pdf") {
        return SourceType::PdfDocument;
    }
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return SourceType::NewsArticle;
    }
    if lower.contains("fn ") || lower.contains("class ") || lower.contains("```") {
        return SourceType::CodeSnippet;
    }
    SourceType::RawNote
}

fn is_x_post(lower: &str) -> bool {
    let on_x_host = lower.contains("://x.com/")
        || lower.contains("://twitter.com/")
        || lower.contains("://www.x.com/")
        || lower.contains("://www.twitter.com/")
        || lower.contains("://mobile.twitter.com/")
        || lower.contains("://nitter.net/");
    on_x_host && (lower.contains("/status/") || lower.contains("/i/web/status/"))
}

fn is_chat_export(lower: &str) -> bool {
    if lower.starts_with("chat-export:")
        || lower.contains("://chat.openai.com/share/")
        || lower.contains("://chatgpt.com/share/")
        || lower.contains("://claude.ai/share/")
        || lower.contains("://claude.ai/chat/")
    {
        return true;
    }
    let head: String = lower.lines().take(6).collect::<Vec<_>>().join("\n");
    let conversation_markers = [
        "chatgpt conversation",
        "claude conversation",
        "**you:**",
        "**assistant:**",
        "user:\n",
        "human:\n",
    ];
    conversation_markers.iter().any(|m| head.contains(m))
}

pub(super) fn extract_url(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return Some(trimmed.to_string());
    }
    None
}

pub(super) fn build_shallow_analysis(
    input: &str,
    source_type: &SourceType,
    deduplicated: bool,
    url: Option<&str>,
) -> ShallowAnalysis {
    let scholarly_metadata = if !deduplicated && matches!(source_type, SourceType::ScholarlyLink) {
        url.and_then(scholarly::fetch_scholarly_metadata)
    } else {
        None
    };
    let github_metadata = if !deduplicated
        && matches!(source_type, SourceType::GithubRepo | SourceType::GithubFile)
    {
        url.and_then(github::fetch_github_metadata)
    } else {
        None
    };

    let title = if let Some(meta) = &scholarly_metadata {
        meta.paper_title.clone()
    } else if let Some(meta) = &github_metadata {
        match &meta.description {
            Some(desc) if !desc.trim().is_empty() => {
                format!(
                    "{} — {}",
                    meta.full_name,
                    desc.chars().take(100).collect::<String>()
                )
            }
            _ => meta.full_name.clone(),
        }
    } else {
        input
            .lines()
            .next()
            .unwrap_or("untitled source")
            .chars()
            .take(80)
            .collect()
    };

    let summary = if deduplicated {
        "Source already present in Athena books; returning existing reference.".to_string()
    } else if let Some(meta) = &scholarly_metadata {
        meta.abstract_text.clone()
    } else if let Some(meta) = &github_metadata {
        github_summary(meta)
    } else {
        format!("Initial shallow ingest completed for {:?}.", source_type)
    };

    let language = github_metadata
        .as_ref()
        .and_then(|m| m.primary_language.clone())
        .map(|s| s.to_ascii_lowercase())
        .or_else(|| detect_language(input));

    let key_dependencies = github_metadata
        .as_ref()
        .map(|m| m.key_dependencies.clone())
        .unwrap_or_default();

    let license = github_metadata.as_ref().and_then(|m| m.license.clone());

    let components_available = github_metadata
        .as_ref()
        .map(github_components)
        .unwrap_or_default();

    let mut relevance_tags = infer_tags(input, source_type, scholarly_metadata.as_ref());
    if let Some(meta) = &github_metadata {
        for topic in &meta.topics {
            relevance_tags.push(topic.to_ascii_lowercase());
        }
        if let Some(lang) = &meta.primary_language {
            relevance_tags.push(lang.to_ascii_lowercase());
        }
        if let Some(kind) = &meta.manifest_kind {
            relevance_tags.push(format!("manifest_{kind}"));
        }
    }
    relevance_tags.sort();
    relevance_tags.dedup();

    ShallowAnalysis {
        title,
        summary,
        language,
        key_dependencies,
        relevance_tags,
        license,
        components_available,
        reuse_potential: None,
        deep_analysis_recommended: !deduplicated,
        deep_analysis_reason: if deduplicated {
            "Deduplicated source; deep analysis already pending or completed.".to_string()
        } else {
            "New source ingested; deep analysis should be scheduled.".to_string()
        },
        scholarly_metadata,
        github_metadata,
    }
}

fn github_summary(meta: &GithubMetadata) -> String {
    let description = meta
        .description
        .clone()
        .unwrap_or_else(|| "No repository description provided.".to_string());
    let mut parts = vec![description];
    if let Some(lang) = &meta.primary_language {
        parts.push(format!("Primary language: {lang}."));
    }
    if let Some(lic) = &meta.license {
        parts.push(format!("License: {lic}."));
    }
    if let (Some(stars), Some(pushed)) = (meta.stargazers_count, meta.pushed_at.as_ref()) {
        parts.push(format!("Stars: {stars}; last pushed {pushed}."));
    } else if let Some(stars) = meta.stargazers_count {
        parts.push(format!("Stars: {stars}."));
    }
    if !meta.key_dependencies.is_empty() {
        let preview = meta
            .key_dependencies
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let more = if meta.key_dependencies.len() > 8 {
            format!(" (+{} more)", meta.key_dependencies.len() - 8)
        } else {
            String::new()
        };
        parts.push(format!("Top deps: {preview}{more}."));
    }
    if let Some(readme) = &meta.readme_excerpt {
        let snippet: String = readme.chars().take(400).collect();
        parts.push(format!("README excerpt: {}", snippet.trim()));
    }
    parts.join(" ")
}

fn github_components(meta: &GithubMetadata) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(kind) = &meta.manifest_kind {
        out.push(format!("manifest:{kind}"));
    }
    for topic in &meta.topics {
        out.push(format!("topic:{topic}"));
    }
    out
}

fn detect_language(input: &str) -> Option<String> {
    let lower = input.to_ascii_lowercase();
    if lower.contains(".rs") || lower.contains("cargo.toml") || lower.contains(" fn ") {
        return Some("rust".to_string());
    }
    if lower.contains(".py") || lower.contains("def ") {
        return Some("python".to_string());
    }
    None
}

pub(super) fn infer_tags(
    input: &str,
    source_type: &SourceType,
    scholarly_metadata: Option<&ScholarlyMetadata>,
) -> Vec<String> {
    let mut tags = vec![format!("{source_type:?}").to_ascii_lowercase()];
    let lower = input.to_ascii_lowercase();
    for candidate in [
        "rust",
        "python",
        "governance",
        "security",
        "docs",
        "api",
        "research",
    ] {
        if lower.contains(candidate) {
            tags.push(candidate.to_string());
        }
    }
    match source_type {
        SourceType::XBookmark => {
            tags.push("social".to_string());
            tags.push("x".to_string());
            tags.push("bookmark".to_string());
        }
        SourceType::XPost => {
            tags.push("social".to_string());
            tags.push("x".to_string());
        }
        SourceType::ChatExport => {
            tags.push("conversation".to_string());
            if lower.contains("chatgpt") || lower.contains("openai") {
                tags.push("chatgpt".to_string());
            }
            if lower.contains("claude") {
                tags.push("claude".to_string());
            }
        }
        _ => {}
    }
    if let Some(meta) = scholarly_metadata {
        tags.push("research".to_string());
        for subject in &meta.subjects {
            tags.push(
                subject
                    .to_ascii_lowercase()
                    .replace([' ', '.', ':', '/', '-'], "_"),
            );
        }
        let abstract_lower = meta.abstract_text.to_ascii_lowercase();
        for candidate in [
            "terminal",
            "agent",
            "scaffolding",
            "harness",
            "context",
            "memory",
            "routing",
            "safety",
            "tool",
        ] {
            if abstract_lower.contains(candidate) {
                tags.push(candidate.to_string());
            }
        }
    }
    tags.sort();
    tags.dedup();
    tags
}

pub(super) fn estimate_joule_cost(shallow: &ShallowAnalysis) -> f64 {
    8.0 + shallow.relevance_tags.len() as f64 * 1.5 + shallow.summary.len() as f64 / 200.0
}

pub(super) fn love_equation_from_tags(tags: &[String]) -> (f64, String) {
    let mut score: f64 = 0.6;
    let mut rationale = "Baseline alignment from successful ingest.".to_string();
    if tags.iter().any(|t| t == "governance") {
        score += 0.15;
        rationale = "High alignment due to governance relevance.".to_string();
    } else if tags.iter().any(|t| t == "security") {
        score += 0.1;
        rationale = "Strong alignment due to security relevance.".to_string();
    } else if tags.iter().any(|t| t == "research") {
        score += 0.08;
        rationale = "Moderate alignment due to research relevance.".to_string();
    }
    (score.min(1.0), rationale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_source_detects_x_posts() {
        assert!(matches!(
            classify_source("https://x.com/some_user/status/1234567890"),
            SourceType::XPost
        ));
        assert!(matches!(
            classify_source("https://twitter.com/some_user/status/1234567890"),
            SourceType::XPost
        ));
        assert!(matches!(
            classify_source("https://nitter.net/some_user/status/1234567890"),
            SourceType::XPost
        ));
        assert!(!matches!(
            classify_source("https://x.com/some_user"),
            SourceType::XPost
        ));
    }

    #[test]
    fn classify_source_detects_chat_exports_by_url_and_marker() {
        assert!(matches!(
            classify_source("https://chat.openai.com/share/abc-123"),
            SourceType::ChatExport
        ));
        assert!(matches!(
            classify_source("https://claude.ai/share/xyz"),
            SourceType::ChatExport
        ));
        assert!(matches!(
            classify_source("# ChatGPT Conversation\n\nUser: hello\n"),
            SourceType::ChatExport
        ));
        assert!(matches!(
            classify_source("**You:** what is rust?\n**Assistant:** a language..."),
            SourceType::ChatExport
        ));
    }

    #[test]
    fn classify_source_does_not_misroute_normal_x_or_news_urls() {
        // x.com profile (no /status/) should not be XPost
        assert!(matches!(
            classify_source("https://x.com/profile_name"),
            SourceType::NewsArticle
        ));
    }

    #[test]
    fn x_post_inputs_are_tagged_as_social_x() {
        let st = SourceType::XPost;
        let tags = infer_tags("https://x.com/u/status/1", &st, None);
        assert!(tags.contains(&"social".to_string()));
        assert!(tags.contains(&"x".to_string()));
        assert!(tags.contains(&"xpost".to_string()));
    }

    #[test]
    fn chat_export_tags_include_provider() {
        let st = SourceType::ChatExport;
        let tags = infer_tags("# ChatGPT Conversation\n\nUser: ...", &st, None);
        assert!(tags.contains(&"conversation".to_string()));
        assert!(tags.contains(&"chatgpt".to_string()));
    }
}
