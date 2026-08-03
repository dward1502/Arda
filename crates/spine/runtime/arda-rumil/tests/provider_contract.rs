#![cfg(feature = "provider")]

use arda_rumil::providers::source::{inspect_sources, SourceInspectionPolicy};
use arda_rumil::providers::{CommandProviderSpec, ProviderRunner};
use arda_rumil::{
    AuditPolicy, BudgetPolicy, CommandReceiptStatus, RootIdentity, PROVIDER_MALFORMED_OUTPUT,
};
use uuid::Uuid;

fn policy(provider_id: &str) -> AuditPolicy {
    AuditPolicy {
        profile_id: "provider-test-v1".into(),
        root_identity: RootIdentity {
            project_id: Uuid::new_v4(),
            name: "fixture".into(),
            kind: "generic".into(),
            remote_url: None,
        },
        root_relative: ".".into(),
        exclusion_rules: Vec::new(),
        budget: BudgetPolicy::default(),
        provider_allowlist: vec![provider_id.to_string()],
        redaction_policy: Vec::new(),
    }
}

fn shell_spec(provider_id: &str, script: &str) -> CommandProviderSpec {
    let mut spec =
        CommandProviderSpec::new(provider_id, "test_capability", "/bin/sh", ["-c", script]);
    spec.version_args.clear();
    spec.timeout_seconds = 2;
    spec.max_stdout_bytes = 1024;
    spec.max_stderr_bytes = 1024;
    spec
}

#[tokio::test]
async fn completed_provider_has_bounded_evidence_receipt() {
    let root = tempfile::tempdir().unwrap();
    let spec = shell_spec("test.completed", "printf '{\"ok\":true}'");
    let execution = ProviderRunner
        .run(&spec, &policy(&spec.provider_id), root.path())
        .await
        .unwrap();

    assert_eq!(execution.receipt.status, CommandReceiptStatus::Completed);
    assert_eq!(execution.receipt.exit_code, Some(0));
    assert!(!execution.receipt.argv_digest.is_empty());
    assert!(execution.receipt.stdout_digest.is_some());
    assert!(execution.receipt.stderr_digest.is_some());
    assert!(execution.receipt.configuration_digest.is_some());
    assert_eq!(execution.receipt.working_directory_relative, ".");
    assert_eq!(execution.receipt.authority, "review_only");
    let (value, outcome) = execution.json_outcome(&spec);
    assert_eq!(value.unwrap()["ok"], true);
    assert_eq!(outcome.status, "completed");
}

#[tokio::test]
async fn denied_provider_never_executes_and_still_has_receipt() {
    let root = tempfile::tempdir().unwrap();
    let marker = root.path().join("must-not-exist");
    let spec = shell_spec("test.denied", "touch must-not-exist");
    let execution = ProviderRunner
        .run(&spec, &policy("different.provider"), root.path())
        .await
        .unwrap();

    assert_eq!(execution.receipt.status, CommandReceiptStatus::Denied);
    assert!(!marker.exists());
    assert!(execution.receipt.finished_at_utc.is_some());
}

#[tokio::test]
async fn nonzero_exit_is_failed_not_success() {
    let root = tempfile::tempdir().unwrap();
    let spec = shell_spec("test.nonzero", "printf problem >&2; exit 7");
    let execution = ProviderRunner
        .run(&spec, &policy(&spec.provider_id), root.path())
        .await
        .unwrap();

    assert_eq!(execution.receipt.status, CommandReceiptStatus::Failed);
    assert_eq!(execution.receipt.exit_code, Some(7));
    assert_eq!(execution.stderr, b"problem");
}

#[tokio::test]
async fn timeout_kills_provider_and_is_disclosed() {
    let root = tempfile::tempdir().unwrap();
    let mut spec = shell_spec("test.timeout", "sleep 2");
    spec.timeout_seconds = 0;
    let execution = ProviderRunner
        .run(&spec, &policy(&spec.provider_id), root.path())
        .await
        .unwrap();

    assert_eq!(execution.receipt.status, CommandReceiptStatus::TimedOut);
    assert!(execution.receipt.timed_out);
    assert_eq!(execution.receipt.exit_code, None);
}

#[tokio::test]
async fn missing_program_is_unavailable() {
    let root = tempfile::tempdir().unwrap();
    let mut spec = CommandProviderSpec::new(
        "test.unavailable",
        "test_capability",
        "/definitely/not/a/rumil-provider",
        std::iter::empty::<String>(),
    );
    spec.version_args.clear();
    let execution = ProviderRunner
        .run(&spec, &policy(&spec.provider_id), root.path())
        .await
        .unwrap();

    assert_eq!(execution.receipt.status, CommandReceiptStatus::Unavailable);
}

#[tokio::test]
async fn stdout_is_retained_to_budget_but_digest_covers_full_stream() {
    let root = tempfile::tempdir().unwrap();
    let mut spec = shell_spec("test.truncated", "printf 123456789");
    spec.max_stdout_bytes = 4;
    let execution = ProviderRunner
        .run(&spec, &policy(&spec.provider_id), root.path())
        .await
        .unwrap();

    assert_eq!(execution.stdout, b"1234");
    assert_eq!(execution.receipt.stdout_bytes_retained, 4);
    assert!(execution.receipt.truncated);
    assert_eq!(
        execution.receipt.stdout_digest.as_deref(),
        Some("15e2b0d3c33891ebb0f1ef609ec419420c20e320ce94c65fbc8c3312448eb225")
    );
    let (value, outcome) = execution.json_outcome(&spec);
    assert!(value.is_none());
    assert_eq!(outcome.status, PROVIDER_MALFORMED_OUTPUT);
}

#[tokio::test]
async fn malformed_json_is_an_explicit_capability_state() {
    let root = tempfile::tempdir().unwrap();
    let spec = shell_spec("test.malformed", "printf not-json");
    let execution = ProviderRunner
        .run(&spec, &policy(&spec.provider_id), root.path())
        .await
        .unwrap();
    let (value, outcome) = execution.json_outcome(&spec);

    assert!(value.is_none());
    assert_eq!(outcome.status, PROVIDER_MALFORMED_OUTPUT);
}

#[tokio::test]
async fn escaped_working_directory_is_denied() {
    let root = tempfile::tempdir().unwrap();
    let mut spec = shell_spec("test.escape", "printf ok");
    spec.working_directory_relative = "../outside".into();
    let execution = ProviderRunner
        .run(&spec, &policy(&spec.provider_id), root.path())
        .await
        .unwrap();

    assert_eq!(execution.receipt.status, CommandReceiptStatus::Denied);
}

#[tokio::test]
async fn tool_version_is_bounded_and_preserved() {
    let root = tempfile::tempdir().unwrap();
    let mut spec = CommandProviderSpec::new("test.version", "test_capability", "printf", ["{}"]);
    spec.version_args = vec!["tool-v1".into()];
    let execution = ProviderRunner
        .run(&spec, &policy(&spec.provider_id), root.path())
        .await
        .unwrap();

    assert_eq!(execution.receipt.tool_version.as_deref(), Some("tool-v1"));
}

#[test]
fn source_inspection_is_selected_bounded_and_redacted() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("selected.rs"), "token=SECRET\nmore text\n").unwrap();
    std::fs::write(root.path().join("not-selected.rs"), "do not read\n").unwrap();
    let excerpts = inspect_sources(
        root.path(),
        &SourceInspectionPolicy {
            relative_paths: vec!["selected.rs".into()],
            max_excerpt_bytes_per_file: 18,
            max_total_excerpt_bytes: 18,
            redaction_patterns: vec!["SECRET".into()],
        },
    )
    .unwrap();

    assert_eq!(excerpts.len(), 1);
    assert_eq!(excerpts[0].relative_path, "selected.rs");
    assert!(excerpts[0].redacted);
    assert!(excerpts[0].truncated);
    assert!(!excerpts[0].content.contains("SECRET"));
    assert!(excerpts[0].content.len() <= 18);
}

#[test]
fn source_inspection_rejects_parent_traversal() {
    let root = tempfile::tempdir().unwrap();
    let result = inspect_sources(
        root.path(),
        &SourceInspectionPolicy {
            relative_paths: vec!["../outside".into()],
            max_excerpt_bytes_per_file: 10,
            max_total_excerpt_bytes: 10,
            redaction_patterns: Vec::new(),
        },
    );
    assert!(result.is_err());
}
