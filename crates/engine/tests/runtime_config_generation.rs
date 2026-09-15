#![cfg(target_os = "linux")]
#[path = "fixtures/installed_policy.rs"]
#[allow(dead_code)] // Shared fixture exports are only partially used here.
mod installed;
use std::{fs, process::Command};

#[test]
fn fifo_policy_validation_fails_without_waiting_for_a_writer() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("policy.fifo");
    assert!(Command::new("mkfifo")
        .arg("-m")
        .arg("600")
        .arg(&path)
        .status()
        .unwrap()
        .success());
    let mut child = Command::new(env!("CARGO_BIN_EXE_arda-snapshot-keeper"))
        .arg("--validate-policy")
        .arg(&path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(!status.success());
            break;
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("policy validator blocked on FIFO input");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[test]
#[ignore = "requires installed system Python and Hermes import roots"]
fn generated_retained_config_is_private_idempotent_and_schema_valid() {
    let temp = tempfile::tempdir_in("/var/tmp").unwrap();
    let ordinary = installed::adapter_config(temp.path());
    let output = temp.path().join("config");
    let state = temp.path().join("sessions");
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/configure-retained-runtime.py");
    let invoke = |apply: bool| {
        let mut command = Command::new("/usr/bin/python3");
        command
            .arg(&script)
            .arg("--legacy-adapter")
            .arg(&ordinary)
            .arg("--output-root")
            .arg(&output)
            .arg("--session-base")
            .arg(&state);
        if apply {
            command.arg("--apply");
        }
        command.output().unwrap()
    };
    let dry = invoke(false);
    assert!(
        dry.status.success(),
        "{}",
        String::from_utf8_lossy(&dry.stderr)
    );
    assert!(!output.exists() && !state.exists());
    let applied = invoke(true);
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    let config = arda_engine::adapters::HermesAdapterConfig::from_toml_str(
        &fs::read_to_string(output.join("hermes-adapter.toml")).unwrap(),
    )
    .unwrap();
    assert!(config.inherit_environment.is_empty());
    let before = fs::read(output.join("runtime-policy.json")).unwrap();
    let second = invoke(true);
    assert!(second.status.success());
    assert_eq!(
        fs::read(output.join("runtime-policy.json")).unwrap(),
        before
    );
    let checked = Command::new(env!("CARGO_BIN_EXE_arda-snapshot-keeper"))
        .arg("--validate-policy")
        .arg(output.join("runtime-policy.json"))
        .output()
        .unwrap();
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    let status: serde_json::Value = serde_json::from_slice(&checked.stdout).unwrap();
    assert_eq!(status["status"], "schema_valid");
    fs::write(output.join("config.yaml"), "operator-modified\n").unwrap();
    assert!(!invoke(true).status.success());
    assert_eq!(
        fs::read_to_string(output.join("config.yaml")).unwrap(),
        "operator-modified\n"
    );
}
