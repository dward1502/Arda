//! Deterministic CLI transport fixture; not genuine provider evidence.
use std::{fs, os::unix::fs::PermissionsExt, path::Path};

#[test]
fn session_exports_are_exact_and_workspace_scoped() {
    let root = tempfile::TempDir::new().unwrap();
    install_fake_hermes(root.path());
    let executable = root.path().join("fake-hermes-context");
    let workspaces = [root.path().join("one"), root.path().join("two")];
    for path in &workspaces {
        fs::create_dir(path).unwrap();
    }
    let mut workers = Vec::new();
    for index in 0..4 {
        let cwd = workspaces[index % 2].clone();
        let executable = executable.clone();
        workers.push(std::thread::spawn(move || {
            let prompt = format!("invocation-{index}\nCanonical node context follows:\n{{\"node\":{{\"kind\":\"execute\"}}}}");
            let output = std::process::Command::new(executable)
                .current_dir(&cwd)
                .args([
                    "-q",
                    &prompt,
                ])
                .output()
                .unwrap();
            assert!(output.status.success(), "{:?}", output);
            let text = String::from_utf8(output.stdout).unwrap();
            (
                cwd,
                text.lines()
                    .next()
                    .unwrap()
                    .strip_prefix("session_id: ")
                    .unwrap()
                    .to_owned(),
                prompt,
            )
        }));
    }
    let sessions: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    let ids: std::collections::BTreeSet<_> = sessions.iter().map(|(_, id, _)| id).collect();
    assert_eq!(ids.len(), 4);
    for (cwd, id, prompt) in &sessions {
        let exported = std::process::Command::new(&executable)
            .current_dir(cwd)
            .args(["sessions", "export", "-", "--session-id", id])
            .output()
            .unwrap();
        assert!(exported.status.success());
        let session: serde_json::Value = serde_json::from_slice(&exported.stdout).unwrap();
        assert_eq!(session["id"].as_str(), Some(id.as_str()));
        use sha2::{Digest, Sha256};
        assert_eq!(
            session["messages"].as_array().unwrap().last().unwrap()["content"],
            format!(
                "fixture prompt sha256:{:x}",
                Sha256::digest(prompt.as_bytes())
            )
        );
        let other = workspaces.iter().find(|path| *path != cwd).unwrap();
        let wrong = std::process::Command::new(&executable)
            .current_dir(other)
            .args(["sessions", "export", "-", "--session-id", id])
            .output()
            .unwrap();
        assert!(!wrong.status.success());
    }
    for cwd in workspaces {
        let missing = std::process::Command::new(&executable)
            .current_dir(&cwd)
            .args([
                "sessions",
                "export",
                "-",
                "--session-id",
                "fresh-context-worker-missing",
            ])
            .output()
            .unwrap();
        assert!(!missing.status.success());
        assert_eq!(
            fs::read_to_string(cwd.join("context-worker-count")).unwrap(),
            "2"
        );
        assert_eq!(
            fs::read_to_string(cwd.join("context-prompts.jsonl"))
                .unwrap()
                .lines()
                .count(),
            2
        );
    }
}

pub fn install_fake_hermes(root: &Path) {
    let executable = root.join("fake-hermes-context");
    fs::write(
        &executable,
        r#"#!/usr/bin/python3
import fcntl, hashlib, json, sys, uuid
from pathlib import Path
root = Path(__file__).parent
capture_root = Path.cwd()
sessions = capture_root / "context-sessions"
args = sys.argv[1:]
if args[:2] == ["sessions", "export"]:
    session_id = args[args.index("--session-id") + 1]
    if not session_id.startswith("fresh-context-worker-") or any(c not in "abcdefghijklmnopqrstuvwxyz0123456789-" for c in session_id):
        raise SystemExit("invalid session id")
    transcript = sessions / (session_id + ".json")
    print(transcript.read_text(encoding="utf-8"), flush=True)
    raise SystemExit(0)
prompt = args[args.index("-q") + 1]
count_path = capture_root / "context-worker-count"
session_id = "fresh-context-worker-" + uuid.uuid4().hex
sessions.mkdir(exist_ok=True)
transcript = sessions / (session_id + ".json")
with (capture_root / "context-worker.lock").open("a") as lock:
    fcntl.flock(lock, fcntl.LOCK_EX)
    count = int(count_path.read_text() if count_path.exists() else "0") + 1
    count_path.write_text(str(count), encoding="utf-8")
    with (capture_root / "context-prompts.jsonl").open("a", encoding="utf-8") as handle:
        handle.write(json.dumps({"worker":count,"session_id":session_id,"prompt":prompt}) + "\n")
tool_result = json.dumps({"output":"ok\n","exit_code":0,"error":None}, separators=(",", ":"))
session = {
 "id":session_id,"source":"tool","model":"fixture-model",
 "billing_provider":"fixture-provider","estimated_cost_usd":0.0,"actual_cost_usd":0.0,
 "input_tokens":10,"output_tokens":10,"api_call_count":1,
 "messages":[
  {"role":"assistant","content":None,"tool_calls":[{"id":"call-test-1","type":"function","function":{"name":"terminal","arguments":json.dumps({"command":"python3 verify-context-bootstrap.py"})}}]},
  {"role":"tool","tool_call_id":"call-test-1","tool_name":"terminal","content":tool_result}
 ]
}
transcript.write_text(json.dumps(session), encoding="utf-8")
result={"schema_version":"arda.hermes-job-result.v1","status":"succeeded","summary":"Fresh worker completed the bounded task from governed context.","tool_evidence":[{"tool_call_id":"call-test-1"}],"test_evidence":[{"check_id":"test","tool_call_id":"call-test-1"}],"artifacts":[]}
node_context = json.JSONDecoder().raw_decode(prompt.split("Canonical node context follows:\n", 1)[1])[0]
if node_context["node"]["kind"] == "inspect" or (node_context["node"]["kind"] == "verify" and not node_context["checks"]):
    result["test_evidence"] = []
    session["messages"] = [
        {"role":"assistant","content":None,"tool_calls":[{"id":"call-test-1","type":"function","function":{"name":"read_file","arguments":json.dumps({"path":"verify-context-bootstrap.py"})}}]},
        {"role":"tool","tool_call_id":"call-test-1","tool_name":"read_file","content":json.dumps({"content":"Synthetic fixture source inspection"})}
    ]
if node_context["node"]["kind"] == "review":
    result["summary"] = "VERDICT: APPROVE\nDeterministic fixture review accepted the bounded result."
    result["test_evidence"] = []
    receipt_path = root / "data/runs" / node_context["run_id"] / "execution-receipts/verify.json"
    receipt_text = receipt_path.read_text(encoding="utf-8")
    session["messages"] = [
        {"role":"assistant","content":None,"tool_calls":[{"id":"call-test-1","type":"function","function":{"name":"read_file","arguments":json.dumps({"path":str(receipt_path)})}}]},
        {"role":"tool","tool_call_id":"call-test-1","tool_name":"read_file","content":json.dumps({"content":receipt_text})}
    ]
    transcript.write_text(json.dumps(session), encoding="utf-8")
session["messages"].append({"role":"user","content":"fixture prompt sha256:" + hashlib.sha256(prompt.encode("utf-8")).hexdigest()})
transcript.write_text(json.dumps(session), encoding="utf-8")
print(f"session_id: {session_id}")
print(json.dumps(result), flush=True)
"#,
    )
    .unwrap();
    let mut permissions = fs::metadata(&executable).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&executable, permissions).unwrap();
    let config = root.join("config/adapters/hermes-workbench.toml");
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    fs::write(
        config,
        format!(
            "schema_version = \"arda.hermes-adapter.v1\"\nadapter_version = \"context-bootstrap-test\"\nexecutable = \"{}\"\nmax_timeout_ms = 10000\ncancellation_grace_ms = 100\nmax_turns = 4\nmax_prompt_bytes = 131072\nmax_output_bytes = 1048576\ninherit_environment = [\"PATH\"]\n\n[toolsets]\nread_only = [\"file\"]\nhuman_approval = []\nexecute_with_approval = [\"file\", \"terminal\"]\nverify = [\"file\", \"terminal\"]\ncompensate_with_approval = [\"file\", \"terminal\"]\n",
            executable.display()
        ),
    )
    .unwrap();
}
