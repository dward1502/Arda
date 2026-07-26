// sigil: REPAIR
//
// Store layout and bootstrap helpers: default Athena home resolution,
// human/machine library roots, and the permission-aware fallback path
// used when the primary store root cannot be opened.

use arda_core::error::{ArdaError, Result};
use std::path::{Path, PathBuf};

use super::AthenaStore;

#[derive(Debug, Clone)]
pub struct AthenaStorePaths {
    pub root: PathBuf,
    pub books_dir: PathBuf,
    pub digest_path: PathBuf,
    pub crawl_receipts_path: PathBuf,
    pub uncertainty_selections_path: PathBuf,
    pub crawl_artifacts_dir: PathBuf,
    pub deep_queue_path: PathBuf,
    pub scholarly_reenrichment_path: PathBuf,
    pub deep_graph_path: PathBuf,
    pub policy_readiness_path: PathBuf,
    pub planning_task_receipts_path: PathBuf,
    pub digest_index_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct WorkspaceLayout {
    pub store: AthenaStorePaths,
    pub human_sources_dir: PathBuf,
    pub machine_index_path: PathBuf,
    pub hades_queue_path: PathBuf,
    pub warden_queue_path: PathBuf,
}

impl WorkspaceLayout {
    pub fn for_store_root(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        let workspace_root = arda_root();
        let human_library_root = std::env::var("ARDA_ATHENA_HUMAN_LIBRARY_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| workspace_root.join("human/library/athena"));
        let machine_library_root = std::env::var("ARDA_ATHENA_MACHINE_LIBRARY_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| workspace_root.join("data/knowledge/athena"));
        let hades_queue_path = std::env::var("ARDA_HADES_ACTION_QUEUE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| workspace_root.join("data/hades/action_queue.jsonl"));
        let warden_queue_path = std::env::var("ARDA_WARDEN_QUEUE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| workspace_root.join("data/warden/informant_queue.jsonl"));
        Self {
            store: AthenaStorePaths {
                books_dir: root.join("books"),
                digest_path: root.join("digest.jsonl"),
                crawl_receipts_path: root.join("crawl_receipts.jsonl"),
                uncertainty_selections_path: root.join("uncertainty_selections.jsonl"),
                crawl_artifacts_dir: root.join("crawls"),
                deep_queue_path: root.join("deep_queue.jsonl"),
                scholarly_reenrichment_path: root.join("scholarly_reenrichment.jsonl"),
                deep_graph_path: root.join("deep_graph.jsonl"),
                policy_readiness_path: root.join("policy_readiness.jsonl"),
                planning_task_receipts_path: root.join("planning_task_receipts.jsonl"),
                digest_index_path: root.join("digest-index-v1.json"),
                root,
            },
            human_sources_dir: human_library_root.join("sources"),
            machine_index_path: machine_library_root.join("index").join("sources.jsonl"),
            hades_queue_path,
            warden_queue_path,
        }
    }
}

impl std::ops::Deref for WorkspaceLayout {
    type Target = AthenaStorePaths;

    fn deref(&self) -> &Self::Target {
        &self.store
    }
}

pub(crate) fn arda_root() -> PathBuf {
    if let Ok(path) = std::env::var("ARDA_ROOT") {
        return PathBuf::from(path);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

impl AthenaStore {
    pub fn default_path() -> PathBuf {
        if let Ok(custom) = std::env::var("ARDA_ATHENA_HOME") {
            return PathBuf::from(custom);
        }
        arda_root().join("data/athena")
    }

    pub fn from_default_or_workspace_fallback() -> Result<Self> {
        let primary = Self::default_path();
        match Self::new(&primary) {
            Ok(store) => Ok(store),
            Err(err) => {
                if !is_permission_error(&err) {
                    return Err(err);
                }
                let fallback = arda_root().join("data").join("athena");
                Self::new(fallback)
            }
        }
    }
}

fn is_permission_error(err: &ArdaError) -> bool {
    matches!(
        err,
        ArdaError::Ledger(ioe) if ioe.kind() == std::io::ErrorKind::PermissionDenied
    )
}

#[cfg(test)]
mod tests {
    use super::WorkspaceLayout;

    #[test]
    fn workspace_layout_owns_store_and_library_paths() {
        let root = tempfile::tempdir().expect("tempdir");
        let layout = WorkspaceLayout::for_store_root(root.path());

        assert_eq!(layout.store.root, root.path());
        assert_eq!(layout.store.books_dir, root.path().join("books"));
        assert_eq!(
            layout.store.deep_queue_path,
            root.path().join("deep_queue.jsonl")
        );
        assert!(layout
            .human_sources_dir
            .ends_with("human/library/athena/sources"));
        assert!(layout
            .machine_index_path
            .ends_with("data/knowledge/athena/index/sources.jsonl"));
    }
}
