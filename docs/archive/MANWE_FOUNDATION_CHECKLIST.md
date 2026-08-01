# manwe — Foundation Checklist (Archived)

Owner: HADES
Reviewed: 2026-07-27
Scope: `crates/spine/runtime/manwe`

This completed checklist was archived on 2026-07-27 after the release gates
were rerun against the live workspace. It is retained as implementation
history, not an active backlog. Keep port `7171` frozen until every engine,
launcher, registry, and operator consumer is updated together.

Final closure evidence: all-feature check and Clippy, rustfmt, 278 library and
29 binary tests, both maintained process smokes, documentation validation, and
the downstream `arda-engine` suite passed.

## Evolution follow-up — 2026-07-26

- [x] Add bounded deterministic task-class benchmark receipts without running
  judge-model work on constrained edge nodes.
- [x] Keep the three obsolete backbone services intentionally retired, repair
  the Beelink Carnice systemd lane, verify live inference, and enroll
  `edge_carnice` in governed provider configuration.
- [x] Align static and governed metrics on the `manwe_*` namespace, Prometheus
  base units, and bounded `provider_id`/`model`/`route_class` labels; remove
  generated `charon_*` aliases and free-form task labels.

## Review baseline

- [x] Trace the active library and binary module graphs from `lib.rs` and
  `main.rs`.
- [x] Audit Cargo features, HTTP routes, config inputs, persisted receipts, and
  workspace consumers.
- [x] Run default check/test, adaptive check/test, gRPC check, and rustfmt.
- [x] Separate fresh compile/test evidence from historical live-runtime evidence.
- [x] Rewrite `README.md`, `BREAKDOWN.md`, and `STATUS.md` from live source.

## P0 — Resolve the runtime contract

- [x] Repair the registry schema mismatch by making
  `arda-engine::registry::Registry` consume the canonical singular
  `[[service]]` command/cwd records and reject empty registries.
- [x] Decide which process owns the canonical Manwe service instead of the
  stale `cargo run -p arda-aule -- serve` command.
  - Completion: registry schema, command, health URL, README, and engine
    supervision behavior all identify and launch the same process.
  - Decision: `cargo run -p manwe -- --config manwe.toml` is canonical.
    `arda-engine` now parses the manifest's singular command/cwd schema,
    resolves commands through `PATH`, and preserves the declared working
    directory. The registered health contract is
    `http://127.0.0.1:7171/healthz`.
- [x] Decide the intended meaning of `--adaptive`.
  - Option A: wire `src/main.rs` to `adaptive::service::ManweService` and its
    governed routing path.
  - Option B: keep the current `ProviderCatalog::resolve_with_policy` path and
    name it explicitly as adaptive-lite, without implying the full service is
    active.
  - Completion: `/v1/capabilities`, CLI help, docs, and tests describe one
    consistent behavior.
  - Decision: Option A. `--adaptive` starts the full governed `ManweService`
    and `adaptive::transport::http`; adaptive-lite is not advertised as the
    adaptive runtime.
- [x] Add a maintained process-level smoke test for the default binary.
  - Completion: bind to a temporary port; assert `/healthz`, `/v1/models`,
    `/v1/capabilities`, and one controlled `/v1/chat/completions` response.
- [x] Add the equivalent process-level smoke test for the chosen adaptive
  runtime.
  - Completion: prove the expected policy authority, response headers, and
    receipt output rather than only compiling adaptive modules.

## P1 — Unify configuration and state

- [x] Define precedence and ownership for `manwe.toml`, `config/fleet.toml`, and
  the adaptive provider/state files.
- [x] Make the health/capabilities surfaces report which config source and
  catalog generation are active.
- [x] Reconcile remaining Annunimas/Charon compatibility variables with canonical
  `ARDA_MANWE_*` names; document aliases before deprecating any variable.
  - Provider, fleet-catalog, and state-directory path variables now use
    canonical-first precedence; policy/tuning variables still require inventory.
- [x] Verify startup behavior for absent, malformed, and partially valid static
  and fleet configs at process level.
- [x] Keep provider credentials out of health, provider, receipt, and log
  payloads while adding source diagnostics.

## P0 — Canonical path ownership follow-up

- [x] Resolve static fleet defaults, static route receipts, adaptive state, and
  daemon socket paths through `ARDA_ROOT` or the build-derived workspace root.
- [x] Replace the remaining adaptive `data/charon` default with `data/manwe`.
- [x] Restore canonical-first fleet precedence with
  `ANNUNIMAS_CHARON_FLEET_CONFIG` retained as the lower-priority compatibility
  alias.
- [x] Retain `ARDA_ROUTE_*` as the intentional shared route-policy namespace;
  workspace consumers include Manwe, Varda, and Aule.
- [x] Run static and adaptive process smoke tests from the Manwe crate directory
  and reject any crate-local `core/`, `data/`, or operator-library mutation.

## P1 — Reconcile the source graph

- [x] Classify each unattached root file listed in `BREAKDOWN.md` as active
  migration input, intentionally retained compatibility surface, or removable
  residue.
- [x] Classify the parallel files directly under `src/adaptive/` against the
  active `src/adaptive/service/` implementations.
- [x] Decide whether `adaptive/transport/` is a future runtime or obsolete
  transport tree; do not advertise its endpoints while it is unattached.
- [x] Decide whether `adaptive/service/fleet_persistence.rs` should be declared
  and tested or removed.
- [x] Remove files only after workspace consumer searches and feature-scoped
  checks prove there is no required path.
- [x] Keep `src/types.rs` as the canonical public domain model unless a planned
  migration updates every consumer together.
- [x] Remove the 33 undeclared files directly under
  `src/adaptive/service/` after confirming `full_service.rs` attaches only the
  implementations under `service/full/`; repair active path records to the
  canonical locations.
- [x] Remove tracked Python bytecode from `tests/__pycache__/` and locally ignore
  regenerated cache files.

## P1 — Close known implementation gaps

- [x] Implement or intentionally retire
  `ManweService::read_lane_fitness_snapshot`. The active service loads and
  decays persisted `lane_fitness.json`; focused tests cover valid, malformed,
  stale, and concurrent-update behavior.
- [x] Decide whether the binary promises true streaming. It currently buffers
  the complete upstream body even when `stream=true`.
  - Decision: buffered SSE is the supported binary contract. README documents
    the limitation, responses carry `x-manwe-streaming-mode: buffered`, and a
    test verifies byte/content-type preservation plus best-effort final receipt
    handling.
- [x] Add a gRPC runtime smoke test using an ephemeral bind and both exposed
  services.
- [x] Run `cargo check -p manwe --all-features` and
  `cargo test -p manwe --all-features`.
- [x] Repair the `telemetry` feature contract:
  - export `arda_aule::telemetry` when its feature is enabled, or update Manwe
    to the actual supported telemetry API;
  - replace missing `observability::tracer` / `service::telemetry` paths;
  - fix the `ardea_aule` misspellings in `service_events.rs`.

## P2 — Documentation cleanup

- [x] Refresh `PROVIDERS.md`; static forwarding/fleet inputs and the full
  governed provider/state contract are now documented separately from live
  source behavior.
- [x] Regenerate `src/INDEX.md` from the live direct-child layout.
- [x] Repair `src/README.md` with the canonical Manwe source path and current
  review metadata.
- [x] Confirm the pre-existing `ARCHITECTURE.md` deletion is intentional now
  that architecture decisions live in `BREAKDOWN.md`. Commit `287d327` removed
  a stale module inventory, and a repository-wide Markdown search found no
  inbound links to repair.
- [x] Add `tests/check_docs.py` to validate every local Markdown link, the exact
  `src/INDEX.md` direct-child set, and the canonical source path.

## Release gate

- [x] `cargo fmt -p manwe -- --check`
- [x] `cargo check -p manwe`
- [x] `cargo test -p manwe`
- [x] `cargo check -p manwe --features adaptive`
- [x] `cargo test -p manwe --features adaptive`
- [x] `cargo check -p manwe --features grpc`
- [x] `cargo check -p manwe --all-features`
- [x] `cargo test -p manwe --all-features`
- [x] Process smoke tests pass for the canonical static and adaptive paths.
- [x] `README.md`, `BREAKDOWN.md`, `STATUS.md`, `CHECKLIST.md`, and
  `PROVIDERS.md` agree on process owner, routes, config sources, and feature
  boundaries.

## Implementation evidence

- 2026-07-22: `cargo test -p arda-engine` passed all 4 tests, including the
  workspace registry contract, headless Manwe resolution, empty-registry
  rejection, and supervisor shutdown coverage.
- 2026-07-22: `cargo check --bin arda` passed with the command/cwd-aware
  supervisor model.
- 2026-07-23: `cargo test -p manwe --features adaptive --lib` passed all 263
  governed service tests after restoring the complete service/proxy spine.
- 2026-07-23: `python crates/spine/runtime/manwe/tests/process_smoke.py` passed
  static and full adaptive process contracts, including controlled chat,
  governed route headers, and state/governance receipt assertions.
- 2026-07-23: source-graph reconciliation removed seven unattached root files,
  eleven incomplete parallel adaptive files, and undeclared
  `fleet_persistence.rs` after workspace-wide consumer searches found no live
  users. Pre-removal and post-removal `cargo check -p manwe --all-targets
  --features adaptive` and `cargo test -p manwe --features adaptive` passed;
  `src/types.rs` remains the canonical public model.
- 2026-07-23: `arda-aule` now exports its supported telemetry event, tracing
  layer, schema, and shutdown API behind `telemetry`; Manwe's active adaptive
  service emits state and governance events after the event writer accepts them,
  and labels telemetry-only memory delivery explicitly. Manwe installs the OTLP
  layer when configured and flushes it on exit; focused tests cover destination
  routing, attribute preservation, and OTLP layer lifecycle. The stale parallel
  `service_events.rs` containing missing paths
  and `ardea_aule` misspellings was removed. `cargo check -p manwe --all-targets
  --all-features` and `cargo test -p manwe --all-features` passed (264 library +
  21 binary), and all 4 focused `arda-aule` telemetry surface tests passed.
- 2026-07-23: lane-fitness persistence has focused valid, malformed, stale-decay,
  and concurrent-update coverage; the binary streaming contract is explicitly
  buffered SSE with a best-effort final receipt and
  `x-manwe-streaming-mode: buffered`; and the gRPC runtime smoke binds an
  ephemeral listener and calls both generated service clients.
  `cargo check -p manwe --all-targets --all-features`, `cargo test -p manwe
  --all-features` (268 library + 24 binary), `cargo fmt -p manwe -- --check`,
  and the static/adaptive process smoke suite passed.
- 2026-07-23: refreshed the provider contract and source indexes, retained
  architecture decisions in `BREAKDOWN.md`, and added `tests/check_docs.py`.
  The validator passed across 8 Markdown files, 25 local links, and 12 source
  index entries. `python -m py_compile` for the validator,
  `cargo check -p manwe`, `cargo fmt -p manwe -- --check`, and
  `git diff --check -- crates/spine/runtime/manwe` also passed; Cargo emitted
  only the documented workspace-profile and pre-existing dead-code warnings.
- 2026-07-25: completed the foundation audit. Removed 33 undeclared parallel
  `adaptive/service/*.rs` implementations after tracing the explicit
  `service/full/` module graph and checking workspace consumers; corrected the
  two active contract-registry paths and one production-plan evidence path.
  Removed two tracked Python bytecode files and added a local cache ignore.
  Default, adaptive, and all-feature checks/tests retained their prior counts;
  static/adaptive process smoke, rustfmt, and documentation validation passed.
- 2026-07-26: recovered the failed resource-policy implementation. Restored
  bounded transient-cooldown fallback, wired per-host fleet concurrency through
  provider selection and resource acquisition, released the limiter catalog
  mutex before queue waits, and preferred equivalent alternate resource groups
  when the selected group is saturated. Default and all-feature Clippy pass
  with `-D warnings`; all-feature tests pass with 273 library and 27 binary
  tests.
- 2026-07-26: canonical path ownership now roots fleet defaults, static route
  receipts, adaptive state, the daemon socket, and Bacon-Lite outputs outside
  the crate working directory. Default and adaptive Clippy pass with
  `-D warnings`; default tests pass with 1 library + 25 binary tests; adaptive
  tests pass with 274 library + 26 binary tests; and the crate-CWD static/full
  adaptive process smoke passes without mutating crate-local output trees.
  All-feature Clippy now passes with `-D warnings`, and all-feature tests pass
  with 274 library + 27 binary tests after restoring Orome's documented module
  boundary.
