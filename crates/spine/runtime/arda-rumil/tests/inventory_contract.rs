#![cfg(feature = "walkdir")]

use arda_rumil::{
    inventory_repo, AuditPolicy, BudgetPolicy, ExclusionKind, ExclusionRule, RootIdentity,
    TreeEntryKind,
};
use uuid::Uuid;

fn policy() -> AuditPolicy {
    AuditPolicy {
        profile_id: "generic-test-v1".into(),
        root_identity: RootIdentity {
            project_id: Uuid::new_v4(),
            name: "fixture".into(),
            kind: "generic".into(),
            remote_url: None,
        },
        root_relative: ".".into(),
        exclusion_rules: Vec::new(),
        budget: BudgetPolicy::default(),
        provider_allowlist: Vec::new(),
        redaction_policy: Vec::new(),
    }
}

#[test]
fn generic_non_cargo_root_produces_relative_inventory() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("docs")).unwrap();
    std::fs::write(root.path().join("docs/readme.txt"), b"hello").unwrap();

    let report = inventory_repo(root.path(), &policy()).unwrap();
    let paths: Vec<&str> = report
        .entries
        .iter()
        .map(|entry| entry.relative_path.as_str())
        .collect();

    assert!(paths.contains(&"."));
    assert!(paths.contains(&"docs"));
    assert!(paths.contains(&"docs/readme.txt"));
    assert!(paths.iter().all(|path| !path.starts_with('/')));
    assert!(report.truncation_reasons.is_empty());
}

#[test]
fn empty_root_is_a_complete_root_only_inventory() {
    let root = tempfile::tempdir().unwrap();
    let report = inventory_repo(root.path(), &policy()).unwrap();

    assert!(report.is_complete());
    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.entries[0].relative_path, ".");
    assert_eq!(report.entries[0].kind, TreeEntryKind::Directory);
}

#[test]
fn default_exclusions_are_disclosed_and_not_descended() {
    let root = tempfile::tempdir().unwrap();
    for name in [".git", "target", "node_modules"] {
        std::fs::create_dir(root.path().join(name)).unwrap();
        std::fs::write(root.path().join(name).join("hidden.txt"), b"hidden").unwrap();
    }
    std::fs::create_dir(root.path().join("nested")).unwrap();
    std::fs::write(root.path().join("nested/.env"), b"SECRET=x").unwrap();

    let report = inventory_repo(root.path(), &policy()).unwrap();
    for name in [".git", "target", "node_modules", "nested/.env"] {
        let entry = report
            .entries
            .iter()
            .find(|entry| entry.relative_path == name)
            .expect("excluded entry");
        assert_eq!(entry.kind, TreeEntryKind::Excluded);
    }
    assert!(!report
        .entries
        .iter()
        .any(|entry| entry.relative_path.ends_with("hidden.txt")));
    assert!(!report
        .entries
        .iter()
        .any(|entry| { entry.relative_path == "nested/.env" && entry.content_sha256.is_some() }));
}

#[test]
fn policy_directory_and_glob_exclusions_are_applied() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("private")).unwrap();
    std::fs::write(root.path().join("private/data.txt"), b"private").unwrap();
    std::fs::write(root.path().join("token.secret"), b"secret").unwrap();

    let mut policy = policy();
    policy.exclusion_rules = vec![
        ExclusionRule {
            pattern: "private".into(),
            kind: ExclusionKind::Directory,
        },
        ExclusionRule {
            pattern: "*.secret".into(),
            kind: ExclusionKind::Glob,
        },
    ];

    let report = inventory_repo(root.path(), &policy).unwrap();
    assert!(report.entries.iter().any(|entry| {
        entry.relative_path == "private" && entry.kind == TreeEntryKind::Excluded
    }));
    assert!(report.entries.iter().any(|entry| {
        entry.relative_path == "token.secret" && entry.kind == TreeEntryKind::Excluded
    }));
    assert!(!report
        .entries
        .iter()
        .any(|entry| entry.relative_path == "private/data.txt"));
}

#[test]
fn ordering_is_deterministic() {
    let root = tempfile::tempdir().unwrap();
    for name in ["z.txt", "a.txt", "m.txt"] {
        std::fs::write(root.path().join(name), name.as_bytes()).unwrap();
    }

    let first: Vec<String> = inventory_repo(root.path(), &policy())
        .unwrap()
        .entries
        .into_iter()
        .map(|entry| entry.relative_path)
        .collect();
    let second: Vec<String> = inventory_repo(root.path(), &policy())
        .unwrap()
        .entries
        .into_iter()
        .map(|entry| entry.relative_path)
        .collect();

    assert_eq!(first, second);
    assert_eq!(first, vec![".", "a.txt", "m.txt", "z.txt"]);
}

#[test]
fn byte_budget_is_disclosed_without_hashing_oversize_file() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("large.bin"), [0_u8; 64]).unwrap();
    let mut policy = policy();
    policy.budget.max_total_bytes = 32;

    let report = inventory_repo(root.path(), &policy).unwrap();
    assert!(report
        .truncation_reasons
        .iter()
        .any(|reason| reason.contains("byte_budget")));
    assert!(!report
        .entries
        .iter()
        .any(|entry| entry.relative_path == "large.bin"));
}

#[test]
fn zero_timeout_is_truthfully_disclosed() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("file.txt"), b"data").unwrap();
    let mut policy = policy();
    policy.budget.scan_timeout_seconds = 0;

    let report = inventory_repo(root.path(), &policy).unwrap();
    assert!(report
        .truncation_reasons
        .iter()
        .any(|reason| reason.contains("scan_timeout")));
}

#[test]
fn missing_root_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let missing = root.path().join("missing");
    assert!(inventory_repo(&missing, &policy()).is_err());
}

#[test]
fn file_root_and_root_policy_escape_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("file.txt");
    std::fs::write(&file, b"data").unwrap();
    assert!(inventory_repo(&file, &policy()).is_err());

    let mut escape = policy();
    escape.root_relative = "../outside".into();
    assert!(inventory_repo(root.path(), &escape).is_err());

    let mut absolute = policy();
    absolute.root_relative = "/tmp".into();
    assert!(inventory_repo(root.path(), &absolute).is_err());
}

#[test]
fn selected_subtree_remains_project_relative() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("docs")).unwrap();
    std::fs::write(root.path().join("docs/guide.md"), b"guide").unwrap();
    std::fs::write(root.path().join("outside.txt"), b"outside").unwrap();
    let mut subtree = policy();
    subtree.root_relative = "docs".into();

    let report = inventory_repo(root.path(), &subtree).unwrap();
    assert!(report
        .entries
        .iter()
        .any(|entry| entry.relative_path == "docs/guide.md"));
    assert!(!report
        .entries
        .iter()
        .any(|entry| entry.relative_path == "outside.txt"));
}

#[test]
fn file_count_budget_is_disclosed() {
    let root = tempfile::tempdir().unwrap();
    for name in ["a.txt", "b.txt", "c.txt"] {
        std::fs::write(root.path().join(name), b"x").unwrap();
    }
    let mut bounded = policy();
    bounded.budget.max_files = 1;

    let report = inventory_repo(root.path(), &bounded).unwrap();
    assert!(!report.is_complete());
    assert!(report
        .truncation_reasons
        .iter()
        .any(|reason| reason.contains("file_count_budget")));
    assert_eq!(report.summary().total_files, 1);
}

#[test]
fn small_binary_file_gets_a_digest_and_summary() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("sample.bin"), [0_u8, 159, 255, 0]).unwrap();

    let report = inventory_repo(root.path(), &policy()).unwrap();
    let entry = report
        .entries
        .iter()
        .find(|entry| entry.relative_path == "sample.bin")
        .unwrap();
    assert_eq!(entry.content_sha256.as_deref().map(str::len), Some(64));
    assert_eq!(report.summary().sampled_files, 1);
    assert_eq!(report.file_records().len(), report.entries.len());
}

#[cfg(unix)]
#[test]
fn external_symlink_target_never_leaks_absolute_path() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    symlink("/etc/passwd", root.path().join("outside-link")).unwrap();
    let report = inventory_repo(root.path(), &policy()).unwrap();
    let entry = report
        .entries
        .iter()
        .find(|entry| entry.relative_path == "outside-link")
        .unwrap();
    assert_eq!(entry.kind, TreeEntryKind::Symlink);
    assert_eq!(entry.symlink_target_relative, None);
}

#[cfg(unix)]
#[test]
fn symlink_loop_is_recorded_without_being_followed() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("loop")).unwrap();
    symlink("../loop", root.path().join("loop/back")).unwrap();

    let report = inventory_repo(root.path(), &policy()).unwrap();
    let loops: Vec<_> = report
        .entries
        .iter()
        .filter(|entry| entry.relative_path == "loop/back")
        .collect();
    assert_eq!(loops.len(), 1);
    assert_eq!(loops[0].kind, TreeEntryKind::Symlink);
}

#[cfg(unix)]
#[test]
fn unreadable_directory_is_disclosed() {
    use std::os::unix::fs::PermissionsExt;

    let root = tempfile::tempdir().unwrap();
    let private = root.path().join("unreadable");
    std::fs::create_dir(&private).unwrap();
    std::fs::write(private.join("hidden.txt"), b"hidden").unwrap();
    std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o000)).unwrap();

    let report = inventory_repo(root.path(), &policy()).unwrap();
    std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o700)).unwrap();

    assert!(report.entries.iter().any(|entry| {
        entry.kind == TreeEntryKind::Unreadable && entry.relative_path.starts_with("unreadable")
    }));
}
