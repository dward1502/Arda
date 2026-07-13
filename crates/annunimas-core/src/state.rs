//! Typed read/write helpers for the v0.1 agent state contract.
//!
//! Single source of disk semantics for the Phase 1 loop. Every
//! reader/writer of `Goal`, `Plan`, `Reflection`, `MemoryRecord`
//! goes through here so paths and file shapes stay in one place.
//!
//! Canonical paths follow `docs/plans/FILE_LAYOUT.md` §3.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{de::DeserializeOwned, Serialize};

use crate::contract::{Goal, MemoryKind, MemoryRecord, Plan, Reflection};
use crate::error::{AnnunimasError, Result};
use crate::task::Task;

/// Root layout. All paths derive from this.
pub struct StateRoot {
    root: PathBuf,
}

impl StateRoot {
    /// `<repo>/core/state` is the conventional root.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn default_for_repo(repo_root: impl AsRef<Path>) -> Self {
        Self::new(repo_root.as_ref().join("core").join("state"))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn goals_dir(&self) -> PathBuf {
        self.root.join("goals")
    }

    pub fn plans_dir(&self) -> PathBuf {
        self.root.join("plans")
    }

    pub fn reflections_dir(&self) -> PathBuf {
        self.root.join("reflections")
    }

    pub fn memory_dir(&self, kind: MemoryKind) -> PathBuf {
        let leaf = match kind {
            MemoryKind::Episodic => "episodic",
            MemoryKind::Semantic => "semantic",
        };
        self.root.join("memory").join(leaf)
    }

    pub fn agents_dir(&self) -> PathBuf {
        self.root.join("agents")
    }

    /// Canonical task queue. Per FILE_LAYOUT §4.2 it lives outside
    /// `core/state/` at `<repo>/core/projects/tasks/queue.jsonl`. Take
    /// the path explicitly so a caller working with a non-default
    /// state root can still point at the right queue.
    pub fn queue_path(&self, repo_root: &Path) -> PathBuf {
        repo_root
            .join("core")
            .join("projects")
            .join("tasks")
            .join("queue.jsonl")
    }
}

/// Append a Task to the canonical queue jsonl. Creates the parent dir
/// and the file if missing.
pub fn append_task(queue_path: &Path, task: &Task) -> Result<()> {
    if let Some(parent) = queue_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let line = serde_json::to_string(task)?;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(queue_path)?;
    use std::io::Write;
    writeln!(f, "{line}")?;
    Ok(())
}

/// Read all queue entries that are recognizable as v0.1 `Task`
/// records. Pre-contract entries (different shape) are silently
/// skipped.
pub fn read_contract_tasks(queue_path: &Path) -> Result<Vec<Task>> {
    if !queue_path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(queue_path)?;
    let mut out = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(t) = serde_json::from_str::<Task>(line) {
            out.push(t);
        }
    }
    Ok(out)
}

/// Write a record as `<dir>/<id>.json` atomically (write + rename).
fn write_record_atomic<T: Serialize>(dir: &Path, id: &str, record: &T) -> Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let final_path = dir.join(format!("{id}.json"));
    let tmp_path = dir.join(format!(".{id}.json.tmp"));
    let json = serde_json::to_vec_pretty(record)?;
    fs::write(&tmp_path, &json)?;
    fs::rename(&tmp_path, &final_path)?;
    Ok(final_path)
}

fn read_record<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = fs::read(path)?;
    let v: T = serde_json::from_slice(&bytes)?;
    Ok(v)
}

fn list_records<T: DeserializeOwned>(dir: &Path) -> Result<Vec<T>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        if path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.starts_with('.'))
            .unwrap_or(false)
        {
            continue; // skip .tmp files
        }
        match read_record::<T>(&path) {
            Ok(v) => out.push(v),
            Err(e) => {
                return Err(AnnunimasError::Config(format!(
                    "state: failed to parse {}: {}",
                    path.display(),
                    e
                )));
            }
        }
    }
    Ok(out)
}

// -- Goals --

pub fn write_goal(state: &StateRoot, goal: &Goal) -> Result<PathBuf> {
    write_record_atomic(&state.goals_dir(), &goal.id, goal)
}

pub fn read_goal(state: &StateRoot, id: &str) -> Result<Goal> {
    read_record(&state.goals_dir().join(format!("{id}.json")))
}

pub fn list_goals(state: &StateRoot) -> Result<Vec<Goal>> {
    list_records(&state.goals_dir())
}

// -- Plans --

pub fn write_plan(state: &StateRoot, plan: &Plan) -> Result<PathBuf> {
    write_record_atomic(&state.plans_dir(), &plan.id, plan)
}

pub fn read_plan(state: &StateRoot, id: &str) -> Result<Plan> {
    read_record(&state.plans_dir().join(format!("{id}.json")))
}

pub fn list_plans(state: &StateRoot) -> Result<Vec<Plan>> {
    list_records(&state.plans_dir())
}

// -- Reflections --

pub fn write_reflection(state: &StateRoot, reflection: &Reflection) -> Result<PathBuf> {
    write_record_atomic(&state.reflections_dir(), &reflection.task_id, reflection)
}

pub fn read_reflection(state: &StateRoot, task_id: &str) -> Result<Reflection> {
    read_record(&state.reflections_dir().join(format!("{task_id}.json")))
}

pub fn list_reflections(state: &StateRoot) -> Result<Vec<Reflection>> {
    list_records(&state.reflections_dir())
}

// -- Memory --

pub fn write_memory(state: &StateRoot, record: &MemoryRecord) -> Result<PathBuf> {
    write_record_atomic(&state.memory_dir(record.kind), &record.id, record)
}

pub fn list_memory(state: &StateRoot, kind: MemoryKind) -> Result<Vec<MemoryRecord>> {
    list_records(&state.memory_dir(kind))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{GoalPriority, MemoryKind, ReflectionOutcome};

    fn tmp_root() -> (tempfile::TempDir, StateRoot) {
        let dir = tempfile::tempdir().unwrap();
        let root = StateRoot::new(dir.path().to_path_buf());
        (dir, root)
    }

    #[test]
    fn goal_roundtrip() {
        let (_dir, root) = tmp_root();
        let g = Goal::new("g1", "Title", "Intent", "prometheus", GoalPriority::High);
        write_goal(&root, &g).unwrap();
        let back = read_goal(&root, "g1").unwrap();
        assert_eq!(back.id, "g1");
        let all = list_goals(&root).unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn plan_roundtrip() {
        let (_dir, root) = tmp_root();
        let p = Plan::new("p1", "g1", "summary", vec![]);
        write_plan(&root, &p).unwrap();
        let back = read_plan(&root, "p1").unwrap();
        assert_eq!(back.goal_id, "g1");
    }

    #[test]
    fn reflection_indexed_by_task_id() {
        let (_dir, root) = tmp_root();
        let r = Reflection::new("r1", "task_42", "p1", ReflectionOutcome::Success, 0.8);
        write_reflection(&root, &r).unwrap();
        // Reflection file is keyed by task_id per FILE_LAYOUT §3
        let back = read_reflection(&root, "task_42").unwrap();
        assert_eq!(back.id, "r1");
    }

    #[test]
    fn memory_split_episodic_semantic() {
        let (_dir, root) = tmp_root();
        let ep = MemoryRecord::new("m1", MemoryKind::Episodic, "oracle", "saw a thing");
        let se = MemoryRecord::new("m2", MemoryKind::Semantic, "mnemosyne", "the thing means X");
        write_memory(&root, &ep).unwrap();
        write_memory(&root, &se).unwrap();
        assert_eq!(list_memory(&root, MemoryKind::Episodic).unwrap().len(), 1);
        assert_eq!(list_memory(&root, MemoryKind::Semantic).unwrap().len(), 1);
    }

    #[test]
    fn list_skips_non_json_and_tmp() {
        let (_dir, root) = tmp_root();
        let g = Goal::new("g1", "T", "I", "p", GoalPriority::Low);
        write_goal(&root, &g).unwrap();
        // Drop a junk file
        std::fs::write(root.goals_dir().join("notes.txt"), b"junk").unwrap();
        std::fs::write(root.goals_dir().join(".half.json.tmp"), b"{}").unwrap();
        assert_eq!(list_goals(&root).unwrap().len(), 1);
    }
}
