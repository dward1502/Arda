# Warden Outpost Scout Stream

**Status:** Active plan — implementation-first, governance-gated
**Scope:** Warden scout creates structured observations, stores them as Arda memory via `arda-vaire`, and lets Arda recall/learn from them. Outpost code lives at `outposts/`, not under `crates/spine/`.

## Goal

Let Warden operate as an Arda outpost that:
1. surveys repo/app structure and operations,
2. encodes each finding as a governed `OutpostObservation`,
3. persists observations into `arda-vaire` with provenance,
4. allows later recall by council/manwe/relic as advisory evidence.

This is the general scaffold for later feelers/web/environmental streams.

## Boundaries

- Outpost implementation lives in `outposts/` at the repo root.
- `arda-vaire` remains the canonical memory layer in `crates/spine/memory/arda-vaire`.
- Scout observations are advisory evidence only. They do not approve/reject/queue-mutate.
- Hardware-specific adapters stay outpost-local; Arda core owns policy/routing/receipts.

## Deliverables

1. `outposts/arda-outpost-protocol/` — shared types/schema constants used across outposts.
2. `outposts/arda-outpost-scout/` — survey + observation + memory ingestion.
3. `arda-vaire` integration path for outpost observations with source metadata.
4. Tests + fixtures proving authority boundaries and memory round-trips.

## Tasks

### Task P0 — Shared protocol scaffold

Create `outposts/arda-outpost-protocol/Cargo.toml` and `src/lib.rs` with:
- `OutpostObservation` fields: id, source node, timestamp, scope, freshness, confidence, classification, payload, provenance.
- Authority marker enum: `advisory | presentation | execution_prohibited`.
- Schema version constant and JSON round-trip fixtures.

### Task P1 — Scout survey and local fixtures

In `outposts/arda-outpost-scout/`:
- `src/observation.rs`: validate/serialize/deserialize `OutpostObservation`.
- `src/survey.rs`: bounded filesystem survey over `crates/` and `apps/`.
- `tests/observation_fixtures.rs`: authority + JSON round-trip tests.
- `tests/survey_fixtures.rs`: fixture repo survey tests.

### Task P2 — Memory ingestion bridge

Add a forwarder from scout observations into `arda-vaire`:
- encode each observation as a recallable memory event with source metadata,
- preserve observation id, freshness, confidence, classification,
- append governance-visible promotion receipt,
- never promote observational memory into execution authority.

### Task P3 — Recall wiring

Add a read path so council/manwe/relic can query scout memory:
- scoped recall by crate/app/path/query,
- time-bounded results with confidence/trust metadata,
- degraded behavior when memory is stale/unavailable.

## Out of Scope

- Magnetometer/environmental hardware on Warden.
- Relic visualization changes.
- Clinical/health biometric inference.
- Autonomous queue mutation from scout memory.

## Verification

- `cargo check` / `cargo test` for new outpost crates.
- `cargo check` / `cargo test` for `arda-vaire` integration changes.
- Fixture-backed end-to-end observation -> store -> recall flow.
