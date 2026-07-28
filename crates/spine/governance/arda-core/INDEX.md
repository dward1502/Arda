# Index: crates/spine/governance/arda-core

The foundational Spine governance crate for Arda. It owns the shared spine
surface: contract versioning, routing, message types, learning primitives,
registry types, and the unified Arda error type.

- `Cargo.toml` — crate manifest
- `INDEX.md` — this deterministic direct-child map
- `INDEX.jsonl` — generated Soterion index records
- `README.md` — purpose and verified baseline
- `BREAKDOWN.md` — source inventory, boundaries, and evidence
- `STATUS.md` — current verification and known follow-ups
- `OWNERSHIP.md` — authority and integration boundaries
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
- `OWNERSHIP.md`: owned and non-owned authority boundaries

The list above exactly covers the crate's ten direct children. The completed foundation
`PLAN.md` was retired on 2026-07-28 after its durable decisions and evidence were absorbed into
the maintained documents.
