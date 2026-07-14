// sigil: REPAIR
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;
use yaml_rust::{Yaml, YamlLoader};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SoterionMeta {
    pub sigil: Option<String>,
    pub realm: Option<String>,
    pub tags: Vec<String>,
    pub resonance: Option<f64>,
    pub triad_gate: Option<String>,
    pub joule_cost: Option<f64>,
    pub clearance: Option<String>,
    pub extra: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SoterionMachineSigil {
    #[serde(default)]
    pub sigil_id: Option<String>,
    pub sigil_code: String,
    #[serde(default)]
    pub sigil_tags: Vec<String>,
    #[serde(default)]
    pub sigil_severity: Option<String>,
    #[serde(default)]
    pub sigil_retention: Option<String>,
    #[serde(default)]
    pub sigil_source: Option<String>,
    #[serde(default)]
    pub sigil_render: Option<String>,
}

impl SoterionMachineSigil {
    pub fn new(
        sigil_code: impl Into<String>,
        sigil_tags: Vec<String>,
        sigil_severity: impl Into<String>,
        sigil_retention: impl Into<String>,
        sigil_source: impl Into<String>,
    ) -> Self {
        Self {
            sigil_id: None,
            sigil_code: sigil_code.into(),
            sigil_tags,
            sigil_severity: Some(sigil_severity.into()),
            sigil_retention: Some(sigil_retention.into()),
            sigil_source: Some(sigil_source.into()),
            sigil_render: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SoterionRegistry {
    pub version: Option<String>,
    pub status: Option<String>,
    pub principles: Vec<String>,
    pub agent_identity: HashMap<String, SoterionGlyphEntry>,
    pub state_signals: HashMap<String, SoterionGlyphEntry>,
    pub protocol_markers: HashMap<String, SoterionGlyphEntry>,
    pub flow_directives: HashMap<String, SoterionGlyphEntry>,
    pub confidence_levels: HashMap<String, SoterionConfidenceEntry>,
    pub file_lifecycle_sigils: HashMap<String, SoterionFileSigilEntry>,
    pub machine_sigils: HashMap<String, SoterionRegistryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SoterionGlyphEntry {
    pub glyph: Option<String>,
    pub code_point: Option<String>,
    pub meaning: Option<String>,
    pub role: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SoterionConfidenceEntry {
    pub glyph: Option<String>,
    pub code_point: Option<String>,
    pub min: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SoterionFileSigilEntry {
    pub glyph: Option<String>,
    pub code_point: Option<String>,
    pub meaning: Option<String>,
    pub retention: Option<String>,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SoterionRegistryEntry {
    pub id: Option<String>,
    pub source: Option<String>,
    pub tags: Vec<String>,
    pub severity: Option<String>,
    pub retention: Option<String>,
    pub render: HashMap<String, String>,
}

/// Global sigil dictionary (expandable)
pub static SIGIL_DICTIONARY: phf::Map<&'static str, &'static str> = phf::phf_map! {
    "𓀀" => "Command / CEO",
    "𓁿" => "Knowledge / Memory",
    "𓂀" => "Observation / Ingestion",
    "𓆣" => "Energy / JouleWork",
    "𓋹" => "Human Override",
    "𓃭" => "Guardian / WARDEN",
    "𓆓" => "Balance / Love Equation",
    "𓊝" => "Gate / Triad",
    "𓅃" => "Messenger / Hermes",
    "𓁷" => "Keeper / Hades",
};

/// Simple in-memory index for fast lookup
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SoterionIndex {
    pub by_sigil: HashMap<String, Vec<String>>,
    pub by_realm: HashMap<String, Vec<String>>,
    pub by_resonance: Vec<(String, f64)>,
    pub by_tag: HashMap<String, Vec<String>>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub total_entries: usize,
}

impl SoterionIndex {
    pub fn new() -> Self {
        Self {
            last_updated: chrono::Utc::now(),
            ..Default::default()
        }
    }

    pub fn add(&mut self, path: String, meta: &SoterionMeta) {
        if let Some(sigil) = &meta.sigil {
            self.by_sigil
                .entry(sigil.clone())
                .or_default()
                .push(path.clone());
        }
        if let Some(realm) = &meta.realm {
            self.by_realm
                .entry(realm.clone())
                .or_default()
                .push(path.clone());
        }
        if let Some(res) = meta.resonance {
            self.by_resonance.push((path.clone(), res));
            self.by_resonance
                .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        }
        for tag in &meta.tags {
            self.by_tag
                .entry(tag.clone())
                .or_default()
                .push(path.clone());
        }
        self.last_updated = chrono::Utc::now();
    }

    pub fn find_by_sigil(&self, sigil: &str) -> Vec<String> {
        self.by_sigil.get(sigil).cloned().unwrap_or_default()
    }

    pub fn find_by_realm(&self, realm: &str) -> Vec<String> {
        self.by_realm.get(realm).cloned().unwrap_or_default()
    }

    pub fn find_high_resonance(&self, min_res: f64) -> Vec<String> {
        self.by_resonance
            .iter()
            .filter(|(_, res)| *res >= min_res)
            .map(|(path, _)| path.clone())
            .collect()
    }

    pub fn find_by_tag(&self, tag: &str) -> Vec<String> {
        self.by_tag.get(tag).cloned().unwrap_or_default()
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        if !path.as_ref().exists() {
            return Ok(Self::new());
        }
        let content = fs::read_to_string(path)?;
        let index: SoterionIndex = serde_json::from_str(&content)?;
        Ok(index)
    }

    pub fn scan_directory(&mut self, dir: impl AsRef<Path>) -> Result<usize> {
        let mut count = 0;
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                count += self.scan_directory(&path)?;
            } else if path.extension().is_some_and(|e| e == "md" || e == "jsonl") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(Some(meta)) = parse_header_from_content(&content) {
                        self.add(path.to_string_lossy().to_string(), &meta);
                        count += 1;
                    }
                }
            }
        }
        Ok(count)
    }

    pub fn persist_if_changed(
        &self,
        path: impl AsRef<Path>,
        last_save: &chrono::DateTime<chrono::Utc>,
    ) -> Result<bool> {
        if self.last_updated > *last_save {
            self.save(path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// Parse Soterion header from file content
pub fn parse_header_from_content(content: &str) -> Result<Option<SoterionMeta>> {
    if let Some(start) = content.find("---") {
        if let Some(end) = content[start + 3..].find("---") {
            let header_str = &content[start + 3..start + 3 + end];
            let docs = YamlLoader::load_from_str(header_str).context("YAML parse failed")?;

            if docs.is_empty() {
                return Ok(None);
            }

            if let Yaml::Hash(map) = &docs[0] {
                if let Some(Yaml::Hash(s_map)) = map.get(&Yaml::String("soterion".to_string())) {
                    let meta = SoterionMeta {
                        sigil: s_map
                            .get(&Yaml::String("sigil".to_string()))
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        realm: s_map
                            .get(&Yaml::String("realm".to_string()))
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        tags: s_map
                            .get(&Yaml::String("tags".to_string()))
                            .and_then(|v| v.as_vec())
                            .map_or(vec![], |vec| {
                                vec.iter()
                                    .filter_map(|t| t.as_str().map(String::from))
                                    .collect()
                            }),
                        resonance: s_map
                            .get(&Yaml::String("resonance".to_string()))
                            .and_then(|v| v.as_f64()),
                        triad_gate: s_map
                            .get(&Yaml::String("triad_gate".to_string()))
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        joule_cost: s_map
                            .get(&Yaml::String("jw_cost".to_string()))
                            .and_then(|v| v.as_f64()),
                        clearance: s_map
                            .get(&Yaml::String("clearance".to_string()))
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        ..Default::default()
                    };

                    return Ok(Some(meta));
                }
            }
        }
    }
    Ok(None)
}

pub fn parse_header_from_path(path: impl AsRef<Path>) -> Result<Option<SoterionMeta>> {
    let content = fs::read_to_string(path)?;
    parse_header_from_content(&content)
}

pub fn default_soterion_registry_path() -> String {
    std::env::var("ANNUNIMAS_SOTERION_REGISTRY_PATH").unwrap_or_else(|_| {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("meta/soterion_sigils.yaml")
            .display()
            .to_string()
    })
}

pub fn load_default_soterion_registry() -> Result<SoterionRegistry> {
    load_soterion_registry(default_soterion_registry_path())
}

pub fn load_soterion_registry(path: impl AsRef<Path>) -> Result<SoterionRegistry> {
    let content = fs::read_to_string(path.as_ref())
        .with_context(|| format!("failed reading {}", path.as_ref().display()))?;
    let docs =
        YamlLoader::load_from_str(&content).context("Soterion registry YAML parse failed")?;
    let Some(root) = docs.first() else {
        return Ok(SoterionRegistry::default());
    };

    Ok(SoterionRegistry {
        version: yaml_string(root, "version"),
        status: yaml_string(root, "status"),
        principles: yaml_string_list(root, "principles"),
        agent_identity: yaml_glyph_map(root, "agent_identity"),
        state_signals: yaml_glyph_map(root, "state_signals"),
        protocol_markers: yaml_glyph_map(root, "protocol_markers"),
        flow_directives: yaml_glyph_map(root, "flow_directives"),
        confidence_levels: yaml_confidence_map(root, "confidence_levels"),
        file_lifecycle_sigils: yaml_file_sigil_map(root, "file_lifecycle_sigils"),
        machine_sigils: yaml_registry_map(root, "machine_sigils"),
    })
}

pub fn file_sigil_name_from_registry(value: &str) -> Option<String> {
    let needle = value.trim();
    if needle.is_empty() {
        return None;
    }
    let registry = cached_soterion_registry();
    for (name, entry) in &registry.file_lifecycle_sigils {
        if name.eq_ignore_ascii_case(needle)
            || entry.glyph.as_deref() == Some(needle)
            || entry
                .code_point
                .as_deref()
                .is_some_and(|code_point| code_point.eq_ignore_ascii_case(needle))
            || entry
                .aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(needle))
        {
            return Some(name.to_string());
        }
    }
    None
}

pub fn machine_sigil_from_registry(code: &str) -> Option<SoterionMachineSigil> {
    let registry = cached_soterion_registry();
    let entry = registry.machine_sigils.get(code)?;
    Some(SoterionMachineSigil {
        sigil_id: entry.id.clone(),
        sigil_code: code.to_string(),
        sigil_tags: entry.tags.clone(),
        sigil_severity: entry.severity.clone(),
        sigil_retention: entry.retention.clone(),
        sigil_source: entry.source.clone(),
        sigil_render: render_signature(registry, &entry.render),
    })
}

pub fn machine_sigil_or_default(
    code: &str,
    fallback_tags: Vec<String>,
    fallback_severity: impl Into<String>,
    fallback_retention: impl Into<String>,
    fallback_source: impl Into<String>,
) -> SoterionMachineSigil {
    machine_sigil_from_registry(code).unwrap_or_else(|| {
        SoterionMachineSigil::new(
            code,
            fallback_tags,
            fallback_severity,
            fallback_retention,
            fallback_source,
        )
    })
}

fn cached_soterion_registry() -> &'static SoterionRegistry {
    static REGISTRY: OnceLock<SoterionRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| load_default_soterion_registry().unwrap_or_default())
}

fn render_signature(
    registry: &SoterionRegistry,
    render: &HashMap<String, String>,
) -> Option<String> {
    let mut signature = String::new();
    if let Some(agent) = render.get("agent") {
        if let Some(entry) = registry.agent_identity.get(agent) {
            if let Some(glyph) = &entry.glyph {
                signature.push_str(glyph);
            }
        }
    }
    if let Some(state) = render.get("state") {
        if let Some(entry) = registry.state_signals.get(state) {
            if let Some(glyph) = &entry.glyph {
                signature.push_str(glyph);
            }
        }
    }
    if let Some(flow) = render.get("flow") {
        if let Some(entry) = registry.flow_directives.get(flow) {
            if let Some(glyph) = &entry.glyph {
                signature.push_str(glyph);
            }
        }
    }
    if signature.is_empty() {
        None
    } else {
        Some(signature)
    }
}

fn yaml_string(root: &Yaml, key: &str) -> Option<String> {
    root[key].as_str().map(ToString::to_string)
}

fn yaml_string_list(root: &Yaml, key: &str) -> Vec<String> {
    root[key]
        .as_vec()
        .map(|items| {
            items
                .iter()
                .filter_map(|value| value.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn yaml_glyph_map(root: &Yaml, key: &str) -> HashMap<String, SoterionGlyphEntry> {
    let mut out = HashMap::new();
    let Some(hash) = root[key].as_hash() else {
        return out;
    };
    for (entry_key, entry_value) in hash {
        let Some(name) = entry_key.as_str() else {
            continue;
        };
        out.insert(
            name.to_string(),
            SoterionGlyphEntry {
                glyph: entry_value["glyph"].as_str().map(ToString::to_string),
                code_point: entry_value["code_point"].as_str().map(ToString::to_string),
                meaning: entry_value["meaning"].as_str().map(ToString::to_string),
                role: entry_value["role"].as_str().map(ToString::to_string),
                description: entry_value["description"].as_str().map(ToString::to_string),
            },
        );
    }
    out
}

fn yaml_confidence_map(root: &Yaml, key: &str) -> HashMap<String, SoterionConfidenceEntry> {
    let mut out = HashMap::new();
    let Some(hash) = root[key].as_hash() else {
        return out;
    };
    for (entry_key, entry_value) in hash {
        let Some(name) = entry_key.as_str() else {
            continue;
        };
        out.insert(
            name.to_string(),
            SoterionConfidenceEntry {
                glyph: entry_value["glyph"].as_str().map(ToString::to_string),
                code_point: entry_value["code_point"].as_str().map(ToString::to_string),
                min: entry_value["min"].as_f64(),
            },
        );
    }
    out
}

fn yaml_file_sigil_map(root: &Yaml, key: &str) -> HashMap<String, SoterionFileSigilEntry> {
    let mut out = HashMap::new();
    let Some(hash) = root[key].as_hash() else {
        return out;
    };
    for (entry_key, entry_value) in hash {
        let Some(name) = entry_key.as_str() else {
            continue;
        };
        out.insert(
            name.to_string(),
            SoterionFileSigilEntry {
                glyph: entry_value["glyph"].as_str().map(ToString::to_string),
                code_point: entry_value["code_point"].as_str().map(ToString::to_string),
                meaning: entry_value["meaning"].as_str().map(ToString::to_string),
                retention: entry_value["retention"].as_str().map(ToString::to_string),
                aliases: entry_value["aliases"]
                    .as_vec()
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|value| value.as_str().map(ToString::to_string))
                            .collect()
                    })
                    .unwrap_or_default(),
            },
        );
    }
    out
}

fn yaml_registry_map(root: &Yaml, key: &str) -> HashMap<String, SoterionRegistryEntry> {
    let mut out = HashMap::new();
    let Some(hash) = root[key].as_hash() else {
        return out;
    };
    for (entry_key, entry_value) in hash {
        let Some(name) = entry_key.as_str() else {
            continue;
        };
        out.insert(
            name.to_string(),
            SoterionRegistryEntry {
                id: yaml_scalar_string(&entry_value["id"]),
                source: entry_value["source"].as_str().map(ToString::to_string),
                tags: entry_value["tags"]
                    .as_vec()
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|value| value.as_str().map(ToString::to_string))
                            .collect()
                    })
                    .unwrap_or_default(),
                severity: entry_value["severity"].as_str().map(ToString::to_string),
                retention: entry_value["retention"].as_str().map(ToString::to_string),
                render: yaml_string_hash(&entry_value["render"]),
            },
        );
    }
    out
}

fn yaml_scalar_string(value: &Yaml) -> Option<String> {
    value
        .as_str()
        .map(ToString::to_string)
        .or_else(|| value.as_i64().map(|v| v.to_string()))
}

fn yaml_string_hash(value: &Yaml) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Some(hash) = value.as_hash() else {
        return out;
    };
    for (key, entry_value) in hash {
        let Some(k) = key.as_str() else {
            continue;
        };
        if let Some(v) = yaml_scalar_string(entry_value) {
            out.insert(k.to_string(), v);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        file_sigil_name_from_registry, machine_sigil_or_default, parse_header_from_content,
        render_signature, SoterionGlyphEntry, SoterionIndex, SoterionRegistry,
    };
    use chrono::{Duration, Utc};
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "annunimas-core-soterion-{label}-{}",
            Uuid::new_v4()
        ))
    }

    #[test]
    fn scan_directory_load_and_persist_round_trip_index() {
        let dir = temp_dir("scan");
        fs::create_dir_all(&dir).expect("create dir");
        let nested = dir.join("nested");
        fs::create_dir_all(&nested).expect("create nested dir");

        let doc = nested.join("entry.md");
        fs::write(
            &doc,
            r#"---
soterion:
  sigil: "𓁿"
  realm: "knowledge"
  tags: ["alpha", "beta"]
  resonance: 0.91
---
body
"#,
        )
        .expect("write doc");

        let mut index = SoterionIndex::new();
        let count = index.scan_directory(&dir).expect("scan directory");
        assert_eq!(count, 1);

        let path = doc.to_string_lossy().to_string();
        assert!(index.find_by_sigil("𓁿").contains(&path));
        assert!(index.find_by_realm("knowledge").contains(&path));
        assert!(index.find_by_tag("alpha").contains(&path));
        assert!(index.find_high_resonance(0.9).contains(&path));

        let persisted = dir.join("index.json");
        index.save(&persisted).expect("save index");
        let loaded = SoterionIndex::load(&persisted).expect("load index");
        assert!(loaded.find_by_tag("beta").contains(&path));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn persist_if_changed_only_writes_newer_index() {
        let dir = temp_dir("persist");
        fs::create_dir_all(&dir).expect("create dir");
        let persisted = dir.join("index.json");

        let mut index = SoterionIndex::new();
        index.add(
            "memory/book.md".to_string(),
            &parse_header_from_content(
                r#"---
soterion:
  sigil: "𓁿"
  realm: "knowledge"
---
"#,
            )
            .expect("parse")
            .expect("meta"),
        );

        let stale = Utc::now() - Duration::seconds(5);
        assert!(index
            .persist_if_changed(&persisted, &stale)
            .expect("persist changed"));

        let newer = Utc::now() + Duration::seconds(5);
        assert!(!index
            .persist_if_changed(&persisted, &newer)
            .expect("skip unchanged"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn render_signature_concatenates_without_intermediate_glyph_vec() {
        let mut registry = SoterionRegistry::default();
        registry.agent_identity.insert(
            "HERMES".to_string(),
            SoterionGlyphEntry {
                glyph: Some("🜁".to_string()),
                ..Default::default()
            },
        );
        registry.state_signals.insert(
            "OK".to_string(),
            SoterionGlyphEntry {
                glyph: Some("◆".to_string()),
                ..Default::default()
            },
        );
        registry.flow_directives.insert(
            "DELIVER".to_string(),
            SoterionGlyphEntry {
                glyph: Some("◀".to_string()),
                ..Default::default()
            },
        );

        let render = HashMap::from([
            ("agent".to_string(), "HERMES".to_string()),
            ("state".to_string(), "OK".to_string()),
            ("flow".to_string(), "DELIVER".to_string()),
        ]);

        assert_eq!(render_signature(&registry, &render).as_deref(), Some("🜁◆◀"));
    }

    #[test]
    fn machine_sigil_or_default_preserves_fallback_contract() {
        let fallback = machine_sigil_or_default(
            "SG_UNKNOWN_TEST",
            vec!["core".to_string(), "fallback".to_string()],
            "info",
            "summarize",
            "core",
        );

        assert_eq!(fallback.sigil_code, "SG_UNKNOWN_TEST");
        assert_eq!(fallback.sigil_tags, vec!["core", "fallback"]);
        assert_eq!(fallback.sigil_severity.as_deref(), Some("info"));
        assert_eq!(fallback.sigil_retention.as_deref(), Some("summarize"));
        assert_eq!(fallback.sigil_source.as_deref(), Some("core"));
    }

    #[test]
    fn file_sigil_lookup_resolves_unicode_and_legacy_aliases() {
        assert_eq!(file_sigil_name_from_registry("🪙").as_deref(), Some("COIN"));
        assert_eq!(
            file_sigil_name_from_registry("U+1FA99").as_deref(),
            Some("COIN")
        );
        assert_eq!(
            file_sigil_name_from_registry("SCROLL").as_deref(),
            Some("SCROLL")
        );
    }
}
