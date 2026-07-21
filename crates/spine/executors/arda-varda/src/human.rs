use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

const CONTRACT: &str = "arda.human_ingestion_result.v1";
const REQUIRED_FRONTMATTER_KEYS: &[&str] = &[
    "arda_contract",
    "title",
    "status",
    "source_type",
    "authority",
    "owner",
    "created",
    "updated",
    "supersedes",
    "superseded_by",
    "affected_agents",
    "affected_paths",
    "privacy",
    "review_required",
    "confidence",
    "sigils",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HumanIngestionRecord {
    pub contract: String,
    pub source_path: String,
    pub content_hash: String,
    pub detected_status: String,
    pub detected_authority: String,
    pub source_type: String,
    pub affected_agents: Vec<String>,
    pub affected_paths: Vec<String>,
    pub summary: String,
    pub conflicts: Vec<String>,
    pub recommendation: String,
    pub review_required: bool,
    pub frontmatter_valid: bool,
    pub missing_frontmatter_keys: Vec<String>,
    pub generated_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HumanScanReport {
    pub schema_version: String,
    pub generated_at_utc: String,
    pub human_root: String,
    pub output_path: String,
    pub contradiction_path: Option<String>,
    pub scanned_total: usize,
    pub emitted_total: usize,
    pub contradiction_total: usize,
}

pub fn scan_human_root(
    human_root: &Path,
    output_path: &Path,
    contradiction_path: Option<&Path>,
    limit: Option<usize>,
) -> io::Result<HumanScanReport> {
    let mut files = discover_human_files(human_root)?;
    files.sort();
    if let Some(limit) = limit {
        files.truncate(limit);
    }

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let output_file = fs::File::create(output_path)?;
    let mut output = BufWriter::new(output_file);

    let mut contradiction_writer = match contradiction_path {
        Some(path) => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            Some(BufWriter::new(fs::File::create(path)?))
        }
        None => None,
    };

    let mut contradiction_total = 0usize;
    for path in &files {
        let record = classify_human_file(human_root, path)?;
        serde_json::to_writer(&mut output, &record)?;
        output.write_all(b"\n")?;
        if !record.conflicts.is_empty() {
            contradiction_total += 1;
            if let Some(writer) = contradiction_writer.as_mut() {
                serde_json::to_writer(
                    &mut *writer,
                    &json!({
                        "contract": "arda.human_contradiction_candidate.v1",
                        "source_path": record.source_path,
                        "content_hash": record.content_hash,
                        "conflicts": record.conflicts,
                        "recommendation": record.recommendation,
                        "review_required": true,
                        "generated_at_utc": record.generated_at_utc,
                    }),
                )?;
                writer.write_all(b"\n")?;
            }
        }
    }
    output.flush()?;
    if let Some(writer) = contradiction_writer.as_mut() {
        writer.flush()?;
    }

    Ok(HumanScanReport {
        schema_version: "arda.human-scan-report.v1".to_string(),
        generated_at_utc: now_utc(),
        human_root: human_root.display().to_string(),
        output_path: output_path.display().to_string(),
        contradiction_path: contradiction_path.map(|path| path.display().to_string()),
        scanned_total: files.len(),
        emitted_total: files.len(),
        contradiction_total,
    })
}

pub fn classify_human_file(human_root: &Path, path: &Path) -> io::Result<HumanIngestionRecord> {
    let bytes = fs::read(path)?;
    let content = String::from_utf8_lossy(&bytes);
    let relative = path
        .strip_prefix(human_root)
        .map(|relative| Path::new("human").join(relative))
        .unwrap_or_else(|_| path.to_path_buf());
    let source_path = normalize_path(&relative);
    let frontmatter = parse_frontmatter(&content);
    let missing_frontmatter_keys = missing_frontmatter_keys(frontmatter.as_ref());
    let frontmatter_valid = missing_frontmatter_keys.is_empty();

    let detected_status = frontmatter
        .as_ref()
        .and_then(|map| map.get("status"))
        .filter(|value| is_allowed_status(value))
        .cloned()
        .unwrap_or_else(|| infer_status(&source_path, &content));
    let detected_authority = frontmatter
        .as_ref()
        .and_then(|map| map.get("authority"))
        .filter(|value| is_allowed_authority(value))
        .cloned()
        .unwrap_or_else(|| infer_authority(&source_path, &content));
    let source_type = frontmatter
        .as_ref()
        .and_then(|map| map.get("source_type"))
        .filter(|value| is_allowed_source_type(value))
        .cloned()
        .unwrap_or_else(|| infer_source_type(&source_path, path));
    let affected_agents = infer_affected_agents(&source_path, &content);
    let affected_paths = infer_affected_paths(&source_path, &content);
    let conflicts = infer_conflicts(
        &source_path,
        &content,
        &detected_status,
        &detected_authority,
        frontmatter.as_ref(),
    );
    let review_required = !frontmatter_valid
        || !conflicts.is_empty()
        || detected_status != "canonical"
        || detected_authority == "raw";

    Ok(HumanIngestionRecord {
        contract: CONTRACT.to_string(),
        source_path,
        content_hash: format!("sha256:{:x}", Sha256::digest(&bytes)),
        detected_status: detected_status.clone(),
        detected_authority: detected_authority.clone(),
        source_type: source_type.clone(),
        affected_agents,
        affected_paths,
        summary: summarize(&content, &source_type),
        conflicts: conflicts.clone(),
        recommendation: recommendation(&detected_status, &detected_authority, &conflicts),
        review_required,
        frontmatter_valid,
        missing_frontmatter_keys,
        generated_at_utc: now_utc(),
    })
}

fn discover_human_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !root.exists() {
        return Ok(out);
    }
    discover_recursive(root, &mut out)?;
    Ok(out)
}

fn discover_recursive(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        let name = entry.file_name().to_string_lossy().to_string();
        if file_type.is_dir() {
            if !should_skip_human_dir(&name) {
                discover_recursive(&path, out)?;
            }
        } else if file_type.is_file() && !name.starts_with('.') && is_supported_human_file(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn should_skip_human_dir(name: &str) -> bool {
    name.starts_with('.') || matches!(name, "target" | "node_modules")
}

fn is_supported_human_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref(),
        Some("md" | "txt" | "json" | "jsonl" | "toml" | "yaml" | "yml")
    )
}

fn parse_frontmatter(content: &str) -> Option<BTreeMap<String, String>> {
    let mut lines = content.lines();
    if lines.next().map(str::trim) != Some("---") {
        return None;
    }
    let mut map = BTreeMap::new();
    for line in lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            return Some(map);
        }
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once(':') {
            map.insert(
                key.trim().to_string(),
                value
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string(),
            );
        }
    }
    None
}

fn missing_frontmatter_keys(frontmatter: Option<&BTreeMap<String, String>>) -> Vec<String> {
    let Some(frontmatter) = frontmatter else {
        return REQUIRED_FRONTMATTER_KEYS
            .iter()
            .map(|key| (*key).to_string())
            .collect();
    };
    REQUIRED_FRONTMATTER_KEYS
        .iter()
        .filter(|key| !frontmatter.contains_key(**key))
        .map(|key| (*key).to_string())
        .collect()
}

fn infer_status(source_path: &str, content: &str) -> String {
    let lower_path = source_path.to_ascii_lowercase();
    let lower_content = content.to_ascii_lowercase();
    if lower_path.contains("/inbox/") {
        "inbox"
    } else if lower_path.contains("/archive/") || lower_content.contains("superseded") {
        "archived"
    } else if lower_path.contains("/canonical/") || lower_path.contains("/decisions/") {
        "canonical"
    } else if lower_path.contains("/plans/")
        || lower_path.contains("/working/")
        || lower_path.ends_with("thoughts.md")
    {
        "working"
    } else if lower_content.contains("candidate") || lower_content.contains("proposal") {
        "candidate"
    } else {
        "working"
    }
    .to_string()
}

fn infer_authority(source_path: &str, content: &str) -> String {
    let lower_path = source_path.to_ascii_lowercase();
    let lower_content = content.to_ascii_lowercase();
    if lower_path.contains("/summaries/") || lower_content.contains("generated") {
        "agent_generated"
    } else if lower_path.contains("/governance/")
        || lower_path.contains("/decisions/")
        || lower_content.contains("override")
        || lower_content.contains("covenant")
    {
        "governance"
    } else if lower_path.contains("/sources/") || lower_path.contains("/inbox/") {
        "raw"
    } else {
        "human"
    }
    .to_string()
}

fn infer_source_type(source_path: &str, path: &Path) -> String {
    let lower_path = source_path.to_ascii_lowercase();
    if lower_path.contains("/plans/") {
        "plan"
    } else if lower_path.contains("/decisions/") || lower_path.contains("/governance/") {
        "decision"
    } else if lower_path.contains("/summaries/") {
        "summary"
    } else if lower_path.contains("/sources/") {
        "source"
    } else if lower_path.contains("/triggers/") {
        "trigger"
    } else if matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref(),
        Some("json" | "jsonl")
    ) {
        "source"
    } else {
        "note"
    }
    .to_string()
}

fn infer_affected_agents(source_path: &str, content: &str) -> Vec<String> {
    let haystack = format!("{}\n{}", source_path, content).to_ascii_lowercase();
    let candidates = [
        "athena",
        "hades",
        "prometheus",
        "mnemosyne",
        "manwe",
        "hermes",
        "apollo",
        "oracle",
        "plutus",
        "warden",
    ];
    let mut agents = candidates
        .iter()
        .filter(|agent| haystack.contains(**agent))
        .map(|agent| (*agent).to_string())
        .collect::<Vec<_>>();
    if agents.is_empty() {
        agents.push("athena".to_string());
    }
    agents
}

fn infer_affected_paths(source_path: &str, content: &str) -> Vec<String> {
    let mut paths = vec!["human/".to_string()];
    for marker in ["config/", "crates/", "docs/", "core/", "scripts/", "data/"] {
        if source_path.contains(marker) || content.contains(marker) {
            paths.push(marker.to_string());
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

fn infer_conflicts(
    source_path: &str,
    content: &str,
    detected_status: &str,
    detected_authority: &str,
    frontmatter: Option<&BTreeMap<String, String>>,
) -> Vec<String> {
    let lower_content = content.to_ascii_lowercase();
    let mut conflicts = Vec::new();
    if detected_status == "canonical"
        && (lower_content.contains("todo") || lower_content.contains("draft"))
    {
        conflicts.push("canonical_file_contains_draft_or_todo_language".to_string());
    }
    if detected_authority == "agent_generated" && detected_status == "canonical" {
        conflicts.push("agent_generated_content_marked_canonical".to_string());
    }
    if source_path.to_ascii_lowercase().contains("/archive/") && detected_status == "canonical" {
        conflicts.push("archived_location_marked_canonical".to_string());
    }
    if let Some(frontmatter) = frontmatter {
        if frontmatter.get("review_required").map(String::as_str) == Some("false")
            && detected_status != "canonical"
        {
            conflicts.push("noncanonical_file_disables_review".to_string());
        }
    }
    conflicts
}

fn recommendation(status: &str, authority: &str, conflicts: &[String]) -> String {
    if !conflicts.is_empty() {
        "mark-review-conflict"
    } else if status == "canonical" && authority != "raw" {
        "retain-canonical"
    } else if authority == "raw" || status == "inbox" {
        "retain-working-review"
    } else if status == "archived" || status == "superseded" {
        "hades-lifecycle-review"
    } else {
        "retain-working"
    }
    .to_string()
}

fn summarize(content: &str, source_type: &str) -> String {
    let first_meaningful = content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && *line != "---")
        .unwrap_or("");
    let summary = if first_meaningful.is_empty() {
        format!("{source_type} file with no textual summary candidate")
    } else {
        first_meaningful.chars().take(240).collect::<String>()
    };
    summary.replace('\n', " ")
}

fn is_allowed_status(value: &str) -> bool {
    matches!(
        value,
        "inbox" | "working" | "candidate" | "canonical" | "superseded" | "archived" | "quarantine"
    )
}

fn is_allowed_authority(value: &str) -> bool {
    matches!(
        value,
        "raw" | "agent_generated" | "human" | "governance" | "runtime"
    )
}

fn is_allowed_source_type(value: &str) -> bool {
    matches!(
        value,
        "note"
            | "plan"
            | "decision"
            | "source"
            | "summary"
            | "research"
            | "transcript"
            | "media"
            | "trigger"
    )
}

fn normalize_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/")
}

fn now_utc() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::{classify_human_file, scan_human_root};
    use serde_json::Value;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn scanner_emits_contract_jsonl_without_moving_human_files() {
        let dir = tempdir().expect("tempdir");
        let human = dir.path().join("human");
        let inbox = human.join("inbox");
        fs::create_dir_all(&inbox).expect("mkdir inbox");
        let note = inbox.join("idea.md");
        fs::write(
            &note,
            "# Charon idea\n\nRoute ATHENA through config/manwe.providers.toml",
        )
        .expect("write note");
        let out = dir.path().join("data/athena/human_ingestion_results.jsonl");
        let contradictions = dir
            .path()
            .join("data/athena/human_contradiction_candidates.jsonl");

        let report = scan_human_root(&human, &out, Some(&contradictions), None).expect("scan");

        assert_eq!(report.scanned_total, 1);
        assert!(
            note.exists(),
            "scanner must not move or delete source files"
        );
        let line = fs::read_to_string(&out).expect("jsonl");
        let value: Value = serde_json::from_str(line.trim()).expect("record json");
        assert_eq!(value["contract"], "arda.human_ingestion_result.v1");
        assert_eq!(value["source_path"], "human/inbox/idea.md");
        assert_eq!(value["detected_status"], "inbox");
        assert_eq!(value["detected_authority"], "raw");
        assert_eq!(value["source_type"], "note");
        assert_eq!(value["review_required"], true);
        assert!(value["content_hash"]
            .as_str()
            .expect("hash")
            .starts_with("sha256:"));
        assert!(value["affected_agents"]
            .as_array()
            .expect("agents")
            .contains(&Value::String("manwe".to_string())));
    }

    #[test]
    fn classifier_validates_frontmatter_and_flags_contradiction_candidates() {
        let dir = tempdir().expect("tempdir");
        let human = dir.path().join("human");
        let canonical = human.join("canonical");
        fs::create_dir_all(&canonical).expect("mkdir canonical");
        let path = canonical.join("policy.md");
        fs::write(
            &path,
            "---\narda_contract: human_knowledge.v1\ntitle: Policy\nstatus: canonical\nsource_type: decision\nauthority: governance\nowner: human\ncreated: 2026-05-13\nupdated: 2026-05-13\nsupersedes: []\nsuperseded_by: []\naffected_agents: [athena]\naffected_paths: [human/]\nprivacy: private\nreview_required: true\nconfidence: high\nsigils: [◈]\n---\n\n# Policy\n\nTODO: draft this canonical policy later.\n",
        )
        .expect("write canonical");

        let record = classify_human_file(&human, &path).expect("classify");

        assert!(record.frontmatter_valid);
        assert_eq!(record.detected_status, "canonical");
        assert_eq!(record.detected_authority, "governance");
        assert_eq!(record.source_type, "decision");
        assert_eq!(record.recommendation, "mark-review-conflict");
        assert!(record
            .conflicts
            .contains(&"canonical_file_contains_draft_or_todo_language".to_string()));
    }
}
