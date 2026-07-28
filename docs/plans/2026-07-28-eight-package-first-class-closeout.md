# Remaining Cargo Packages First-Class Closeout Plan

> **For Hermes:** Execute each packet in order with live-source audit, TDD for genuine behavior gaps, strict producer gates, direct-consumer verification, documentation reconciliation, and dirty-worktree-safe closeout.

**Goal:** Bring every remaining Arda Cargo package to the evidence-backed first-class standard already established for `arda-vaire`, `arda-orome`, `arda-aule`, `arda-core`, and `arda-governance`.

**Architecture:** Close dependency-defining packages before their consumers: engine and contract authority first, economics and Mandos services next, protocol before scout, launcher after its backend dependencies, and the root daemon last. A package closes only when its supported source graph is classified, genuine implementation gaps are fixed, direct consumers compile or test, canonical docs agree with live evidence, and completed trackers are removed from the active surface.

**Tech Stack:** Rust 2021/2024, Cargo workspace, Tokio, Axum/Reqwest, Tauri 2, React/TypeScript, JSON/JSONL persistence, TOML service/fleet configuration.

---

## Global safety and acceptance contract

### Starting state

- Branch: `manwe`.
- The workspace is heavily dirty from prior first-class crate work and runtime evidence generation.
- Preserve all pre-existing modified and untracked paths unless a packet explicitly names one as task-owned.
- Never reset, reformat, delete, or claim unrelated paths.
- `~/Annunimas` is reference-only and must not be modified.

### Required discovery for every package

1. Capture exact package-scoped pre-task `git status --short`.
2. Read every crate-root Markdown tracker and canonical document.
3. Read `Cargo.toml`, crate roots, binaries, feature declarations, tests, and build scripts.
4. Trace every Rust file to `lib.rs`, `main.rs`, a binary target, a test target, a build script, or an explicit supported feature.
5. Classify files as production/default, production/feature-gated, generated include, test-only, integration/build, or unwired.
6. Find reverse dependencies from manifests and corroborate with `cargo tree --invert <package>`.
7. Treat checklist claims as discovery prompts; live source and Cargo output override stale prose.

### Required closeout gates for every Rust package

Run the meaningful subset and record exact output:

```text
cargo fmt -p <package> -- --check
cargo check -p <package> --no-default-features
cargo test -p <package> --no-default-features
cargo check -p <package> --all-targets --all-features
cargo test -p <package> --all-features
cargo clippy -p <package> --all-targets --all-features -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc -p <package> --no-deps --all-features
cargo check -p <direct-consumer> --all-targets --all-features
```

If a package has no features, record that fact instead of manufacturing feature distinctions. A filtered test run with zero executed tests is compilation evidence only.

### Documentation contract

At closeout, maintain the smallest truthful crate-local authority:

- `README.md`: mission, public API, runtime/configuration, consumers, verification commands.
- `STATUS.md`: current status and exact recent evidence.
- `BREAKDOWN.md`: exhaustive supported source graph and dependency/consumer boundaries.
- `INDEX.md`: all direct children and nested documentation entry points.
- `OWNERSHIP.md` where authority or operational responsibility is non-obvious.
- Active `PLAN.md`/`CHECKLIST.md` only while genuine implementation work remains.
- Remove completed trackers from active directories only after code, consumer, docs, links, and strict gates pass.

### Final package acceptance

- No supported Rust source is detached or ambiguously wired.
- No accidental crate-local `core/` or `data/` runtime artifacts were generated.
- Markdown links and index coverage pass.
- `git diff --check` passes over task-owned paths, including untracked docs by direct inspection.
- The post-task status is compared with the exact starting path snapshot.

---

## Packet 1 — `arda-engine`

**Objective:** Remove the fake boot boundary, make daemon smoke startup validate real registry wiring, verify supervision/harness behavior, and retire completed crate trackers.

**Status:** Complete on 2026-07-28. The no-op boot API was removed, real
registry validation now precedes `--once`, a Manwe bearer-forwarding regression
test was added, strict crate/root gates passed, ownership was documented, and
the completed crate-local plan/checklist were retired.

**Primary files:**

- Modify: `crates/engine/src/lib.rs`
- Modify: `src/main.rs`
- Inspect/modify as evidence requires: `crates/engine/src/harness.rs`
- Inspect/modify as evidence requires: `crates/engine/src/registry.rs`
- Test: `crates/engine/src/registry.rs`
- Test: `crates/engine/tests/orome_smoke.rs`
- Reconcile: `crates/engine/{README,STATUS,BREAKDOWN,INDEX}.md`
- Create if valuable: `crates/engine/OWNERSHIP.md`
- Retire after acceptance: `crates/engine/{PLAN,CHECKLIST}.md`

### Task 1.1 — Prove the boot mismatch

1. [x] Confirm `arda_engine::boot()` callers and verify it performs no initialization.
2. [x] Add or identify a smoke assertion that `arda --once` validates the real registry rather than exiting after a logging-only function.
3. [x] Run the focused assertion and retain the RED behavior before implementation.

### Task 1.2 — Remove the no-op boundary

1. [x] Remove `arda_engine::boot()` rather than inventing hidden global initialization.
2. [x] Make root-daemon startup load and resolve `services.toml` before honoring `--once`.
3. [x] Preserve required-service errors and `--no-ui` filtering.
4. [x] Keep service process spawning and harness startup disabled in `--once` mode.
5. [x] Verify no callers remain and `arda --once --no-ui` performs a real configuration smoke.

### Task 1.3 — Verify harness truth

1. [x] Confirm `HarnessState` owns an explicitly configured client timeout and optional bearer forwarding.
2. [x] Pin bearer forwarding with a focused harness/upstream regression test.
3. [x] Correct stale status text instead of changing the working proxy contract.

### Task 1.4 — Close documentation and consumers

1. [x] Record the exact eight-file Rust classification.
2. [x] Reconcile current test counts and direct consumers.
3. [x] Add engine/root-daemon ownership boundaries.
4. [x] Retire completed plan/checklist after strict gates.
5. [x] Pass the root `arda` all-target/all-feature consumer check.

---

## Packet 2 — `arda-contract-registry` — complete 2026-07-28

**Result:** The crate now owns typed read-only schema loading, launcher duplicate
parsing is removed, parser/error tests use temporary fixtures, the canonical
workspace acceptance remains read-only, and the completed trackers are retired.

**Primary files:**

- Inspect/modify: `crates/spine/contract/arda-contract-registry/src/**`
- Inspect/modify: `crates/spine/contract/arda-contract-registry/tests/**`
- Inspect consumer: `crates/spine/governance/arda-core/src/contract/**`
- Inspect consumer: `crates/spine/governance/arda-governance/src/**`
- Reconcile: `crates/spine/contract/arda-contract-registry/{README,STATUS,BREAKDOWN,INDEX}.md`
- Create: `crates/spine/contract/arda-contract-registry/OWNERSHIP.md`
- Retire after acceptance: `PLAN.md` and completed `CHECKLIST.md`

### Task 2.1 — Isolate persistence tests

1. [x] Found every test/default that reads or writes `core/state/contract_registry.json`.
2. [x] Added isolated temporary-fixture tests for valid, missing, and malformed artifacts.
3. [x] Kept only explicit read-only workspace acceptance against canonical state.
4. [x] Asserted the workspace registry hash is unchanged across the suite.

### Task 2.2 — Resolve authority

1. [x] Traced all registry loaders; no public mutation API exists.
2. [x] Defined crate schema/loading, repository artifact, and launcher projection ownership.
3. [x] Removed duplicate launcher file reading and JSON parsing.
4. [x] Added no runtime mutation merely to satisfy wiring.

### Task 2.3 — Consumer and docs closeout

1. [x] Proved launcher is the only direct Cargo consumer; governance/core are not.
2. [x] Added exact 3-file source classification and `OWNERSHIP.md`.
3. [x] Retired completed trackers after strict crate and launcher gates.

---

## Packet 3 — `arda-economics` — complete 2026-07-28

**Result:** Stale checklist claims were reconciled against `meter.rs` (there is
no `runtime.rs`), deterministic backend fallback and observability landed, the
10,000-event operator test passed, direct consumers compiled, and completed
trackers were retired.

**Primary files:**

- Modify: `crates/spine/runtime/arda-economics/CHECKLIST.md`
- Inspect/modify: `crates/spine/runtime/arda-economics/src/runtime.rs`
- Inspect/modify: `crates/spine/runtime/arda-economics/src/transport/**`
- Inspect/modify: `crates/spine/runtime/arda-economics/src/**meter**`
- Test: adjacent unit/integration tests discovered from the live tree
- Consumer: `crates/spine/observability/arda-aule`
- Consumer: `crates/spine/executors/arda-varda`
- Reconcile canonical docs; retire `CRATE_PLAN.md`/`CHECKLIST.md` only if no active work remains

### Task 3.1 — Reconcile shipped finance metrics

1. [x] Confirmed and exported the existing finance stream implementation.
2. [x] Added budget-pressure, snapshot-freshness, and transport-latency fields.
3. [x] Proved Aule is not a direct consumer; verified actual direct consumers
   `arda-mandos`, `arda-vaire`, and `arda-varda` instead.

### Task 3.2 — Hardware/estimator fallback matrix

1. [x] Added tests for hardware failure, non-finite sample rejection, estimator
   fallback, and invalid-work terminal failure.
2. [x] Implemented typed `MeterError` and ordered `MeterRegistry::estimate`.
3. [x] Kept tests independent of physical hardware. Timeout is not part of the
   current in-process meter contract and was not fabricated.

### Task 3.3 — Observability hooks

1. [x] Added bounded budget pressure, tariff/source freshness, snapshot age, and
   IPC/HTTP request-latency aggregates at their owning boundaries.
2. [x] Kept Prometheus exposition out of this crate.
3. [x] Added producer tests; no nonexistent Aule direct-consumer test was invented.

### Task 3.4 — Operator-scale JouleWork proof

1. [x] Added an ignored deterministic 10,000-event invariant test.
2. [x] Executed it: total 10,600; observed 5,300; default fallback 5,300;
   average confidence 0.5.
3. [x] No Mandos authority was required for the in-process deterministic contract.

---

## Packet 4 — `arda-mandos`

**Objective:** Complete Mandos in bounded safety-first phases rather than treating its large active checklist as a documentation cleanup.

**Status:** Complete. All producer, feature-matrix, documentation, and direct-consumer gates passed; temporary crate-local trackers were retired.

**Primary files:**

- Policy/scoring: `crates/spine/runtime/arda-mandos/src/{reasoning,scoring,evidence,context,pageindex}.rs`
- Persistence: `crates/spine/runtime/arda-mandos/src/service.rs`
- Transport: `crates/spine/runtime/arda-mandos/src/transport/{dispatch,ipc,http,mod}.rs`
- Notification: `crates/spine/runtime/arda-mandos/src/notify.rs`
- Direct consumers: `arda-aule`, `arda-orome`; `arda-governance` is an upstream dependency
- Canonical docs: `README.md`, `INDEX.md`, `BREAKDOWN.md`, `STATUS.md`, `OWNERSHIP.md`
- Created `README.md` and `INDEX.md`
- Retired `CHECKLIST.md`, `CRATE_PLAN.md`, and `PLAN_CLOSEOUT.md` after all accepted slices closed

### Task 4.1 — Complete remaining P0 policy decisions

1. [x] Encoded escalation as a typed advisory disposition with operator-actionable conditions.
2. [x] Normalized lexical matching once and case-insensitively.
3. [x] Kept compatibility scoring APIs explicit and deprecated obsolete notification compatibility surfaces after consumer search.
4. [x] Preserved and passed direct-consumer contracts before closeout.

### Task 4.2 — Make persistence authoritative

1. [x] Versioned and digest-linked persisted verdict records with canonical JSON hashing.
2. [x] Hydrated query identities, ordered history, chain state, and status counters on restart.
3. [x] Detected malformed, truncated, gapped, tampered, and future-schema records while retaining the valid prefix as degraded state.
4. [x] Used atomic snapshots and bounded recent-history access.
5. [x] Added corruption, restart, duplicate, interrupted-append, and trailing-fragment fixtures.

### Task 4.3 — Harden side effects and transports

1. [x] Replaced untracked telemetry tasks with bounded delivery state and deterministic draining.
2. [x] Shared one typed dispatcher/error contract across direct, IPC, and HTTP paths.
3. [x] Returned status-appropriate HTTP responses with structured error codes.
4. [x] Bounded payloads, request concurrency, history limits, connection work, and telemetry work.
5. [x] Supervised sibling transports and socket ownership with graceful shutdown and listener-failure tests.

### Task 4.4 — API, consumer, and operator quality

1. [x] Fixed Unicode-safe notifier truncation.
2. [x] Used typed outcomes and public gate kinds.
3. [x] Surfaced policy version, advisory/authoritative mode, uncertainty, conditions, concerns, and escalation evidence through the public contract.
4. [x] Added direct, IPC, and HTTP ledger verification/export workflows plus low-cardinality gate and telemetry counters.
5. [x] Reconciled canonical docs only after all behavior, strict quality, and direct-consumer gates passed.

Closeout evidence:

- Mandos all features: 75 unit + 2 integration tests passed.
- Mandos no default features: 68 unit + 2 integration tests passed.
- Rustfmt, strict Clippy for both feature modes, and warning-denied Rustdoc passed.
- `arda-aule` all-feature tests/check passed: 164 library, 8 CLI, 14 focused integration, and 2 doc tests.
- `arda-orome` all-feature tests/check passed: 86 library and 10 integration tests.

---

## Packet 5 — `arda-outpost-protocol`

**Objective:** Close the shared outpost wire contract before changing scout behavior.

**Primary files:**

- `outposts/arda-outpost-protocol/src/**`
- `outposts/arda-outpost-protocol/tests/**`
- `outposts/arda-outpost-protocol/README.md`
- Create canonical status/breakdown/index/ownership documents
- Consumer: `arda-outpost-scout`

Tasks:

1. Classify every protocol type and schema constant.
2. Pin round-trip and malformed-fixture behavior for manifests, dispatches, findings, evidence, chat, health, and authority fields actually present in live source.
3. Require advisory/presentation/execution-prohibited boundaries without expanding authority.
4. Verify scout against the finalized contract.

---

## Packet 6 — `arda-outpost-scout`

**Objective:** Prove bounded, receipted, source-bearing scouting without queue/approval authority and reconcile it with the active Pi5 outpost plan.

**Primary files:**

- `outposts/arda-outpost-scout/src/**`
- `outposts/arda-outpost-scout/tests/**`
- `outposts/arda-outpost-scout/README.md`
- Create canonical status/breakdown/index/ownership documents
- Reconcile: `docs/plans/2026-07-23-pi5-outpost-integration-plan.md`
- Consumers: root daemon harness and Athena/council projection owners discovered live

Tasks:

1. Preserve current user work in `src/memory.rs` and `tests/survey_fixtures.rs`.
2. Verify allowlist, provenance, expiry, advisory authority, bounded model/tool access, and append-only receipt behavior.
3. Prove the root harness consumer and one projection consumer.
4. Update the existing Pi5 plan’s own checklist to live paths (`outposts/**`, not stale `crates/outposts/**`) and actual completion status.

---

## Packet 7 — `arda-launcher`

**Objective:** Replace placeholder Tauri wiring with the real onboarding command surface, exercise existing tests, and establish truthful frontend/backend first-class documentation.

**Primary files:**

- Backend: `apps/arda-launcher/src-tauri/src/lib.rs`
- Backend tests: `apps/arda-launcher/src-tauri/src/onboarding/tests.rs`
- Frontend: `apps/arda-launcher/src/**`
- Build/config: `apps/arda-launcher/{package.json,vite.config.*,src-tauri/tauri.conf.json}`
- Reconcile: app-root `README.md`, `BREAKDOWN.md`
- Create app-root `STATUS.md`, `INDEX.md`, `OWNERSHIP.md` if useful

Tasks:

1. Verify whether onboarding tests are compiled; correct module wiring if they are not.
2. Remove the sample `greet` command only when real typed onboarding commands replace it.
3. Add frontend command-contract tests before wiring onboarding panels.
4. Replace hardcoded `:7171` assumptions only through the coordinated environment/fleet discovery contract; preserve compatibility until all consumers are verified.
5. Run Rust strict gates plus `pnpm` lint/test/build and `pnpm run tauri build` when packaging prerequisites are available.

---

## Packet 8 — root `arda` package

**Objective:** Close the daemon package last as the verified composition root for registry resolution, supervision, harness, outpost discovery, and shutdown.

**Primary files:**

- `src/main.rs`
- `Cargo.toml`
- `services.toml`
- `config/fleet.toml` only if a verified contract repair is required
- Root `README.md`
- Create root package status/breakdown/ownership sections or a focused package document without competing with repository-wide docs

Tasks:

1. Add focused tests around repository-root discovery, Warden URL precedence, registry validation, and `--once` semantics where extraction is needed for testability.
2. Prove required-service failures, `--no-ui`, signal shutdown, harness startup, and no child spawn during `--once`.
3. Compile all direct workspace dependencies and run the maintained daemon smoke.
4. Run workspace-wide check/test/Clippy only after package-local gates are green; classify any unrelated blocker precisely.
5. Reconcile root documentation with the actual composition boundary.

---

## Final workspace closeout

1. Re-run `cargo metadata --no-deps --format-version=1` and account for every package.
2. Run `cargo check --workspace --all-targets --all-features`.
3. Run `cargo test --workspace --all-features` if runtime-state isolation is proven.
4. Run strict Clippy package-by-package, then workspace-wide.
5. Validate Markdown links and exact index coverage for all closed packages.
6. Verify active plans contain only genuine future work and completed crate-local trackers are absent.
7. Compare final status with the initial dirty path snapshot and report task-owned changes separately from preserved work.
8. Do not commit or push unless explicitly requested.
