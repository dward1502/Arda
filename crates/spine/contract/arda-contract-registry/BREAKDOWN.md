---
soterion:
  sigil: "SPINE"
  glyph: "📄"
  role: "contract_registry"
  owner: "arda"
  status: "active"
  last_reviewed: "2026-07-28"
---

> 📄 arda-contract-registry: 📄 contract_registry | owner: arda | status: active | reviewed: 2026-07-28

# Breakdown: crates/spine/contract/arda-contract-registry

## Purpose (one sentence)

`arda-contract-registry` models the canonical contract registry itself: a schema-pinned manifest of tracks, source modules, evidence classes, schema versions, receipt stores, and CLI verbs so downstream tooling can validate Arda artifacts against an authoritative, path-based contract surface.

## Why it exists

Arda governance and production workflows need a stable, machine-readable source of truth for contract tracks. Instead of scattering track metadata across configs or docs, this crate owns the data model and exposes live verification tests so CI can prove declared schema versions actually exist in the referenced source/surface modules.

## What it does

| capability | owner module | notes |
|---|---|---|
| Track/registry data model | `registry.rs` | `TrackDefinition` + `ContractRegistry` with `track_ids()` |
| Smoke validation | `tests/registry_smoke.rs` | Validates registry JSON against filesystem paths |
| Schema pinning | tests | Enforces `schema_version == "arda.contract-registry.v1"` |
| Source module existence | tests | Every `source_modules` entry must resolve on disk |
| Schema version presence | tests | Declared `schema_versions` must appear in source or receipt surfaces |

## Crate layout

```
crates/spine/contract/arda-contract-registry
├── Cargo.toml
├── README.md
├── STATUS.md
├── INDEX.md
├── OWNERSHIP.md
├── src
    ├── lib.rs
    └── registry.rs
└── tests
    └── registry_smoke.rs
```

## Crate dependencies

```
arda-contract-registry
├── serde         workspace  // derive + deserialization
├── serde_json    workspace  // JSON parsing for registry manifests
├── thiserror     workspace  // typed read/parse errors
├── tempfile      dev        // test fixtures
└── walkdir       dev        // recursive receipt surface scanning in smoke tests
```

## Usage contract

A consumer using `arda_contract_registry` can:

1. Load a `ContractRegistry` through `ContractRegistry::load(path)` or
   `ContractRegistry::load_from_root(root)`
2. Inspect `registry.tracks` for `TrackDefinition` metadata
3. Call `registry.track_ids()` to enumerate declared tracks
4. Handle missing and malformed artifacts through typed `RegistryLoadError` variants
5. Run `tests/registry_smoke.rs` against `core/state/contract_registry.json` to verify live state matches the declarative contract

## Supported source classification

| Classification | Count | Paths |
|---|---:|---|
| Production/default | 2 | `src/lib.rs`, `src/registry.rs` |
| Production/feature-gated | 0 | No features are declared |
| Generated include | 0 | None |
| Standalone test-only source | 0 | Unit tests are inline |
| Integration test | 1 | `tests/registry_smoke.rs` |
| Build script | 0 | None |
| Unwired | 0 | None |

The only direct Cargo consumer is `arda-launcher`. Governance/core crates have
no manifest or source dependency on this crate.

## Verification status

- `cargo check -p arda-contract-registry`: successful
- `cargo test -p arda-contract-registry`: 6 passed (3 unit + 3 integration)
  - `registry_schema_version_is_pinned`
  - `every_track_has_source_modules`
  - `every_track_schema_version_is_present_in_a_source_or_surface_module`
- `cargo check --all-features`: successful
- `cargo test --all-features`: 6 passed, 0 failed
- strict all-target Clippy and strict rustdoc: passed
- `arda-launcher` all-target consumer check and 8-test library suite: passed

