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
fn selected_runtime_configuration_is_the_only_required_endpoint() {
    let (root, _) = fixture();
    let manifest = root.path().join("services.toml");
    fs::write(
        &manifest,
        r#"[[service]]
name = "manwe"
start = { command = "manwe", args = ["--bind", "127.0.0.1", "--port", "17171"] }
health = { url = "http://127.0.0.1:17171/healthz" }
"#,
    )
    .unwrap();
    let profile = build_environment_profile(Some(root.path()), None, None).unwrap();
    let endpoint = profile.endpoints.manwe_base_url.unwrap();
    assert_eq!(endpoint.value, "http://127.0.0.1:17171");
    assert!(matches!(endpoint.source, ValueSource::ServiceRegistry));
    assert!(endpoint.health.is_none());
    assert!(profile.missing_gates.is_empty());
    for invalid in ["not toml", ""] {
        fs::write(&manifest, invalid).unwrap();
        let profile = build_environment_profile(Some(root.path()), None, None).unwrap();
        assert!(profile.endpoints.manwe_base_url.is_none());
        assert!(profile
            .missing_gates
            .iter()
            .any(|gate| gate.starts_with("services.toml:")));
        assert!(!profile
            .missing_gates
            .iter()
            .any(|gate| gate.contains("HERMES_BASE_URL")));
    }
}

#[test]
fn proposal_uses_shipped_template_and_escapes_paths() {
    let (root, mut profile) = fixture();
    let template = root.path().join("config/templates/arda.local.profile.toml");
    fs::create_dir_all(template.parent().unwrap()).unwrap();
    fs::write(
        &template,
        include_str!("../../../../../config/templates/arda.local.profile.toml"),
    )
    .unwrap();
    profile.paths.data_dir.value = "/private/a\"quoted\\path".to_string();
    profile.endpoints.hermes_base_url = None;
    let path = crate::onboarding::io::build_proposed_config(&profile, root.path()).unwrap();
    let output = fs::read_to_string(path).unwrap();
    let parsed: toml::Value = toml::from_str(&output).unwrap();
    assert_eq!(
        parsed["paths"]["data_dir"].as_str().unwrap(),
        profile.paths.data_dir.value
    );
    assert!(parsed["endpoints"].get("hermes_base_url").is_none());
    assert!(!output.contains("${"));
    for populated in [true, false] {
        fs::write(&template, "[profile]\n[paths]\n[endpoints]\n[safety]\n").unwrap();
        profile.endpoints.local_model_default =
            populated.then(|| crate::onboarding::types::LocalModelDefaultValue {
                value: "selected-model".into(),
                source: ValueSource::OperatorInput,
            });
        profile.endpoints.crawl4ai_url = populated.then(|| crate::onboarding::types::UrlValue {
            value: "http://127.0.0.1:1234".into(),
            source: ValueSource::OperatorInput,
            health: None,
        });
        profile.endpoints.search_runtime_url = profile.endpoints.crawl4ai_url.clone();
        let path = crate::onboarding::io::build_proposed_config(&profile, root.path()).unwrap();
        let parsed: toml::Value = toml::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(
            parsed["endpoints"]
                .get("local_model_default")
                .and_then(toml::Value::as_str),
            populated.then_some("selected-model")
        );
        for key in ["crawl4ai_url", "search_runtime_url"] {
            assert_eq!(
                parsed["endpoints"].get(key).and_then(toml::Value::as_str),
                populated.then_some("http://127.0.0.1:1234")
            );
        }
        assert_eq!(
            parsed["profile"]["id"].as_str(),
            Some(profile.profile.as_str())
        );
    }
    fs::write(&template, "[paths]\n").unwrap();
    assert!(crate::onboarding::io::build_proposed_config(&profile, root.path()).is_err());
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
