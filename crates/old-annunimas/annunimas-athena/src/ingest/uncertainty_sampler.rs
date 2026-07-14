// sigil: REPAIR

use annunimas_core::error::Result;
use annunimas_core::llm::LlmProvider;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::{athena_error, AthenaStore, QueryMatch};
use annunimas_core::llm::{ChatMessage, ChatRequest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UncertainChunk {
    pub source_id: String,
    pub chunk_id: String,
    pub content: String,
    pub uncertainty_score: f64,
    pub relevance_tags: Vec<String>,
    pub concepts_hit: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UncertaintySelectionReceipt {
    pub schema_version: String,
    pub event: String,
    pub selected_at_utc: String,
    pub query: String,
    pub requested_limit: usize,
    pub total_selected: usize,
    pub chunks: Vec<UncertainChunk>,
}

impl AthenaStore {
    pub fn select_uncertain_chunks(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<UncertainChunk>> {
        let Some(llm) = self.llm.clone() else {
            return Err(athena_error(
                "cannot select uncertain chunks without an attached LLM provider",
            ));
        };

        self.select_uncertain_chunks_with_llm(llm, query, limit)
    }

    pub fn select_and_record_uncertain_chunks(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<UncertaintySelectionReceipt> {
        let chunks = self.select_uncertain_chunks(query, limit)?;
        self.record_uncertain_chunks(query, limit, chunks)
    }

    pub fn select_uncertain_chunks_with_llm(
        &self,
        llm: Arc<dyn LlmProvider>,
        query: &str,
        limit: usize,
    ) -> Result<Vec<UncertainChunk>> {
        let matches = self.query(query, limit)?;
        let mut uncertain_chunks = Vec::new();

        for match_item in matches.matches {
            let uncertainty_score = self.calculate_uncertainty_score(&llm, &match_item)?;

            uncertain_chunks.push(UncertainChunk {
                source_id: match_item.source_id,
                chunk_id: format!("chunk_{}", uncertain_chunks.len()),
                content: match_item.summary,
                uncertainty_score,
                relevance_tags: match_item.relevance_tags,
                concepts_hit: match_item.concepts_hit,
            });
        }

        uncertain_chunks.sort_by(|a, b| b.uncertainty_score.total_cmp(&a.uncertainty_score));
        Ok(uncertain_chunks)
    }

    pub fn record_uncertain_chunks(
        &self,
        query: &str,
        requested_limit: usize,
        chunks: Vec<UncertainChunk>,
    ) -> Result<UncertaintySelectionReceipt> {
        let receipt = UncertaintySelectionReceipt {
            schema_version: "athena.uncertainty_selection.v1".to_string(),
            event: "uncertainty_selection_recorded".to_string(),
            selected_at_utc: Utc::now().to_rfc3339(),
            query: query.to_string(),
            requested_limit,
            total_selected: chunks.len(),
            chunks,
        };
        self.append_jsonl(&self.uncertainty_selections_path, &receipt)?;
        Ok(receipt)
    }

    fn calculate_uncertainty_score(
        &self,
        llm: &Arc<dyn LlmProvider>,
        match_item: &QueryMatch,
    ) -> Result<f64> {
        let prompt = format!(
            "Analyze the following content and provide an uncertainty score between 0 and 1:\nContent: {}\nRelevance Tags: {:?}\nConcepts Hit: {:?}\nRespond with just the numeric score.",
            match_item.summary,
            match_item.relevance_tags,
            match_item.concepts_hit,
        );
        let request = ChatRequest::new(vec![ChatMessage::user(prompt)]);
        let chat_resp = futures::executor::block_on(llm.chat(request))?;
        let response = chat_resp.content;
        let score = response
            .trim()
            .parse::<f64>()
            .map_err(|e| athena_error(format!("Failed to parse uncertainty score: {}", e)))?;
        Ok(score)
    }
}
