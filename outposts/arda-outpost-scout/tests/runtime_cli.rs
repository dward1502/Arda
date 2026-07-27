use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn scout_runtime_help_exposes_serve_and_run_topics_commands() {
    let mut command = Command::cargo_bin("arda-outpost-scout").expect("scout binary");
    command
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("serve"))
        .stdout(predicate::str::contains("run-topics"));
}
