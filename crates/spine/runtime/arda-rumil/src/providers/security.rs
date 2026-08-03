use super::CommandProviderSpec;

pub fn cargo_audit() -> CommandProviderSpec {
    CommandProviderSpec::new(
        "rumil.cargo_audit.v1",
        "dependency_security",
        "cargo",
        ["audit", "--json"],
    )
}

pub fn cargo_deny() -> CommandProviderSpec {
    CommandProviderSpec::new(
        "rumil.cargo_deny.v1",
        "dependency_policy",
        "cargo",
        ["deny", "check", "--format", "json"],
    )
}
