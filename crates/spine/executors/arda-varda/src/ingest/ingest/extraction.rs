// sigil: REPAIR
//
// LLM-driven knowledge extraction. Phase 2 of ATHENA digestion.
// Takes a shallow record (GitHub repo or scholarly link) and asks the
// configured LLM to extract structured concepts, patterns, novel ideas,
// applicability, integration hooks, comparable systems, and risks.
//
// Strict-JSON output via system prompt; raw fallback retained when the
// model returns text we can't parse.

use arda_core::error::Result;
use arda_core::llm::{ChatMessage, ChatRequest, LlmProvider};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::{athena_error, ShallowAnalysis};

const SYSTEM_PROMPT: &str = "You are ATHENA, the knowledge synthesis arm of the Arda autonomous agent system. \
Arda is a Rust workspace of sovereign crates (charon=router, hermes=comm, athena=knowledge, hades=ops, prometheus=planner, plutus=economy, warden=governance, mnemosyne=memory). \
Your job is to extract structured, actionable knowledge from source materials (GitHub repos, research papers, articles). \
Output FORMAT IS NON-NEGOTIABLE: respond with ONE valid JSON object only. \
Your VERY FIRST CHARACTER must be `{` and your VERY LAST CHARACTER must be `}`. \
Do NOT reason out loud. Do NOT plan. Do NOT write a thinking process. Do NOT use markdown code fences. Do NOT add commentary before or after the JSON. \
If a field has nothing useful, return an empty array `[]` or empty string `\"\"` — never null, never omit. \
/no_think";

const MAX_TOKENS: u32 = 4096;
const TEMPERATURE: f64 = 0.2;
const README_INPUT_CHAR_LIMIT: usize = 8_000;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtractedKnowledge {
    #[serde(default)]
    pub concepts: Vec<String>,
    #[serde(default)]
    pub patterns: Vec<String>,
    #[serde(default)]
    pub novel_ideas: Vec<String>,
    #[serde(default)]
    pub applicability_to_arda: String,
    #[serde(default)]
    pub integration_hooks: Vec<String>,
    #[serde(default)]
    pub comparable_systems: Vec<String>,
    #[serde(default)]
    pub risks_or_concerns: Vec<String>,
    #[serde(default)]
    pub confidence_self_report: f64,
    #[serde(default)]
    pub summary_one_paragraph: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_response_excerpt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parse_error: Option<String>,
}

pub(super) async fn extract_knowledge(
    llm: Arc<dyn LlmProvider>,
    shallow: &ShallowAnalysis,
) -> Result<ExtractedKnowledge> {
    let prompt = build_user_prompt(shallow);
    let request = ChatRequest::new(vec![
        ChatMessage::system(SYSTEM_PROMPT),
        ChatMessage::user(prompt),
    ])
    .with_temperature(TEMPERATURE)
    .with_max_tokens(MAX_TOKENS);

    let response = llm
        .chat(request)
        .await
        .map_err(|e| athena_error(format!("extraction LLM call failed: {e}")))?;

    let provider = llm.provider_name().to_string();
    let model = response.model.clone();
    let mut extracted = parse_response(&response.content);
    extracted.model = Some(model);
    extracted.provider = Some(provider);
    Ok(extracted)
}

pub(super) fn build_user_prompt(shallow: &ShallowAnalysis) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(meta) = &shallow.github_metadata {
        parts.push("SOURCE_TYPE: github_repo".to_string());
        parts.push(format!("REPO: {}", meta.full_name));
        if let Some(desc) = &meta.description {
            parts.push(format!("DESCRIPTION: {desc}"));
        }
        if let Some(lang) = &meta.primary_language {
            parts.push(format!("PRIMARY_LANGUAGE: {lang}"));
        }
        if let Some(license) = &meta.license {
            parts.push(format!("LICENSE: {license}"));
        }
        if !meta.topics.is_empty() {
            parts.push(format!("TOPICS: {}", meta.topics.join(", ")));
        }
        if !meta.key_dependencies.is_empty() {
            let deps_preview = meta
                .key_dependencies
                .iter()
                .take(30)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            parts.push(format!("KEY_DEPENDENCIES: {deps_preview}"));
        }
        if let Some(readme) = &meta.readme_excerpt {
            let trimmed: String = readme.chars().take(README_INPUT_CHAR_LIMIT).collect();
            parts.push(format!("README_EXCERPT:\n{trimmed}"));
        }
    } else if let Some(meta) = &shallow.scholarly_metadata {
        parts.push("SOURCE_TYPE: scholarly_link".to_string());
        parts.push(format!("PAPER_TITLE: {}", meta.paper_title));
        if !meta.authors.is_empty() {
            parts.push(format!("AUTHORS: {}", meta.authors.join(", ")));
        }
        if !meta.subjects.is_empty() {
            parts.push(format!("SUBJECTS: {}", meta.subjects.join(", ")));
        }
        parts.push(format!("ABSTRACT: {}", meta.abstract_text));
    } else {
        parts.push("SOURCE_TYPE: generic".to_string());
        parts.push(format!("TITLE: {}", shallow.title));
        parts.push(format!("SUMMARY: {}", shallow.summary));
        if !shallow.relevance_tags.is_empty() {
            parts.push(format!("TAGS: {}", shallow.relevance_tags.join(", ")));
        }
    }

    parts.push(String::new());
    parts.push(String::from(
        "Extract knowledge from the source above. Output ONLY the JSON object (begin with `{` immediately, end with `}`). No thinking out loud, no preamble, no code fences. Schema (every key present):\n\
{\n\
  \"concepts\": [string],                       // 3-7 core ideas this source contributes\n\
  \"patterns\": [string],                       // architectural / design patterns visible\n\
  \"novel_ideas\": [string],                    // ideas that are surprising or non-obvious\n\
  \"applicability_to_arda\": string,       // 1-3 sentences: how this could integrate into Arda (concrete, not generic)\n\
  \"integration_hooks\": [string],              // specific Arda crates/files/abstractions to attach to (e.g. crates/annunimas-charon/src/router.rs)\n\
  \"comparable_systems\": [string],             // other systems with similar approach\n\
  \"risks_or_concerns\": [string],              // adoption or correctness risks\n\
  \"confidence_self_report\": number,           // 0.0 to 1.0, your confidence the source material was rich enough for solid extraction\n\
  \"summary_one_paragraph\": string             // 2-4 sentence synthesis suitable for quick agent recall\n\
}",
    ));

    parts.join("\n")
}

pub(super) fn parse_response(raw: &str) -> ExtractedKnowledge {
    let candidate = extract_json_object(raw).unwrap_or_else(|| raw.trim().to_string());
    match serde_json::from_str::<ExtractedKnowledge>(&candidate) {
        Ok(mut k) => {
            if k.confidence_self_report.is_nan() {
                k.confidence_self_report = 0.0;
            }
            k.confidence_self_report = k.confidence_self_report.clamp(0.0, 1.0);
            k
        }
        Err(e) => {
            let excerpt: String = raw.chars().take(2_000).collect();
            ExtractedKnowledge {
                parse_error: Some(format!("json parse failed: {e}")),
                raw_response_excerpt: Some(excerpt),
                ..Default::default()
            }
        }
    }
}

fn extract_json_object(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    // Strip common markdown fences the model may emit despite instructions.
    let trimmed = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .unwrap_or(trimmed);
    let trimmed = trimmed.trim_end_matches("```").trim();

    let start = trimmed.find('{')?;
    let bytes = trimmed.as_bytes();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        let c = b as char;
        if escape {
            escape = false;
            continue;
        }
        if in_string {
            if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(trimmed[start..=i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::GithubMetadata;

    fn shallow_with_github(readme: &str) -> ShallowAnalysis {
        ShallowAnalysis {
            title: "owner/repo — desc".to_string(),
            summary: "x".to_string(),
            language: Some("rust".to_string()),
            key_dependencies: vec!["serde".to_string()],
            relevance_tags: vec!["githubrepo".to_string()],
            license: Some("MIT".to_string()),
            components_available: vec![],
            reuse_potential: None,
            deep_analysis_recommended: true,
            deep_analysis_reason: "test".to_string(),
            scholarly_metadata: None,
            github_metadata: Some(GithubMetadata {
                owner: "owner".to_string(),
                repo: "repo".to_string(),
                full_name: "owner/repo".to_string(),
                description: Some("a test repo".to_string()),
                primary_language: Some("Rust".to_string()),
                license: Some("MIT".to_string()),
                default_branch: Some("main".to_string()),
                stargazers_count: Some(10),
                forks_count: Some(1),
                open_issues_count: Some(0),
                pushed_at: None,
                topics: vec!["agentic".to_string()],
                readme_excerpt: Some(readme.to_string()),
                manifest_kind: Some("cargo".to_string()),
                key_dependencies: vec!["serde".to_string(), "tokio".to_string()],
                file_path: None,
                ref_name: None,
                source_url: "https://github.com/owner/repo".to_string(),
            }),
        }
    }

    #[test]
    fn parses_strict_json_response() {
        let body = r#"{"concepts":["a","b"],"patterns":[],"novel_ideas":["x"],"applicability_to_arda":"integrate as router plugin","integration_hooks":["crates/charon"],"comparable_systems":["foo"],"risks_or_concerns":["mem"],"confidence_self_report":0.8,"summary_one_paragraph":"summary"}"#;
        let k = parse_response(body);
        assert_eq!(k.concepts, vec!["a", "b"]);
        assert_eq!(k.applicability_to_arda, "integrate as router plugin");
        assert!((k.confidence_self_report - 0.8).abs() < 1e-9);
        assert!(k.parse_error.is_none());
    }

    #[test]
    fn parses_json_inside_markdown_fence() {
        let body = "```json\n{\"concepts\":[\"a\"],\"patterns\":[],\"novel_ideas\":[],\"applicability_to_arda\":\"x\",\"integration_hooks\":[],\"comparable_systems\":[],\"risks_or_concerns\":[],\"confidence_self_report\":0.5,\"summary_one_paragraph\":\"s\"}\n```";
        let k = parse_response(body);
        assert_eq!(k.concepts, vec!["a"]);
        assert!(k.parse_error.is_none());
    }

    #[test]
    fn parses_json_with_preamble() {
        let body = "Here is the extraction:\n{\"concepts\":[\"x\"],\"patterns\":[],\"novel_ideas\":[],\"applicability_to_arda\":\"\",\"integration_hooks\":[],\"comparable_systems\":[],\"risks_or_concerns\":[],\"confidence_self_report\":0.3,\"summary_one_paragraph\":\"\"}\nHope this helps.";
        let k = parse_response(body);
        assert_eq!(k.concepts, vec!["x"]);
        assert!(k.parse_error.is_none());
    }

    #[test]
    fn invalid_json_falls_back_to_raw_excerpt() {
        let body = "i'm sorry i can't comply";
        let k = parse_response(body);
        assert!(k.parse_error.is_some());
        assert_eq!(
            k.raw_response_excerpt.as_deref(),
            Some("i'm sorry i can't comply")
        );
        assert!(k.concepts.is_empty());
    }

    #[test]
    fn clamps_confidence_above_one() {
        let body = r#"{"concepts":[],"patterns":[],"novel_ideas":[],"applicability_to_arda":"","integration_hooks":[],"comparable_systems":[],"risks_or_concerns":[],"confidence_self_report":7.0,"summary_one_paragraph":""}"#;
        let k = parse_response(body);
        assert!((k.confidence_self_report - 1.0).abs() < 1e-9);
    }

    #[test]
    fn user_prompt_includes_repo_metadata() {
        let s = shallow_with_github("# My README\nSome details about the project.");
        let prompt = build_user_prompt(&s);
        assert!(prompt.contains("owner/repo"));
        assert!(prompt.contains("PRIMARY_LANGUAGE: Rust"));
        assert!(prompt.contains("KEY_DEPENDENCIES: serde, tokio"));
        assert!(prompt.contains("README_EXCERPT"));
        assert!(prompt.contains("\"concepts\""));
    }
}
