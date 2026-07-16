---
soterion:
  sigil: "RETIRE"
  glyph: "♻️"
  role: "service_registry"
  owner: "arda"
  status: "transferred_to_arda-core"
  last_reviewed: "2026-07-15"
---

> ♻️ arda-service-registry: ♻️ service_registry | owner: arda | status: transferred_to_arda-core | reviewed: 2026-07-15

# Index: crates/spine/executors/arda-service-registry

This crate is retired. Its service-registry types and logic were moved into
`arda-core` under the `service_registry` module. `arda-engine` now re-exports
`arda_core::service_registry as service_registry` to preserve the old public
path during the migration.

## Retired contents
- `contract.rs` — service-kind / service-contract types → `arda-core` `contract.rs`
- `service.rs` — service status / handle / state persistence → `arda-core`
- `registry.rs` — in-memory registry store → `arda-core`
- `test_support.rs` — test utility → `arda-core`
- `tests/contract_smoke.rs` — smoke tests

## Migration status
- [x] Fold types + registry into `arda-core` `service_registry` module
- [x] Re-export `ServiceContract`, `ServiceKind`, `ServiceRegistry`, ... from `arda-core`
- [x] Update `arda-engine` to import from `arda_core::service_registry`
- [ ] Remove workspace entry after confirming no other crate depends on the path
- [ ] Drop crate directory after deprecation period or after full migration
