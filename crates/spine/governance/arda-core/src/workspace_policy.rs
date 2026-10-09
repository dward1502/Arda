//! Shared pre-admission Git workspace policy. This function grants no authority.
use anyhow::{anyhow, Context, Result};
use std::{path::Path, process::Command};

pub fn ensure_clean_for_fresh_mutation(workspace_root: &Path) -> Result<()> {
    let output = Command::new("git")
        .args([
            "-c",
            "core.fsmonitor=false",
            "status",
            "--porcelain=v1",
            "--untracked-files=all",
            "--ignore-submodules=none",
        ])
        .current_dir(workspace_root)
        .env("LC_ALL", "C")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .env_remove("GIT_CONFIG")
        .env_remove("GIT_CONFIG_GLOBAL")
        .env_remove("GIT_CONFIG_SYSTEM")
        .env_remove("GIT_CONFIG_COUNT")
        .output()
        .with_context(|| {
            format!(
                "inspect Git worktree `{}` before mutation admission",
                workspace_root.display()
            )
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !has_git_control_marker(workspace_root)? {
            return Ok(());
        }
        return Err(anyhow!(
            "could not inspect Git worktree `{}` before mutation admission: {}",
            workspace_root.display(),
            stderr.trim()
        ));
    }
    if !output.stdout.is_empty() {
        return Err(anyhow!(
            "refusing mutation in dirty Git worktree `{}`; preserve unrelated local changes in a clean registered worktree",
            workspace_root.display()
        ));
    }
    Ok(())
}

fn has_git_control_marker(workspace_root: &Path) -> Result<bool> {
    for ancestor in workspace_root.ancestors() {
        match std::fs::symlink_metadata(ancestor.join(".git")) {
            Ok(_) => return Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "inspect Git control metadata above `{}`",
                        workspace_root.display()
                    )
                });
            }
        }
    }
    Ok(false)
}
