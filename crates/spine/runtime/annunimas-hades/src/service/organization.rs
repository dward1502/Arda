use super::{append_jsonl, HadesService};
use annunimas_core::error::{AnnunimasError, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

const ORGANIZATION_AUDIT_CONTRACT: &str = "annunimas.hades.organization_audit_report.v1";
const ORGANIZATION_PLAN_CONTRACT: &str = "annunimas.hades.organization_plan.v1";
const ORGANIZATION_CANDIDATE_CONTRACT: &str = "annunimas.hades.organization_candidate.v1";
const ORGANIZATION_APPROVAL_PACKET_CONTRACT: &str =
    "annunimas.hades.organization_operator_approval_packet.v1";
const ORGANIZATION_APPLY_RECEIPT_CONTRACT: &str = "annunimas.hades.organization_apply_receipt.v1";
const SOTERION_VISIBLE_MARKER: &str = "🜏 Soterion:";

#[derive(Debug, Clone, Serialize)]
struct OrganizationCoverage {
    markdown_files_total: usize,
    markdown_with_yaml_frontmatter_total: usize,
    markdown_with_soterion_heading_total: usize,
    markdown_with_coin_heading_total: usize,
    directories_total: usize,
    directories_with_readme_total: usize,
    directories_with_index_total: usize,
    directories_missing_readme_total: usize,
    directories_missing_index_total: usize,
}

impl OrganizationCoverage {
    fn markdown_frontmatter_percent(&self) -> f64 {
        percent(
            self.markdown_with_yaml_frontmatter_total,
            self.markdown_files_total,
        )
    }

    fn markdown_soterion_percent(&self) -> f64 {
        percent(
            self.markdown_with_soterion_heading_total,
            self.markdown_files_total,
        )
    }

    fn readme_percent(&self) -> f64 {
        percent(self.directories_with_readme_total, self.directories_total)
    }

    fn index_percent(&self) -> f64 {
        percent(self.directories_with_index_total, self.directories_total)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrganizationCandidate {
    contract: String,
    candidate_id: String,
    path: String,
    candidate_type: String,
    severity: String,
    recommended_action: String,
    glyph: String,
    no_delete: bool,
    apply_default: bool,
    evidence: Value,
}

impl HadesService {
    pub fn organization_audit_report(
        &self,
        root_path: impl AsRef<Path>,
        limit: usize,
    ) -> Result<Value> {
        let root_path = root_path.as_ref();
        let bounded_limit = limit.max(1);
        let coverage = measure_organization_coverage(root_path)?;
        let candidates = organization_candidates(root_path, None, bounded_limit)?;
        let report_path = self.organization_audit_report_path();
        let generated_at_utc = Utc::now().to_rfc3339();
        let out = serde_json::json!({
            "contract": ORGANIZATION_AUDIT_CONTRACT,
            "generated_at_utc": generated_at_utc,
            "root_path": root_path.display().to_string(),
            "report_path": report_path.display().to_string(),
            "no_delete": true,
            "apply_default": false,
            "coin": {
                "glyph": "🪙",
                "code_point": "U+1FA99",
                "hex": "0x0001FA99",
                "registry": "meta/soterion_sigils.yaml"
            },
            "coverage": {
                "markdown_files_total": coverage.markdown_files_total,
                "markdown_with_yaml_frontmatter_total": coverage.markdown_with_yaml_frontmatter_total,
                "markdown_with_soterion_heading_total": coverage.markdown_with_soterion_heading_total,
                "markdown_with_coin_heading_total": coverage.markdown_with_coin_heading_total,
                "markdown_frontmatter_percent": coverage.markdown_frontmatter_percent(),
                "markdown_soterion_percent": coverage.markdown_soterion_percent(),
                "directories_total": coverage.directories_total,
                "directories_with_readme_total": coverage.directories_with_readme_total,
                "directories_with_index_total": coverage.directories_with_index_total,
                "directories_missing_readme_total": coverage.directories_missing_readme_total,
                "directories_missing_index_total": coverage.directories_missing_index_total,
                "readme_percent": coverage.readme_percent(),
                "index_percent": coverage.index_percent()
            },
            "candidate_preview_total": candidates.len(),
            "candidate_preview": candidates,
            "safety_gates": [
                "read_only_audit_first",
                "dry_run_plan_before_mutation",
                "operator_review_before_apply",
                "no_delete_in_organization_job",
                "git_diff_check_after_apply"
            ]
        });
        write_pretty_json(&report_path, &out)?;
        append_jsonl(&self.organization_findings_path(), &out)?;
        self.log_event(
            "organization_audit_report_generated",
            Some(&report_path.display().to_string()),
            out.clone(),
        )?;
        Ok(out)
    }

    pub fn organization_plan_report(
        &self,
        root_path: impl AsRef<Path>,
        scope: Option<&str>,
        limit: usize,
    ) -> Result<Value> {
        let root_path = root_path.as_ref();
        let bounded_limit = limit.max(1);
        let scope_path = scope.map(PathBuf::from);
        let candidates = organization_candidates(root_path, scope_path.as_deref(), bounded_limit)?;
        let plan_path = self.organization_plan_path();
        let out = serde_json::json!({
            "contract": ORGANIZATION_PLAN_CONTRACT,
            "generated_at_utc": Utc::now().to_rfc3339(),
            "root_path": root_path.display().to_string(),
            "scope": scope.unwrap_or("."),
            "plan_path": plan_path.display().to_string(),
            "dry_run": true,
            "no_delete": true,
            "apply_default": false,
            "mutation_requires_operator_approval": true,
            "candidates_total": candidates.len(),
            "candidates": candidates,
            "apply_sequence": [
                "confirm clean/understood working tree",
                "run organization-audit and archive report",
                "review organization-plan candidates",
                "apply one directory scope at a time only after approval",
                "run cargo fmt/test and git diff --check"
            ]
        });
        write_pretty_json(&plan_path, &out)?;
        for candidate in candidates_from_value(&out) {
            append_jsonl(&self.organization_plan_queue_path(), &candidate)?;
        }
        self.log_event(
            "organization_plan_generated",
            Some(&plan_path.display().to_string()),
            out.clone(),
        )?;
        Ok(out)
    }

    pub fn organization_approval_packet(
        &self,
        root_path: impl AsRef<Path>,
        scope: Option<&str>,
        limit: usize,
        out_path: impl AsRef<Path>,
        operator_id: &str,
        approved: bool,
    ) -> Result<Value> {
        let root_path = root_path.as_ref();
        let out_path = out_path.as_ref();
        let bounded_limit = limit.max(1);
        let scope_path = scope.map(PathBuf::from);
        let candidates = organization_candidates(root_path, scope_path.as_deref(), bounded_limit)?;
        let approval_status = if approved {
            "approved"
        } else {
            "pending_operator_review"
        };
        let packet = serde_json::json!({
            "contract": ORGANIZATION_APPROVAL_PACKET_CONTRACT,
            "generated_at_utc": Utc::now().to_rfc3339(),
            "operator_id": operator_id,
            "approval_status": approval_status,
            "root_path": root_path.display().to_string(),
            "scope": scope.unwrap_or("."),
            "source_plan_contract": ORGANIZATION_PLAN_CONTRACT,
            "no_delete": true,
            "destructive_actions_allowed": false,
            "candidates_total": candidates.len(),
            "candidates": candidates,
            "required_before_apply": [
                "operator_approval_status_approved",
                "bounded_directory_scope",
                "git_diff_check_after_apply"
            ]
        });
        write_pretty_json(out_path, &packet)?;
        self.log_event(
            "organization_operator_approval_packet_generated",
            Some(&out_path.display().to_string()),
            packet.clone(),
        )?;
        Ok(packet)
    }

    pub fn execute_organization_apply(
        &self,
        approval_packet: impl AsRef<Path>,
        root_path: impl AsRef<Path>,
        apply: bool,
    ) -> Result<Value> {
        let approval_packet = approval_packet.as_ref();
        let root_path = root_path.as_ref();
        let packet: Value = serde_json::from_str(&fs::read_to_string(approval_packet)?)?;
        if packet.get("contract").and_then(Value::as_str)
            != Some(ORGANIZATION_APPROVAL_PACKET_CONTRACT)
        {
            return Err(AnnunimasError::Task(format!(
                "invalid organization approval packet contract in {}",
                approval_packet.display()
            )));
        }
        let approved = packet
            .get("approval_status")
            .and_then(Value::as_str)
            .map(|status| status == "approved")
            .unwrap_or(false);
        let candidates = parse_candidate_array(&packet)?;
        let blocked_reason = if apply && !approved {
            Some("approval_packet_not_approved")
        } else {
            None
        };
        let mut actions = Vec::new();
        if blocked_reason.is_none() {
            for candidate in &candidates {
                let action = execute_candidate(root_path, candidate, apply)?;
                actions.push(action);
            }
        }
        let receipt = serde_json::json!({
            "contract": ORGANIZATION_APPLY_RECEIPT_CONTRACT,
            "generated_at_utc": Utc::now().to_rfc3339(),
            "approval_packet": approval_packet.display().to_string(),
            "mode": if apply { "apply_requested" } else { "dry_run" },
            "approved": approved,
            "executed": apply && approved,
            "blocked_reason": blocked_reason.unwrap_or(""),
            "destructive_actions_performed": false,
            "candidates_total": candidates.len(),
            "actions_total": actions.len(),
            "actions": actions,
        });
        write_pretty_json(&self.organization_apply_receipt_path(), &receipt)?;
        self.log_event(
            "organization_apply_receipt_generated",
            Some(&approval_packet.display().to_string()),
            receipt.clone(),
        )?;
        Ok(receipt)
    }

    pub(crate) fn organization_audit_report_path(&self) -> PathBuf {
        self.root.join("organization_audit_report.json")
    }

    pub(crate) fn organization_plan_path(&self) -> PathBuf {
        self.root.join("organization_plan.json")
    }

    pub(crate) fn organization_findings_path(&self) -> PathBuf {
        self.root.join("organization_findings.jsonl")
    }

    pub(crate) fn organization_plan_queue_path(&self) -> PathBuf {
        self.root.join("organization_plan_queue.jsonl")
    }

    pub(crate) fn organization_apply_receipt_path(&self) -> PathBuf {
        self.root.join("organization_apply_receipt.json")
    }
}

fn measure_organization_coverage(root_path: &Path) -> Result<OrganizationCoverage> {
    let mut coverage = OrganizationCoverage {
        markdown_files_total: 0,
        markdown_with_yaml_frontmatter_total: 0,
        markdown_with_soterion_heading_total: 0,
        markdown_with_coin_heading_total: 0,
        directories_total: 0,
        directories_with_readme_total: 0,
        directories_with_index_total: 0,
        directories_missing_readme_total: 0,
        directories_missing_index_total: 0,
    };

    for entry in WalkDir::new(root_path)
        .into_iter()
        .filter_entry(should_descend)
        .filter_map(|entry| entry.ok())
    {
        let path = entry.path();
        if entry.file_type().is_dir() {
            coverage.directories_total += 1;
            if has_child_named(path, "README.md") {
                coverage.directories_with_readme_total += 1;
            } else {
                coverage.directories_missing_readme_total += 1;
            }
            if has_child_named(path, "INDEX.md") {
                coverage.directories_with_index_total += 1;
            } else {
                coverage.directories_missing_index_total += 1;
            }
            continue;
        }

        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        coverage.markdown_files_total += 1;
        let content = fs::read_to_string(path)?;
        let head = content.lines().take(30).collect::<Vec<_>>().join("\n");
        if content.starts_with("---\n") {
            coverage.markdown_with_yaml_frontmatter_total += 1;
        }
        if head.contains("soterion:") || head.contains("sigil:") {
            coverage.markdown_with_soterion_heading_total += 1;
        }
        if head.contains('🪙') || head.contains("0x0001FA99") || head.contains("U+1FA99") {
            coverage.markdown_with_coin_heading_total += 1;
        }
    }

    Ok(coverage)
}

fn organization_candidates(
    root_path: &Path,
    scope: Option<&Path>,
    limit: usize,
) -> Result<Vec<OrganizationCandidate>> {
    let scan_root = scope
        .map(|scope_path| root_path.join(scope_path))
        .unwrap_or_else(|| root_path.to_path_buf());
    let mut candidates = Vec::new();

    for entry in WalkDir::new(&scan_root)
        .min_depth(0)
        .max_depth(3)
        .into_iter()
        .filter_entry(should_descend)
        .filter_map(|entry| entry.ok())
    {
        if candidates.len() >= limit {
            break;
        }
        let path = entry.path();
        if entry.file_type().is_dir() {
            add_directory_candidates(root_path, path, &mut candidates, limit);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            add_markdown_candidate(root_path, path, &mut candidates, limit)?;
        }
    }

    Ok(candidates)
}

fn add_directory_candidates(
    root_path: &Path,
    path: &Path,
    candidates: &mut Vec<OrganizationCandidate>,
    limit: usize,
) {
    if candidates.len() >= limit || path == root_path {
        return;
    }
    let rel = display_relative(root_path, path);
    if !has_child_named(path, "README.md") {
        candidates.push(candidate(
            root_path,
            path,
            "missing_readme",
            "medium",
            "Generate README.md summarizing directory purpose, owner, important files, and Soterion scope.",
            serde_json::json!({ "directory": rel, "missing": "README.md" }),
        ));
    }
    if candidates.len() >= limit {
        return;
    }
    if !has_child_named(path, "INDEX.md") {
        candidates.push(candidate(
            root_path,
            path,
            "missing_index",
            "low",
            "Generate INDEX.md with deterministic child file/directory listing and last-reviewed metadata.",
            serde_json::json!({ "directory": rel, "missing": "INDEX.md" }),
        ));
    }
}

fn add_markdown_candidate(
    root_path: &Path,
    path: &Path,
    candidates: &mut Vec<OrganizationCandidate>,
    limit: usize,
) -> Result<()> {
    if candidates.len() >= limit {
        return Ok(());
    }
    let content = fs::read_to_string(path)?;
    let head = content.lines().take(30).collect::<Vec<_>>().join("\n");
    let has_frontmatter = content.starts_with("---\n") && head.contains("soterion:");
    let has_visible_marker = content.contains(SOTERION_VISIBLE_MARKER);
    if has_frontmatter && has_visible_marker {
        return Ok(());
    }
    let candidate_type = if has_frontmatter {
        "missing_visible_soterion_marker"
    } else {
        "missing_soterion_frontmatter"
    };
    let recommended_action = if has_frontmatter {
        "Add visible Soterion page marker near the top of the authored Markdown file."
    } else {
        "Add Soterion YAML frontmatter and visible page marker with stable sigil, glyph, owner, status, and last_reviewed fields."
    };
    candidates.push(candidate(
        root_path,
        path,
        candidate_type,
        "medium",
        recommended_action,
        serde_json::json!({
            "file": display_relative(root_path, path),
            "has_yaml_frontmatter": content.starts_with("---\n"),
            "has_soterion_heading": head.contains("soterion:"),
            "has_visible_soterion_marker": has_visible_marker
        }),
    ));
    Ok(())
}

fn candidate(
    root_path: &Path,
    path: &Path,
    candidate_type: &str,
    severity: &str,
    recommended_action: &str,
    evidence: Value,
) -> OrganizationCandidate {
    let rel = display_relative(root_path, path);
    OrganizationCandidate {
        contract: ORGANIZATION_CANDIDATE_CONTRACT.to_owned(),
        candidate_id: stable_candidate_id(candidate_type, &rel),
        path: rel,
        candidate_type: candidate_type.to_owned(),
        severity: severity.to_owned(),
        recommended_action: recommended_action.to_owned(),
        glyph: "🪙".to_owned(),
        no_delete: true,
        apply_default: false,
        evidence,
    }
}

fn stable_candidate_id(candidate_type: &str, rel: &str) -> String {
    let mut hash: u64 = 1469598103934665603;
    for byte in format!("{candidate_type}:{rel}").bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(1099511628211);
    }
    format!("hog_{hash:016x}")
}

fn candidates_from_value(value: &Value) -> Vec<Value> {
    value
        .get("candidates")
        .and_then(Value::as_array)
        .map(|items| items.to_vec())
        .unwrap_or_default()
}

fn parse_candidate_array(packet: &Value) -> Result<Vec<OrganizationCandidate>> {
    let candidates = packet
        .get("candidates")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    serde_json::from_value(candidates).map_err(AnnunimasError::from)
}

fn execute_candidate(
    root_path: &Path,
    candidate: &OrganizationCandidate,
    apply: bool,
) -> Result<Value> {
    let target = safe_candidate_path(root_path, &candidate.path)?;
    let action = match candidate.candidate_type.as_str() {
        "missing_readme" => {
            let readme = target.join("README.md");
            if apply && !readme.exists() {
                write_generated_readme(&readme, &candidate.path)?;
            }
            "write_readme"
        }
        "missing_index" => {
            let index = target.join("INDEX.md");
            if apply && !index.exists() {
                write_generated_index(&index, &target, &candidate.path)?;
            }
            "write_index"
        }
        "missing_soterion_frontmatter" => {
            if apply {
                add_soterion_frontmatter(&target)?;
            }
            "add_soterion_frontmatter"
        }
        "missing_visible_soterion_marker" => {
            if apply {
                add_soterion_frontmatter(&target)?;
            }
            "add_visible_soterion_marker"
        }
        other => other,
    };
    Ok(serde_json::json!({
        "candidate_id": candidate.candidate_id,
        "path": candidate.path,
        "candidate_type": candidate.candidate_type,
        "action": action,
        "executed": apply,
        "destructive": false,
    }))
}

fn safe_candidate_path(root_path: &Path, rel: &str) -> Result<PathBuf> {
    let rel_path = Path::new(rel);
    if rel_path.is_absolute()
        || rel_path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(AnnunimasError::Task(format!(
            "unsafe organization candidate path: {rel}"
        )));
    }
    Ok(root_path.join(rel_path))
}

fn write_generated_readme(path: &Path, rel: &str) -> Result<()> {
    let title = Path::new(rel)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(rel);
    let content = format!(
        "---\nsoterion:\n  sigil: \"SCROLL\"\n  glyph: \"📜\"\n  code_point: \"U+1F4DC\"\n  role: \"organization_index\"\n  owner: \"HADES\"\n  status: \"active\"\n  last_reviewed: \"{}\"\n---\n\n> 🜏 Soterion: 📜 organization_index | owner: HADES | status: active | reviewed: {}\n\n# {title}\n\nPurpose: HADES-generated directory overview for `{rel}`.\n\n## Contents\n\nSee `INDEX.md` for deterministic child listing.\n",
        Utc::now().date_naive(),
        Utc::now().date_naive()
    );
    fs::write(path, content)?;
    Ok(())
}

fn write_generated_index(path: &Path, dir: &Path, rel: &str) -> Result<()> {
    let mut entries = Vec::new();
    if let Ok(read_dir) = fs::read_dir(dir) {
        for entry in read_dir.filter_map(|entry| entry.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "INDEX.md" {
                continue;
            }
            entries.push(name);
        }
    }
    entries.sort();
    let listing = if entries.is_empty() {
        "- No child entries detected.\n".to_owned()
    } else {
        entries
            .into_iter()
            .map(|entry| format!("- `{entry}`\n"))
            .collect::<String>()
    };
    let content = format!(
        "---\nsoterion:\n  sigil: \"SCROLL\"\n  glyph: \"📜\"\n  code_point: \"U+1F4DC\"\n  role: \"directory_index\"\n  owner: \"HADES\"\n  status: \"active\"\n  last_reviewed: \"{}\"\n---\n\n> 🜏 Soterion: 📜 directory_index | owner: HADES | status: active | reviewed: {}\n\n# Index: {rel}\n\n{listing}",
        Utc::now().date_naive(),
        Utc::now().date_naive()
    );
    fs::write(path, content)?;
    Ok(())
}

fn add_soterion_frontmatter(path: &Path) -> Result<()> {
    let content = fs::read_to_string(path)?;
    let head = content.lines().take(30).collect::<Vec<_>>().join("\n");
    if content.starts_with("---\n")
        && head.contains("soterion:")
        && content.contains(SOTERION_VISIBLE_MARKER)
    {
        return Ok(());
    }
    let visible_marker = visible_soterion_marker(&content);
    let soterion_block = format!(
        "soterion:\n  sigil: \"SCROLL\"\n  glyph: \"📜\"\n  code_point: \"U+1F4DC\"\n  role: \"documentation\"\n  owner: \"HADES\"\n  status: \"active\"\n  last_reviewed: \"{}\"\n",
        Utc::now().date_naive()
    );
    let updated = if content.starts_with("---\n") {
        let body = content.trim_start_matches("---\n");
        if head.contains("soterion:") {
            if let Some((frontmatter, rest)) = body.split_once("\n---\n") {
                format!(
                    "---\n{frontmatter}\n---\n\n{visible_marker}{}",
                    rest.trim_start()
                )
            } else {
                format!("{content}\n\n{visible_marker}")
            }
        } else {
            if let Some((frontmatter, rest)) = body.split_once("\n---\n") {
                format!(
                    "---\n{soterion_block}{frontmatter}\n---\n\n{visible_marker}{}",
                    rest.trim_start()
                )
            } else {
                format!("---\n{soterion_block}---\n\n{visible_marker}{body}")
            }
        }
    } else {
        format!(
            "---\n{soterion_block}---\n\n{visible_marker}{}",
            content.trim_start()
        )
    };
    fs::write(path, updated)?;
    Ok(())
}

fn write_pretty_json(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn visible_soterion_marker(content: &str) -> String {
    let role = frontmatter_string_value(content, "role").unwrap_or("documentation");
    let glyph = frontmatter_string_value(content, "glyph").unwrap_or("📜");
    let owner = frontmatter_string_value(content, "owner").unwrap_or("HADES");
    let status = frontmatter_string_value(content, "status").unwrap_or("active");
    format!(
        "> 🜏 Soterion: {glyph} {role} | owner: {owner} | status: {status} | reviewed: {}\n\n",
        Utc::now().date_naive()
    )
}

fn frontmatter_string_value<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    let body = content.strip_prefix("---\n")?;
    let (frontmatter, _) = body.split_once("\n---\n")?;
    let prefix = format!("{key}:");
    frontmatter.lines().find_map(|line| {
        let trimmed = line.trim();
        let value = trimmed.strip_prefix(&prefix)?.trim();
        Some(value.trim_matches('"'))
    })
}

fn should_descend(entry: &DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();
    !matches!(
        name.as_ref(),
        ".git"
            | "target"
            | ".cache"
            | ".target-local"
            | ".tmp"
            | ".agents"
            | ".claude"
            | ".hermes"
            | ".opencode"
            | "node_modules"
            | "dist"
            | "build"
            | "gen"
            | "generated"
            | ".next"
            | "data"
            | "archive"
            | "__pycache__"
            | ".venv"
            | "venv"
            | ".pytest_cache"
            | ".cargo"
    )
}

fn has_child_named(path: &Path, name: &str) -> bool {
    path.join(name).exists()
}

fn display_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|relative| relative.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

fn percent(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    ((numerator as f64 / denominator as f64) * 10000.0).round() / 100.0
}

#[allow(dead_code)]
fn _count_by_type(candidates: &[OrganizationCandidate]) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    for candidate in candidates {
        *out.entry(candidate.candidate_type.clone()).or_insert(0) += 1;
    }
    out
}
