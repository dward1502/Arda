//! Synthetic authority fixtures; not installed keeper/mount acceptance.
use super::*;
use crate::{inference_broker::Broker, snapshot_runtime::CapturedRuntime};
use arda_engine::objectives::{
    runtime_operation::RuntimeOperation, snapshot_protocol::CapturedRuntimeGrant,
};
use serde_json::json;
use std::time::{Duration, Instant};

fn fixture(port: Option<u16>, role: &str) -> (tempfile::TempDir, CapturedRuntime) {
    let root = tempfile::tempdir().unwrap();
    let profile = root.path().join("profile");
    let manifest = root.path().join("manifest");
    let sessions = root.path().join("sessions");
    std::fs::create_dir(&sessions).unwrap();
    let endpoint = format!("http://127.0.0.1:{}", port.unwrap_or(7171));
    std::fs::write(&profile, serde_json::to_vec(&json!({"model":{"default":"fixture-model"},"custom_providers":[{"base_url":format!("{endpoint}/v1"),"extra_body":{"routing":{"local_only":true,"inference_origin":"local","origin_preference":"local","force_provider_id":"fixture-provider","force_model_id":"fixture-model","allow_forced_provider_fallback":false,"tool_use_required":true}}}]})).unwrap()).unwrap();
    std::fs::write(&manifest, format!("[[service]]\nname='manwe'\nstart={{args=['--bind=127.0.0.1','--port={}']}}\nhealth={{url='{endpoint}/healthz'}}\n", port.unwrap_or(7171))).unwrap();
    let mut value = json!({"version":1,"entrypoint":{"interpreter":"/usr/bin/python3","ordered_import_roots":["/usr/lib"]},"grants":[
        {"id":"runtime","source":"/usr","destination":"/usr","kind":"directory","access":"read_only","role":"runtime"},
        {"id":"profile","source":profile,"destination":"/hermes/profiles/retained/config.yaml","kind":"file","access":"read_only","role":"profile_input"},
        {"id":"state","source":sessions,"destination":"/hermes/profiles/retained","kind":"directory","access":"read_write","role":"session_state"}
    ],"fixed_environment":{"HOME":"/hermes/profiles/retained","HERMES_HOME":"/hermes/profiles/retained","PATH":"/usr/bin:/bin"}});
    if port.is_some() {
        value["grants"].as_array_mut().unwrap().push(json!({"id":"manifest","source":manifest,"destination":DESTINATION,"kind":"file","access":"read_only","role":role}));
    }
    let policy = ValidatedRuntimePolicy::parse(&serde_json::to_vec(&value).unwrap()).unwrap();
    let grants: Vec<_> = policy
        .policy()
        .grants
        .iter()
        .map(|g| {
            let descriptor = File::open(&g.source).unwrap();
            let metadata = descriptor.metadata().unwrap();
            CapturedGrant {
                descriptor,
                identity: CapturedRuntimeGrant {
                    id: g.id.clone(),
                    destination: g.destination.clone(),
                    kind: g.kind.clone(),
                    access: g.access.clone(),
                    device: metadata.dev(),
                    inode: metadata.ino(),
                    topology_digest: "0".repeat(64),
                },
            }
        })
        .collect();
    (
        root,
        CapturedRuntime {
            policy,
            grants,
            endpoint: None,
        },
    )
}

#[test]
fn missing_authority_gates_chat_and_export_not_probe_or_artifacts() {
    let (_root, runtime) = fixture(None, "network_input");
    assert!(capture(&runtime.policy, &runtime.grants).unwrap().is_none());
    for writable in [false, true] {
        assert!(runtime
            .operation_argv(&RuntimeOperation::Chat {
                query: "fixture".into(),
                max_turns: 1,
                toolsets: vec!["file".into()],
                workspace_writable: writable
            })
            .is_err());
    }
    assert!(runtime.operation_argv(&RuntimeOperation::Probe {}).is_ok());
    assert!(runtime
        .operation_argv(&RuntimeOperation::VerifyArtifacts {
            paths: vec!["README.md".into()]
        })
        .is_ok());
    let export = RuntimeOperation::Export {
        session_id: "fixture".into(),
    };
    assert!(runtime.operation_argv(&export).is_err());
}

#[test]
fn wrong_role_and_captured_identity_refuse() {
    let (_root, runtime) = fixture(Some(17171), "runtime");
    assert!(capture(&runtime.policy, &runtime.grants).is_err());
    let (_root, mut runtime) = fixture(Some(17171), "network_input");
    assert_eq!(
        capture(&runtime.policy, &runtime.grants)
            .unwrap()
            .as_deref(),
        Some("http://127.0.0.1:17171")
    );
    runtime
        .grants
        .iter_mut()
        .find(|g| g.identity.id == "manifest")
        .unwrap()
        .identity
        .inode += 1;
    assert!(capture(&runtime.policy, &runtime.grants).is_err());
}

#[tokio::test]
async fn production_broker_uses_captured_nondefault_endpoint_after_manifest_mutation() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    tokio::time::timeout(Duration::from_secs(5), async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let (root, mut runtime) = fixture(Some(port), "network_input");
        runtime.endpoint = capture(&runtime.policy, &runtime.grants).unwrap();
        let argv = runtime.operation_argv(&RuntimeOperation::Chat { query: "fixture".into(), max_turns: 1, toolsets: vec!["file".into()], workspace_writable: false }).unwrap();
        assert_eq!(argv[8], format!("http://127.0.0.1:{port}"));
        std::fs::write(root.path().join("manifest"), "invalid after capture").unwrap();
        let (mut broker, payload) = Broker::start(&runtime, Instant::now() + Duration::from_secs(5)).unwrap();
        payload.set_nonblocking(true).unwrap();
        let mut payload = tokio::net::UnixStream::from_std(payload).unwrap();
        let body = serde_json::to_vec(&json!({"model":"fixture-model","messages":[]})).unwrap();
        payload.write_u32(body.len() as u32).await.unwrap();
        payload.write_all(&body).await.unwrap();
        let (mut upstream, _) = listener.accept().await.unwrap();
        let mut headers = Vec::new();
        while !headers.ends_with(b"\r\n\r\n") { headers.push(upstream.read_u8().await.unwrap()); }
        let headers = String::from_utf8(headers).unwrap();
        assert!(headers.starts_with("POST /v1/chat/completions HTTP/1.1"));
        let length: usize = headers.lines().find_map(|line| line.to_ascii_lowercase().strip_prefix("content-length: ").map(|n| n.parse().unwrap())).unwrap();
        let mut received = vec![0; length];
        upstream.read_exact(&mut received).await.unwrap();
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&received).unwrap()["routing"]["force_provider_id"], "fixture-provider");
        upstream.write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\nx-manwe-route-id: synthetic\r\nx-manwe-provider-id: fixture-provider\r\nx-manwe-model-id: fixture-model\r\nx-manwe-route-class: tool_oriented\r\n\r\n{}").await.unwrap();
        let length = payload.read_u32().await.unwrap();
        let mut received = vec![0; length as usize];
        payload.read_exact(&mut received).await.unwrap();
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&received).unwrap()["status"], 200);
        assert_eq!(broker.evidence().routes.len(), 1);
        broker.stop();
        // Profile authority cannot retarget the already captured worker.
        let mut profile: serde_json::Value = serde_json::from_slice(&std::fs::read(root.path().join("profile")).unwrap()).unwrap();
        profile["custom_providers"][0]["base_url"] = json!("http://127.0.0.1:7171/v1");
        std::fs::write(root.path().join("profile"), serde_json::to_vec(&profile).unwrap()).unwrap();
        assert!(Broker::start(&runtime, Instant::now() + Duration::from_secs(1)).is_err());
    }).await.unwrap();
}
