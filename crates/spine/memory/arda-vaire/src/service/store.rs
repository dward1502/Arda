use super::scope_policy::{self, ConsumerContext, PolicyDisposition, PolicyOperation};
use super::{EpisodicRecord, InformantEvent, MnemosyneService, RecallRecentEntry};
use crate::schema::{EPISODIC_SCHEMA_VERSION, LEGACY_EPISODIC_SCHEMA_VERSION};
use arda_core::contract::{MemoryKind, MemoryRecord};
use arda_core::error::{ArdaError, Result};
use chrono::Utc;
use fs2::FileExt;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

impl MnemosyneService {
    pub fn from_default_or_fallback() -> Result<Self> {
        let primary = default_root();
        let svc = match Self::new(&primary) {
            Ok(v) => v,
            Err(err) => {
                if !is_permission_error(&err) {
                    return Err(err);
                }
                Self::new(arda_root().join("data").join("mnemosyne"))?
            }
        };
        let arda_root = arda_root();
        Ok(apply_contract_dual_write_from_env(svc)
            .with_metrics_root(
                arda_root
                    .join("core")
                    .join("metrics")
                    .join("by_crate")
                    .join("mnemosyne"),
            )
            .with_human_projection_root(arda_root.join("human")))
    }

    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let metrics_root = inferred_metrics_root(&root);
        let human_projection_root = inferred_human_projection_root(&root);
        let episodic_root = root.join("episodic");
        let semantic_root = root.join("semantic");
        let procedural_root = root.join("procedural");
        let archive_root = root.join("archive");
        let chain_head_path = root.join("chain_head");
        let noise_ledger_path = root.join("noise.jsonl");
        let obsidian_index_path = root.join("obsidian_index.jsonl");
        let last_consolidation_path = root.join("last_consolidation_utc");
        let persona_root = root.join("persona");

        fs::create_dir_all(&episodic_root)?;
        fs::create_dir_all(&semantic_root)?;
        fs::create_dir_all(&procedural_root)?;
        fs::create_dir_all(&archive_root)?;
        fs::create_dir_all(&persona_root)?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&noise_ledger_path)?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&obsidian_index_path)?;
        if !chain_head_path.exists() {
            std::fs::write(&chain_head_path, "")?;
        }
        if !last_consolidation_path.exists() {
            std::fs::write(&last_consolidation_path, "")?;
        }

        Ok(Self {
            root,
            episodic_root,
            semantic_root,
            procedural_root,
            archive_root,
            chain_head_path,
            noise_ledger_path,
            obsidian_index_path,
            last_consolidation_path,
            persona_root,
            human_projection_root,
            contract_memory_root: None,
            metrics_root,
            observability: Default::default(),
        })
    }

    /// Enable Phase 1 dual-write: every encode() also emits a v0.1
    /// `MemoryRecord` to `<contract_memory_root>/episodic/<id>.json`.
    /// Pass `<repo>/core/state/memory` for the canonical layout.
    /// Existing on-disk Mnemosyne files are untouched.
    pub fn with_contract_memory_root(mut self, contract_memory_root: PathBuf) -> Self {
        self.contract_memory_root = Some(contract_memory_root);
        self
    }

    /// Configure a local human-readable projection tree.
    ///
    /// This filesystem path may be opened as an Obsidian vault, but it does not
    /// configure an Obsidian account, plugin, or cloud synchronization API.
    pub fn with_human_projection_root(mut self, human_projection_root: PathBuf) -> Self {
        self.human_projection_root = Some(human_projection_root);
        self
    }

    /// Configure the durable metrics directory consumed by `arda-aule`.
    pub fn with_metrics_root(mut self, metrics_root: PathBuf) -> Self {
        self.metrics_root = Some(metrics_root);
        self
    }

    pub fn encode(&self, event: InformantEvent) -> Result<Option<RecallRecentEntry>> {
        self.encode_with_context(event, None)
    }

    /// Encode with explicit consumer provenance for scope-policy mediation.
    pub fn encode_with_context(
        &self,
        event: InformantEvent,
        context: Option<&ConsumerContext>,
    ) -> Result<Option<RecallRecentEntry>> {
        let significance = self.apply_adaptive_significance(&event);
        let memory_scope = derive_memory_scope(&event);
        let memory_id = format!("mem_{}", uuid::Uuid::new_v4().simple());
        let contract_record =
            contract_memory_record(&memory_id, &event, &memory_scope, significance.significance);
        match scope_policy::evaluate(&contract_record, PolicyOperation::Write, context) {
            PolicyDisposition::Block | PolicyDisposition::Redact(_) => {
                return Err(ArdaError::Agent {
                    agent: "vaire".to_owned(),
                    message: "memory encode blocked by scope policy".to_owned(),
                });
            }
            PolicyDisposition::Quarantine => {
                if self.contract_memory_root.is_none() {
                    return Err(ArdaError::Agent {
                        agent: "vaire".to_owned(),
                        message: "quarantined encode requires a contract memory root".to_owned(),
                    });
                }
                self.write_governed_memory(contract_record, context)?;
                return Ok(None);
            }
            PolicyDisposition::Allow => {}
        }
        if significance.class == "noise" {
            append_jsonl(
                &self.noise_ledger_path,
                &serde_json::json!({
                    "ts": Utc::now().to_rfc3339(),
                    "event": event,
                    "memory_scope": memory_scope,
                    "significance": significance,
                }),
            )?;
            return Ok(None);
        }

        if self.contract_memory_root.is_some() {
            self.write_governed_memory(contract_record, context)?;
        }
        let month_dir = self
            .episodic_root
            .join(Utc::now().format("%Y-%m").to_string());
        fs::create_dir_all(&month_dir)?;
        let memory_path = month_dir.join(format!("{memory_id}.jsonl"));

        let prev_hash = std::fs::read_to_string(&self.chain_head_path).unwrap_or_default();
        let body_hash = compute_hash(&prev_hash, &event, &significance);
        std::fs::write(&self.chain_head_path, &body_hash)?;

        let header = serde_json::json!({
            "schema_version": EPISODIC_SCHEMA_VERSION,
            "sigil": significance.sigil,
            "memory_id": memory_id,
            "created_at_utc": Utc::now().to_rfc3339(),
            "authored_by": "mnemosyne",
            "version": "0.1.0",
            "hash": format!("sha256:{body_hash}")
        });
        let body = serde_json::json!({
            "schema_version": EPISODIC_SCHEMA_VERSION,
            "type":"episodic",
            "source_crate": event.crate_name,
            "event_type": event.event_type,
            "memory_scope": memory_scope,
            "significance": significance.significance,
            "joulework": significance.joulework,
            "love_eq": significance.love_eq,
            "triad": significance.triad,
            "bacon_lite_confidence": significance.bacon_lite_confidence,
            "confidence": event.confidence_hint.unwrap_or(significance.bacon_lite_confidence).clamp(0.0, 1.0),
            "trust": if significance.triad { significance.bacon_lite_confidence } else { significance.bacon_lite_confidence * 0.5 },
            "content": event.content,
            "tags": event.tags,
            "ts_utc": event.ts_utc
        });

        append_jsonl(&memory_path, &header)?;
        append_jsonl(&memory_path, &body)?;

        self.emit_work_signal_background(
            "mnemosyne",
            significance.joulework.max(0.2),
            arda_economics::JouleWorkUnit::Reasoning,
            Some(memory_id.clone()),
        );

        Ok(Some(RecallRecentEntry {
            schema_version: EPISODIC_SCHEMA_VERSION.to_owned(),
            migrated_from_schema: None,
            memory_id,
            source_crate: event.crate_name,
            event_type: event.event_type,
            memory_scope,
            significance: significance.significance,
            confidence: event
                .confidence_hint
                .unwrap_or(significance.bacon_lite_confidence)
                .clamp(0.0, 1.0),
            trust: if significance.triad {
                significance.bacon_lite_confidence
            } else {
                significance.bacon_lite_confidence * 0.5
            },
            sigil: significance.sigil,
            content: event.content,
            ts_utc: event.ts_utc,
            tags: event.tags,
        }))
    }

    pub(super) fn read_episodic_records(&self) -> Result<Vec<EpisodicRecord>> {
        let mut out = Vec::new();
        for month in fs::read_dir(&self.episodic_root)? {
            let month = month?;
            if !month.path().is_dir() {
                continue;
            }
            for file in fs::read_dir(month.path())? {
                let file = file?;
                if file.path().extension().and_then(|v| v.to_str()) != Some("jsonl") {
                    continue;
                }
                let content = fs::read_to_string(file.path())?;
                let mut sigil = "MNEME_ACTIVE".to_owned();
                let mut record_schema = LEGACY_EPISODIC_SCHEMA_VERSION.to_owned();
                let mut body: Option<serde_json::Value> = None;
                let mut malformed = false;

                for (i, line) in content.lines().enumerate() {
                    let value: serde_json::Value = match serde_json::from_str(line) {
                        Ok(value) => value,
                        Err(_) => {
                            malformed = true;
                            break;
                        }
                    };
                    if i == 0 {
                        record_schema = value
                            .get("schema_version")
                            .and_then(|v| v.as_str())
                            .unwrap_or(LEGACY_EPISODIC_SCHEMA_VERSION)
                            .to_owned();
                        sigil = value
                            .get("sigil")
                            .and_then(|v| v.as_str())
                            .unwrap_or("MNEME_ACTIVE")
                            .to_owned();
                    } else if i == 1 {
                        body = Some(value);
                        break;
                    }
                }

                if malformed || body.is_none() {
                    continue;
                }

                if record_schema != EPISODIC_SCHEMA_VERSION
                    && record_schema != LEGACY_EPISODIC_SCHEMA_VERSION
                {
                    continue;
                }

                if let Some(value) = body {
                    out.push(EpisodicRecord {
                        schema_version: EPISODIC_SCHEMA_VERSION.to_owned(),
                        migrated_from_schema: (record_schema == LEGACY_EPISODIC_SCHEMA_VERSION)
                            .then(|| LEGACY_EPISODIC_SCHEMA_VERSION.to_owned()),
                        sigil,
                        memory_id: file
                            .path()
                            .file_stem()
                            .and_then(|v| v.to_str())
                            .unwrap_or("")
                            .to_owned(),
                        source_crate: value
                            .get("source_crate")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_owned(),
                        event_type: value
                            .get("event_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_owned(),
                        memory_scope: value
                            .get("memory_scope")
                            .and_then(|v| v.as_str())
                            .unwrap_or("system_continuity")
                            .to_owned(),
                        significance: value
                            .get("significance")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0),
                        confidence: value
                            .get("confidence")
                            .and_then(|v| v.as_f64())
                            .or_else(|| value.get("bacon_lite_confidence").and_then(|v| v.as_f64()))
                            .unwrap_or(0.0)
                            .clamp(0.0, 1.0),
                        trust: value
                            .get("trust")
                            .and_then(|v| v.as_f64())
                            .unwrap_or_else(|| {
                                let confidence = value
                                    .get("bacon_lite_confidence")
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0);
                                if value
                                    .get("triad")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false)
                                {
                                    confidence
                                } else {
                                    confidence * 0.5
                                }
                            })
                            .clamp(0.0, 1.0),
                        content: value
                            .get("content")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_owned(),
                        ts_utc: value
                            .get("ts_utc")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_owned(),
                        tags: value
                            .get("tags")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|x| x.as_str().map(str::to_owned))
                                    .collect()
                            })
                            .unwrap_or_default(),
                    });
                }
            }
        }
        Ok(out)
    }
}

pub(super) fn episodic_schema_counts(root: &Path) -> (usize, usize) {
    let mut legacy = 0usize;
    let mut unsupported = 0usize;
    for path in walk_dir(root).unwrap_or_default() {
        if path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
            continue;
        }
        let Ok(content) = fs::read_to_string(path) else {
            continue;
        };
        let Some(Ok(header)) = content
            .lines()
            .find(|line| !line.trim().is_empty())
            .map(serde_json::from_str::<serde_json::Value>)
        else {
            continue;
        };
        match header
            .get("schema_version")
            .and_then(|value| value.as_str())
        {
            None | Some(LEGACY_EPISODIC_SCHEMA_VERSION) => legacy += 1,
            Some(EPISODIC_SCHEMA_VERSION) => {}
            Some(_) => unsupported += 1,
        }
    }
    (legacy, unsupported)
}

fn inferred_metrics_root(memory_root: &Path) -> Option<PathBuf> {
    let data_root = memory_root.parent()?;
    if memory_root.file_name()? != "mnemosyne" || data_root.file_name()? != "data" {
        return None;
    }
    Some(
        data_root
            .parent()?
            .join("core")
            .join("metrics")
            .join("by_crate")
            .join("mnemosyne"),
    )
}

fn inferred_human_projection_root(memory_root: &Path) -> Option<PathBuf> {
    let data_root = memory_root.parent()?;
    if memory_root.file_name()? != "mnemosyne" || data_root.file_name()? != "data" {
        return None;
    }
    Some(data_root.parent()?.join("human"))
}

pub(super) fn append_jsonl<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.lock_exclusive()?;
    let line = serde_json::to_string(value)?;
    let write_result = (|| -> Result<()> {
        writeln!(file, "{line}")?;
        file.sync_data()?;
        Ok(())
    })();
    let unlock_result = file.unlock().map_err(ArdaError::Ledger);
    write_result?;
    unlock_result?;
    Ok(())
}

pub(super) fn write_atomic_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    write_atomic(path, &serde_json::to_vec_pretty(value)?)
}

pub(super) fn write_atomic(path: &Path, content: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or_else(|| ArdaError::Agent {
        agent: "mnemosyne".to_owned(),
        message: format!("atomic-write path has no parent: {}", path.display()),
    })?;
    fs::create_dir_all(parent)?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("projection");
    let temporary = parent.join(format!(".{file_name}.tmp"));
    fs::write(&temporary, content)?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn compute_hash(
    prev_hash: &str,
    event: &InformantEvent,
    sig: &crate::significance::SignificanceResult,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prev_hash.as_bytes());
    hasher.update(event.informant_id.as_bytes());
    hasher.update(event.crate_name.as_bytes());
    hasher.update(event.event_type.as_bytes());
    hasher.update(event.ts_utc.as_bytes());
    hasher.update(event.content.as_bytes());
    hasher.update(format!("{:.6}", sig.significance).as_bytes());
    let out = hasher.finalize();
    out.iter().map(|b| format!("{b:02x}")).collect()
}

pub(super) fn walk_dir(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !root.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk_dir(&path)?);
        } else {
            out.push(path);
        }
    }
    Ok(out)
}

fn arda_root() -> PathBuf {
    arda_core::layout::arda_root_from(env!("CARGO_MANIFEST_DIR"))
}

fn default_root() -> PathBuf {
    if let Ok(custom) = std::env::var("ARDA_MNEMOSYNE_HOME") {
        return PathBuf::from(custom);
    }
    arda_root().join("data/mnemosyne")
}

/// Phase 1 dual-write opt-in. Honors `ARDA_CONTRACT_MEMORY_ROOT`
/// when set; defaults to `<repo>/core/state/memory` if the env var is
/// the literal string "auto"; otherwise leaves dual-write disabled
/// (preserving existing behavior). Tests that don't set the var get
/// no dual-write — exactly the existing semantics.
fn apply_contract_dual_write_from_env(svc: MnemosyneService) -> MnemosyneService {
    let raw = match std::env::var("ARDA_CONTRACT_MEMORY_ROOT") {
        Ok(v) if !v.is_empty() => v,
        _ => return svc,
    };
    let path = if raw == "auto" {
        arda_root().join("core/state/memory")
    } else {
        PathBuf::from(raw)
    };
    svc.with_contract_memory_root(path)
}

fn is_permission_error(err: &ArdaError) -> bool {
    matches!(
        err,
        ArdaError::Ledger(ioe) if ioe.kind() == std::io::ErrorKind::PermissionDenied
    )
}

fn derive_memory_scope(event: &InformantEvent) -> String {
    for tag in &event.tags {
        if let Some(scope) = tag.strip_prefix("scope:") {
            let normalized = scope.trim().to_ascii_lowercase().replace('-', "_");
            if !normalized.is_empty() {
                return normalized;
            }
        }
    }

    let tags = event
        .tags
        .iter()
        .map(|tag| tag.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let event_type = event.event_type.to_ascii_lowercase();
    let crate_name = event.crate_name.to_ascii_lowercase();

    if tags
        .iter()
        .any(|tag| matches!(tag.as_str(), "boardroom" | "council" | "delegation"))
        || event_type.contains("boardroom")
        || event_type.contains("council")
    {
        "boardroom_council".to_owned()
    } else if tags
        .iter()
        .any(|tag| matches!(tag.as_str(), "human" | "obsidian"))
        || event_type.contains("obsidian")
    {
        "human_context".to_owned()
    } else if tags.iter().any(|tag| {
        matches!(
            tag.as_str(),
            "edge" | "provider" | "proxy" | "fleet" | "informant" | "runtime"
        )
    }) || matches!(crate_name.as_str(), "charon" | "warden" | "prometheus")
        && tags
            .iter()
            .any(|tag| matches!(tag.as_str(), "provider" | "proxy" | "route" | "fleet"))
    {
        "edge_runtime".to_owned()
    } else {
        "system_continuity".to_owned()
    }
}

fn contract_memory_record(
    memory_id: &str,
    event: &InformantEvent,
    memory_scope: &str,
    salience: f64,
) -> MemoryRecord {
    let agent = if event.crate_name.is_empty() {
        "mnemosyne"
    } else {
        event.crate_name.as_str()
    };
    let mut record = MemoryRecord::new(memory_id, MemoryKind::Episodic, agent, &event.content);
    record.salience = salience;
    let domain = memory_domain(event, memory_scope);
    record.extensions.insert(
        "memory_domain".into(),
        serde_json::to_value(domain).expect("memory domain serializes"),
    );
    record
        .extensions
        .insert("memory_scope".into(), serde_json::json!(memory_scope));
    record.extensions.insert(
        "evidence_class".into(),
        serde_json::json!(tag_value(&event.tags, "evidence_class").unwrap_or("inferred")),
    );
    if event.tags.iter().any(|tag| tag == "source_external") {
        record
            .extensions
            .insert("source_external".into(), serde_json::json!(true));
    }
    if let Some(source) = tag_value(&event.tags, "source_reference") {
        record
            .extensions
            .insert("source_reference".into(), serde_json::json!(source));
    }
    for key in ["source_expected", "source_observed"] {
        if let Some(value) = tag_value(&event.tags, key) {
            record
                .extensions
                .insert(key.to_owned(), serde_json::json!(value));
        }
    }
    if let Some(summary) = tag_value(&event.tags, "public_summary") {
        record
            .extensions
            .insert("public_summary".into(), serde_json::json!(summary));
    }
    for tag in &event.tags {
        if let Some((key, value)) = tag.split_once(':') {
            if matches!(key, "sensitivity.health" | "sensitivity.identity") {
                record
                    .extensions
                    .insert(key.to_owned(), serde_json::json!(value));
            }
        }
    }
    if domain == super::scope_policy::MemoryDomain::Personal
        && event.tags.iter().any(|tag| tag == "operator_authored")
    {
        record
            .extensions
            .insert("operator_authored".into(), serde_json::json!(true));
    }
    record
}

fn memory_domain(event: &InformantEvent, memory_scope: &str) -> super::scope_policy::MemoryDomain {
    use super::scope_policy::MemoryDomain;
    match tag_value(&event.tags, "memory_domain") {
        Some("personal") => MemoryDomain::Personal,
        Some("business") => MemoryDomain::Business,
        Some("system") => MemoryDomain::System,
        _ if matches!(memory_scope, "human_context" | "operator_persona") => MemoryDomain::Personal,
        _ if matches!(memory_scope, "boardroom_council" | "project_execution") => {
            MemoryDomain::Business
        }
        _ => MemoryDomain::System,
    }
}

fn tag_value<'a>(tags: &'a [String], key: &str) -> Option<&'a str> {
    tags.iter().find_map(|tag| {
        tag.strip_prefix(key)
            .and_then(|value| value.strip_prefix(':'))
    })
}
