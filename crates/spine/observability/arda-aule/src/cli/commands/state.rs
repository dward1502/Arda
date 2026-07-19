#![cfg(feature = "full-cli")]
// sigil: ∇ ◈ ↝
//
// `arda-cli state validate` — walks core/state/ and parses every
// record against its declared contract version. See:
//   spec/agent-state-contract.md
//   docs/plans/FILE_LAYOUT.md

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use arda_core::contract::{
    Goal, LedgerEntry, MemoryKind, MemoryRecord, Plan, Reflection, CONTRACT_VERSION,
};
use clap::Subcommand;
use serde::Deserialize;

use crate::arda_root;

#[derive(Subcommand)]
pub(crate) enum StateCommands {
    /// Validate on-disk state under core/state/ against the v0.1 contract
    Validate {
        /// State root override (defaults to <arda_ROOT>/core/state)
        #[arg(long)]
        root: Option<PathBuf>,
        /// Print every record visited, not just failures
        #[arg(long)]
        verbose: bool,
    },
}

pub(crate) fn handle(cmd: StateCommands) -> anyhow::Result<()> {
    match cmd {
        StateCommands::Validate { root, verbose } => {
            let root = root.unwrap_or_else(|| arda_root().join("core/state"));
            let report = validate(&root, verbose)?;
            report.print();
            if report.has_errors() {
                std::process::exit(1);
            }
            Ok(())
        }
    }
}

#[derive(Debug, Default)]
struct Report {
    visited: usize,
    errors: Vec<String>,
    warnings: Vec<String>,
    skipped: Vec<String>,
}

impl Report {
    fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
    fn err(&mut self, path: &Path, msg: impl Into<String>) {
        self.errors
            .push(format!("error: {}: {}", path.display(), msg.into()));
    }
    fn warn(&mut self, path: &Path, msg: impl Into<String>) {
        self.warnings
            .push(format!("warn:  {}: {}", path.display(), msg.into()));
    }
    fn skip(&mut self, path: &Path, msg: impl Into<String>) {
        self.skipped
            .push(format!("skip:  {}: {}", path.display(), msg.into()));
    }

    fn print(&self) {
        for line in &self.errors {
            eprintln!("{line}");
        }
        for line in &self.warnings {
            eprintln!("{line}");
        }
        println!(
            "state validate: {} records visited, {} errors, {} warnings, {} skipped",
            self.visited,
            self.errors.len(),
            self.warnings.len(),
            self.skipped.len()
        );
    }
}

#[derive(Deserialize)]
struct VersionProbe {
    contract_version: Option<String>,
}

fn parse_version(raw: &str) -> Option<(u64, u64)> {
    let mut parts = raw.split('.');
    let major: u64 = parts.next()?.parse().ok()?;
    let minor: u64 = parts.next()?.parse().ok()?;
    Some((major, minor))
}

fn check_version(report: &mut Report, path: &Path, raw: &str) -> bool {
    let Some((file_major, file_minor)) = parse_version(raw) else {
        report.err(path, format!("unparseable contract_version {raw:?}"));
        return false;
    };
    let Some((curr_major, curr_minor)) = parse_version(CONTRACT_VERSION) else {
        report.err(
            path,
            format!("internal CONTRACT_VERSION is unparseable: {CONTRACT_VERSION:?}"),
        );
        return false;
    };
    if file_major != curr_major {
        report.err(
            path,
            format!(
                "major version mismatch: file {file_major}.{file_minor} vs current {curr_major}.{curr_minor}"
            ),
        );
        return false;
    }
    if file_minor != curr_minor {
        report.warn(
            path,
            format!(
                "minor version drift: file {file_major}.{file_minor} vs current {curr_major}.{curr_minor}"
            ),
        );
    }
    true
}

fn validate(root: &Path, verbose: bool) -> anyhow::Result<Report> {
    let mut report = Report::default();
    if !root.exists() {
        report.err(root, "state root does not exist");
        return Ok(report);
    }

    let mut goal_ids: HashSet<String> = HashSet::new();
    let mut plan_goal_refs: Vec<(PathBuf, String)> = Vec::new();
    let mut plan_ids: HashSet<String> = HashSet::new();
    let mut reflection_plan_refs: Vec<(PathBuf, String)> = Vec::new();

    walk_typed::<Goal, _>(&root.join("goals"), &mut report, verbose, |_, _, g| {
        goal_ids.insert(g.id.clone());
        None
    });

    walk_typed::<Plan, _>(&root.join("plans"), &mut report, verbose, |_, path, p| {
        plan_ids.insert(p.id.clone());
        plan_goal_refs.push((path.to_path_buf(), p.goal_id.clone()));
        None
    });

    walk_typed::<Reflection, _>(
        &root.join("reflections"),
        &mut report,
        verbose,
        |_, path, r| {
            reflection_plan_refs.push((path.to_path_buf(), r.plan_id.clone()));
            None
        },
    );

    for kind in [MemoryKind::Episodic, MemoryKind::Semantic] {
        let sub = match kind {
            MemoryKind::Episodic => "memory/episodic",
            MemoryKind::Semantic => "memory/semantic",
        };
        walk_typed::<MemoryRecord, _>(&root.join(sub), &mut report, verbose, |_, _, m| {
            if m.kind != kind {
                Some(format!(
                    "memory kind {:?} found under {sub}/ — directory and field disagree",
                    m.kind
                ))
            } else {
                None
            }
        });
    }

    walk_ledger(&root.join("ledger"), &mut report, verbose);

    for (path, goal_id) in &plan_goal_refs {
        if !goal_ids.contains(goal_id) {
            report.err(path, format!("plan references unknown goal_id {goal_id:?}"));
        }
    }
    for (path, plan_id) in &reflection_plan_refs {
        if !plan_ids.contains(plan_id) {
            report.warn(
                path,
                format!("reflection references unknown plan_id {plan_id:?}"),
            );
        }
    }

    Ok(report)
}

fn walk_typed<T, F>(dir: &Path, report: &mut Report, verbose: bool, mut on_ok: F)
where
    T: for<'de> Deserialize<'de>,
    F: FnMut(&str, &Path, &T) -> Option<String>,
{
    if !dir.exists() {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            report.err(dir, format!("read_dir failed: {e}"));
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            report.skip(&path, "non-json file");
            continue;
        }
        report.visited += 1;
        let raw = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                report.err(&path, format!("read failed: {e}"));
                continue;
            }
        };
        let version_probe: VersionProbe = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => {
                report.err(&path, format!("invalid json: {e}"));
                continue;
            }
        };
        let raw_version = match version_probe.contract_version {
            Some(v) => v,
            None => {
                report.err(&path, "missing contract_version");
                continue;
            }
        };
        if !check_version(report, &path, &raw_version) {
            continue;
        }
        let parsed: T = match serde_json::from_str(&raw) {
            Ok(t) => t,
            Err(e) => {
                report.err(&path, format!("schema mismatch: {e}"));
                continue;
            }
        };
        if verbose {
            println!("ok:    {}", path.display());
        }
        if let Some(warn_msg) = on_ok(&raw_version, &path, &parsed) {
            report.warn(&path, warn_msg);
        }
    }
}

fn walk_ledger(dir: &Path, report: &mut Report, verbose: bool) {
    if !dir.exists() {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            report.err(dir, format!("read_dir failed: {e}"));
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
            report.skip(&path, "non-jsonl file");
            continue;
        }
        let raw = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                report.err(&path, format!("read failed: {e}"));
                continue;
            }
        };
        for (idx, line) in raw.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            report.visited += 1;
            let line_label = format!("{}:{}", path.display(), idx + 1);
            let line_path = PathBuf::from(&line_label);
            match serde_json::from_str::<LedgerEntry>(line) {
                Ok(e) => {
                    if !check_version(report, &line_path, &e.contract_version) {
                        continue;
                    }
                    if verbose {
                        println!("ok:    {line_label}");
                    }
                }
                Err(_) => {
                    // Pre-contract ledger lines are tolerated as warnings —
                    // the existing Ledger writer accepts arbitrary Serialize
                    // values, so legacy lines are expected here.
                    report.warn(&line_path, "line is not a v0.1 LedgerEntry (legacy)");
                }
            }
        }
    }
}
