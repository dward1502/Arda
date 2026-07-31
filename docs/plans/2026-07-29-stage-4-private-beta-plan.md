# Arda Stage 4 — Private Beta Plan

> **For Hermes:** Execute this plan through the linked Workbench plan. Stage 4 is an evidence gate, not a documentation label. Do not mark a criterion complete without a reproducible artifact or test.

**Stage objective:** Move Arda from integrated platform alpha to a private beta that one operator can use repeatedly for real software work and one invited evaluator can understand without reconstructing the architecture.

**Primary product:** Arda Workbench  
**Required application dependencies:** Warden Research for bounded evidence; HUD/Launcher for operation  
**Non-gating incubators:** Personal Operations, Mirromere, RELIC/CITADEL, Company Operations

---

## Exit definition

Stage 4 is complete when Arda repeatedly performs this vertical slice:

```text
clean install -> attach repository -> objective -> proposed run graph
              -> bounded approval -> edit -> native tests -> review
              -> restart/resume -> accept or reject -> receipts
```

It must work on:
1. Arda itself or another representative Rust repository;
2. one Python repository through the polyglot project contract;
3. one deliberately failing/recovering run.

## Stage 4 principles

- Product outcome outranks package count.
- One coherent run ID crosses routing, tools, tests, costs, evidence, and approvals.
- Existing receipts are integrated, not replaced by a second event system.
- The UI shows real state and explicit unknown/stale states.
- Every mutation remains bounded, reviewable, and attributable.
- Private beta may be local-machine-only; remote/cloud operation is not required.

## Workstream 1 — Canonical contracts

**Owner surfaces**
- `crates/spine/governance/arda-core/`
- `crates/spine/contract/arda-contract-registry/`
- `spec/project-contract/`
- `spec/run-graph/`

**Deliverables**
- `arda.project-contract.v1` and JSON Schema;
- run graph/node/edge/checkpoint contracts;
- authority and approval requirement encoding;
- stable IDs and append-only lifecycle events;
- version compatibility and migration policy;
- contract fixtures for Rust and Python.

**Gate S4-C1**
- Registry rejects incompatible or undeclared versions.
- Fixed-event replay produces the same run projection.
- Contract fixtures validate in Rust and Python.

## Workstream 2 — Durable Workbench engine

**Owner surfaces**
- `crates/engine/src/`
- existing Mandos, Manwë, Oromë, Aulë, Varda, Vairë, economics, and tool contracts

**Deliverables**
- project registry and command discovery;
- run planner and lifecycle transition reducer;
- bounded tool adapter;
- checkpoint/resume;
- cancellation, timeout, retry, and compensation states;
- one receipt correlation model across all services;
- read APIs and event stream for HUD.

**Gate S4-E1**
- Kill the process during a test run; restart resumes or safely marks intervention without duplicating an edit or external action.
- Failed tests become graph evidence and a review/revision state, not a false completion.
- Unknown service/provider state blocks unsafe continuation.

## Workstream 3 — Workbench HUD

**Owner surfaces**
- `apps/arda-hud/src/`
- `apps/arda-hud/src-tauri/`

**Deliverables**
- project attach/import;
- objective composer;
- graph with state, authority, and critical path;
- approval panel with proposed commands/files/network scope;
- diff and test evidence;
- route/model/tool/cost provenance;
- failure and resume explanations;
- acceptance/rejection/revision closure.

**Gate S4-U1**
An invited evaluator can:
- identify current state in under 30 seconds;
- find the next required human decision;
- explain what Arda changed and how it verified the result;
- distinguish stale, failed, waiting, and complete.

## Workstream 4 — Polyglot adapter proof

**Deliverables**
- JSONL-over-stdio Python reference adapter;
- project-owned commands, not hardcoded ecosystem guesses;
- capability and environment negotiation;
- cancellation and timeout;
- stdout/stderr/artifact receipt framing;
- no secret values in project contracts or telemetry.

**Gate S4-P1**
- Attach a Python repository without modifying Arda source.
- Execute its declared test command and collect structured evidence.
- Adapter crash does not crash the kernel or lose run truth.

## Workstream 5 — Research support

Use `docs/plans/2026-07-27-warden-varda-ceo-learning-loop.md` plus the Warden application plan.

**Stage 4 minimum**
- explicit question only; recurring autonomous watchlists may remain experimental;
- Warden preview → Varda fetch/evaluation → cited brief;
- research output can inform a Workbench plan but cannot authorize execution.

**Gate S4-R1**
- One objective requests missing documentation evidence and produces a cited, contradiction-aware brief linked to the run.

## Workstream 6 — Packaging, onboarding, and recovery

**Owner surfaces**
- `apps/arda-launcher/`
- root `package.json` / Tauri config
- `scripts/`
- operator docs

**Deliverables**
- clean workstation install path;
- launcher readiness checks and honest degraded state;
- default local model/provider configuration or explicit setup path;
- backup/restore for beta state;
- reset/recovery without deleting source repositories;
- uninstall documentation;
- diagnostics bundle with redaction.

**Gate S4-O1**
- Test from a clean local user profile or clean VM.
- No undocumented manual state edits.
- Failed service startup yields a useful diagnostic and recovery path.

## Workstream 7 — Safety and observability

**Deliverables**
- tool/path/network scope visible before approval;
- process-tree cancellation;
- secret redaction tests;
- bounded logs/events/state growth;
- health/freshness for all golden-path dependencies;
- provider/model route and cost receipts;
- prompt-injection treatment for repository and web content.

**Gate S4-S1**
- Negative tests cover traversal, undeclared command, secret-like output, stale approval, repeated event, adapter protocol corruption, and interrupted mutation.

## Required dogfood runs

1. Documentation-only bounded edit.
2. Rust code change with focused tests.
3. Python code change through adapter.
4. Intentionally failing test and revision.
5. Mid-run restart.
6. Provider unavailable/degraded path.
7. Operator rejection before mutation.
8. Warden/Varda research-assisted plan.

Each run records setup, objective, graph, approvals, diff, commands, tests, timings, cost/resource evidence, faults, and operator assessment.

## Stage 4 verification command set

```bash
cargo metadata --no-deps --format-version=1
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-features -- --test-threads=1
cargo clippy --workspace --all-targets --all-features -- -D warnings
pnpm --dir apps/arda-hud test
pnpm --dir apps/arda-hud lint
pnpm --dir apps/arda-hud build
pnpm --dir apps/arda-launcher test
pnpm --dir apps/arda-launcher lint
pnpm --dir apps/arda-launcher build
python3 scripts/hades_markdown_link_check.py --root docs --out /tmp/arda-stage4-links.json
```

Workspace-wide gates run only after focused package/app tests pass and runtime-state isolation is proven.

## Stage 4 evidence packet

Create `docs/releases/stage-4-private-beta-evidence.md` only when gates are run. Include:
- environment and commit/worktree identity;
- clean-install evidence;
- package/app gate output;
- golden-path run IDs;
- restart/recovery evidence;
- Python adapter evidence;
- known limitations and explicit non-goals;
- evaluator findings and disposition.

## Exit criteria

### Current implementation evidence — 2026-07-30

- `cargo test -p arda-core --test project_contract -- --test-threads=1`: 8 passed.
- `cargo test -p arda-core --test run_graph`: 4 passed.
- `cargo test -p arda-engine --test run_recovery`: 3 passed, including corrupt-tail failure and restart idempotency recovery.
- `pnpm --dir apps/arda-hud exec vitest run src/components/arda/modules/WorkbenchModule.test.tsx`: 3 passed, including native-boundary contract validation projection.
- `cargo test --manifest-path apps/arda-hud/src-tauri/Cargo.toml`: 13 passed, including valid-contract projection and missing-contract fail-closed coverage.
- `cargo check --workspace --all-targets --all-features`: passed.
- `cargo test --workspace --all-features -- --test-threads=1`: passed.
- `cargo clippy -p arda-core -p arda-engine --all-targets --all-features -- -D warnings`: passed.
- `pnpm --dir apps/arda-hud test`: 71 files / 261 tests passed (existing React `act(...)` warnings remain non-fatal).
- `pnpm --dir apps/arda-hud run build`: passed.
- `pnpm --dir apps/arda-hud run tauri dev`: passed startup; the native HUD launched and its Vite development surface returned HTTP 200 on `http://localhost:1421`.
- `pnpm --dir apps/arda-hud lint`: unavailable because `apps/arda-hud/package.json` defines no `lint` script.
- `python3 scripts/hades_markdown_link_check.py --root docs --out /tmp/arda-stage4-links.json`: 36 local links checked, 0 broken.
- `cargo test -p arda-core --test personal_ops -- --test-threads=1`: 6 passed; the parallel Personal Operations capture/reminder contract is preserved without activating its service or UI.
- Mirromere/RELIC/CITADEL provenance is recorded in `docs/research/2026-07-30-mirromere-relic-provenance-audit.md`; no external source was migrated and no external-product spike was authorized.

### Bounded Workbench tranche evidence — 2026-07-31

- Contract-registry commit `dae505c` declares `arda.project-contract.v1` and `arda.run-graph.v1`; undeclared versions fail closed and registry smoke tests pin both declarations.
- Workbench API commit `2470832` adds the typed project/run harness routes without absorbing the concurrent presence-harness work; launcher follow-up `0a59210` removes the stale four-track assumption exposed by the registry addition.
- `cargo test -p arda-contract-registry -- --test-threads=1`: 8 passed.
- `cargo test -p arda-core --test project_contract --test run_graph -- --test-threads=1`: 12 passed.
- `cargo test -p arda-engine --test run_recovery --test harness_projects --test harness_runs -- --test-threads=1`: 7 passed.
- `cargo test -p arda-engine -- --test-threads=1`: 26 passed across unit, presence, project/run harness, Oromë smoke, and recovery targets.
- `cargo check --workspace --all-targets --all-features`: passed. Concurrent uncommitted presence-harness work emits unused/dead-code warnings and remains outside the Workbench API commit boundary.
- `cargo test --workspace --all-features -- --test-threads=1`: passed after repairing the launcher registry-count regression; all workspace unit, integration, and doc-test targets completed successfully (with the same unrelated presence-harness warnings).
- Task 1.2's eight typed project/run endpoints are now present on the existing harness. Mutations are loopback-only, require canonical Oromë `TaskApprovalEnvelope` plus idempotency, serialize concurrent state writes, and reject arbitrary browser shell fields.
- The first workspace-wide test attempt exposed a stale launcher assertion that hard-coded four registry tracks after the Workbench track became the fifth. The repaired assertion derives the count from the returned checks and explicitly verifies the Workbench track; its focused test and the final full-workspace rerun passed.
- S4-C1 closeout adds fixed project-contract, run-graph, run-event journal, and expected-projection fixtures under `spec/`, plus a Draft 2020-12 run-graph schema.
- `cargo test -p arda-core --test project_contract --test run_graph -- --test-threads=1`: 16 passed; canonical Rust parsing accepts both valid fixtures and rejects their fixed incompatible-version and invalid counterparts.
- `uv run --with jsonschema python -m unittest tests/test_workbench_contract_fixtures.py -v`: 2 passed; Python independently applies the JSON Schemas to those exact fixtures.
- `cargo test -p arda-engine --test run_recovery -- --test-threads=1`: 6 passed; the fixed nine-event journal replays twice to the exact expected projection with byte-identical serialization, while malformed, truncated, sequence-gap, and invalid transition inputs fail closed without mutating the base projection.
- Task 2.1 adds the versioned `arda.project-adapter.v1` JSONL-over-stdio protocol and Draft 2020-12 message schema, a stdlib-only Python reference server, and a one-request Rust process boundary. MCP remains bridgeable rather than mandatory.
- `uv run --with jsonschema python -m unittest sdk/python/tests/test_conformance.py -v`: 6 passed; every valid conformance request and response is independently checked against the message schema, including capability denial, progress, provenance, recovery-token round trip, and cooperative cancellation.
- `cargo test -p arda-engine --test project_adapter_jsonl -- --test-threads=1`: 5 passed; absolute executable, canonical project-root/cwd containment, cleared environment plus explicit allowlist, process timeout, cancellation, child reaping, and fail-closed rejection of unevaluated adapter fields are proven against the Python reference process.
- `cargo test -p arda-engine --all-features -- --test-threads=1`: 34 passed across engine unit, presence, project/run harness, Oromë smoke, adapter-boundary, and recovery targets. Existing concurrent presence-harness warnings remain outside Task 2.1.
- `cargo check -p arda-engine --all-targets --all-features`: passed with the same unrelated presence-harness warnings.

The HUD now exposes a read-only `validate_project_contract` Tauri command backed by the canonical `arda-core` parser. Before attachment it shows project identity, runtime adapter, requested authority, network/filesystem posture, and declared command/check identifiers, while explicitly stating that validation did not attach the project or start a command. This is the first native Workbench boundary, not a complete attachment flow.

These focused gates establish contract, deterministic replay, recovery-store, typed harness API, native validation, and honest HUD-draft foundations. S4-C1 is closed: registry and canonical Rust version enforcement, independently reproduced Python schema validation, exact fixed-event replay, repeatability, and fail-closed corrupt-journal behavior are proven. They do **not** satisfy the full Stage 4 exit definition: no native HUD attach/run invocation, real adapter-backed edit/test/review flow, Rust/Python adapter golden run, clean-install reproduction, or invited evaluator evidence exists yet.

- [x] Canonical project and run-graph contracts are versioned and tested.
- [ ] Rust and Python golden paths pass.
- [ ] Interrupted run resumes safely.
- [ ] HUD supports objective-to-review without terminal reconstruction.
- [ ] Research-assisted run preserves evidence boundaries.
- [ ] Clean install/onboarding is reproduced.
- [ ] One external evaluator completes a guided run.
- [ ] No open critical safety/data-loss issue.
- [x] Private beta limitations and non-goals are explicit in this plan.

## Stage 4 deferrals

- unrestricted autonomous execution;
- public plugin marketplace;
- mobile applications;
- cloud multi-tenancy;
- autonomous finance or legal commitments;
- camera-based identity or clinical inference;
- Mirromere/RELIC availability as a Workbench dependency.
