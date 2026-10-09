use super::*;
use crate::onboarding::environment::build_environment_profile;
use std::fs;

struct Fixture(std::path::PathBuf);
impl Fixture {
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture() -> (Fixture, EnvironmentProfile) {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = Fixture(
        std::env::temp_dir().join(format!("arda-readiness-{}-{nonce}", std::process::id())),
    );
    fs::create_dir(root.path()).unwrap();
    fs::create_dir_all(root.path().join("core/state")).unwrap();
    fs::write(root.path().join("AGENTS.md"), "instructions").unwrap();
    fs::write(root.path().join("ARDA_ROOT_PROTOCOL.md"), "protocol").unwrap();
    fs::write(
        root.path()
            .join("core/state/environment_profile.schema.json"),
        "{}",
    )
    .unwrap();
    let mut profile = build_environment_profile(Some(root.path()), None, None).unwrap();
    profile.missing_gates.clear();
    (root, profile)
}

#[test]
fn canonical_protocol_is_case_sensitive() {
    let (root, profile) = fixture();
    let status = || {
        build_readiness_projection(&profile, root.path())
            .checks
            .into_iter()
            .find(|c| c.check_id == "ARDA_ROOT_PROTOCOL.md")
            .unwrap()
    };
    assert_eq!(status().status, "pass");
    fs::rename(
        root.path().join("ARDA_ROOT_PROTOCOL.md"),
        root.path().join("arda_ROOT_PROTOCOL.md"),
    )
    .unwrap();
    assert_ne!(status().status, "pass");
    assert_eq!(status().evidence, vec!["missing: ARDA_ROOT_PROTOCOL.md"]);
}

#[test]
fn historical_audit_is_context_not_an_installation_prerequisite() {
    let (root, profile) = fixture();
    let projection = build_readiness_projection(&profile, root.path());
    assert!(!projection
        .checks
        .iter()
        .any(|c| c.check_id == "portability.receipt"));
    assert_eq!(projection.portability_status["status"], "missing");
    assert!(projection.portability_status["active_blocker_findings"].is_null());
    let receipt = root
        .path()
        .join("audit/PORTABILITY_AUDIT_2026-05-24/summary.json");
    fs::create_dir_all(receipt.parent().unwrap()).unwrap();
    for malformed in [
        "not json",
        "{}",
        r#"{"summary":{}}"#,
        r#"{"summary":{"active_blocker_findings":-1,"findings_total":2}}"#,
    ] {
        fs::write(&receipt, malformed).unwrap();
        let projection = build_readiness_projection(&profile, root.path());
        assert_eq!(projection.portability_status["status"], "invalid");
        assert!(projection.portability_status["active_blocker_findings"].is_null());
    }
    fs::write(
        &receipt,
        r#"{"summary":{"active_blocker_findings":2,"findings_total":3}}"#,
    )
    .unwrap();
    let projection = build_readiness_projection(&profile, root.path());
    assert_eq!(projection.portability_status["status"], "warn");
    assert_eq!(projection.portability_status["active_blocker_findings"], 2);
    fs::remove_file(root.path().join("AGENTS.md")).unwrap();
    assert_ne!(
        build_readiness_projection(&profile, root.path()).gate_status,
        "pass"
    );
}
