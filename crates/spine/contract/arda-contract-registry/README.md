# arda-contract-registry

Typed, read-only loading boundary for Arda's canonical contract registry
artifact.

## Public surface

- `registry::ContractRegistry` and `registry::TrackDefinition`.
- `ContractRegistry::load(path)` for an explicit artifact.
- `ContractRegistry::load_from_root(root)` for the canonical
  `core/state/contract_registry.json` location.
- `RegistryLoadError::{Read, Parse}` for typed operator diagnostics.
- `DEFAULT_REGISTRY_PATH` and `ContractRegistry::track_ids()`.

The crate does not mutate or generate the canonical artifact. See
[OWNERSHIP.md](OWNERSHIP.md) for the producer/consumer boundary.

## Consumers

`arda-launcher` is the only direct Cargo consumer. It uses this crate's loader
and owns onboarding readiness projection. Governance/core crates are not direct
consumers in the current workspace.

## Verification

Run `cargo test -p arda-contract-registry -- --test-threads=1`. The closeout
suite contains 3 isolated unit tests and 3 read-only live-workspace integration
tests. Strict evidence is recorded in [STATUS.md](STATUS.md).
