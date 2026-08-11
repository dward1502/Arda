use super::CommandProviderSpec;

/// Optional provider. Absence is represented by an `Unavailable` receipt.
pub fn cargo_modules_structure() -> CommandProviderSpec {
    CommandProviderSpec::new(
        "rumil.cargo_modules.v1",
        "rust_module_structure",
        "cargo",
        ["modules", "structure", "--lib"],
    )
}
