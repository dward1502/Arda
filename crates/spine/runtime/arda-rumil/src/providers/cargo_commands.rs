use super::CommandProviderSpec;

pub fn cargo_check() -> CommandProviderSpec {
    CommandProviderSpec::new(
        "rumil.cargo_check.v1",
        "rust_build_check",
        "cargo",
        ["check", "--workspace", "--message-format=json"],
    )
}
