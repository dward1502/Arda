# Warden Outpost Scout Stream

**Status:** Complete — verified and retained as implementation evidence
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
- return a governance-visible ingestion receipt backed by the canonical memory id;
  promotion receipts remain exclusive to `arda-vaire` consolidation,
- never promote observational memory into execution authority.

### Task P3 — Recall wiring

Add a read path so council/manwe/relic can query scout memory:
- scoped recall by crate/app/path/query,
- time-bounded results with confidence/trust metadata,
- degraded behavior when memory is stale/unavailable.

## Completion review (2026-07-27)

All scoped tasks are complete.

| Task | State | Verified implementation |
|---|---|---|
| P0 protocol | Complete | Versioned observation schema, authority markers, JSON round-trip, feedback conversion, and bounded queue contract in `arda-outpost-protocol`. |
| P1 survey | Complete | Survey restricted to `crates/`, `apps/`, and `outposts/`; nested workspace paths are discovered up to the bounded depth and build-output directories are skipped. |
| P2 ingestion | Complete | Full `OutpostObservation` JSON is stored through `arda-vaire`; id, timestamp, freshness, confidence, classification, authority, payload, and provenance survive recall. The returned memory id is the ingestion receipt. |
| P3 recall | Complete | Public scoped query supports scope, crate/app name, path, free text, time window, result limit, and maximum age; results expose confidence/trust and structured `available`, `stale`, or `unavailable` state. |

Review repairs:
- fixed queue capacity being derived from the topic vector's allocator capacity;
  capacity is now enforced independently per topic across queued and in-flight work,
- implemented failed-ack retry metadata instead of the previous no-op ack path,
- fixed survey depth so real nested Arda workspace crates are visible,
- changed memory content from payload-only JSON to the complete governed observation,
- separated canonical memory scope (`outpost_scout`) from observation scope tags,
- added primary/fallback memory-root handling and structured unavailable recall,
- added exact scoped filtering and stale-result behavior.

## Out of Scope

- Magnetometer/environmental hardware on Warden.
- Relic visualization changes.
- Clinical/health biometric inference.
- Autonomous queue mutation from scout memory.

## Verification

- `cargo check` / `cargo test` for new outpost crates.
- `cargo check` / `cargo test` for `arda-vaire` integration changes.
- Fixture-backed end-to-end observation -> store -> recall flow.

## Milestone 2 completion evidence

- `outposts/arda-outpost-protocol/src/observation.rs`: explicit `AgentFeedback -> OutpostObservation` path with provenance/schema/confidence
- `outposts/arda-outpost-protocol/src/queue.rs`: `OutpostQueue`, `generate_queue`, `consume_queue`
- `outposts/arda-outpost-protocol/tests/observation_feedback.rs`: round-trip + schema mismatch
- `outposts/arda-outpost-protocol/tests/queue_generate_consume.rs`: generate/consume acceptance test
- `outposts/arda-outpost-scout/src/observation.rs`: `CrateObservation -> AgentFeedback` mappings + authority rules
- `cargo test -p arda-outpost-protocol -p arda-outpost-scout` exits 0 (2026-07-27)

Verification:
```text
     Running tests/observation_feedback.rs
     running 2 tests
     test schema_mismatch_rejects_with_conversion_error ... ok
     test scout_feedback_round_trip_preserves_fields_schema_and_confidence ... ok

     Running tests/queue_generate_consume.rs
     running 1 test
     test queue_accepts_scout_feedback_in_generate_consume_path ... ok

     Running arda-outpost-scout tests
     running 6 tests
     test observation::tests::active_status_maps_to_derived_estimate ... ok
     test observation::tests::shell_and_unknown_statuses_map_to_self_report ... ok
     test observation::tests::stubbed_and_deprecated_statuses_map_to_unavailable ... ok
     test suggestion::tests::empty_survey_returns_action_advisory ... ok
     test suggestion::tests::shell_without_tests_is_caution ... ok
     test suggestion::tests::summarize_advisories_reports_all_entries ... ok

     Running tests/observation_fixtures.rs
     running 1 test
     test survey_repo_discovers_crates_and_apps ... ok

     Running tests/survey_fixtures.rs
     running 2 tests
     test survey_report_validates_schema_and_attributes ... ok
     test advisory_reports_use_active_as_base ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured
```

## Milestone 3 — Arda Light scout memory bridge (complete)

### Task M3.1 — Memory mapping + degraded fallback
Add an opt-in memory bridge in `arda-outpost-scout` that maps `OutpostObservation` -> `InformantEvent` for `arda-vaire` and gracefully falls back when memory is unavailable.

Deliverables:
1. `outposts/arda-outpost-scout/src/memory.rs`: public compatibility helpers plus
   `ObservationMemoryBridge` for explicit root selection and scoped recall.
2. Fallback path when `MnemosyneService::new()` or `encode()` returns `Err`/`None`:
   return an advisory `MemoryFallback` record but do not fail the observation path.
3. Typed `ScoutRecallQuery` / `ScoutRecallReport` API for downstream council,
   Manwe, and Relic callers without coupling those consumers into the outpost crate.

Acceptance:
- `cargo test -p arda-outpost-scout --test memory_fixtures` exits 0:
  - full observation encode/reload and canonical memory scope,
  - scope/path/query filtering with confidence/trust metadata,
  - stale and unavailable recall states,
  - structured fallback on an invalid root,
  - successful configured fallback-root selection.

Final live evidence (2026-07-27):
```text
$ cargo test -p arda-vaire -p arda-outpost-protocol -p arda-outpost-scout

arda-outpost-protocol:
test observation_json_round_trip_preserves_authoritative_fields ... ok
test scout_feedback_round_trip_preserves_fields_schema_and_confidence ... ok
test each_topic_enforces_the_requested_capacity ... ok
test failed_ack_requeues_with_retry_metadata ... ok

arda-outpost-scout memory_fixtures:
running 5 tests
test encode_preserves_the_observation_and_returns_an_ingestion_receipt ... ok
test scoped_recall_filters_and_reports_stale_memory ... ok
test invalid_root_returns_structured_memory_fallback ... ok
test recall_degrades_when_memory_root_is_unavailable ... ok
test configured_fallback_root_is_used_when_the_primary_root_fails ... ok

arda-outpost-scout observation_fixtures:
test survey_repo_discovers_crates_and_apps ... ok
test survey_repo_discovers_nested_workspace_crates ... ok

arda-vaire:
29 unit tests, 3 knowledge-delta tests, and 2 public-flow tests passed.

$ cargo check --workspace
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.91s
```

The only emitted build warning is the pre-existing workspace warning that the
non-root profile in `apps/arda-launcher/src-tauri/Cargo.toml` is ignored.
