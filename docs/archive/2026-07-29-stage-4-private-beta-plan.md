# Arda Stage 4 — Private Beta Plan

> **For Hermes:** Stage 4 is complete and operator-accepted as of 2026-07-31. The completed Workbench implementation tranche is archived at `docs/archive/2026-07-29-arda-workbench-private-beta-plan.md`. External evaluator evidence is an optional future confidence signal, not an exit gate.

**Lifecycle status:** COMPLETE and operator-accepted on 2026-07-31.
**Stage objective:** Move Arda from integrated platform alpha to a private beta that one operator can use repeatedly for real software work. Independent evaluator usability evidence remains desirable but non-gating while a separate evaluator and clean machine are unavailable.

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

**Optional validation S4-U1 — non-gating**
If an invited evaluator becomes available, assess whether they can:
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

Use `docs/archive/2026-07-27-warden-varda-ceo-learning-loop.md` plus the Warden application plan.

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

### Native Gate 3 closeout — 2026-07-31

- Native Tauri/WebKitGTK run `run-d84891e8-1d2a-4f5c-a6fb-1ec22d654df4` completed one correlated objective-to-close lifecycle for project `550e8400-e29b-41d4-a716-446655440010` through the HUD. The operator boundary captured the objective, planned the graph, displayed the `$0.00 / 25 J` human approval, recorded approval, and completed execute → verify → review → close.
- The final durable graph has all six nodes (`plan`, `approval`, `execute`, `verify`, `review`, `close`) in `succeeded`; the journal contains sequences 1–23 and ends with `result_projected` for `close`. The close checkpoint is sequence 22 with recovery token `run-d84891e8-1d2a-4f5c-a6fb-1ec22d654df4:close:22`.
- Non-empty review evidence was entered through the native typed boundary and projected durably: `crates/engine/src/harness/runs.rs modified +360 -9`; engine harness `4 passed`; focused HUD Workbench `6 passed`; and the `openai-codex / gpt-5.6-sol` receipt through `hermes-agent` with digest `sha256:0d68e8409e456d8d059a271789e2defba2bf19c946659485ae6a08642b4e45bb`.
- The native process was terminated and relaunched (`1476345` → `1487332`). The fresh AT-SPI application reported `Resumed run run-d84891e8-1d2a-4f5c-a6fb-1ec22d654df4 from the durable harness`, rendered all six succeeded nodes, and restored the same changed-file, test, provider/model/adapter, receipt, timeline, and recovery-token evidence.
- Focused closeout gates passed immediately before the native lifecycle: `cargo test -q -p arda-engine --test harness_runs` (4 passed) and focused Workbench Vitest targets (2 files / 6 tests passed).
- The authoritative native record is `docs/evidence/stage-4-private-beta/native-tauri-workbench-acceptance.json`; durable source artifacts are under `data/runs/run-d84891e8-1d2a-4f5c-a6fb-1ec22d654df4/`.

### Workstream 6 clean-profile closeout — 2026-07-31

- The release launcher was installed into an empty dedicated `HOME` inside a fresh Fedora 44 rootless container userspace. The Arda checkout was mounted read-only, so install, launch, recovery, diagnostics, and uninstall could not mutate source.
- The first dependency-free startup failed on `libgdk-3.so.0`. That exposed a readiness defect: executable presence was treated as sufficient even when native libraries were unresolved. `arda_beta_ops.py` now performs a high-severity `ldd` gate and reports exact missing libraries plus the documented GTK3/WebKitGTK recovery path.
- After installing the documented runtime packages and explicit build/runtime prerequisites, readiness passed all 11 checks with 0 warnings and 0 failures. The native X11 launcher survived the bounded five-second smoke interval; isolated-container AT-SPI/EGL warnings were recorded without being promoted into click-through evidence.
- Backup excluded the secret canary and preserved two durable files with manifest hashes. Safe reset quarantined config, data, cache, runtime, and the newly discovered `arda.launcher` WebKit state. Restore returned settings and run data without restoring the excluded secret.
- Diagnostics omitted both canaries and the real selected-home/source paths. Uninstall removed the binary, desktop entry, and command symlink while preserving restored state.
- The run exposed and fixed a second recovery defect: resolving an existing managed symlink aliased the command path to its target, leaving a broken launcher symlink after uninstall. Fresh-layout regression coverage now proves complete managed uninstall.
- `python3 -m unittest tests.test_arda_beta_ops -v`: 11 passed. The complete machine-readable record is `docs/evidence/stage-4-private-beta/workstream-6-clean-profile-container-run.json`.
- Evidence boundary: this is clean, independently provisioned Linux userspace/profile evidence with native display forwarding, not a full VM, separately logged-in human account, or external-evaluator observation.

### Workbench live-provider and research closeout — 2026-07-31

- Workbench-originated run `run-stage4-workbench-live-provider-accepted` invoked the configured Hermes adapter through `POST /v1/runs/{run_id}/nodes/execute/execute-provider`, not through an external projection script. The live `nous` route selected `stepfun/step-3.7-flash:free`, made 7 API calls, recorded 13,366 input and 1,819 output tokens at an estimated `$0.00`, and returned canonical receipt `sha256:f47667146e3bfb1bb9c436b9376aa680ed49adfb6463dba584f77ccae4010209`.
- The accepted provider receipt records the bounded `patch` that changed only the isolated fixture's `src/lib.rs` from return value 0 to 42. Its artifact digest is `sha256:f9169cc7313db0a63ff22f955048f877e75877462413fd871a88adc7227dc050`; the declared `cargo-test` evidence references the exact `cargo test --quiet` command, exit 0, with output digest `sha256:c2a47a552508021ca51ee42b4e382053b59bc8120d1d8bfb8d7339af84254131`.
- The first live probe exposed an evidence-integrity defect: any successful terminal call could satisfy a declared check ID. The adapter now carries the project check-command map and rejects unrelated terminal evidence. It permits only the exact declared command or that command behind an exact `cd <adapter cwd> &&` wrapper. Focused adapter coverage passes 9 tests, including rejection and safe-wrapper cases.
- Provider failure/restart recovery was exercised separately on recovery run `run-stage4-workbench-live-provider-verified`. After process restart the graph recovered, execution completed, and a second restart restored the succeeded execute node and byte-identical persisted provider receipt. Both recovery responses have SHA-256 `194d25780b5b733f17adcabb93e113f924f82196d5a4643ca07c1e3ce669f5e1`; the receipt timestamp remained `1785532959299`, proving the provider was not invoked twice after success. The accepted mutation run and the recovery run are deliberately identified separately in the machine record.
- Independent closeout review prompted an additional integrity hardening pass: typed receipts can now recompute and verify their canonical digest, persisted provider receipts fail closed on a mismatch, and operator review evidence rejects malformed SHA-256 values, traversal/absolute paths, oversized payloads, excessive item counts, and provider/completion digest mismatches before mutating run state. Focused engine coverage passes 13 tests across the adapter and harness run suites.
- The HUD now invokes this typed provider boundary, displays provider/model/token/cost and test evidence, persists the objective by run ID, and restores it with a resumed run. Focused HUD tests passed 7 tests across 2 files; `npm run build` passed; native Workbench command tests passed 7 tests.
- The complete machine-readable provider record is `docs/evidence/stage-4-private-beta/workbench-live-provider-run.json`. The prior `live-provider-golden-result.json` remains deterministic adapter evidence; it is not substituted for this Workbench-originated run.
- Research-assisted run `stage4-research-20260731T181502Z` asked one explicit Tokio cancellation-safety question. Warden discovery fed canonical Varda fetch/evaluation for two cited sources, disclosed one failed source fetch, preserved both citations at `reference_only`, and linked `research-5642156a5306e462` to the run as `advisory_research_evidence` without changing graph authority.
- The research brief explicitly records `execution_authorized: false`; its event is `evidence_linked`, not an approval or execution transition. Focused research tests passed 3 tests, including bounded excerpts, stable IDs, and private/local target rejection. The live record is `docs/evidence/stage-4-private-beta/research-chain-live-stage4-research-20260731T181502Z.json`.

The HUD retains read-only contract validation before attachment and now carries the typed flow through attach, objective, plan, approval, provider execution, evidence review, close, and durable resume. Validation still shows project identity, runtime adapter, requested authority, network/filesystem posture, and declared command/check identifiers without mutating state.

These focused gates plus the archived Workbench implementation plan establish the canonical contracts, deterministic replay, typed harness/HUD boundary, Rust and Python adapter-backed golden paths, interrupted-run exact-once recovery, native objective-to-review/restart acceptance, clean-profile packaging/onboarding/recovery, a Workbench-originated live-provider execution, and advisory research-assisted evidence. The operator accepts this evidence as the Stage 4 exit. Independent evaluator evidence remains an optional confidence improvement and does not block Stage 5.

- [x] Canonical project and run-graph contracts are versioned and tested.
- [x] Rust and Python deterministic golden paths pass.
- [x] Interrupted run resumes safely without duplicate observable mutation.
- [x] HUD supports objective-to-review without terminal reconstruction.
- [x] Research-assisted run preserves evidence boundaries.
- [x] Clean install/onboarding is reproduced in an isolated Fedora userspace/profile with the source mounted read-only.
- [x] External evaluator evidence is explicitly optional and non-gating under the operator-approved local-only profile.
- [x] No open critical or high-severity issue remains in the bounded deterministic golden path.
- [x] Private beta limitations and non-goals are explicit in this plan.

### Optional future validation

If another evaluator and clean machine become available, run `docs/operator/private-beta-evaluator-guide.md` and capture their independent time-to-orientation, next-decision identification, change/test explanation, stale/failed/waiting/complete distinction, and disposition using `docs/operator/templates/stage-4-invited-evaluator-record.json`. This is supplementary evidence and does not reopen Stage 4 or block Stage 5.

## Stage 4 deferrals

- unrestricted autonomous execution;
- public plugin marketplace;
- mobile applications;
- cloud multi-tenancy;
- autonomous finance or legal commitments;
- camera-based identity or clinical inference;
- Mirromere/RELIC availability as a Workbench dependency.
