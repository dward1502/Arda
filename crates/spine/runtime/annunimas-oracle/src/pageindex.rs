// sigil: REPAIR
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PageNode {
    pub id: String,
    pub title: String,
    pub level: u8,
    pub content_preview: String,
    pub page_ref: Option<u32>,
    pub children: Vec<PageNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageNodeMeta {
    pub id: String,
    pub title: String,
    pub level: u8,
    pub page_ref: Option<u32>,
    pub path: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageTree {
    pub doc_id: String,
    pub title: String,
    pub total_pages: u32,
    pub nodes: Vec<PageNodeMeta>,
    pub index: HashMap<String, Vec<String>>,
}

pub struct PageIndex {
    trees: HashMap<String, PageTree>,
}

impl PageIndex {
    pub fn new() -> Self {
        Self {
            trees: HashMap::new(),
        }
    }

    pub fn index_document(
        &mut self,
        doc_id: String,
        title: String,
        toc: Vec<TocEntry>,
    ) -> &PageTree {
        let mut nodes = Vec::new();
        let mut index = HashMap::new();

        for entry in &toc {
            let node_id = Uuid::new_v4().to_string();
            let path = Self::build_path(&toc, &entry.id);

            for keyword in Self::extract_keywords(&entry.title) {
                index
                    .entry(keyword)
                    .or_insert_with(Vec::new)
                    .push(node_id.clone());
            }

            nodes.push(PageNodeMeta {
                id: node_id,
                title: entry.title.clone(),
                level: entry.level,
                page_ref: entry.page,
                path,
            });
        }

        let total_pages = toc.iter().filter_map(|e| e.page).max().unwrap_or(0);

        let tree = PageTree {
            doc_id: doc_id.clone(),
            title,
            total_pages,
            nodes,
            index,
        };

        self.trees.entry(doc_id).or_insert(tree)
    }

    pub fn search(&self, doc_id: &str, query: &str) -> Vec<SearchResult> {
        let tree = match self.trees.get(doc_id) {
            Some(t) => t,
            None => return Vec::new(),
        };

        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scores: HashMap<&str, f64> = HashMap::new();

        for (keyword, node_ids) in &tree.index {
            let kw_lower = keyword.to_lowercase();
            for word in &query_words {
                if kw_lower.contains(word) || word.contains(&kw_lower) {
                    for node_id in node_ids {
                        *scores.entry(node_id).or_insert(0.0) += 1.0;
                    }
                }
            }
        }

        let mut results: Vec<SearchResult> = scores
            .into_iter()
            .map(|(node_id, score)| {
                let node = tree.nodes.iter().find(|n| n.id == node_id);
                SearchResult {
                    node_id: node_id.to_string(),
                    title: node.map(|n| n.title.clone()).unwrap_or_default(),
                    page_ref: node.and_then(|n| n.page_ref),
                    relevance_score: score,
                    path: node.map(|n| n.path.clone()).unwrap_or_default(),
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(10);
        results
    }

    pub fn get_tree(&self, doc_id: &str) -> Option<&PageTree> {
        self.trees.get(doc_id)
    }

    pub fn list_documents(&self) -> Vec<String> {
        self.trees.keys().cloned().collect()
    }

    fn build_path(toc: &[TocEntry], current_id: &str) -> Vec<String> {
        let mut path = Vec::new();

        if let Some(current) = toc.iter().find(|e| e.id == current_id) {
            for entry in toc.iter().take_while(|e| e.id != current_id) {
                if entry.level < current.level {
                    path.insert(0, entry.title.clone());
                }
            }
            path.push(current.title.clone());
        }

        path
    }

    fn extract_keywords(title: &str) -> Vec<String> {
        let stop_words = [
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by",
        ];

        title
            .to_lowercase()
            .split_whitespace()
            .filter(|w| w.len() > 2 && !stop_words.contains(w))
            .map(String::from)
            .collect()
    }
}

impl Default for PageIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct TocEntry {
    pub id: String,
    pub title: String,
    pub level: u8,
    pub page: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub node_id: String,
    pub title: String,
    pub page_ref: Option<u32>,
    pub relevance_score: f64,
    pub path: Vec<String>,
}

impl PageTree {
    pub fn navigate(&self, query: &str) -> Vec<&PageNodeMeta> {
        let results = PageIndex::new().search(&self.doc_id, query);
        results
            .iter()
            .filter_map(|r| self.nodes.iter().find(|n| n.id == r.node_id))
            .collect()
    }
}
