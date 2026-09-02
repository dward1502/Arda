use std::fs;
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::time::Duration;

use serde_json::Value;
use tempfile::TempDir;

fn arda_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_arda"))
}

fn write_registry(root: &Path, body: &str) {
    fs::write(root.join("services.toml"), body).expect("write services registry");
}

fn write_executable(path: &Path, body: &str) {
    fs::write(path, body).expect("write executable fixture");
    let mut permissions = fs::metadata(path).expect("fixture metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("make fixture executable");
}

fn run_once(root: &Path, cwd: &Path, extra_args: &[&str]) -> ExitStatus {
    Command::new(arda_bin())
        .current_dir(cwd)
        .env("ARDA_REPO_ROOT", root)
        .args(["--once"])
        .args(extra_args)
        .status()
        .expect("run arda --once")
}

#[test]
fn once_discovers_registry_from_an_ancestor_directory() {
    let temp = TempDir::new().expect("temporary repository");
    let root = temp.path();
    let nested = root.join("crates/example");
    fs::create_dir_all(&nested).expect("create nested directory");
    write_registry(
        root,
        r#"[[service]]
name = "headless"
required = true
tags = ["gateway"]
start.command = "/usr/bin/true"
start.cwd = "."
"#,
    );

    let status = Command::new(arda_bin())
        .current_dir(&nested)
        .arg("--once")
        .status()
        .expect("run arda from nested directory");

    assert!(status.success(), "nested-root smoke must succeed: {status}");
}

#[test]
fn once_rejects_an_invalid_registry() {
    let temp = TempDir::new().expect("temporary repository");
    write_registry(temp.path(), "service = []\n");

    let status = run_once(temp.path(), temp.path(), &[]);

    assert!(!status.success(), "empty registry must fail startup");
}

#[test]
fn once_rejects_a_missing_required_service() {
    let temp = TempDir::new().expect("temporary repository");
    write_registry(
        temp.path(),
        r#"[[service]]
name = "required-ghost"
required = true
start.command = "arda-definitely-missing-command"
"#,
    );

    let status = run_once(temp.path(), temp.path(), &[]);

    assert!(
        !status.success(),
        "missing required service must fail startup"
    );
}

#[test]
fn no_ui_drops_required_ui_and_once_never_spawns_resolved_children() {
    let temp = TempDir::new().expect("temporary repository");
    let marker = temp.path().join("spawned.marker");
    let worker = temp.path().join("worker.sh");
    write_executable(
        &worker,
        &format!("#!/bin/sh\nprintf spawned > '{}'\n", marker.display()),
    );
    write_registry(
        temp.path(),
        &format!(
            r#"[[service]]
name = "missing-ui"
required = true
tags = ["ui"]
start.command = "arda-definitely-missing-ui"

[[service]]
name = "headless"
required = true
tags = ["gateway"]
start.command = "{}"
start.cwd = "."
"#,
            worker.display()
        ),
    );

    let status = run_once(temp.path(), temp.path(), &["--no-ui"]);

    assert!(status.success(), "--no-ui --once must validate: {status}");
    assert!(
        !marker.exists(),
        "--once must exit before spawning a resolved child"
    );
}

#[tokio::test]
async fn harness_uses_warden_override_and_signal_shutdown_reaps_child() {
    let temp = TempDir::new().expect("temporary repository");
    let root = temp.path();
    let pid_file = root.join("worker.pid");
    let worker = root.join("worker.sh");
    write_executable(
        &worker,
        &format!(
            "#!/bin/sh\nprintf '%s' \"$$\" > '{}'\nexec /usr/bin/sleep 30\n",
            pid_file.display()
        ),
    );
    write_registry(
        root,
        &format!(
            r#"[[service]]
name = "headless-worker"
required = true
tags = ["gateway"]
start.command = "{}"
start.cwd = "."
"#,
            worker.display()
        ),
    );
    fs::create_dir_all(root.join("config")).expect("create config directory");
    fs::write(
        root.join("config/fleet.toml"),
        r#"[[nodes]]
id = "node-pi5-warden"
scout_url = "http://fleet.example:8092"
"#,
    )
    .expect("write fleet fixture");

    let port = {
        let listener = TcpListener::bind("127.0.0.1:0").expect("reserve harness port");
        listener.local_addr().expect("reserved address").port()
    };
    let harness_addr = format!("127.0.0.1:{port}");
    let mut daemon = Command::new(arda_bin())
        .current_dir(root)
        .env("ARDA_REPO_ROOT", root)
        .env("ARDA_OPERATOR_ID", "root-daemon-test-operator")
        .env("ARDA_WARDEN_SCOUT_URL", "http://env.example:8092")
        .args(["--no-ui", "--harness-addr", &harness_addr])
        .spawn()
        .expect("start daemon");

    let client = reqwest::Client::new();
    let status_json = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Ok(response) = client
                .get(format!("http://{harness_addr}/v1/status"))
                .send()
                .await
            {
                if response.status().is_success() {
                    break response.json::<Value>().await.expect("status JSON");
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("harness startup timeout");
    assert!(
        root.join("data/arda/objectives.sqlite3").exists(),
        "resident daemon must open the canonical ObjectiveStore"
    );
    assert_eq!(
        status_json["warden_scout_url"], "http://env.example:8092",
        "environment override must take precedence over fleet discovery"
    );

    tokio::time::timeout(Duration::from_secs(5), async {
        while !pid_file.exists() {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("supervised child did not start");
    let child_pid = fs::read_to_string(&pid_file).expect("read child pid");

    let runtime_status = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let response = client
                .get(format!("http://{harness_addr}/v1/status"))
                .send()
                .await
                .expect("runtime status request");
            let body = response.json::<Value>().await.expect("runtime status JSON");
            if body["service_statuses"][0]["state"] == "healthy" {
                break body;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("service never reached healthy state");
    assert_eq!(
        runtime_status["service_statuses"][0]["name"],
        "headless-worker"
    );
    assert_eq!(runtime_status["service_statuses"][0]["required"], true);
    assert_eq!(runtime_status["service_statuses"][0]["state"], "healthy");
    assert_eq!(
        runtime_status["service_statuses"][0]["pid"],
        child_pid.trim().parse::<u32>().expect("numeric child pid")
    );

    let signal = Command::new("kill")
        .args(["-INT", &daemon.id().to_string()])
        .status()
        .expect("send SIGINT");
    assert!(signal.success(), "SIGINT delivery must succeed");

    let daemon_status = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Some(status) = daemon.try_wait().expect("poll daemon") {
                break status;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("daemon did not stop after SIGINT");
    assert!(
        daemon_status.success(),
        "daemon shutdown failed: {daemon_status}"
    );

    let child_alive = Command::new("kill")
        .args(["-0", child_pid.trim()])
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    assert!(!child_alive, "supervised child survived daemon shutdown");
}
