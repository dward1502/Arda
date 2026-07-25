use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use arda_governance::{
    load_governance_chain, load_philosopher_profiles, load_realm_policy, GovernanceChainConfig,
    GovernancePaths, PhilosopherProfileSet, RealmPolicyConfig,
};

fn temp_base() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    std::env::temp_dir().join(format!("arda-governance-paths-{nonce}"))
}

#[test]
fn loaders_work_from_an_injected_base_outside_the_repository() {
    let base = temp_base();
    let paths = GovernancePaths::new(&base);
    fs::create_dir_all(paths.chain_config().parent().expect("config parent"))
        .expect("create temporary config directory");
    fs::write(
        paths.chain_config(),
        include_str!("../../../../../config/governance/chains.toml"),
    )
    .expect("write chain fixture");
    fs::write(
        paths.philosopher_profiles(),
        include_str!("../../../../../config/governance/philosophers.toml"),
    )
    .expect("write profile fixture");
    fs::write(
        paths.realm_policy(),
        include_str!("../../../../../config/governance/realm_policies.toml"),
    )
    .expect("write realm policy fixture");

    let chain = load_governance_chain(paths.chain_config()).expect("load chain from injected root");
    let profiles = load_philosopher_profiles(paths.philosopher_profiles())
        .expect("load profiles from injected root");
    let realm_policy =
        load_realm_policy(paths.realm_policy()).expect("load realm policy from injected root");

    assert_eq!(chain.schema_version, GovernanceChainConfig::SCHEMA_VERSION);
    assert_eq!(
        profiles.schema_version,
        PhilosopherProfileSet::SCHEMA_VERSION
    );
    assert_eq!(profiles.profiles.len(), 3);
    assert_eq!(
        realm_policy.schema_version,
        RealmPolicyConfig::SCHEMA_VERSION
    );
    assert!(!realm_policy.global_default.autonomous_blocking_enabled);

    fs::remove_dir_all(base).expect("remove temporary fixture root");
}
