// sigil: REPAIR
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;
use unicode_categories::UnicodeCategories;
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

const MAX_SEARCH_RESULTS: usize = 10;

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IndexingDisposition {
    Inserted,
    Replaced,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexingReport {
    pub doc_id: String,
    pub disposition: IndexingDisposition,
    pub previous_node_count: usize,
    pub indexed_node_count: usize,
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
    ) -> IndexingReport {
        let mut nodes = Vec::new();
        let mut index = HashMap::new();
        let mut ancestor_stack: Vec<&TocEntry> = Vec::new();

        for (location, entry) in toc.iter().enumerate() {
            while ancestor_stack
                .last()
                .is_some_and(|ancestor| ancestor.level >= entry.level)
            {
                ancestor_stack.pop();
            }
            let mut path: Vec<String> = ancestor_stack
                .iter()
                .map(|ancestor| ancestor.title.clone())
                .collect();
            path.push(entry.title.clone());
            let canonical_path = path
                .iter()
                .map(|part| Self::normalize_tokens(part).join("-"))
                .collect::<Vec<_>>()
                .join("/");
            let identity = format!("{doc_id}\n{canonical_path}\nlocation={location}");
            let node_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, identity.as_bytes()).to_string();

            for keyword in Self::search_terms(&entry.title) {
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
            ancestor_stack.push(entry);
        }

        let total_pages = toc.iter().filter_map(|e| e.page).max().unwrap_or(0);

        let tree = PageTree {
            doc_id: doc_id.clone(),
            title,
            total_pages,
            nodes,
            index,
        };

        let replaced = self.trees.contains_key(&doc_id);
        let previous_node_count = self
            .trees
            .insert(doc_id.clone(), tree)
            .map_or(0, |previous| previous.nodes.len());
        IndexingReport {
            doc_id,
            disposition: if replaced {
                IndexingDisposition::Replaced
            } else {
                IndexingDisposition::Inserted
            },
            previous_node_count,
            indexed_node_count: toc.len(),
        }
    }

    pub fn search(&self, doc_id: &str, query: &str) -> Vec<SearchResult> {
        self.trees.get(doc_id).map_or_else(Vec::new, |tree| {
            Self::search_tree(tree, query, MAX_SEARCH_RESULTS)
        })
    }

    pub fn search_all(&self, query: &str) -> Vec<SearchResult> {
        let mut results: Vec<_> = self
            .trees
            .values()
            .flat_map(|tree| Self::search_tree(tree, query, usize::MAX))
            .collect();
        Self::sort_results(&mut results);
        results.truncate(MAX_SEARCH_RESULTS);
        results
    }

    pub fn get_tree(&self, doc_id: &str) -> Option<&PageTree> {
        self.trees.get(doc_id)
    }

    pub fn list_documents(&self) -> Vec<String> {
        let mut documents: Vec<_> = self.trees.keys().cloned().collect();
        documents.sort();
        documents
    }

    fn search_tree(tree: &PageTree, query: &str, limit: usize) -> Vec<SearchResult> {
        let query_terms: BTreeSet<_> = Self::search_terms(query).into_iter().collect();
        if query_terms.is_empty() {
            return Vec::new();
        }
        let mut matched_terms: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for term in &query_terms {
            if let Some(node_ids) = tree.index.get(term) {
                for node_id in node_ids {
                    matched_terms.entry(node_id).or_default().insert(term);
                }
            }
        }
        let mut results: Vec<_> = matched_terms
            .into_iter()
            .filter_map(|(node_id, terms)| {
                tree.nodes
                    .iter()
                    .find(|node| node.id == node_id)
                    .map(|node| SearchResult {
                        doc_id: tree.doc_id.clone(),
                        node_id: node.id.clone(),
                        source_ref: format!(
                            "pageindex://{}/{}",
                            Self::encode_source_component(&tree.doc_id),
                            node.id
                        ),
                        title: node.title.clone(),
                        page_ref: node.page_ref,
                        relevance_score: terms.len() as f64 / query_terms.len() as f64,
                        path: node.path.clone(),
                    })
            })
            .collect();
        Self::sort_results(&mut results);
        results.truncate(limit);
        results
    }

    fn sort_results(results: &mut [SearchResult]) {
        results.sort_by(|left, right| {
            right
                .relevance_score
                .total_cmp(&left.relevance_score)
                .then_with(|| left.doc_id.cmp(&right.doc_id))
                .then_with(|| {
                    left.page_ref
                        .unwrap_or(u32::MAX)
                        .cmp(&right.page_ref.unwrap_or(u32::MAX))
                })
                .then_with(|| left.path.cmp(&right.path))
                .then_with(|| left.node_id.cmp(&right.node_id))
        });
    }

    fn normalize_tokens(text: &str) -> Vec<String> {
        let normalized: String = text
            .nfkc()
            .flat_map(char::to_lowercase)
            .map(|character| {
                if character.is_alphanumeric() || character.is_mark() {
                    character
                } else {
                    ' '
                }
            })
            .collect();
        normalized
            .split_whitespace()
            .filter(|term| !term.is_empty())
            .map(ToString::to_string)
            .collect()
    }

    fn search_terms(text: &str) -> Vec<String> {
        let stop_words = [
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by",
        ];
        Self::normalize_tokens(text)
            .into_iter()
            .filter(|term| !stop_words.contains(&term.as_str()))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn encode_source_component(component: &str) -> String {
        let mut encoded = String::with_capacity(component.len());
        for byte in component.bytes() {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
                encoded.push(char::from(byte));
            } else {
                write!(encoded, "%{byte:02X}").expect("writing to a String cannot fail");
            }
        }
        encoded
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchResult {
    pub doc_id: String,
    pub node_id: String,
    pub source_ref: String,
    pub title: String,
    pub page_ref: Option<u32>,
    pub relevance_score: f64,
    pub path: Vec<String>,
}

impl PageTree {
    pub fn navigate(&self, query: &str) -> Vec<&PageNodeMeta> {
        let results = PageIndex::search_tree(self, query, MAX_SEARCH_RESULTS);
        results
            .iter()
            .filter_map(|r| self.nodes.iter().find(|n| n.id == r.node_id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, title: &str, level: u8, page: u32) -> TocEntry {
        TocEntry {
            id: id.to_string(),
            title: title.to_string(),
            level,
            page: Some(page),
        }
    }

    #[test]
    fn navigation_uses_the_tree_owned_index() {
        let mut index = PageIndex::new();
        index.index_document(
            "handbook".to_string(),
            "Handbook".to_string(),
            vec![entry("safety", "Safety Controls", 1, 3)],
        );
        let matches = index
            .get_tree("handbook")
            .expect("indexed tree")
            .navigate("safety");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].title, "Safety Controls");
    }

    #[test]
    fn reindexing_replaces_the_document_and_reports_the_change() {
        let mut index = PageIndex::new();
        let inserted = index.index_document(
            "handbook".to_string(),
            "Handbook".to_string(),
            vec![entry("old", "Old Guidance", 1, 1)],
        );
        let replaced = index.index_document(
            "handbook".to_string(),
            "Handbook v2".to_string(),
            vec![entry("new", "Current Guidance", 1, 2)],
        );
        assert_eq!(inserted.disposition, IndexingDisposition::Inserted);
        assert_eq!(inserted.previous_node_count, 0);
        assert_eq!(replaced.disposition, IndexingDisposition::Replaced);
        assert_eq!(replaced.previous_node_count, 1);
        assert!(index.search("handbook", "old").is_empty());
        assert_eq!(index.search("handbook", "current").len(), 1);
    }

    #[test]
    fn heading_paths_follow_the_active_ancestor_stack() {
        let mut index = PageIndex::new();
        index.index_document(
            "guide".to_string(),
            "Guide".to_string(),
            vec![
                entry("a", "Alpha", 1, 1),
                entry("a-child", "Alpha Child", 2, 2),
                entry("b", "Beta", 1, 3),
                entry("b-child", "Beta Child", 2, 4),
                entry("b-grandchild", "Beta Grandchild", 3, 5),
            ],
        );
        let nodes = &index.get_tree("guide").expect("indexed tree").nodes;
        assert_eq!(nodes[1].path, vec!["Alpha", "Alpha Child"]);
        assert_eq!(nodes[3].path, vec!["Beta", "Beta Child"]);
        assert_eq!(nodes[4].path, vec!["Beta", "Beta Child", "Beta Grandchild"]);
    }

    #[test]
    fn node_ids_are_stable_for_identical_document_locations() {
        let toc = vec![
            entry("overview", "Résumé Overview", 1, 1),
            entry("details", "Evidence Details", 2, 2),
        ];
        let mut first = PageIndex::new();
        first.index_document("report".to_string(), "Report".to_string(), toc.clone());
        let first_ids: Vec<_> = first
            .get_tree("report")
            .expect("first tree")
            .nodes
            .iter()
            .map(|node| node.id.clone())
            .collect();
        let mut repaginated_toc = toc;
        repaginated_toc[0].page = Some(41);
        repaginated_toc[1].page = Some(42);
        let mut second = PageIndex::new();
        second.index_document("report".to_string(), "Renamed".to_string(), repaginated_toc);
        let second_ids: Vec<_> = second
            .get_tree("report")
            .expect("second tree")
            .nodes
            .iter()
            .map(|node| node.id.clone())
            .collect();
        assert_eq!(first_ids, second_ids);
    }

    #[test]
    fn unicode_and_punctuation_are_normalized_without_duplicate_hits() {
        let mut index = PageIndex::new();
        index.index_document(
            "unicode".to_string(),
            "Unicode".to_string(),
            vec![entry("resume", "Résumé—résumé: safety!!!", 1, 1)],
        );
        let composed = index.search("unicode", "RÉSUMÉ, safety");
        let decomposed = index.search("unicode", "re\u{301}sume\u{301} safety");
        assert_eq!(composed.len(), 1);
        assert_eq!(decomposed.len(), 1);
        assert_eq!(composed[0].node_id, decomposed[0].node_id);
        assert_eq!(composed[0].relevance_score, 1.0);
    }

    #[test]
    fn combining_marks_and_source_reference_components_remain_distinct() {
        let mut index = PageIndex::new();
        index.index_document(
            "reports/שלום".to_string(),
            "Reports".to_string(),
            vec![
                entry("pointed", "שָׁלוֹם", 1, 1),
                entry("plain", "שלום", 1, 2),
                entry("indic-marked", "का", 1, 3),
                entry("indic-plain", "क", 1, 4),
            ],
        );

        let hebrew_results = index.search("reports/שלום", "שָׁלוֹם");
        let indic_results = index.search("reports/שלום", "का");

        assert_eq!(hebrew_results.len(), 1);
        assert_eq!(hebrew_results[0].title, "שָׁלוֹם");
        assert_eq!(indic_results.len(), 1);
        assert_eq!(indic_results[0].title, "का");
        assert!(hebrew_results[0]
            .source_ref
            .starts_with("pageindex://reports%2F%D7%A9%D7%9C%D7%95%D7%9D/"));
    }

    #[test]
    fn all_document_search_is_normalized_and_deterministically_tied() {
        let mut index = PageIndex::new();
        index.index_document(
            "zeta".to_string(),
            "Zeta".to_string(),
            vec![entry("z", "Shared Evidence", 1, 1)],
        );
        index.index_document(
            "alpha".to_string(),
            "Alpha".to_string(),
            vec![entry("a", "Shared Evidence", 1, 1)],
        );
        let first = index.search_all("shared evidence");
        let second = index.search_all("shared evidence");
        assert_eq!(first, second);
        assert_eq!(first.len(), 2);
        assert_eq!(first[0].doc_id, "alpha");
        assert_eq!(first[1].doc_id, "zeta");
        assert!(first
            .iter()
            .all(|result| (0.0..=1.0).contains(&result.relevance_score)));
    }
}
