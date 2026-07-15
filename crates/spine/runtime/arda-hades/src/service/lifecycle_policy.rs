use super::HadesService;
use arda_core::error::Result;
use chrono::Utc;
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum LifecycleDisposition {
    Hold,
    Archive,
    Remove,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct LifecycleDecision {
    pub memory_scope: String,
    pub disposition: LifecycleDisposition,
    pub recommended_sigil_retention: String,
    pub consistency_ok: bool,
    pub consistency_issues: Vec<String>,
    pub rationale: String,
}

impl HadesService {
    pub(super) fn lifecycle_decision_for(&self, file: &Path) -> LifecycleDecision {
        let memory_scope = infer_memory_scope(file);
        let file_sigil = super::read_sigil(file).unwrap_or(crate::types::SigilState::Unknown);
        let mut consistency_issues = Vec::new();

        if matches!(file_sigil, crate::types::SigilState::Coin)
            && matches!(memory_scope, "human_context" | "boardroom_council")
        {
            consistency_issues
                .push("destructive coin sigil conflicts with protected memory scope".to_owned());
        }

        let (disposition, recommended_sigil_retention, rationale): (
            LifecycleDisposition,
            &'static str,
            &'static str,
        ) = match memory_scope {
            "boardroom_council" => (
                LifecycleDisposition::Archive,
                "keep",
                "boardroom directives should archive conservatively",
            ),
            "human_context" => (
                LifecycleDisposition::Archive,
                "keep",
                "human context favors archive over destructive cleanup",
            ),
            "edge_runtime" => {
                if self.should_archive(file) {
                    (
                        LifecycleDisposition::Archive,
                        "summarize",
                        "edge runtime artifacts can be compacted aggressively but archive jsonl traces",
                    )
                } else {
                    (
                        LifecycleDisposition::Remove,
                        "vacuum",
                        "edge runtime artifacts default to bounded vacuum",
                    )
                }
            }
            _ => {
                if is_archive_favored_extension(file) {
                    (
                        LifecycleDisposition::Archive,
                        "summarize",
                        "system continuity artifacts should archive before removal",
                    )
                } else if self.should_archive(file) {
                    (
                        LifecycleDisposition::Archive,
                        "summarize",
                        "jsonl continuity traces should archive instead of hard delete",
                    )
                } else {
                    (
                        LifecycleDisposition::Remove,
                        "vacuum",
                        "unreferenced continuity residue can be vacuumed",
                    )
                }
            }
        };

        if matches!(memory_scope, "human_context" | "boardroom_council")
            && matches!(disposition, LifecycleDisposition::Remove)
        {
            consistency_issues
                .push("protected scope resolved to remove instead of archive".to_owned());
        }

        let consistency_ok = consistency_issues.is_empty();
        let disposition = if consistency_ok {
            disposition
        } else {
            LifecycleDisposition::Hold
        };

        LifecycleDecision {
            memory_scope: memory_scope.to_owned(),
            disposition,
            recommended_sigil_retention: recommended_sigil_retention.to_owned(),
            consistency_ok,
            consistency_issues,
            rationale: rationale.to_owned(),
        }
    }

    pub(super) fn memory_referenced(&self, file: &Path) -> bool {
        let needle = file
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if needle.is_empty() {
            return false;
        }
        if let Ok(content) = std::fs::read_to_string(&self.world_state_path) {
            return content.contains(needle);
        }
        false
    }

    pub(super) fn should_archive(&self, file: &Path) -> bool {
        file.extension().and_then(|value| value.to_str()) == Some("jsonl")
    }

    pub(super) fn archive_file(&self, file: &Path) -> Result<()> {
        let date_dir = self
            .archive_root
            .join(Utc::now().format("%Y-%m-%d").to_string());
        fs::create_dir_all(&date_dir)?;
        let target = date_dir.join(
            file.file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("archived_file"),
        );
        fs::rename(file, target)?;
        Ok(())
    }
}

fn infer_memory_scope(file: &Path) -> &'static str {
    let lower = file.to_string_lossy().to_ascii_lowercase();
    if lower.contains("/human/")
        || lower.starts_with("human/")
        || lower.contains("/data/human/")
        || lower.contains("/core/personal/")
    {
        return "human_context";
    }
    if lower.contains("/boardroom/")
        || lower.contains("/council/")
        || lower.contains("/core/projects/")
        || lower.contains("/core/queue/")
        || lower.contains("/data/hermes/")
    {
        return "boardroom_council";
    }
    if lower.contains("/edge/")
        || lower.contains("/fleet/")
        || lower.contains("/warden/")
        || lower.contains("/remote_operator/")
    {
        return "edge_runtime";
    }
    "system_continuity"
}

fn is_archive_favored_extension(file: &Path) -> bool {
    matches!(
        file.extension().and_then(|value| value.to_str()),
        Some("jsonl" | "json" | "md" | "toml" | "yaml" | "yml")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_holds_coin_sigil_in_human_context() {
        let dir = tempfile::tempdir().expect("tempdir");
        let human_dir = dir.path().join("human");
        fs::create_dir_all(&human_dir).expect("human dir");
        let note = human_dir.join("context.md");
        fs::write(&note, "# sigil: COIN\nprotected human context\n").expect("write note");

        let service = HadesService::new(dir.path()).expect("service");
        let decision = service.lifecycle_decision_for(&note);

        assert_eq!(decision.memory_scope, "human_context");
        assert_eq!(decision.disposition, LifecycleDisposition::Hold);
        assert!(!decision.consistency_ok);
        assert!(decision
            .consistency_issues
            .iter()
            .any(|issue| issue.contains("protected memory scope")));
    }

    #[test]
    fn lifecycle_archives_boardroom_jsonl() {
        let dir = tempfile::tempdir().expect("tempdir");
        let boardroom_dir = dir.path().join("data/hermes/boardroom");
        fs::create_dir_all(&boardroom_dir).expect("boardroom dir");
        let log = boardroom_dir.join("session.jsonl");
        fs::write(&log, "{\"directive\":\"hold\"}\n").expect("write log");

        let service = HadesService::new(dir.path()).expect("service");
        let decision = service.lifecycle_decision_for(&log);

        assert_eq!(decision.memory_scope, "boardroom_council");
        assert_eq!(decision.disposition, LifecycleDisposition::Archive);
        assert!(decision.consistency_ok);
        assert_eq!(decision.recommended_sigil_retention, "keep");
    }
}
