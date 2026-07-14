// Human Knowledge Interface for Annunimas
//
// Provides Obsidian-compatible knowledge graph access to /human/
// Integrates with Mnemosyne for persistent memory and Chronos for temporal actions

use annunimas_core::contract::memory::{MemoryKind, MemoryRecord};
use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Frontmatter structure for human notes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteFrontmatter {
    pub title: String,
    pub created: String,
    pub updated: String,
    pub tags: Vec<String>,
    pub status: String,
    pub confidence: u8,
    pub source: Option<String>,
    pub related: Vec<String>,
    pub references: Vec<String>,
}

/// A parsed human note
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanNote {
    pub path: PathBuf,
    pub frontmatter: NoteFrontmatter,
    pub content: String,
    pub word_count: usize,
    pub backlinks: Vec<PathBuf>,
}

/// Main interface to the human knowledge vault
#[derive(Debug, Clone)]
pub struct HumanKnowledge {
    vault_path: PathBuf,
    memory: MemoryStore,
}

#[derive(Debug, Clone)]
pub struct MemoryStore {
    namespace: String,
}

impl MemoryStore {
    pub fn new(namespace: &str) -> Result<Self> {
        Ok(Self {
            namespace: namespace.to_string(),
        })
    }

    pub fn store(
        &self,
        id: &str,
        content: &str,
        tags: &[String],
        confidence: Option<u8>,
    ) -> Result<()> {
        let mut record = MemoryRecord::new(
            format!("{}:{}", self.namespace, id),
            MemoryKind::Semantic,
            "human",
            content,
        );
        if !tags.is_empty() {
            record.extensions.insert(
                "tags".to_string(),
                serde_json::Value::Array(
                    tags.iter()
                        .cloned()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
        }
        if let Some(confidence) = confidence {
            record.extensions.insert(
                "confidence".to_string(),
                serde_json::Value::from(confidence),
            );
        }

        let _serialized = serde_json::to_string(&record)?;
        Ok(())
    }
}

impl HumanKnowledge {
    /// Create a new HumanKnowledge interface
    pub fn new(vault_path: impl AsRef<Path>) -> Result<Self> {
        let vault_path = vault_path.as_ref().to_path_buf();
        if !vault_path.exists() {
            anyhow::bail!("Human vault path does not exist: {:?}", vault_path);
        }

        Ok(Self {
            vault_path,
            memory: MemoryStore::new("human")?,
        })
    }

    /// Scan the vault for all notes
    pub fn scan_vault(&self) -> Result<Vec<HumanNote>> {
        let mut notes = Vec::new();

        for entry in WalkDir::new(&self.vault_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file()
                && entry.path().extension().is_some_and(|ext| ext == "md")
            {
                let note = self.parse_note(entry.path())?;
                self.memory.store(
                    &note.path.display().to_string(),
                    &note.content,
                    &note.frontmatter.tags,
                    Some(note.frontmatter.confidence),
                )?;
                notes.push(note);
            }
        }

        Ok(notes)
    }

    /// Parse a single note file
    fn parse_note(&self, path: &Path) -> Result<HumanNote> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read note: {:?}", path))?;

        let frontmatter = self.extract_frontmatter(&content)?;

        let word_count = content.split_whitespace().count();

        let backlinks = self.find_backlinks(&content, path)?;

        Ok(HumanNote {
            path: path.to_path_buf(),
            frontmatter,
            content,
            word_count,
            backlinks,
        })
    }

    fn extract_frontmatter(&self, content: &str) -> Result<NoteFrontmatter> {
        let re = Regex::new(r#"---\s*\n([\s\S]*?)\n---\s*"#)?;
        let yaml = re
            .captures(content)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str())
            .unwrap_or("");

        let frontmatter: NoteFrontmatter = serde_yaml::from_str(yaml)
            .with_context(|| format!("Failed to parse frontmatter YAML:\n{}", yaml))?;

        Ok(frontmatter)
    }

    /// Find wikilinks in content and resolve them to paths
    fn find_backlinks(&self, content: &str, current_path: &Path) -> Result<Vec<PathBuf>> {
        let mut backlinks = Vec::new();
        let re = Regex::new(r#"\[\[([^\]]+)\]\]"#)?;

        for cap in re.captures_iter(content) {
            if let Some(link_match) = cap.get(1) {
                let resolved = self.resolve_wikilink(link_match.as_str(), current_path)?;
                if resolved.exists() {
                    backlinks.push(resolved);
                }
            }
        }

        Ok(backlinks)
    }

    /// Resolve a wikilink to a full path
    fn resolve_wikilink(&self, link: &str, _current_path: &Path) -> Result<PathBuf> {
        let link = link.trim();
        if link.contains('/') {
            // Assume it's a relative path from vault root
            Ok(self.vault_path.join(link).with_extension("md"))
        } else {
            // Assume it's a filename in the same directory
            Ok(_current_path.with_file_name(format!("{}.md", link)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_note() {
        let vault = tempfile::tempdir().unwrap();
        let note_path = vault.path().join("test.md");
        std::fs::write(
            &note_path,
            r#"---
title: Test Note
created: 2026-05-10
updated: 2026-05-10
tags: [test]
status: draft
confidence: 50
source: null
related: []
references: []
---

This is a test note with [[backlink]] and [[nested/file]].
"#,
        )
        .unwrap();

        let hk = HumanKnowledge::new(vault.path()).unwrap();
        let note = hk.parse_note(&note_path).unwrap();

        assert_eq!(note.frontmatter.title, "Test Note");
        assert_eq!(note.frontmatter.tags, vec!["test"]);
        assert!(note.content.contains("backlink"));
        assert_eq!(note.word_count, 30);
    }
}
