use std::fs;
use std::process::Command;

#[test]
fn retired_autopilot_commands_refuse_before_reading_legacy_history() {
    let root = tempfile::tempdir().unwrap();
    let tasks = root.path().join("core/projects/tasks");
    fs::create_dir_all(&tasks).unwrap();
    fs::write(tasks.join("queue.jsonl"), "not valid JSON\n").unwrap();
    for command in ["once", "run", "status"] {
        let output = Command::new(env!("CARGO_BIN_EXE_arda-cli"))
            .args(["prometheus", "autopilot", command, "--root"])
            .arg(root.path())
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("legacy JSONL autopilot is retired")
        );
        assert!(output.stdout.is_empty());
        assert!(!root.path().join("core/state/queue_active.json").exists());
        assert!(!root.path().join("data/ceo").exists());
    }
    assert_eq!(
        fs::read_to_string(tasks.join("queue.jsonl")).unwrap(),
        "not valid JSON\n"
    );
}
