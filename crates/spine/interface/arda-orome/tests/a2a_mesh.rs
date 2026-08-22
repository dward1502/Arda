use arda_orome::a2a_mesh::{
    A2aMeshError, CapabilityObservation, MeshRegistry, NodeEnrollment, NodeIdentity,
    ResourcePressureObservation, WorkEnvelope,
};
use chrono::{Duration, TimeZone, Utc};
use serde_json::json;
use tempfile::tempdir;

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 22, 18, 0, 0).unwrap()
}

fn enrollment(node_id: &str, domain: &str, bearer_env: &str) -> NodeEnrollment {
    NodeEnrollment {
        schema_version: "arda.node-enrollment.v1".into(),
        identity: NodeIdentity {
            schema_version: "arda.node-identity.v1".into(),
            node_id: node_id.into(),
            agent_id: format!("agent:{node_id}"),
            trust_domain: domain.into(),
            enrollment_epoch: 1,
        },
        agent_card_url: format!("http://127.0.0.1:9999/{node_id}/.well-known/agent-card.json"),
        bearer_env: bearer_env.into(),
        allowed_capabilities: vec!["arda.echo.typed.v1".into()],
        allowed_data_domains: vec!["system".into()],
        issued_at: now() - Duration::minutes(1),
        expires_at: now() + Duration::hours(1),
        revoked_at: None,
    }
}

fn observation(node_id: &str) -> CapabilityObservation {
    CapabilityObservation {
        schema_version: "arda.node-capability-observation.v1".into(),
        observation_id: format!("obs:{node_id}:1"),
        node_id: node_id.into(),
        capabilities: vec!["arda.echo.typed.v1".into()],
        pressure: ResourcePressureObservation {
            cpu: 0.1,
            memory: 0.2,
            gpu: None,
            queue_depth: 0,
        },
        observed_at: now(),
        expires_at: now() + Duration::minutes(5),
    }
}

fn envelope() -> WorkEnvelope {
    WorkEnvelope {
        schema_version: "arda.work-envelope.v1".into(),
        envelope_id: "env:typed-echo:1".into(),
        objective_id: "objective:mesh-proof".into(),
        run_id: "run:mesh-proof:1".into(),
        worker_id: "worker:mesh-proof:remote".into(),
        capability: "arda.echo.typed.v1".into(),
        data_domain: "system".into(),
        payload: json!({"kind": "typed_echo", "text": "hello independent node"}),
        issued_at: now(),
        expires_at: now() + Duration::minutes(2),
        nonce: "nonce:mesh-proof:1".into(),
        route_trace: vec!["node-root".into()],
        max_hops: 3,
    }
}

#[test]
fn registry_persists_identity_enrollment_observation_and_revocation() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("a2a-mesh.jsonl");
    let mut registry = MeshRegistry::open(&path).unwrap();

    registry
        .enroll(
            enrollment("node-peer", "home", "ARDA_TEST_PEER_TOKEN"),
            now(),
        )
        .unwrap();
    registry
        .publish_observation(observation("node-peer"), now())
        .unwrap();

    let active = registry.route(&envelope(), now()).unwrap();
    assert_eq!(active.enrollment.identity.node_id, "node-peer");
    assert_eq!(active.observation.observation_id, "obs:node-peer:1");

    drop(registry);
    let mut reloaded = MeshRegistry::open(&path).unwrap();
    assert_eq!(reloaded.projection(now()).peers.len(), 1);
    reloaded
        .revoke("node-peer", "operator_revoked", now())
        .unwrap();
    assert_eq!(
        reloaded.route(&envelope(), now()).unwrap_err(),
        A2aMeshError::NoEligiblePeer
    );

    let rows = std::fs::read_to_string(path).unwrap();
    assert!(rows.contains("node_enrolled"));
    assert!(rows.contains("capability_observed"));
    assert!(rows.contains("node_revoked"));
    assert!(!rows.contains("test-secret"));
}

#[test]
fn stable_node_identity_cannot_be_rebound_by_reenrollment() {
    let dir = tempdir().unwrap();
    let mut registry = MeshRegistry::open(dir.path().join("mesh.jsonl")).unwrap();
    registry
        .enroll(
            enrollment("node-peer", "home", "ARDA_TEST_PEER_TOKEN"),
            now(),
        )
        .unwrap();

    let mut forged = enrollment("node-peer", "home", "ARDA_TEST_PEER_TOKEN");
    forged.identity.agent_id = "agent:attacker".into();
    forged.identity.enrollment_epoch = 2;
    assert_eq!(
        registry.enroll(forged, now()).unwrap_err(),
        A2aMeshError::InvalidContract
    );
}

#[test]
fn routing_fails_closed_for_expiry_domain_capability_and_loops() {
    let dir = tempdir().unwrap();
    let mut registry = MeshRegistry::open(dir.path().join("mesh.jsonl")).unwrap();
    registry
        .enroll(
            enrollment("node-peer", "home", "ARDA_TEST_PEER_TOKEN"),
            now(),
        )
        .unwrap();
    registry
        .publish_observation(observation("node-peer"), now())
        .unwrap();

    let mut expired = envelope();
    expired.issued_at = now() - Duration::minutes(2);
    expired.expires_at = now() - Duration::seconds(1);
    assert_eq!(
        registry.route(&expired, now()).unwrap_err(),
        A2aMeshError::ExpiredEnvelope
    );

    let mut wrong_domain = envelope();
    wrong_domain.data_domain = "personal".into();
    assert_eq!(
        registry.route(&wrong_domain, now()).unwrap_err(),
        A2aMeshError::NoEligiblePeer
    );

    let mut wrong_capability = envelope();
    wrong_capability.capability = "arda.shell.exec.v1".into();
    assert_eq!(
        registry.route(&wrong_capability, now()).unwrap_err(),
        A2aMeshError::NoEligiblePeer
    );

    let mut looped = envelope();
    looped.route_trace.push("node-peer".into());
    assert_eq!(
        registry.route(&looped, now()).unwrap_err(),
        A2aMeshError::NoEligiblePeer
    );

    let mut exhausted = envelope();
    exhausted.route_trace = vec!["a".into(), "b".into(), "c".into()];
    assert_eq!(
        registry.route(&exhausted, now()).unwrap_err(),
        A2aMeshError::HopLimitExceeded
    );
}

#[test]
fn replay_is_rejected_and_offline_state_is_honest_after_observation_expiry() {
    let dir = tempdir().unwrap();
    let mut registry = MeshRegistry::open(dir.path().join("mesh.jsonl")).unwrap();
    registry
        .enroll(
            enrollment("node-peer", "home", "ARDA_TEST_PEER_TOKEN"),
            now(),
        )
        .unwrap();
    registry
        .publish_observation(observation("node-peer"), now())
        .unwrap();

    let work = envelope();
    registry.claim_dispatch(&work, now()).unwrap();
    assert_eq!(
        registry.claim_dispatch(&work, now()).unwrap_err(),
        A2aMeshError::ReplayDetected
    );

    let later = now() + Duration::minutes(6);
    let projection = registry.projection(later);
    assert_eq!(projection.peers[0].availability, "offline");
    let mut still_valid = envelope();
    still_valid.expires_at = now() + Duration::minutes(10);
    assert_eq!(
        registry.route(&still_valid, later).unwrap_err(),
        A2aMeshError::NoEligiblePeer
    );
}

#[test]
fn stale_registry_view_rejects_replay_claimed_by_another_writer() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("mesh.jsonl");
    let mut first = MeshRegistry::open(&path).unwrap();
    let mut stale = MeshRegistry::open(&path).unwrap();
    let work = envelope();

    first.claim_dispatch(&work, now()).unwrap();
    assert_eq!(
        stale.claim_dispatch(&work, now()).unwrap_err(),
        A2aMeshError::ReplayDetected
    );
}

#[test]
fn stale_registry_view_cannot_rebind_identity() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("mesh.jsonl");
    let mut first = MeshRegistry::open(&path).unwrap();
    let mut stale = MeshRegistry::open(&path).unwrap();
    first
        .enroll(
            enrollment("node-peer", "home", "ARDA_TEST_PEER_TOKEN"),
            now(),
        )
        .unwrap();
    let mut forged = enrollment("node-peer", "home", "ARDA_TEST_PEER_TOKEN");
    forged.identity.agent_id = "agent:attacker".into();
    forged.identity.enrollment_epoch = 2;

    assert_eq!(
        stale.enroll(forged, now()).unwrap_err(),
        A2aMeshError::InvalidContract
    );
}

#[test]
fn standard_a2a_mapping_carries_typed_envelope_and_correlation_ids() {
    let work = envelope();
    let request = work.to_a2a_send_message("node-peer").unwrap();

    assert_eq!(request["jsonrpc"], "2.0");
    assert_eq!(request["method"], "SendMessage");
    assert_eq!(request["id"], work.envelope_id);
    assert_eq!(request["params"]["message"]["role"], "ROLE_USER");
    assert_eq!(request["params"]["message"]["contextId"], work.run_id);
    assert_eq!(
        request["params"]["message"]["parts"][0]["mediaType"],
        "application/vnd.arda.work-envelope.v1+json"
    );
    assert_eq!(
        request["params"]["message"]["parts"][0]["data"]["objective_id"],
        work.objective_id
    );
}
