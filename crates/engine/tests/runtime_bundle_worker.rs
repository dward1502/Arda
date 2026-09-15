#![cfg(target_os = "linux")]
use arda_engine::objectives::{
    runtime_policy::{transport, ValidatedRuntimePolicy},
    snapshot_protocol::Manifest,
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::{
        fd::AsRawFd,
        unix::{fs::MetadataExt, net::UnixStream, process::CommandExt},
    },
    process::{Command, Stdio},
    time::{Duration, Instant},
};
struct Worker(std::process::Child);
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn request(socket: &std::path::Path, value: serde_json::Value) -> serde_json::Value {
    let mut stream = UnixStream::connect(socket).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_millis(
            value["timeout_ms"]
                .as_u64()
                .unwrap_or(3000)
                .saturating_add(5000),
        )))
        .unwrap();
    writeln!(stream, "{value}").unwrap();
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).unwrap();
    serde_json::from_str(&line).unwrap()
}
#[test]
#[ignore = "requires qualified user and mount namespaces"]
fn configured_worker_captures_complete_bundle_and_releases() {
    bundle_test(false, false, false, false);
}
#[test]
#[ignore = "requires qualified user/mount namespaces"]
fn configured_keeper_validates_bundle_and_releases() {
    bundle_test(true, false, false, false);
}
#[test]
#[ignore = "requires qualified user/mount namespaces"]
fn configured_keeper_rejects_substitution_between_pin_and_capture() {
    bundle_test(true, true, false, false);
}
#[test]
#[ignore = "requires installed Hermes, Python 3.12 and qualified namespaces"]
fn installed_hermes_probe_uses_captured_import_roots() {
    bundle_test(true, false, true, false);
}

#[test]
#[ignore = "makes bounded real local-Manwe inference calls with installed Hermes"]
fn installed_hermes_chat_and_export_use_retained_state() {
    bundle_test(true, false, true, true);
}

fn bundle_test(keeper: bool, attack: bool, installed: bool, provider_chat: bool) {
    let temp = tempfile::tempdir_in("/var/tmp").unwrap();
    let control = tempfile::Builder::new()
        .prefix("arda-runtime-bundle-")
        .tempdir_in("/dev/shm")
        .unwrap();
    let root = temp.path().join("workspace");
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(control.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let code = temp.path().join("code");
    let home = temp.path().join("state");
    let profile = temp.path().join("profile.yaml");
    for directory in [&root, &code, &home] {
        fs::create_dir(directory).unwrap();
    }
    fs::write(&profile, "# synthetic non-secret profile input\n").unwrap();
    if provider_chat {
        fs::write(
            &profile,
            include_str!("../../../config/retained/hermes-profile.json"),
        )
        .unwrap();
    }
    fs::set_permissions(&home, fs::Permissions::from_mode(0o700)).unwrap();
    let mut raw = serde_json::json!({"version":1,"entrypoint":{"interpreter":"/usr/bin/python3","ordered_import_roots":["/opt/installed-hermes"]},
        "grants":[
            {"id":"usr","source":"/usr","destination":"/usr","kind":"directory","access":"read_only","role":"runtime"},
            {"id":"code","source":code,"destination":"/opt/installed-hermes","kind":"directory","access":"read_only","role":"runtime"},
            {"id":"state","source":home,"destination":"/hermes/profiles/retained","kind":"directory","access":"read_write","role":"session_state"},
            {"id":"profile","source":profile,"destination":"/hermes/profiles/retained/config.yaml","kind":"file","access":"read_only","role":"profile_input"}],
        "fixed_environment":{"HOME":"/hermes/profiles/retained","HERMES_HOME":"/hermes/profiles/retained","PATH":"/usr/bin:/bin"}});
    if installed {
        let user_home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
        let source = user_home.join(".hermes/hermes-agent");
        let site = user_home.join(".local/lib/python3.12/site-packages");
        assert!(source.join("hermes_cli/main.py").is_file());
        assert!(site.is_dir());
        assert!(
            !source.join(".env").exists(),
            "probe must not load ambient source credentials"
        );
        raw["entrypoint"]["ordered_import_roots"] = serde_json::json!([
            source,
            site,
            "/usr/lib/python3.12/site-packages",
            "/usr/lib64/python3.12/site-packages"
        ]);
        for (id, path) in [("installed_code", source), ("user_site", site)] {
            raw["grants"].as_array_mut().unwrap().push(serde_json::json!({"id":id,"source":path,"destination":path,"kind":"directory","access":"read_only","role":"runtime"}));
        }
        if !provider_chat {
            fs::write(&profile, "terminal:\n  backend: local\nmemory:\n  memory_enabled: false\n  user_profile_enabled: false\n").unwrap();
        }
    }
    let policy = ValidatedRuntimePolicy::parse(&serde_json::to_vec(&raw).unwrap()).unwrap();
    let sealed = transport::seal(&policy).unwrap();
    let fd = sealed.as_raw_fd();
    let metadata = fs::metadata(&root).unwrap();
    let identity = serde_json::to_string(&(
        2u32,
        &root,
        &root,
        Some((metadata.dev(), metadata.ino())),
        format!(
            "{:x}",
            Sha256::digest(fs::read("/proc/thread-self/mountinfo").unwrap())
        ),
    ))
    .unwrap();
    let state = control.path().join("worker");
    let mut command = Command::new(env!("CARGO_BIN_EXE_arda-snapshot-worker"));
    command
        .env_clear()
        .arg(&state)
        .arg(&root)
        .arg(&identity)
        .arg(fd.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    unsafe {
        command.pre_exec(move || {
            if libc::fcntl(fd, libc::F_SETFD, 0) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    if keeper {
        let durable = temp.path().join("durable");
        fs::create_dir(&durable).unwrap();
        fs::set_permissions(&durable, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(Command::new(env!("CARGO_BIN_EXE_arda-snapshot-keeper"))
            .arg("--initialize")
            .arg(&durable)
            .status()
            .unwrap()
            .success());
        let policy_file = temp.path().join("policy.json");
        fs::write(&policy_file, serde_json::to_vec(policy.policy()).unwrap()).unwrap();
        fs::set_permissions(&policy_file, fs::Permissions::from_mode(0o600)).unwrap();
        command = Command::new(env!("CARGO_BIN_EXE_arda-snapshot-keeper"));
        let wrapper = temp.path().join("replace-before-worker");
        if attack {
            fs::write(&wrapper, format!("#!/usr/bin/python3\nimport os,sys\nopen({:?},'w').write(str(os.getpid()))\nos.rename({:?},{:?})\nos.mkdir({:?})\nos.execv({:?}, [{:?}]+sys.argv[1:])\n",
                temp.path().join("worker.pid"), code, temp.path().join("admitted-code"), code,
                env!("CARGO_BIN_EXE_arda-snapshot-worker"), env!("CARGO_BIN_EXE_arda-snapshot-worker"))).unwrap();
            fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o700)).unwrap();
        }
        command
            .env_clear()
            .arg(durable)
            .arg(control.path())
            .arg(if attack {
                wrapper.as_os_str()
            } else {
                std::ffi::OsStr::new(env!("CARGO_BIN_EXE_arda-snapshot-worker"))
            })
            .arg("--runtime-policy")
            .arg(policy_file);
    }
    let mut worker = Worker(command.spawn().unwrap());
    let mut socket = if keeper {
        control.path().join("keeper.sock")
    } else {
        state.join("control.sock")
    };
    let deadline = Instant::now() + Duration::from_secs(5);
    while UnixStream::connect(&socket).is_err() {
        assert!(
            worker.0.try_wait().unwrap().is_none(),
            "worker exited before ready"
        );
        assert!(Instant::now() < deadline, "capture readiness timeout");
        std::thread::sleep(Duration::from_millis(10));
    }
    let authority = if keeper {
        use arda_engine::objectives::keeper_client::KeeperRequest;
        let prepared = request(
            &socket,
            serde_json::to_value(KeeperRequest::Prepare {
                run: "bundle-test".into(),
                workspace: root.clone(),
                identity: identity.clone(),
            })
            .unwrap(),
        );
        if attack {
            assert_eq!(prepared["ok"], false, "{prepared}");
            assert!(prepared["snapshot"].is_null());
            let pid = fs::read_to_string(temp.path().join("worker.pid")).unwrap();
            assert!(
                !std::path::Path::new(&format!("/proc/{pid}")).exists(),
                "failed worker was not reaped"
            );
            return;
        }
        assert_eq!(prepared["ok"], true, "keeper refused configured admission");
        let authority: arda_engine::objectives::RetainedSnapshot =
            serde_json::from_value(prepared["snapshot"].clone()).unwrap();
        socket = authority.endpoint.clone().into();
        Some(authority)
    } else {
        None
    };
    let inspected = request(&socket, serde_json::json!({"op":"inspect"}));
    let manifest: Manifest = serde_json::from_value(inspected["manifest"].clone()).unwrap();
    let effective = if keeper {
        let mut effective = policy.policy().clone();
        effective
            .grants
            .iter_mut()
            .find(|g| g.id == "state")
            .unwrap()
            .source = home.join(format!("run-{:x}", Sha256::digest(b"bundle-test")));
        ValidatedRuntimePolicy::parse(&serde_json::to_vec(&effective).unwrap()).unwrap()
    } else {
        policy.clone()
    };
    manifest.validate_runtime(Some(&effective)).unwrap();
    assert_eq!(manifest.admission_digest.is_some(), keeper);
    if keeper {
        use arda_engine::objectives::keeper_client::KeeperRequest;
        let sibling = home.join(format!("run-{:x}", Sha256::digest(b"bundle-test")));
        let sibling_meta = fs::metadata(&sibling).unwrap();
        let sibling_identity = serde_json::to_string(&(
            1,
            &sibling,
            &sibling,
            Some((sibling_meta.dev(), sibling_meta.ino())),
            format!(
                "{:x}",
                Sha256::digest(fs::read("/proc/self/mountinfo").unwrap())
            ),
        ))
        .unwrap();
        let replay = request(
            &control.path().join("keeper.sock"),
            serde_json::to_value(KeeperRequest::Prepare {
                run: "sibling-state-snoop".into(),
                workspace: home.join(format!("run-{:x}", Sha256::digest(b"bundle-test"))),
                identity: sibling_identity,
            })
            .unwrap(),
        );
        assert_eq!(replay["ok"], false);

        let replay = request(
            &control.path().join("keeper.sock"),
            serde_json::to_value(KeeperRequest::Prepare {
                run: "bundle-test".into(),
                workspace: root.clone(),
                identity: identity.clone(),
            })
            .unwrap(),
        );
        assert_eq!(
            replay["snapshot"],
            serde_json::to_value(authority.as_ref().unwrap()).unwrap()
        );
        let second = request(
            &control.path().join("keeper.sock"),
            serde_json::to_value(KeeperRequest::Prepare {
                run: "bundle-other".into(),
                workspace: root.clone(),
                identity: identity.clone(),
            })
            .unwrap(),
        );
        assert_eq!(second["ok"], true);
        let allocation_db =
            rusqlite::Connection::open(temp.path().join("durable/owner.sqlite3")).unwrap();
        let collision = allocation_db.execute("UPDATE runtime_allocations SET (device,inode)=(SELECT device,inode FROM runtime_allocations WHERE run='bundle-test') WHERE run='bundle-other'", []).unwrap_err();
        assert_eq!(
            collision.sqlite_error_code(),
            Some(rusqlite::ErrorCode::ConstraintViolation)
        );
        let second_authority: arda_engine::objectives::RetainedSnapshot =
            serde_json::from_value(second["snapshot"].clone()).unwrap();
        let second_inspection = request(
            std::path::Path::new(&second_authority.endpoint),
            serde_json::json!({"op":"inspect"}),
        );
        let second_manifest: Manifest =
            serde_json::from_value(second_inspection["manifest"].clone()).unwrap();
        let first_state = manifest
            .runtime_bundle
            .as_ref()
            .unwrap()
            .grants
            .iter()
            .find(|g| g.id == "state")
            .unwrap();
        let second_state = second_manifest
            .runtime_bundle
            .as_ref()
            .unwrap()
            .grants
            .iter()
            .find(|g| g.id == "state")
            .unwrap();
        assert_ne!(
            (first_state.device, first_state.inode),
            (second_state.device, second_state.inode)
        );
        let released = request(
            &control.path().join("keeper.sock"),
            serde_json::to_value(KeeperRequest::Release {
                run: "bundle-other".into(),
                snapshot: second_authority,
            })
            .unwrap(),
        );
        assert_eq!(released["ok"], true);
    }
    assert_eq!(
        manifest.runtime_bundle.as_ref().unwrap().grants.len(),
        if installed { 6 } else { 4 }
    );
    fs::rename(&code, temp.path().join("old-code")).unwrap();
    if keeper {
        use arda_engine::objectives::{
            keeper_client::KeeperRequest,
            runtime_operation::RuntimeOperation,
            snapshot_protocol::{Lease, Request},
        };
        let snapshot = authority.clone().unwrap();
        let lease = Lease {
            run_id: "bundle-test".into(),
            generation: 1,
            owner: "test-owner".into(),
            expires_ms: i64::try_from(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis(),
            )
            .unwrap()
                + if provider_chat { 180_000 } else { 60_000 },
        };
        let runtime_request = |capability: &str, lease: &Lease| {
            serde_json::to_value(Request::Runtime {
                capability: capability.into(),
                lease: lease.clone(),
                operation: RuntimeOperation::VerifyArtifacts {
                    paths: vec!["artifact".into()],
                },
                timeout_ms: 10_000,
                max_output_bytes: 65_536,
            })
            .unwrap()
        };
        // A valid capability without a committed lease cannot dispatch.
        assert_eq!(
            request(&socket, runtime_request(&snapshot.capability, &lease))["ok"],
            false
        );
        assert_eq!(
            request(
                &control.path().join("keeper.sock"),
                serde_json::to_value(KeeperRequest::Commit {
                    snapshot: snapshot.clone(),
                    lease: lease.clone()
                })
                .unwrap()
            )["ok"],
            true
        );
        fs::write(root.join("artifact"), b"retained").unwrap();
        assert_eq!(
            request(&socket, runtime_request("wrong-capability", &lease))["ok"],
            false
        );
        let mut stale = lease.clone();
        stale.generation += 1;
        assert_eq!(
            request(&socket, runtime_request(&snapshot.capability, &stale))["ok"],
            false
        );
        stale = lease.clone();
        stale.owner = "wrong-owner".into();
        assert_eq!(
            request(&socket, runtime_request(&snapshot.capability, &stale))["ok"],
            false
        );
        for operation in [
            RuntimeOperation::Chat {
                query: "test".into(),
                max_turns: 1,
                toolsets: vec!["file".into()],
            },
            RuntimeOperation::Export {
                session_id: "test-session".into(),
            },
        ]
        .into_iter()
        .filter(|_| !provider_chat)
        {
            let denied = request(
                &socket,
                serde_json::to_value(Request::Runtime {
                    capability: snapshot.capability.clone(),
                    lease: lease.clone(),
                    operation,
                    timeout_ms: 10_000,
                    max_output_bytes: 65_536,
                })
                .unwrap(),
            );
            assert!(denied["ok"] == false || denied["code"].as_i64().is_some_and(|code| code != 0));
        }
        assert_eq!(
            request(
                &socket,
                serde_json::to_value(Request::Execute {
                    capability: snapshot.capability.clone(),
                    lease: lease.clone(),
                    argv: vec!["/usr/bin/true".into()],
                    environment: std::collections::BTreeMap::new(),
                    timeout_ms: 10_000,
                    max_output_bytes: 65_536,
                })
                .unwrap()
            )["ok"],
            false
        );
        fs::rename(&root, temp.path().join("old-workspace")).unwrap();
        fs::create_dir(&root).unwrap();
        fs::write(root.join("artifact"), b"replacement").unwrap();
        let verified = request(
            &socket,
            serde_json::to_value(Request::Runtime {
                capability: snapshot.capability.clone(),
                lease: lease.clone(),
                operation: RuntimeOperation::VerifyArtifacts {
                    paths: vec!["artifact".into()],
                },
                timeout_ms: 10_000,
                max_output_bytes: 65_536,
            })
            .unwrap(),
        );
        assert_eq!(verified["ok"], true);
        assert_eq!(verified["code"], 0, "{}", verified["stderr"]);
        let stamps: serde_json::Value =
            serde_json::from_str(verified["stdout"].as_str().unwrap()).unwrap();
        assert_eq!(
            stamps[0]["sha256"],
            format!("{:x}", Sha256::digest(b"retained"))
        );
        if installed {
            let probed = request(
                &socket,
                serde_json::to_value(Request::Runtime {
                    capability: snapshot.capability.clone(),
                    lease: lease.clone(),
                    operation: RuntimeOperation::Probe {},
                    timeout_ms: 30_000,
                    max_output_bytes: 262_144,
                })
                .unwrap(),
            );
            assert_eq!(probed["ok"], true);
            assert_eq!(probed["code"], 0, "{}", probed["stderr"]);
            assert!(probed["stdout"]
                .as_str()
                .unwrap()
                .to_ascii_lowercase()
                .contains("hermes"));
            assert!(probed["stdout"].as_str().unwrap().contains("chat"));
            if provider_chat {
                let chatted = request(&socket, serde_json::to_value(Request::Runtime {
                    capability: snapshot.capability.clone(), lease: lease.clone(),
                    operation: RuntimeOperation::Chat {
                        query: "Use the terminal tool to run: printf 'retained-chat\\n' > retained-chat.txt . Run it in the current directory, without changing directory. Then reply DONE. Do not inspect unrelated files or use the network.".into(),
                        max_turns: 4, toolsets: vec!["terminal".into()],
                    }, timeout_ms: 120_000, max_output_bytes: 1_048_576,
                }).unwrap());
                assert_eq!(chatted["ok"], true, "{chatted}");
                assert_eq!(chatted["code"], 0, "{}", chatted["stderr"]);
                let stderr = chatted["stderr"].as_str().unwrap();
                let routes: Vec<_> = stderr
                    .lines()
                    .filter(|line| line.starts_with("ARDA_MANWE_ROUTE "))
                    .collect();
                assert!(!routes.is_empty(), "no observed Manwe route: {stderr}");
                for route in &routes {
                    println!("{route}");
                }
                assert_eq!(
                    fs::read(temp.path().join("old-workspace/retained-chat.txt")).unwrap(),
                    b"retained-chat\n"
                );
                assert!(!root.join("retained-chat.txt").exists());
                let session_id = stderr
                    .lines()
                    .find_map(|line| line.trim().strip_prefix("session_id: "))
                    .expect("real chat session ID")
                    .trim()
                    .to_owned();
                let exported = request(
                    &socket,
                    serde_json::to_value(Request::Runtime {
                        capability: snapshot.capability.clone(),
                        lease: lease.clone(),
                        operation: RuntimeOperation::Export {
                            session_id: session_id.clone(),
                        },
                        timeout_ms: 30_000,
                        max_output_bytes: 1_048_576,
                    })
                    .unwrap(),
                );
                assert_eq!(exported["ok"], true);
                assert_eq!(exported["code"], 0, "{}", exported["stderr"]);
                let transcript: serde_json::Value =
                    serde_json::from_str(exported["stdout"].as_str().unwrap()).unwrap();
                assert_eq!(
                    transcript
                        .get("id")
                        .or_else(|| transcript.get("session_id"))
                        .unwrap(),
                    &session_id
                );
                assert!(transcript["messages"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|message| message["role"] == "tool"));
                let prefix_export = request(
                    &socket,
                    serde_json::to_value(Request::Runtime {
                        capability: snapshot.capability.clone(),
                        lease: lease.clone(),
                        operation: RuntimeOperation::Export {
                            session_id: session_id.chars().take(8).collect(),
                        },
                        timeout_ms: 30_000,
                        max_output_bytes: 1_048_576,
                    })
                    .unwrap(),
                );
                assert!(
                    prefix_export["ok"] == false
                        || prefix_export["code"].as_i64().is_some_and(|code| code != 0)
                );
                println!(
                    "Real installed Chat -> retained workspace write -> exact-ID Export verified"
                );
                // Separate synthetic redaction fixture, added only AFTER the genuine
                // unmodified Chat/export assertions above. These are not provider receipts.
                let state_root = &effective
                    .policy()
                    .grants
                    .iter()
                    .find(|grant| {
                        grant.role
                            == arda_engine::objectives::runtime_policy::GrantRole::SessionState
                    })
                    .unwrap()
                    .source;
                let db = rusqlite::Connection::open(state_root.join("state.db")).unwrap();
                let metadata_canary =
                    format!("sk-{}-metadata", "synthetic_not_a_credential".repeat(2));
                let content_canary =
                    format!("sk-{}-content", "synthetic_not_a_credential".repeat(2));
                let args_canary =
                    format!("sk-{}-arguments", "synthetic_not_a_credential".repeat(2));
                let result_canary = format!("sk-{}-result", "synthetic_not_a_credential".repeat(2));
                assert_eq!(
                    db.execute(
                        "UPDATE sessions SET title=?1 WHERE id=?2",
                        rusqlite::params![
                            format!("redaction-fixture {metadata_canary}"),
                            session_id
                        ]
                    )
                    .unwrap(),
                    1
                );
                let calls=serde_json::json!([{"id":"synthetic-redaction-fixture","type":"function","function":{"name":"terminal","arguments":serde_json::json!({"fixture":args_canary}).to_string()}}]).to_string();
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs_f64();
                db.execute("INSERT INTO messages(session_id,role,content,timestamp,tool_calls) VALUES(?1,'assistant',?2,?3,?4)",rusqlite::params![session_id,format!("synthetic redaction fixture {content_canary}"),timestamp,calls]).unwrap();
                db.execute("INSERT INTO messages(session_id,role,content,timestamp,tool_call_id) VALUES(?1,'tool',?2,?3,'synthetic-redaction-fixture')",rusqlite::params![session_id,serde_json::json!({"fixture":{"result":result_canary}}).to_string(),timestamp]).unwrap();
                drop(db);
                let redacted = request(
                    &socket,
                    serde_json::to_value(Request::Runtime {
                        capability: snapshot.capability.clone(),
                        lease: lease.clone(),
                        operation: RuntimeOperation::Export {
                            session_id: session_id.clone(),
                        },
                        timeout_ms: 30_000,
                        max_output_bytes: 1_048_576,
                    })
                    .unwrap(),
                );
                assert_eq!(redacted["code"], 0, "{}", redacted["stderr"]);
                let text = redacted["stdout"].as_str().unwrap();
                for (location, canary) in [
                    ("metadata", &metadata_canary),
                    ("content", &content_canary),
                    ("arguments", &args_canary),
                    ("result", &result_canary),
                ] {
                    assert!(
                        !text.contains(canary),
                        "synthetic secret survived export: {location}"
                    );
                }
                let cleaned: serde_json::Value = serde_json::from_str(text).unwrap();
                assert!(cleaned["title"]
                    .as_str()
                    .unwrap()
                    .starts_with("redaction-fixture "));
                assert!(
                    cleaned["messages"].as_array().unwrap().len()
                        >= transcript["messages"].as_array().unwrap().len() + 2
                );
                println!("Separate synthetic content/tool-arguments/tool-result/metadata redaction verified");
            }
        }
    }
    if !keeper {
        use arda_engine::objectives::{
            runtime_operation::RuntimeOperation,
            snapshot_protocol::{Lease, Request},
        };
        let capability = manifest.capability.clone();
        let lease = Lease {
            run_id: "direct-test".into(),
            generation: 1,
            owner: "test-owner".into(),
            expires_ms: i64::try_from(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis(),
            )
            .unwrap()
                + 60_000,
        };
        assert_eq!(
            request(
                &socket,
                serde_json::to_value(Request::Commit {
                    capability: capability.clone(),
                    manifest_digest: inspected["manifest_digest"].as_str().unwrap().into(),
                    lease: lease.clone()
                })
                .unwrap()
            )["ok"],
            true
        );
        assert_eq!(
            request(
                &socket,
                serde_json::to_value(Request::Runtime {
                    capability,
                    lease,
                    operation: RuntimeOperation::Probe {},
                    timeout_ms: 10_000,
                    max_output_bytes: 65_536
                })
                .unwrap()
            )["ok"],
            false
        );
    }
    fs::create_dir(&code).unwrap();
    let repeated = request(&socket, serde_json::json!({"op":"inspect"}));
    assert_eq!(inspected["manifest_digest"], repeated["manifest_digest"]);
    if let Some(snapshot) = authority {
        use arda_engine::objectives::keeper_client::KeeperRequest;
        let released = request(
            &control.path().join("keeper.sock"),
            serde_json::to_value(KeeperRequest::Release {
                snapshot,
                run: "bundle-test".into(),
            })
            .unwrap(),
        );
        assert_eq!(released["ok"], true);
        assert!(!socket.parent().unwrap().exists());
        return;
    }
    let released = request(
        &socket,
        serde_json::json!({"op":"release","capability":manifest.capability}),
    );
    assert_eq!(released["ok"], true);
    assert!(worker.0.wait().unwrap().success());
    assert!(!state.exists());
}
