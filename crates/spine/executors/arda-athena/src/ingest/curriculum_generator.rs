// sigil: REPAIR

use arda_core::error::Result;
use arda_core::llm::{ChatMessage, ChatRequest, LlmProvider};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::AthenaStore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurriculumItem {
    pub title: String,
    pub description: String,
    pub source_ids: Vec<String>,
    pub relevance_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Curriculum {
    pub items: Vec<CurriculumItem>,
    pub generated_at_utc: String,
}

impl AthenaStore {
    /// Generate a learning curriculum based on a query.
    /// Returns a `Curriculum` containing a set of curated items.
    pub fn generate_curriculum(
        &self,
        llm: Arc<dyn LlmProvider>,
        query: &str,
        max_items: usize,
    ) -> Result<Curriculum> {
        // Re‑use the existing query engine to find relevant sources.
        let matches = self.query(query, max_items)?;
        let mut items = Vec::new();
        for m in matches.matches {
            // Build a short description via the LLM.
            let prompt = format!(
                "Summarize the following source in one sentence for a curriculum item.\n\nTitle: {}\nSummary: {}",
                m.title, m.summary
            );
            let request = ChatRequest::new(vec![ChatMessage::user(prompt)]);
            let chat_resp = futures::executor::block_on(llm.chat(request))?;
            let description = chat_resp.content;
            items.push(CurriculumItem {
                title: m.title,
                description: description.trim().to_string(),
                source_ids: vec![m.source_id],
                relevance_tags: m.relevance_tags,
            });
        }
        let curriculum = Curriculum {
            items,
            generated_at_utc: chrono::Utc::now().to_rfc3339(),
        };
        Ok(curriculum)
    }
}
