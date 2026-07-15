use arda_warden::crypto::Crypto;
use arda_warden::foreign::{ForeignProtocol, ForeignState};
use arda_warden::monitor::evaluate_execution_harness;

#[test]
fn execution_harness_separates_read_only_from_networked_shell_risk() {
    let read_only = evaluate_execution_harness(
        &serde_json::json!({
            "intent": "inspect local guardhouse status only"
        }),
        None,
    );

    assert_eq!(read_only.risk_level, "guarded");
    assert!(!read_only.approval_required);
    assert!(!read_only.sandbox_required);
    assert_eq!(read_only.network_access, "none");

    let networked_shell = evaluate_execution_harness(
        &serde_json::json!({
            "command": "bash scripts/check.sh && curl https://example.com/status"
        }),
        Some("normal"),
    );

    assert_eq!(networked_shell.risk_level, "elevated");
    assert!(networked_shell.approval_required);
    assert!(networked_shell.sandbox_required);
    assert_eq!(networked_shell.network_access, "restricted");
    assert_eq!(
        networked_shell.verification_steps,
        vec![
            "capture_pre_state",
            "dry_run_or_simulate",
            "record_intent",
            "record_post_state",
            "human_approval",
        ]
    );
}

#[test]
fn foreign_protocol_tracks_registered_agent_and_resonance_updates() -> anyhow::Result<()> {
    let mut protocol = ForeignProtocol::new();
    protocol.register_new("guarded_guest".to_owned(), "ctr-warden-001".to_owned());

    let agent = protocol.get_agent("guarded_guest").ok_or_else(|| {
        anyhow::anyhow!("registered agent should be available for guardhouse review")
    })?;
    assert_eq!(agent.container_id, "ctr-warden-001");
    assert!(matches!(
        agent.state,
        ForeignState::Probation | ForeignState::Quarantined
    ));
    assert!(agent.resonance_history.is_empty());

    protocol.update_resonance("guarded_guest", 81.5);
    protocol.update_resonance("missing_guest", 10.0);

    let agent = protocol.get_agent("guarded_guest").ok_or_else(|| {
        anyhow::anyhow!("registered agent should remain available after resonance update")
    })?;
    assert_eq!(agent.resonance_history, vec![81.5]);

    Ok(())
}

#[test]
fn crypto_report_round_trip_preserves_guardhouse_payload() -> anyhow::Result<()> {
    let crypto = Crypto::new("d2FyZGVuLXB1YmxpYy1rZXk=")?;
    let report = serde_json::json!({
        "node": "edge-warden",
        "status": "guarded",
        "alerts": ["tailscale", "container-health"]
    });

    let encrypted = crypto.encrypt_report(&report)?;
    assert_ne!(encrypted, report.to_string());

    let decrypted = crypto.decrypt_report(&encrypted)?;
    assert_eq!(decrypted, report);

    Ok(())
}
