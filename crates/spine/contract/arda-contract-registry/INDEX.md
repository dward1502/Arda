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

# Index: crates/spine/contract/arda-contract-registry

Typed read-only loader and schema boundary for the canonical contract registry.

- `Cargo.toml` — package and dependency declarations.
- `README.md` — mission, public API, and consumer entry point.
- `STATUS.md` — current strict verification evidence.
- `BREAKDOWN.md` — exhaustive source and dependency classification.
- `OWNERSHIP.md` — schema/artifact/launcher authority boundary.
- `INDEX.md` — this direct-child map.
- `src/`
  - `lib.rs` — public module declaration.
  - `registry.rs` — schema types, typed loaders/errors, and unit tests.
- `tests/`
  - `registry_smoke.rs` — read-only canonical workspace acceptance checks.

## Purpose (one line)
Declares canonical contract tracks, source modules, and declared schema versions for downstream governance tooling.

## Silmarillion rename
Stays `arda-contract-registry` (Arda-native governance spine).

## Current status
- Part of the verified workspace tree.
- Parser behavior is isolated with temporary fixtures; live acceptance remains
  in `tests/registry_smoke.rs` against `core/state/contract_registry.json`.
- `arda-launcher` is the only direct Cargo consumer.
