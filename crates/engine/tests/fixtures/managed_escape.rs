//! Actual configured bootstrap + hostile local descendant, without provider calls.
use super::*;
use std::io::{BufRead, BufReader, Write};

fn unsuccessful_response(value: &serde_json::Value) -> bool {
    match value.get("ok").and_then(serde_json::Value::as_bool) {
        Some(false) => true,
        Some(true) => value
            .get("code")
            .is_some_and(|code| code.is_null() || code.as_i64().is_some_and(|code| code != 0)),
        None => false,
    }
}

#[test]
fn response_oracle_rejects_success_and_malformed_replies() {
    for value in [
        serde_json::json!({"ok":true,"code":0}),
        serde_json::json!({"ok":true}),
        serde_json::json!({"ok":true,"exit_code":1}),
        serde_json::json!({"ok":true,"code":"invalid"}),
        serde_json::json!({}),
    ] {
        assert!(!unsuccessful_response(&value), "accepted {value}");
    }
    for value in [
        serde_json::json!({"ok":false,"error":"stopped"}),
        serde_json::json!({"ok":true,"code":null}),
        serde_json::json!({"ok":true,"code":1}),
    ] {
        assert!(unsuccessful_response(&value), "rejected {value}");
    }
}

pub fn configure(policy: &Path, root: &Path) {
    let code = root.join("escape-code");
    fs::create_dir_all(code.join("hermes_cli")).unwrap();
    fs::write(code.join("hermes_cli/__init__.py"), "").unwrap();
    fs::write(code.join("hermes_cli/env_loader.py"), "").unwrap();
    fs::write(
        code.join("hermes_cli/main.py"),
        include_str!("managed_escape.py"),
    )
    .unwrap();
    let mut raw: serde_json::Value = serde_json::from_slice(&fs::read(policy).unwrap()).unwrap();
    raw["entrypoint"]["ordered_import_roots"] = serde_json::json!(["/opt/escape-fixture"]);
    for grant in raw["grants"].as_array_mut().unwrap() {
        if grant["id"] == "code" {
            grant["source"] = serde_json::json!(code);
            grant["destination"] = serde_json::json!("/opt/escape-fixture");
        }
    }
    fs::write(policy, serde_json::to_vec(&raw).unwrap()).unwrap();
}

pub struct Attack {
    request: std::thread::JoinHandle<()>,
    pids: Vec<String>,
}
impl Attack {
    pub fn after_stop(self) {
        self.request.join().unwrap();
        for pid in self.pids {
            assert!(
                !Path::new("/proc").join(pid).exists(),
                "detached descendant survived managed stop"
            );
        }
    }
}

pub fn start(
    binding: &arda_engine::objectives::RetainedExecution,
    workspace: &Path,
    unit: &str,
) -> Attack {
    let group = command(&[
        "systemctl",
        "--user",
        "show",
        unit,
        "--property=ControlGroup",
        "--value",
    ]);
    let group_path = Path::new("/sys/fs/cgroup").join(group.trim().trim_start_matches('/'));
    let parent = group_path.parent().unwrap();
    let uid = unsafe { libc::geteuid() };
    fs::write(
        workspace.join("escape-targets.json"),
        serde_json::to_vec(&serde_json::json!({
            "cgroup_parent_denied": parent.join("cgroup.procs"),
            "proc_root_cgroup_denied": format!("/proc/1/root{}/cgroup.procs", parent.display()),
            "bus_user_denied": format!("/run/user/{uid}/bus"),
            "bus_manager_denied": format!("/run/user/{uid}/systemd/private")
        }))
        .unwrap(),
    )
    .unwrap();
    let socket = binding.snapshot.endpoint.clone();
    let request = serde_json::json!({"op":"runtime", "capability":binding.snapshot.capability,
        "lease": {"run_id":binding.lease.run_id,"generation":binding.lease.generation,"owner":binding.lease.owner,"expires_ms":binding.lease.expires_ms},
        "operation":{"operation":"probe"}, "timeout_ms":60000,"max_output_bytes":65536});
    let request = std::thread::spawn(move || {
        let mut stream = std::os::unix::net::UnixStream::connect(socket).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(35)))
            .unwrap();
        writeln!(stream, "{request}").unwrap();
        let mut response = String::new();
        // Managed stop may reset the transport, but cannot acknowledge success.
        if let Ok(n) = BufReader::new(stream).read_line(&mut response) {
            if n != 0 {
                let value: serde_json::Value = serde_json::from_str(&response).unwrap();
                assert!(
                    unsuccessful_response(&value),
                    "hostile fixture unexpectedly completed"
                );
            }
        }
    });
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !workspace.join("escape-report.json").exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "escape fixture did not execute"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(workspace.join("escape-report.json")).unwrap()).unwrap();
    for (key, value) in report.as_object().unwrap() {
        if key != "namespace_pid" {
            assert_eq!(value, true, "escape attempt not denied: {key}");
        }
    }
    let pids: Vec<String> = fs::read_to_string(group_path.join("cgroup.procs"))
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    let namespace_pid = report["namespace_pid"].as_u64().unwrap().to_string();
    assert!(
        pids.iter().any(
            |pid| fs::read_to_string(format!("/proc/{pid}/status")).is_ok_and(|status| status
                .lines()
                .any(|line| line.starts_with("NSpid:")
                    && line.split_whitespace().last() == Some(namespace_pid.as_str())))
        ),
        "detached descendant not in managed cgroup"
    );
    Attack { request, pids }
}
