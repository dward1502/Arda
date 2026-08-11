#![cfg(all(feature = "cargo", feature = "git"))]

use std::path::Path;
use std::process::Command;

use arda_rumil::adapters::cargo::{CargoAdapter, PROVIDER_ID as CARGO_PROVIDER};
use arda_rumil::adapters::git::GitAdapter;
use arda_rumil::adapters::{discover_capabilities, GenericAdapter, ProviderAdapter};
use arda_rumil::{AuditPolicy, AuditRequest, BudgetPolicy, RootIdentity};
use chrono::{Duration, Utc};
use uuid::Uuid;

fn policy() -> AuditPolicy {
    AuditPolicy {
        profile_id: "adapter-test-v1".into(),
        root_identity: RootIdentity {
            project_id: Uuid::new_v4(),
            name: "fixture".into(),
            kind: "generic".into(),
            remote_url: None,
        },
        root_relative: ".".into(),
        exclusion_rules: Vec::new(),
        budget: BudgetPolicy::default(),
        provider_allowlist: vec![
            CARGO_PROVIDER.into(),
            arda_rumil::adapters::git::PROVIDER_ID.into(),
            arda_rumil::adapters::generic::PROVIDER_ID.into(),
        ],
        redaction_policy: Vec::new(),
    }
}

fn request() -> AuditRequest {
    AuditRequest {
        request_id: Uuid::new_v4(),
        project_id: Uuid::new_v4(),
        profile_id: "adapter-test-v1".into(),
        source_revision_expectation: None,
        requested_capabilities: vec!["generic_inventory".into()],
        root_policy: ".".into(),
        path_exclusions: Vec::new(),
        file_count_budget: 100,
        byte_budget: 1_000_000,
        source_excerpt_budget: 64_000,
        command_timeout_seconds: 10,
        provider_allowlist: Vec::new(),
        redaction_policy: Vec::new(),
        prior_audit_id: None,
        requested_by: "test".into(),
        expires_at_utc: Utc::now() + Duration::minutes(5),
        authority: "review_only".into(),
    }
}

#[test]
fn cargo_workspace_snapshot_captures_members_targets_and_edges() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path().join("Cargo.toml"),
        br#"[workspace]
members = ["app", "dep"]
resolver = "2"
"#,
    );
    write(
        root.path().join("app/Cargo.toml"),
        br#"[package]
name = "fixture-app"
version = "0.1.0"
edition = "2021"

[dependencies]
fixture-dep = { path = "../dep" }
"#,
    );
    write(root.path().join("app/src/main.rs"), b"fn main() {}\n");
    write(
        root.path().join("dep/Cargo.toml"),
        br#"[package]
name = "fixture-dep"
version = "0.1.0"
edition = "2021"
"#,
    );
    write(root.path().join("dep/src/lib.rs"), b"pub fn value() {}\n");

    let snapshot = CargoAdapter.inspect(root.path()).unwrap();
    assert_eq!(snapshot.workspace_root_relative, ".");
    assert_eq!(snapshot.packages.len(), 2);
    assert!(snapshot.packages.iter().any(|package| {
        package.name == "fixture-app"
            && package.manifest_path_relative == "app/Cargo.toml"
            && package.targets.iter().any(|target| {
                target.source_path_relative == "app/src/main.rs"
                    && target.kinds.iter().any(|kind| kind == "bin")
            })
    }));
    assert!(snapshot
        .dependency_edges
        .iter()
        .any(|edge| { edge.package == "fixture-app" && edge.dependency == "fixture-dep" }));
    let encoded = serde_json::to_string(&snapshot).unwrap();
    assert!(!encoded.contains(root.path().to_string_lossy().as_ref()));
}

#[test]
fn generic_non_cargo_adapter_is_valid_and_cargo_is_unavailable() {
    let root = tempfile::tempdir().unwrap();
    write(root.path().join("notes.txt"), b"generic\n");
    let policy = policy();

    let (generic, outcome) = GenericAdapter
        .run(&request(), &policy, root.path())
        .unwrap();
    assert_eq!(outcome.status, "completed");
    assert!(generic.as_array().is_some());

    let (cargo, outcome) = CargoAdapter.run(&request(), &policy, root.path()).unwrap();
    assert!(cargo.is_null());
    assert_eq!(outcome.status, "unavailable");

    let capabilities = discover_capabilities(root.path());
    assert!(capabilities
        .iter()
        .any(|capability| capability.capability == "generic_inventory" && capability.available));
    assert!(capabilities
        .iter()
        .any(|capability| capability.capability == "cargo_workspace" && !capability.available));
}

#[test]
fn adapters_honor_explicit_provider_allowlist() {
    let root = tempfile::tempdir().unwrap();
    write(root.path().join("notes.txt"), b"generic\n");
    let mut denied = policy();
    denied.provider_allowlist.clear();

    let (value, outcome) = GenericAdapter
        .run(&request(), &denied, root.path())
        .unwrap();
    assert!(value.is_null());
    assert_eq!(outcome.status, "skipped_by_policy");
}

#[test]
fn git_snapshot_is_bounded_relative_and_truthful() {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &["init", "-q"]);
    git(root.path(), &["config", "user.name", "Rúmil Test"]);
    git(
        root.path(),
        &["config", "user.email", "rumil@example.invalid"],
    );
    write(root.path().join("tracked.txt"), b"initial\n");
    git(root.path(), &["add", "tracked.txt"]);
    git(root.path(), &["commit", "-q", "-m", "initial"]);
    write(root.path().join("tracked.txt"), b"modified\n");
    write(root.path().join("untracked.txt"), b"new\n");

    let snapshot = GitAdapter.inspect(root.path(), &policy()).unwrap();
    assert_eq!(snapshot.revision.as_deref().map(str::len), Some(40));
    assert!(snapshot.dirty);
    assert!(!snapshot.truncated);
    assert!(snapshot
        .status_entries
        .iter()
        .any(|entry| entry.relative_path == "tracked.txt"));
    assert!(snapshot
        .status_entries
        .iter()
        .any(|entry| entry.relative_path == "untracked.txt"));
    assert!(snapshot
        .status_entries
        .iter()
        .all(|entry| !entry.relative_path.starts_with('/')));
}

#[test]
fn git_status_budget_discloses_truncation() {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &["init", "-q"]);
    write(root.path().join("a.txt"), b"a\n");
    write(root.path().join("b.txt"), b"b\n");
    let mut bounded = policy();
    bounded.budget.max_files = 1;

    let snapshot = GitAdapter.inspect(root.path(), &bounded).unwrap();
    assert_eq!(snapshot.revision, None);
    assert!(snapshot.truncated);
    assert_eq!(snapshot.status_entries.len(), 1);
    assert!(snapshot
        .truncation_reasons
        .iter()
        .any(|reason| reason.contains("file_budget")));
}

fn write(path: impl AsRef<Path>, contents: &[u8]) {
    let path = path.as_ref();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success(), "git command failed: {args:?}");
}
