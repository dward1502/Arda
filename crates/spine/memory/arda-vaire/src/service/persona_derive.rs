use super::{store, MnemosyneService};
use crate::persona::derive::{derive_projection, MOOD_KEY, TRAIT_EVIDENCE_KEY, VALUE_EVIDENCE_KEY};
use crate::persona::types::PersonaProjection;
use crate::schema::{PERSONA_SCHEMA_ID, PERSONA_SCHEMA_VERSION};
use arda_core::contract::{MemoryKind, MemoryRecord, MemoryState};
use arda_core::error::Result;
use chrono::{DateTime, Utc};

impl MnemosyneService {
    /// Derive and atomically replace the latest persona projection for `actor`.
    ///
    /// Canonical contract records are read when the existing dual-write root is
    /// configured. No scheduler or second store is introduced; consolidation
    /// invokes this method as one more projection step.
    pub fn derive_identity_summary(
        &self,
        actor: &str,
        since: Option<DateTime<Utc>>,
    ) -> Result<PersonaProjection> {
        let records = self.read_persona_source_records(since)?;
        let projection = derive_projection(actor, &records, Utc::now());
        self.write_persona_projection(actor, &projection)?;
        self.write_persona_markdown_projection(actor, &projection, &records)?;
        Ok(projection)
    }

    /// Read the latest cached projection without recomputing it.
    pub fn persona_projection(&self, actor: &str) -> Result<Option<PersonaProjection>> {
        let path = self.persona_projection_path(actor);
        if !path.exists() {
            return Ok(None);
        }
        let record: MemoryRecord = serde_json::from_slice(&std::fs::read(path)?)?;
        Ok(Some(projection_from_record(&record)?))
    }

    fn read_persona_source_records(
        &self,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<MemoryRecord>> {
        let Some(contract_root) = &self.contract_memory_root else {
            return Ok(Vec::new());
        };
        let mut records = Vec::new();
        for path in store::walk_dir(&contract_root.join("episodic"))? {
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let Ok(record) = serde_json::from_slice::<MemoryRecord>(&std::fs::read(&path)?) else {
                tracing::warn!(path = %path.display(), "skipping malformed persona source record");
                continue;
            };
            if since.is_some_and(|cutoff| record.last_seen_at < cutoff) {
                continue;
            }
            records.push(record);
        }
        Ok(records)
    }

    fn write_persona_projection(&self, actor: &str, projection: &PersonaProjection) -> Result<()> {
        let mut record = MemoryRecord::new(
            format!("persona_identity_{}", safe_actor(actor)),
            MemoryKind::Semantic,
            actor,
            format!("Derived persona identity projection for {actor}"),
        );
        record.state = MemoryState::Promoted;
        record.created_at = projection.derived_at;
        record.last_seen_at = projection.derived_at;
        record.evidence_count = projection
            .traits
            .iter()
            .map(|persona_trait| persona_trait.evidence_count as u64)
            .sum::<u64>()
            .min(u64::from(u32::MAX)) as u32;
        record.extensions.insert(
            "persona.schema_version".to_owned(),
            serde_json::json!(PERSONA_SCHEMA_VERSION),
        );
        record.extensions.insert(
            "persona.traits".to_owned(),
            serde_json::to_value(&projection.traits)?,
        );
        record.extensions.insert(
            "persona.mood".to_owned(),
            serde_json::to_value(&projection.mood)?,
        );
        record.extensions.insert(
            "persona.mood_summary".to_owned(),
            serde_json::to_value(&projection.mood_summary)?,
        );
        record.extensions.insert(
            "persona.value_evidence".to_owned(),
            serde_json::to_value(&projection.value_evidence)?,
        );
        record.extensions.insert(
            "derivation".to_owned(),
            serde_json::json!("persona_identity"),
        );
        store::write_atomic_json(&self.persona_projection_path(actor), &record)
    }

    fn persona_projection_path(&self, actor: &str) -> std::path::PathBuf {
        self.persona_root
            .join(format!("{}.json", safe_actor(actor)))
    }

    fn write_persona_markdown_projection(
        &self,
        actor: &str,
        projection: &PersonaProjection,
        records: &[MemoryRecord],
    ) -> Result<()> {
        let Some(human_root) = &self.human_projection_root else {
            return Ok(());
        };
        let path = human_root
            .join("personality")
            .join(safe_actor(actor))
            .join(format!("{}.md", projection.derived_at.format("%Y-%m-%d")));
        let markdown = render_persona_markdown(actor, projection, records);
        store::write_atomic(&path, markdown.as_bytes())
    }
}

fn render_persona_markdown(
    actor: &str,
    projection: &PersonaProjection,
    records: &[MemoryRecord],
) -> String {
    let actor_slug = safe_actor(actor);
    let mut output = format!(
        "---\nschema: {PERSONA_SCHEMA_ID}\nactor: {actor_slug}\nderived_at: {}\n---\n\n# {} Persona\n\n> Generated from canonical Vairë memory evidence. This file is a replaceable human-readable projection, not a source of truth.\n\n## Traits\n",
        projection.derived_at.to_rfc3339(),
        markdown_inline(actor),
    );
    if projection.traits.is_empty() {
        output.push_str("\n_No promoted traits yet._\n");
    } else {
        for persona_trait in &projection.traits {
            let label = markdown_inline(&persona_trait.label);
            let rendered_label = if persona_trait.stale {
                format!("~~{label}~~ (stale)")
            } else {
                format!("**{label}**")
            };
            output.push_str(&format!(
                "\n- {} — confidence `{:.2}`; evidence `{}`\n",
                rendered_label, persona_trait.confidence, persona_trait.evidence_count,
            ));
        }
    }

    output.push_str("\n## Current Mood\n");
    match &projection.mood_summary {
        Some(summary) => output.push_str(&format!(
            "\n- Valence: `{:.3}`\n- Samples: `{}`\n- Updated: `{}`\n",
            summary.weighted_valence,
            summary.sample_count,
            summary.as_of.to_rfc3339(),
        )),
        None => output.push_str("\n_Neutral / no recent mood evidence._\n"),
    }

    output.push_str("\n## Recent Evidence\n");
    let recent = recent_persona_evidence(actor, records);
    if recent.is_empty() {
        output.push_str("\n_No eligible persona evidence._\n");
    } else {
        for record in recent {
            output.push_str(&format!(
                "\n- [[{}]] — `{}`\n",
                markdown_inline(&record.id),
                record.last_seen_at.to_rfc3339(),
            ));
        }
    }
    output.push('\n');
    output
}

fn recent_persona_evidence<'a>(actor: &str, records: &'a [MemoryRecord]) -> Vec<&'a MemoryRecord> {
    let mut eligible = records
        .iter()
        .filter(|record| record.agent.eq_ignore_ascii_case(actor))
        .filter(|record| matches!(record.state, MemoryState::Active | MemoryState::Promoted))
        .filter(|record| {
            [TRAIT_EVIDENCE_KEY, MOOD_KEY, VALUE_EVIDENCE_KEY]
                .iter()
                .any(|key| record.extensions.contains_key(*key))
        })
        .collect::<Vec<_>>();
    eligible.sort_by(|left, right| {
        right
            .last_seen_at
            .cmp(&left.last_seen_at)
            .then_with(|| left.id.cmp(&right.id))
    });
    eligible.truncate(5);
    eligible
}

fn markdown_inline(value: &str) -> String {
    value
        .replace(['\r', '\n'], " ")
        .replace("[[", "[")
        .replace("]]", "]")
}

fn projection_from_record(record: &MemoryRecord) -> Result<PersonaProjection> {
    let value = |key: &str| {
        record
            .extensions
            .get(key)
            .cloned()
            .unwrap_or(serde_json::Value::Null)
    };
    Ok(PersonaProjection {
        traits: serde_json::from_value(value("persona.traits"))?,
        mood: serde_json::from_value(value("persona.mood"))?,
        mood_summary: serde_json::from_value(value("persona.mood_summary"))?,
        value_evidence: serde_json::from_value(value("persona.value_evidence"))?,
        derived_at: record.last_seen_at,
    })
}

fn safe_actor(actor: &str) -> String {
    let sanitized = actor
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    sanitized.trim_matches('_').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persona::derive::TRAIT_EVIDENCE_KEY;
    use crate::persona::types::{PersonaTrait, PersonaTraitEvidence};
    use chrono::Duration;
    use tempfile::tempdir;

    #[test]
    fn service_derivation_replaces_one_projection_record() {
        let directory = tempdir().expect("tempdir");
        let contract_root = directory.path().join("contract");
        let human_root = directory.path().join("human");
        let episodic_root = contract_root.join("episodic");
        std::fs::create_dir_all(&episodic_root).expect("episodic root");
        for index in 0..3 {
            let mut record = MemoryRecord::new(
                format!("evidence-{index}"),
                MemoryKind::Episodic,
                "arandur",
                "explicit trait evidence",
            );
            record.created_at = Utc::now() - Duration::days(index);
            record.last_seen_at = record.created_at;
            record.extensions.insert(
                TRAIT_EVIDENCE_KEY.to_owned(),
                serde_json::to_value(vec![PersonaTraitEvidence {
                    id: "direct".to_owned(),
                    label: "Direct".to_owned(),
                }])
                .expect("evidence value"),
            );
            store::write_atomic_json(
                &episodic_root.join(format!("evidence-{index}.json")),
                &record,
            )
            .expect("source record");
        }

        let service = MnemosyneService::new(directory.path().join("mnemosyne"))
            .expect("service")
            .with_contract_memory_root(contract_root)
            .with_human_projection_root(human_root.clone());
        service.consolidate(24).expect("consolidation derivation");
        let first = service
            .persona_projection("arandur")
            .expect("read first projection")
            .expect("first projection");
        let markdown_path = human_root
            .join("personality/arandur")
            .join(format!("{}.md", first.derived_at.format("%Y-%m-%d")));
        let first_markdown = std::fs::read_to_string(&markdown_path).expect("persona markdown");
        assert!(first_markdown.contains("Direct"));
        assert!(first_markdown.contains("0.30"));
        assert!(first_markdown.contains("[[evidence-0]]"));

        std::fs::write(&markdown_path, "append sentinel").expect("replace generated note");
        service.consolidate(24).expect("repeat consolidation");
        let second = service
            .persona_projection("arandur")
            .expect("read second projection")
            .expect("second projection");
        let second_markdown =
            std::fs::read_to_string(&markdown_path).expect("regenerated markdown");
        assert!(!second_markdown.contains("append sentinel"));

        assert_eq!(first.traits, second.traits);
        assert_eq!(first.traits.len(), 1);
        assert_eq!(first.traits[0].confidence, 0.3);
        assert_eq!(
            std::fs::read_dir(directory.path().join("mnemosyne/persona"))
                .expect("persona root")
                .count(),
            1
        );
        assert_eq!(
            service
                .persona_projection("arandur")
                .expect("read projection")
                .expect("projection")
                .traits,
            second.traits
        );
    }

    #[test]
    fn markdown_visually_marks_stale_traits() {
        let now = Utc::now();
        let projection = PersonaProjection {
            traits: vec![PersonaTrait {
                trait_id: "direct".to_owned(),
                label: "Direct".to_owned(),
                evidence_count: 3,
                confidence: 0.3,
                first_seen: now - Duration::days(63),
                last_seen: now - Duration::days(61),
                last_reinforced_by: Some("evidence-3".to_owned()),
                stale: true,
            }],
            derived_at: now,
            ..PersonaProjection::default()
        };

        let markdown = render_persona_markdown("arandur", &projection, &[]);

        assert!(markdown.contains("~~Direct~~ (stale)"));
    }
}
