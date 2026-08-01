#![cfg(feature = "full-cli")]

use arda_aule::ceo::CoreAutonomyProfile;

#[test]
fn ceo_autonomy_profile_loads_from_the_arda_core_root() {
    let root = tempfile::tempdir().expect("temp core root");
    std::fs::create_dir_all(root.path().join("realm")).expect("realm directory");
    std::fs::create_dir_all(root.path().join("state")).expect("state directory");
    std::fs::write(
        root.path().join("realm/boot.toml"),
        r#"
[ceo]
heartbeat_ms = 750
triad_bypass = false

[joulework.base_costs]
dispatch = 12.5
"#,
    )
    .expect("boot config");
    std::fs::write(
        root.path().join("state/world.json"),
        r#"{"system":{"status":"READY"},"metrics":{"system_resonance":72.0}}"#,
    )
    .expect("world state");

    let profile = CoreAutonomyProfile::load(root.path()).expect("CEO profile");
    assert_eq!(profile.heartbeat_ms, 750);
    assert!(!profile.triad_bypass);
    assert_eq!(profile.base_cost_for("DISPATCH"), Some(12.5));
    assert_eq!(profile.world_status.as_deref(), Some("READY"));
    assert_eq!(profile.world_resonance, Some(72.0));
}
