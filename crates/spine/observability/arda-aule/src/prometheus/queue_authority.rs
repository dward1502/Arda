#![cfg(feature = "full-cli")]

//! Canonical project-task queue authority resolution.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub const DEFAULT_PROJECT_TASK_QUEUE: &str = "core/projects/tasks/queue.jsonl";

pub fn resolve_project_task_queue(root: &Path, configured: Option<&OsStr>) -> PathBuf {
    let configured = configured
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROJECT_TASK_QUEUE));
    if configured.is_absolute() {
        configured
    } else {
        root.join(configured)
    }
}

pub fn canonical_project_task_queue(root: &Path) -> PathBuf {
    let configured = std::env::var_os("ARDA_PROJECT_TASK_QUEUE_PATH")
        .or_else(|| std::env::var_os("ANNUNIMAS_PROJECT_TASK_QUEUE_PATH"));
    resolve_project_task_queue(root, configured.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_the_repository_global_queue_not_varda_local_state() {
        assert_eq!(
            resolve_project_task_queue(Path::new("/arda"), None),
            PathBuf::from("/arda/core/projects/tasks/queue.jsonl")
        );
        assert_ne!(
            resolve_project_task_queue(Path::new("/arda"), None),
            PathBuf::from(
                "/arda/crates/spine/executors/arda-varda/core/projects/tasks/queue.jsonl"
            )
        );
    }

    #[test]
    fn resolves_relative_configured_queue_against_repository_root() {
        assert_eq!(
            resolve_project_task_queue(
                Path::new("/arda"),
                Some(OsStr::new("runtime/queues/project.jsonl")),
            ),
            PathBuf::from("/arda/runtime/queues/project.jsonl")
        );
    }

    #[test]
    fn preserves_absolute_configured_queue() {
        assert_eq!(
            resolve_project_task_queue(
                Path::new("/arda"),
                Some(OsStr::new("/srv/arda/project-queue.jsonl")),
            ),
            PathBuf::from("/srv/arda/project-queue.jsonl")
        );
    }
}
