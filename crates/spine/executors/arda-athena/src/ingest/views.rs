// sigil: REPAIR
//
// Knowledge-view and lifecycle surface: human-source markdown export,
// machine index events, deep graph events, and bounded background signal
// emission into HADES, WARDEN, and PLUTUS.

use arda_core::error::Result;
use arda_core::spawn_bounded_background;
use arda_plutus::{JouleWorkUnit, PlutusService};
use chrono::Utc;
use std::fs;

use super::{
    normalize_graph_token, AthenaStore, DeepBookEntry, DigestEvent, IngestRecord, ShallowAnalysis,
};

impl AthenaStore {
    pub(super) fn sync_knowledge_views(
        &self,
        source_id: &str,
        ingest: Option<&IngestRecord>,
        shallow: Option<&ShallowAnalysis>,
        deep: Option<&DeepBookEntry>,
    ) -> Result<()> {
        self.write_human_source_view(source_id, ingest, shallow, deep)?;
        self.append_machine_source_event(source_id, ingest, shallow, deep)?;
        Ok(())
    }

    fn write_human_source_view(
        &self,
        source_id: &str,
        ingest: Option<&IngestRecord>,
        shallow: Option<&ShallowAnalysis>,
        deep: Option<&DeepBookEntry>,
    ) -> Result<()> {
        let source_type = ingest
            .map(|v| format!("{:?}", v.source_type))
            .unwrap_or_else(|| "unknown".to_string());
        let title = shallow
            .map(|s| s.title.as_str())
            .or_else(|| ingest.map(|i| i.raw_input.as_str()))
            .unwrap_or(source_id);
        let url = ingest
            .and_then(|v| v.url.clone())
            .unwrap_or_else(|| title.to_string());
        let tags = shallow
            .map(|s| s.relevance_tags.join(", "))
            .unwrap_or_default();
        let status = if deep.is_some() { "deep" } else { "shallow" };
        let now = Utc::now().to_rfc3339();
        let path = self.human_sources_dir.join(format!("{source_id}.md"));
        let mut out = String::new();
        out.push_str("# ATHENA Source Book\n\n");
        out.push_str(&format!("- source_id: `{source_id}`\n"));
        out.push_str(&format!("- status: `{status}`\n"));
        out.push_str(&format!("- source_type: `{source_type}`\n"));
        out.push_str(&format!("- updated_at_utc: `{now}`\n"));
        out.push_str(&format!("- url: {url}\n"));
        out.push_str(&format!(
            "- athena_book: `{}`\n",
            self.book_ref_for(source_id)
        ));
        out.push_str("- machine_index: `data/knowledge/athena/index/sources.jsonl`\n");
        out.push_str("\n## Summary\n\n");
        out.push_str(&format!("**Title**: {title}\n\n"));
        if let Some(shallow) = shallow {
            out.push_str(&format!("{}\n\n", shallow.summary));
            out.push_str(&format!("**Tags**: {}\n\n", tags));
            out.push_str(&format!(
                "**Deep Recommended**: {}\n\n",
                shallow.deep_analysis_recommended
            ));
            out.push_str(&format!(
                "**Deep Reason**: {}\n\n",
                shallow.deep_analysis_reason
            ));
        }
        if let Some(deep) = deep {
            out.push_str("## Deep Analysis\n\n");
            out.push_str(&format!("{}\n\n", deep.data.full_summary));
            out.push_str(&format!("- confidence: `{:.4}`\n", deep.data.confidence));
            out.push_str(&format!(
                "- triad_passed: `{}`\n",
                deep.data.triad_analysis.passed
            ));
            out.push_str(&format!(
                "- love_alignment: `{:.4}`\n",
                deep.data.love_equation.alignment_score
            ));
            out.push_str(&format!(
                "- joule_estimated: `{:.4}`\n",
                deep.data.joulework.estimated_cost
            ));
            out.push_str(&format!(
                "- joule_actual: `{:.4}`\n",
                deep.data.joulework.actual_cost
            ));
            out.push('\n');
            if let Some(brief) = &deep.data.implementation_brief {
                out.push_str("## Implementation Brief\n\n");
                if let Some(method) = brief.get("method_summary").and_then(|v| v.as_str()) {
                    out.push_str(&format!("- method_summary: {}\n", method));
                }
                if let Some(source_url) = brief.get("source_url").and_then(|v| v.as_str()) {
                    out.push_str(&format!("- source_url: `{}`\n", source_url));
                }
                if let Some(implications) = brief
                    .get("implementation_implications")
                    .and_then(|v| v.as_array())
                {
                    out.push_str("- implementation_implications:\n");
                    for implication in implications.iter().filter_map(|v| v.as_str()) {
                        out.push_str(&format!("  - {}\n", implication));
                    }
                }
                if let Some(risks) = brief.get("risks").and_then(|v| v.as_array()) {
                    out.push_str("- risks:\n");
                    for risk in risks.iter().filter_map(|v| v.as_str()) {
                        out.push_str(&format!("  - {}\n", risk));
                    }
                }
                out.push('\n');
            }
        }
        fs::write(path, out)?;
        Ok(())
    }

    fn append_machine_source_event(
        &self,
        source_id: &str,
        ingest: Option<&IngestRecord>,
        shallow: Option<&ShallowAnalysis>,
        deep: Option<&DeepBookEntry>,
    ) -> Result<()> {
        let event = serde_json::json!({
            "ts_utc": Utc::now().to_rfc3339(),
            "source_id": source_id,
            "event": if deep.is_some() { "deep_synced" } else { "shallow_synced" },
            "status": if deep.is_some() { "deep" } else { "shallow" },
            "sigil": if deep.is_some() { "EYE" } else { "ANKH" },
            "book_ref": self.book_ref_for(source_id),
            "human_ref": format!("human/library/athena/sources/{source_id}.md"),
            "source_type": ingest.map(|v| format!("{:?}", v.source_type)),
            "url": ingest.and_then(|v| v.url.clone()),
            "tags": shallow.map(|s| s.relevance_tags.clone()).unwrap_or_default(),
            "triad_passed": deep.map(|d| d.data.triad_analysis.passed),
            "confidence": deep.map(|d| d.data.confidence),
            "domain": "knowledge",
            "realm": "athena",
            "soterion": "◈"
        });
        self.append_jsonl(&self.machine_index_path, &event)
    }

    pub(super) fn append_deep_graph_event(
        &self,
        source_id: &str,
        shallow: &ShallowAnalysis,
        deep: &DeepBookEntry,
    ) -> Result<()> {
        let source_node = format!("source:{source_id}");
        let mut nodes = vec![serde_json::json!({
            "id": source_node,
            "kind": "source",
            "label": shallow.title,
        })];
        let mut edges = Vec::new();

        for tag in shallow.relevance_tags.iter().take(24) {
            let node_id = format!("tag:{}", normalize_graph_token(tag));
            nodes.push(serde_json::json!({
                "id": node_id,
                "kind": "tag",
                "label": tag,
            }));
            edges.push(serde_json::json!({
                "from": source_node,
                "to": node_id,
                "relation": "tagged_as",
            }));
        }

        for component in shallow.components_available.iter().take(24) {
            let node_id = format!("component:{}", normalize_graph_token(component));
            nodes.push(serde_json::json!({
                "id": node_id,
                "kind": "component",
                "label": component,
            }));
            edges.push(serde_json::json!({
                "from": source_node,
                "to": node_id,
                "relation": "contains_component",
            }));
        }

        for dependency in shallow.key_dependencies.iter().take(24) {
            let node_id = format!("dependency:{}", normalize_graph_token(dependency));
            nodes.push(serde_json::json!({
                "id": node_id,
                "kind": "dependency",
                "label": dependency,
            }));
            edges.push(serde_json::json!({
                "from": source_node,
                "to": node_id,
                "relation": "depends_on",
            }));
        }

        let event = serde_json::json!({
            "ts_utc": Utc::now().to_rfc3339(),
            "event": "deep_graph_update",
            "source_id": source_id,
            "graph_version": "v1",
            "nodes": nodes,
            "edges": edges,
            "triad_passed": deep.data.triad_analysis.passed,
            "confidence": deep.data.confidence,
            "soterion": "◈"
        });
        self.append_jsonl(&self.deep_graph_path, &event)
    }

    pub(super) fn emit_lifecycle_event(
        &self,
        event: &str,
        source_id: &str,
        details: serde_json::Value,
    ) -> Result<()> {
        let ctx = self.event_ctx(event, source_id);
        self.interceptors.after(
            &ctx,
            &DigestEvent::Lifecycle {
                name: event.to_string(),
                source_id: source_id.to_string(),
                details,
            },
        );
        Ok(())
    }

    pub(super) fn emit_relationship_signal_background(
        &self,
        from: String,
        to: String,
        trust: f64,
        reciprocity: f64,
        longevity: f64,
        context: &'static str,
    ) {
        let _ = spawn_bounded_background(
            "athena_plutus_signal",
            background_signal_limit(),
            move || async move {
                match PlutusService::from_default_or_workspace_fallback() {
                    Ok(service) => {
                        if let Err(err) = service
                            .record_relationship(&from, &to, trust, reciprocity, longevity)
                            .await
                        {
                            tracing::debug!(
                                error = %err,
                                context,
                                "ATHENA relationship signal failed"
                            );
                        }
                    }
                    Err(err) => {
                        tracing::debug!(
                            error = %err,
                            context,
                            "ATHENA could not open PLUTUS service"
                        );
                    }
                }
            },
        );
    }

    pub(super) fn emit_work_signal_background(
        &self,
        agent: String,
        amount: f64,
        unit: JouleWorkUnit,
        context: &'static str,
    ) {
        let _ = spawn_bounded_background(
            "athena_plutus_signal",
            background_signal_limit(),
            move || async move {
                match PlutusService::from_default_or_workspace_fallback() {
                    Ok(service) => {
                        if let Err(err) = service.track_work(&agent, amount, unit, None).await {
                            tracing::debug!(
                                error = %err,
                                context,
                                "ATHENA work signal failed"
                            );
                        }
                    }
                    Err(err) => {
                        tracing::debug!(
                            error = %err,
                            context,
                            "ATHENA could not open PLUTUS service"
                        );
                    }
                }
            },
        );
    }
}

fn background_signal_limit() -> usize {
    #[cfg(test)]
    let default = 64;
    #[cfg(not(test))]
    let default = 4;

    std::env::var("ARDA_BACKGROUND_SIGNAL_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}
