---
soterion:
  sigil: "SPINE"
  glyph: "📄"
  role: "contract_registry"
  owner: "arda"
  status: "active"
  last_reviewed: "2026-07-23"
---

> 📄 arda-contract-registry: 📄 contract_registry | owner: arda | status: active | reviewed: 2026-07-23

# Index: crates/spine/contract/arda-contract-registry

Schema-pinned contract registry source of truth for Arda governance tracks.

- `Cargo.toml`
- `src`
  - `lib.rs` — single public module declaration
  - `registry.rs` — `ContractRegistry` + `TrackDefinition`

## Purpose (one line)
Declares canonical contract tracks, source modules, and declared schema versions for downstream governance tooling.

## Silmarillion rename
Stays `arda-contract-registry` (Arda-native governance spine).

## Current status
- Part of the verified workspace tree.
- Verification lives in `tests/registry_smoke.rs` against `core/state/contract_registry.json`.
