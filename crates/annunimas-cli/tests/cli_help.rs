use std::process::Command;

fn annunimas_cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_annunimas-cli"))
}

#[test]
fn top_level_help_lists_core_operator_surfaces() {
    let output_result = annunimas_cli().arg("--help").output();
    assert!(
        output_result.is_ok(),
        "failed to execute annunimas-cli --help: {:?}",
        output_result.err()
    );
    let output = match output_result {
        Ok(output) => output,
        Err(_) => return,
    };

    assert!(
        output.status.success(),
        "--help should exit successfully; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    for command in [
        "status",
        "tools",
        "utility",
        "charon",
        "hermes",
        "prometheus",
    ] {
        assert!(
            stdout.contains(command),
            "top-level help should list `{command}`; stdout={stdout}"
        );
    }
}

#[test]
fn task_pivot_help_documents_bounded_queue_append_controls() {
    let output_result = annunimas_cli()
        .args(["utility", "task-pivot", "--help"])
        .output();
    assert!(
        output_result.is_ok(),
        "failed to execute task-pivot --help: {:?}",
        output_result.err()
    );
    let output = match output_result {
        Ok(output) => output,
        Err(_) => return,
    };

    assert!(
        output.status.success(),
        "task-pivot --help should exit successfully; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    for flag in ["--owner", "--priority", "--status", "--result", "--dry-run"] {
        assert!(
            stdout.contains(flag),
            "task-pivot help should document `{flag}`; stdout={stdout}"
        );
    }
}

#[test]
fn oracle_help_documents_read_only_philosopher_profile_projection() {
    let output_result = annunimas_cli()
        .args(["oracle", "philosopher-profiles", "--help"])
        .output();
    assert!(
        output_result.is_ok(),
        "failed to execute oracle philosopher-profiles --help: {:?}",
        output_result.err()
    );
    let output = match output_result {
        Ok(output) => output,
        Err(_) => return,
    };

    assert!(
        output.status.success(),
        "oracle philosopher-profiles --help should exit successfully; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    for text in [
        "--profiles-path",
        "--format",
        "compact",
        "json",
        "status",
        "config/governance/philosophers.toml",
        "bootstrap profile status",
    ] {
        assert!(
            stdout.contains(text),
            "oracle philosopher-profiles help should document `{text}`; stdout={stdout}"
        );
    }
}

#[test]
fn oracle_philosopher_profiles_status_format_is_parseable_non_blocking_json() {
    let output_result = annunimas_cli()
        .args([
            "--config",
            "../../config/default.toml",
            "oracle",
            "philosopher-profiles",
            "--format",
            "status",
        ])
        .output();
    assert!(
        output_result.is_ok(),
        "failed to execute oracle philosopher-profiles --format status: {:?}",
        output_result.err()
    );
    let output = match output_result {
        Ok(output) => output,
        Err(_) => return,
    };

    assert!(
        output.status.success(),
        "oracle philosopher-profiles --format status should exit successfully; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("status format should emit parseable JSON");
    assert_eq!(
        parsed.get("surface").and_then(|value| value.as_str()),
        Some("annunimas.governance.philosopher_profiles.status.v1")
    );
    assert_eq!(
        parsed
            .get("autonomous_blocking_enabled")
            .and_then(|value| value.as_bool()),
        Some(false)
    );
    assert_eq!(
        parsed
            .get("generated_corpus_promotion_enabled")
            .and_then(|value| value.as_bool()),
        Some(false)
    );
    assert_eq!(
        parsed.get("profile_count").and_then(|value| value.as_u64()),
        Some(3)
    );
}

#[test]
fn oracle_philosopher_profiles_compact_format_reports_operator_summary() {
    let output_result = annunimas_cli()
        .args([
            "--config",
            "../../config/default.toml",
            "oracle",
            "philosopher-profiles",
            "--format",
            "compact",
        ])
        .output();
    assert!(
        output_result.is_ok(),
        "failed to execute oracle philosopher-profiles --format compact: {:?}",
        output_result.err()
    );
    let output = match output_result {
        Ok(output) => output,
        Err(_) => return,
    };

    assert!(
        output.status.success(),
        "oracle philosopher-profiles --format compact should exit successfully; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    for text in [
        "Triad Philosopher Profiles",
        "Profiles:    3",
        "blocking=disabled",
        "corpus_promotion=disabled",
        "aurelius",
    ] {
        assert!(
            stdout.contains(text),
            "compact philosopher profile output should include `{text}`; stdout={stdout}"
        );
    }
}
