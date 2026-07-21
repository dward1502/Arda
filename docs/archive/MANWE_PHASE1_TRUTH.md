---
soterion:
  sigil: "TRUTH"
  glyph: "𓂀"
  role: "evidence_record"
  owner: "HADES"
  status: "frozen"
  baseline_revision: "5a78dd24a8e2f95afd18677c8a8c29918e3ffa6b"
  review_date: "2026-07-20"
---

# manwe — Phase 1: Established Truth

Evidence record for `crates/spine/runtime/manwe@5a78dd2`.
Use this as the single source of truth for baseline/delta claims.

## Environment

- rustc 1.94.0 `4a4ef493e 2026-03-02`
- cargo 1.94.0 `85eff7c80 2026-01-15`
- host `launcher` / package Rust not relevant for manwe default modes

## Verified baseline

### Default mode

```text
cargo check -p manwe
cargo test -p manwe
```

Result: PASS
- lib: default build succeeds with warnings only
- bin: default build succeeds with warnings only
- tests: 88 passed; 0 failed
- exact warning count in lib: 329
- exact warning count in bin: 9

### Adaptive feature mode

```text
cargo check -p manwe --features adaptive
cargo test -p manwe --features adaptive
```

Result: PASS
- lib: adaptive build succeeds with warnings only
- tests: 88 passed; 0 failed
- exact warning count in lib: 329

Numeric baseline after Phase 1 stabilization:
- warnings reduced from 342 -> 329
- adaptive compile errors reduced from 278 -> 0
- adaptive tests reduced from 278 compile errors -> 88 passing tests

## Stable naming/status decisions

Naming policy:
- `Charon*` types remain temporary legacy terminology
- future rename/replacement should be deferred until adaptive subtree is fully stable
- visible API prefers `Manwe*` surfaces where already introduced

Adaptive work status for Phase 1:
- adaptive subtree compiles and all tests pass
- adaptive recovery was executed as bounded layers:
  - added missing `tempfile` dependency and feature wiring
  - repaired duplicate type definitions across crate boundaries
  - added missing `Default` impl for adaptive `ProviderState`
  - gated `route_policy_tests` module under `#[cfg(test)]`
  - unified test-local type imports to adaptive namespaces
  - fixed 4 runtime test failures in bootstrap_overlay, fleet_persistence, state_mutation

## Next recommended action

Proceed to Phase 2: Lock the stable gateway contract
- add integration tests for `/healthz`, `/v1/models`, forwarding, malformed inputs, unreachable upstream
