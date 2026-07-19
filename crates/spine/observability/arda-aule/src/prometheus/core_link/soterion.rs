#![cfg(feature = "full-cli")]
use super::CORE_STATE_SCHEMA_VERSION;
use annunimas_core::{load_default_soterion_registry, SoterionRegistry, SoterionRegistryEntry};
use chrono::Utc;
use serde_json::{json, Map, Value};
use std::cmp::Ordering;
use std::fs;
use std::path::Path;

pub(super) fn write_soterion_render_projection(core_root: &Path) {
    let snapshot_path = core_root
        .join("state")
        .join("soterion_render_contract.json");
    if let Some(parent) = snapshot_path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }

    let registry = load_default_soterion_registry().unwrap_or_default();
    let mut machine_sigils = Map::new();

    for (code, entry) in sorted_machine_sigils(&registry) {
        let render = render_projection(&registry, &entry.render);
        machine_sigils.insert(
            code.clone(),
            json!({
                "id": entry.id,
                "source": entry.source,
                "tags": entry.tags,
                "severity": entry.severity,
                "retention": entry.retention,
                "render": entry.render,
                "glyphs": render,
            }),
        );
    }

    let source_defaults = source_defaults_projection(&registry);

    let snapshot = json!({
        "schema_version": CORE_STATE_SCHEMA_VERSION,
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "soterion_render_projection",
        "registry_version": registry.version,
        "registry_status": registry.status,
        "principles": registry.principles,
        "glyph_groups": {
            "agent_identity": registry.agent_identity,
            "state_signals": registry.state_signals,
            "protocol_markers": registry.protocol_markers,
            "flow_directives": registry.flow_directives,
            "confidence_levels": registry.confidence_levels,
        },
        "machine_sigils": machine_sigils,
        "source_defaults": source_defaults,
        "arda_hints": {
            "primary_panel": "soterion_language",
            "render_contract_ready": true,
            "machine_semantics_first": true
        }
    });

    if let Some(existing) = fs::read_to_string(&snapshot_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
    {
        if soterion_render_projection_semantically_equal(&existing, &snapshot) {
            return;
        }
    }

    let _ = fs::write(
        snapshot_path,
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
    );
}

fn soterion_render_projection_semantically_equal(left: &Value, right: &Value) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    if let Some(obj) = left.as_object_mut() {
        obj.remove("generated_at_utc");
    }
    if let Some(obj) = right.as_object_mut() {
        obj.remove("generated_at_utc");
    }
    left == right
}

fn sorted_machine_sigils(registry: &SoterionRegistry) -> Vec<(&String, &SoterionRegistryEntry)> {
    let mut entries = registry.machine_sigils.iter().collect::<Vec<_>>();
    entries.sort_by(|(left_code, left_entry), (right_code, right_entry)| {
        compare_soterion_ids(left_entry.id.as_deref(), right_entry.id.as_deref())
            .then_with(|| left_code.cmp(right_code))
    });
    entries
}

fn compare_soterion_ids(left: Option<&str>, right: Option<&str>) -> Ordering {
    match (parse_soterion_id(left), parse_soterion_id(right)) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => left.cmp(&right),
    }
}

fn parse_soterion_id(value: Option<&str>) -> Option<u64> {
    let value = value?.trim();
    let hex = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"));
    if let Some(hex) = hex {
        return u64::from_str_radix(hex, 16).ok();
    }
    value.parse::<u64>().ok()
}

fn source_defaults_projection(registry: &SoterionRegistry) -> Map<String, Value> {
    let mut source_defaults = Map::new();

    for (_, entry) in sorted_machine_sigils(registry) {
        if let Some(source) = &entry.source {
            source_defaults.entry(source.clone()).or_insert_with(|| {
                json!({
                    "source": source,
                    "glyphs": render_projection(registry, &entry.render),
                })
            });
        }
    }

    source_defaults
}

fn render_projection(
    registry: &SoterionRegistry,
    render: &std::collections::HashMap<String, String>,
) -> Value {
    let agent_key = render.get("agent").cloned();
    let state_key = render.get("state").cloned();
    let flow_key = render.get("flow").cloned();

    let signature = [
        agent_key
            .as_ref()
            .and_then(|key| registry.agent_identity.get(key))
            .and_then(|entry| entry.glyph.clone()),
        state_key
            .as_ref()
            .and_then(|key| registry.state_signals.get(key))
            .and_then(|entry| entry.glyph.clone()),
        flow_key
            .as_ref()
            .and_then(|key| registry.flow_directives.get(key))
            .and_then(|entry| entry.glyph.clone()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("");

    json!({
        "agent": agent_key.as_ref().and_then(|key| registry.agent_identity.get(key)).and_then(|entry| entry.glyph.clone()),
        "state": state_key.as_ref().and_then(|key| registry.state_signals.get(key)).and_then(|entry| entry.glyph.clone()),
        "flow": flow_key.as_ref().and_then(|key| registry.flow_directives.get(key)).and_then(|entry| entry.glyph.clone()),
        "signature": signature
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use annunimas_core::SoterionGlyphEntry;
    use std::collections::HashMap;

    #[test]
    fn source_defaults_use_numeric_id_order_for_mixed_hex_and_decimal_ids() {
        let mut registry = SoterionRegistry::default();
        registry.agent_identity.insert(
            "TEST".to_string(),
            SoterionGlyphEntry {
                glyph: Some("T".to_string()),
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
        registry.state_signals.insert(
            "FAIL".to_string(),
            SoterionGlyphEntry {
                glyph: Some("⚠".to_string()),
                ..Default::default()
            },
        );

        registry.machine_sigils.insert(
            "TEST_FAIL".to_string(),
            SoterionRegistryEntry {
                id: Some("512".to_string()),
                source: Some("test_source".to_string()),
                render: HashMap::from([
                    ("agent".to_string(), "TEST".to_string()),
                    ("state".to_string(), "FAIL".to_string()),
                ]),
                ..Default::default()
            },
        );
        registry.machine_sigils.insert(
            "TEST_OK".to_string(),
            SoterionRegistryEntry {
                id: Some("0x0100".to_string()),
                source: Some("test_source".to_string()),
                render: HashMap::from([
                    ("agent".to_string(), "TEST".to_string()),
                    ("state".to_string(), "OK".to_string()),
                ]),
                ..Default::default()
            },
        );

        let defaults = source_defaults_projection(&registry);
        let default_state = defaults
            .get("test_source")
            .and_then(|value| value.get("glyphs"))
            .and_then(|value| value.get("state"))
            .and_then(Value::as_str);
        let default_signature = defaults
            .get("test_source")
            .and_then(|value| value.get("glyphs"))
            .and_then(|value| value.get("signature"))
            .and_then(Value::as_str);

        assert_eq!(default_state, Some("◆"));
        assert_eq!(default_signature, Some("T◆"));
    }

    #[test]
    fn source_defaults_match_lowest_default_registry_ids() {
        let registry = load_default_soterion_registry().unwrap_or_default();
        let defaults = source_defaults_projection(&registry);

        let charon_signature = defaults
            .get("charon")
            .and_then(|value| value.get("glyphs"))
            .and_then(|value| value.get("signature"))
            .and_then(Value::as_str);
        let hermes_signature = defaults
            .get("hermes")
            .and_then(|value| value.get("glyphs"))
            .and_then(|value| value.get("signature"))
            .and_then(Value::as_str);
        let hades_signature = defaults
            .get("hades")
            .and_then(|value| value.get("glyphs"))
            .and_then(|value| value.get("signature"))
            .and_then(Value::as_str);

        assert_eq!(charon_signature, Some("☿◆►"));
        assert_eq!(hermes_signature, Some("🜁◆◀"));
        assert_eq!(hades_signature, Some("▽"));
    }

    #[test]
    fn soterion_render_projection_semantic_equality_ignores_generated_at_only() {
        let left = json!({
            "schema_version": CORE_STATE_SCHEMA_VERSION,
            "generated_at_utc": "2026-05-29T01:00:00Z",
            "authority": "soterion_render_projection",
            "machine_sigils": {"SG_TEST": {"id": "0x0001"}}
        });
        let right = json!({
            "schema_version": CORE_STATE_SCHEMA_VERSION,
            "generated_at_utc": "2026-05-29T02:00:00Z",
            "authority": "soterion_render_projection",
            "machine_sigils": {"SG_TEST": {"id": "0x0001"}}
        });

        assert!(soterion_render_projection_semantically_equal(&left, &right));
    }
}
