# Index: crates/spine/governance/arda-core

The foundational Spine governance crate for Arda. It owns the shared spine
surface: contract versioning, routing, message types, learning primitives,
registry types, and the unified Arda error type.

- `Cargo.toml` — crate manifest
- `INDEX.md` — this deterministic direct-child map
- `INDEX.jsonl` — generated Soterion index records
- `README.md` — purpose and verified baseline
- `BREAKDOWN.md` — source inventory, boundaries, and evidence
- `PLAN.md` — completed foundation plan and future-growth boundary
- `STATUS.md` — current verification and known follow-ups
- `docs/` — interop landscape, implemented GEN3 consumers, and open questions
- `src/` — compiled crate surface; see `src/lib.rs` and `src/INDEX.md`
- `tests/` — sovereign tool-contract smoke coverage

## Purpose (one line)
Shared spine surface for the Arda governance bus.

## Notes
`arda-service-registry` functionality is folded into this crate via the
`service_registry` module. No separate workspace crate is needed.

## Evidence
- `STATUS.md` captures build/test counts and env knobs
- `BREAKDOWN.md`: module inventory and responsibilities
- `PLAN.md`: completed foundation plan and execution record
