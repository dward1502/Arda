# Arda System Unification and Usability Plan

> **For Hermes:** Execute this plan as a scope-frozen integration and release
> program subordinate to the [Arda 1.0 Personal Agent Ecosystem master plan](2026-08-08-arda-1.0-personal-agent-ecosystem-plan.md).
> Use existing domain plans as implementation authorities; do not create new
> agents, applications, queues, memory systems, policy layers, or speculative
> capability domains.

**Status:** Archived 2026-08-12 — U0 through U5 and the bounded convergence
implementation are historical evidence; unresolved final-1.0 qualification was
deferred rather than treated as active 0.9 work.
**Adopted:** 2026-08-02
**Last reconciled:** 2026-08-12
**Owner:** Arda system/runtime maintainers
**Goal:** Finish and compose the release-critical capabilities assigned here,
then qualify them as a functional, supportable distribution without replacing
the master plan's broader product identity or optional-capability boundaries.

**Architecture:** The root `arda` daemon, `services.toml`, `arda-engine`, Manwë, the launcher, and the HUD form the canonical runtime and operator path. Existing domain crates remain the owners of their contracts and state; this plan coordinates composition, end-to-end proof, simplification, release readiness, and plan closeout rather than cloning domain work.

**Primary release authorities:**

- [Arda 1.0 Personal Agent Ecosystem master convergence plan](2026-08-08-arda-1.0-personal-agent-ecosystem-plan.md) — product scope and required vertical proofs
- [Stage 5 — Release Candidate](2026-07-29-stage-5-release-candidate-plan.md)
- [Stage 6 — Legitimate 1.0](2026-07-29-stage-6-legitimate-1.0-plan.md)
- [HUD Frontend–Backend Contract Convergence](2026-08-06-hud-frontend-backend-contract-convergence-plan.md)
- [Arda Product Plan Suite](ARDA_PRODUCT_PLAN_SUITE.md)

---

## 1. Binding scope lock

The feature and subsystem inventory already present in `docs/`, `docs/plans/`, the root workspace, and checked-in application/outpost plans is the upper bound of Arda's current build-out.

### Allowed work

A change is admissible only when it does at least one of the following:

1. completes an existing unchecked plan requirement;
2. composes an implemented capability into the canonical runtime;
3. closes a missing operator-reachable path;
4. fixes a correctness, security, privacy, accessibility, recovery, or performance defect;
5. removes duplication, a parallel authority, stale state, or dead runtime code;
6. improves installation, configuration, diagnostics, documentation, supportability, or user comprehension;
7. replaces a weaker implementation with a simpler or better-tested implementation while preserving the existing product promise.

### Disallowed work

- No new named agent, application, subsystem, control plane, queue, memory authority, policy authority, receipt universe, or parallel runtime.
- No new default-enabled external product or dependency without a measured requirement from an existing plan and an explicit adopt/reject gate.
- No optional preview may become a Workbench release blocker.
- No architecture work may be declared complete from workspace membership, compilation, or documentation alone.
- No post-1.0 feature roadmap is created by this plan. After closeout, work is defect-driven and evidence-driven improvement of the shipped system.

### Scope-admission test

Before starting a task, record:

- the existing plan checkbox, defect, usability failure, or operational gate it closes;
- the current owner of the affected contract/state;
- the operator-reachable behavior that improves;
- the focused test or receipt that will prove the improvement;
- what duplicate or obsolete surface can be removed, if any.

If none can be identified, the task is out of scope.

---

## 2. Authority and non-duplication rules

| Domain | Existing authority | This plan's role |
|---|---|---|
| Workbench release candidate | `2026-07-29-stage-5-release-candidate-plan.md` | Coordinate the remaining signed-artifact, soak, and independent-evaluator gates; do not duplicate them. |
| Arda 1.0 product convergence | `2026-08-08-arda-1.0-personal-agent-ecosystem-plan.md` | Consume required vertical proofs and optional-capability boundaries; do not redefine product identity from this release slice. |
| Stage 6 release qualification | `2026-07-29-stage-6-legitimate-1.0-plan.md` | Enter only after Stage 5 closes; preserve its artifact, compatibility, support, and release evidence requirements. |
| Product/application classification | `docs/plans/ARDA_PRODUCT_PLAN_SUITE.md` and application plans | Keep Workbench release-critical while classifying optional applications and distributions honestly. |
| Root process composition | `src/main.rs`, `services.toml`, `crates/engine/src/registry.rs`, `crates/engine/src/supervisor.rs` | Establish one canonical startup, health, shutdown, and recovery path. |
| Routing/runtime convergence | `../archive/MANWE_SINGLE_RUNTIME_CONVERGENCE.md` | Consume the completed result; do not create another routing runtime. |
| Contracts and governance | Existing spine contract/governance crates | Reuse typed contracts and verdicts; do not add parallel policy or receipt formats. |
| Memory | `arda-vaire` and its active plans | Verify storage, retrieval, and behavioral use without creating another memory store. |
| Audit/evidence | `arda-rumil` and its live crate docs/tests | Use bounded audit evidence; do not reopen the archived Rúmil implementation plan. |
| Operator UI | `apps/arda-launcher`, `apps/arda-hud` and their plans | Present live runtime truth and recovery actions; do not invent backend authority in UI code. |

When two active plans claim the same deliverable, assign it to one authority and reduce the other to a dependency link before implementation.

---

## 3. Maturity model

Every release-critical capability must be classified independently at these levels:

1. **implemented and tested** — source and focused tests exist;
2. **active compile closure** — enabled by the supported feature/profile;
3. **root-composed** — constructed, supervised, or reached by the canonical runtime;
4. **operator-reachable** — available through the supported CLI, API, launcher, or HUD;
5. **workflow-proven** — exercised inside an end-to-end operational path;
6. **failure-proven** — restart, timeout, denial, malformed state, and degraded dependencies are tested;
7. **release-supported** — installed artifact, documentation, diagnostics, compatibility, and support boundaries are complete.

A capability is not called operational merely because its crate compiles or its unit tests pass.

---

## 4. Track A — unify the working system

### U0 — Reconcile the bounded plan estate

**Objective:** Turn the active plan set into a finite, non-overlapping completion ledger.

**Files:**

- Modify: `docs/plans/ARDA_PRODUCT_PLAN_SUITE.md`
- Modify as evidence warrants: `docs/plans/*.md`
- Modify on closeout: `docs/archive/README.md`

**Work:**

- [x] Classify every active plan as release-critical, optional but bounded, dependency-blocked, or stale/superseded.
- [x] Map every unchecked requirement to exactly one implementation authority.
- [x] Remove stale duplicate requirements and repair links without deleting unique acceptance criteria.
- [x] Keep optional applications out of the Stage 5/6 critical path.
- [x] Archive each completed plan immediately after its live gates pass.
- [x] Record the finite Stage 5 blockers as the first release-critical queue. The
  valid uninterrupted 24-hour reliability receipt closed on 2026-08-07; the
  remaining blockers are final signed-artifact reconciliation/lifecycle and one
  qualifying independent non-author evaluator receipt. Security closed through
  the bounded, checksum-pinned `glib 0.18.5` upstream backport on 2026-08-05.

**U0 closeout evidence (2026-08-04):** `docs/plans/ARDA_PRODUCT_PLAN_SUITE.md`
now contains the finite active-plan ownership ledger, including plan-local open
counts/acceptance surfaces and the exact Stage 5 blocker queue. The fully
accepted Warden Research plan moved to `docs/archive/`; stale OpenFang and
Platform OS architecture queues moved to historical records after their unique
remaining boundaries were assigned to current authorities. Optional products
remain outside the Workbench Stage 5/6 critical path.

**Gate U0:** Every open plan item has one owner, one acceptance surface, and one release classification; no completed plan remains in `docs/plans/`.

**P0.2 authority reconciliation — 2026-08-08:** U0 remains the release-plan
estate result, while the new master plan owns product convergence. The active
directory contains eight Markdown authorities: six plans with unchecked work,
one active HUD convergence plan whose prose phases/gates are unresolved, and
one role-aware portfolio index. No completed implementation plan was reopened
or retained merely to rewrite historical evidence.

### U1 — Establish one canonical runtime topology

**Objective:** Make one root invocation own the supported Arda runtime topology and expose truthful service health.

**Files:**

- Modify: `services.toml`
- Modify: `src/main.rs`
- Modify: `crates/engine/src/registry.rs`
- Modify: `crates/engine/src/supervisor.rs`
- Modify: `tests/root_daemon.rs`
- Coordinate with: `docs/archive/MANWE_SINGLE_RUNTIME_CONVERGENCE.md` (completed U1 authority)

**TDD sequence:**

1. Add or tighten a failing `tests/root_daemon.rs` fixture for the exact supported service set, required/optional policy, health state, and clean shutdown.
2. Run `cargo test --test root_daemon -- --test-threads=1` and retain the expected failure.
3. Make the smallest registry/supervisor/root-daemon change.
4. Re-run the focused test and the Manwë process/integration smoke named by its convergence plan.
5. Remove only the parallel runtime/config path whose parity and consumer independence are proven.

**Work:**

- [x] Declare the canonical root command and supported headless/UI profiles.
- [x] Reconcile repository supervision with user-systemd declarations so one process owner exists per service.
- [x] Complete Manwë single-runtime convergence without changing the coordinated `:7171` contract.
- [x] Expose required, optional, starting, healthy, degraded, restarting, and stopped states through one operator-readable status surface.
- [x] Prove bounded restart backoff, clean shutdown, and no duplicate service ownership.
- [x] Remove dual-runtime language and stale launch instructions after cutover evidence passes.

**Closeout evidence (2026-08-04):**

- `arda --no-ui` is the canonical headless invocation; the former `--harness-only` parallel-ownership profile is rejected by a CLI regression test.
- `services.toml` declares required Manwë ownership under the root daemon on the coordinated `:7171` contract.
- `/v1/status` reports requirement, lifecycle state, PID, restart count, bounded backoff, and readiness/failure detail for every resolved service.
- `config/systemd/arda.service` owns the root process and aliases `arda-manwe.service`; it does not define a second Manwë process owner.
- Root and supervisor tests prove required-service failure, readiness state, bounded restart, signal shutdown, and child reaping.
- Manwë has one unconditional governed executable path; static-only modules and the gRPC process path are retired.
- Manwë admission control reserves interactive capacity under explicitly marked bulk execution/background load; the saturation regression proves an interactive request is still admitted.
- Focused gates pass: root integration `5/5`; `arda-engine` `25` unit tests plus integration suites; Manwë `281` library tests plus `3` binary tests; single-process smoke; documentation validation.
- Installing the new binary/unit over the currently running legacy user session remains U4 installation/recovery scope, not a second supported topology.

**Gate U1:** One documented command starts the supported topology; every required service becomes healthy or produces a precise blocking state; shutdown leaves no owned child process; no service is simultaneously owned by two supervisors.

### U2 — Prove the canonical Workbench operational loop

**Objective:** Carry one real request through the complete governed system using the installed/supported runtime rather than isolated crate fixtures alone.

**Primary files:**

- Modify: `crates/engine/src/harness.rs`
- Modify as required: `crates/engine/src/harness/runs.rs`
- Modify as required: `crates/engine/src/runs/executor.rs`
- Modify as required: `crates/engine/src/runs/recovery.rs`
- Modify as required: `apps/arda-hud/src/`
- Test: `crates/engine/tests/workbench_rust_golden.rs`
- Test: `crates/engine/tests/workbench_python_golden.rs`
- Test: `crates/engine/tests/workbench_javascript_golden.rs`
- Test: `crates/engine/tests/workbench_boundary_recovery.rs`

**Required chain:**

`objective → evidence/context → plan/run graph → authority preview → approval or rejection → bounded execution → project-native verification → receipt/evidence projection → restart-safe terminal state`

**Work:**

- [x] Pin one Rust and one Python golden workflow through the root-composed runtime.
- [x] Add the JavaScript/TypeScript workflow already required by Stage 5/6 adapter scope without creating another execution authority.
- [x] Prove accepted, rejected, revised, cancelled, failed, and resumed outcomes.
- [x] Correlate audit, policy, execution, verification, memory, and HUD projections with one stable lineage identifier.
- [x] Ensure every HUD claim is backed by live receipt/state and visibly distinguishes stale, partial, unavailable, and failed evidence.
- [x] Prove that no audit, evaluation, memory, or UI projection can authorize execution.

**U2 closeout evidence (2026-08-04):**

- `crates/engine/tests/harness_runs.rs` drives the canonical harness routes for
  plan, approval, operator completion, cancellation, SSE projection, durable
  reads, and failed verification. `complete_run_node` now makes a `verify` node
  terminally `failed` when project-native check evidence is absent, not passed,
  or still running; downstream review therefore cannot report false success.
- `workbench_rust_golden.rs`, `workbench_python_golden.rs`, and the added
  `workbench_javascript_golden.rs` all use the canonical project contract,
  run-graph, run-store, receipt, and bounded adapter owners. The JavaScript
  fixture imports `sdk/javascript`, fails closed before mutation on mismatched
  arguments, performs one idempotent mutation, runs `node --test`, and reaches a
  restart-safe closed graph. No language adapter introduces an execution or
  approval authority.
- `workbench_boundary_recovery.rs`, `run_recovery.rs`, and
  `project_adapter_jsonl.rs` cover every graph-boundary restart, duplicate
  idempotency keys, corrupt/truncated state, partial/noisy adapter output,
  timeout, cancellation, process reaping, and exact-once mutation.
- `WorkbenchModule.tsx` now persists the proposal and approval lineage needed
  after native restart, wires rejection to canonical run cancellation before a
  revised objective can be planned, and keeps the `run_id` on every graph,
  receipt, evidence, and event projection. `RunTimeline.tsx` labels live,
  partial, stale, and unavailable stream states explicitly; failed nodes and
  failed project checks remain visible from canonical engine state.
- Authority remains in `arda-core` run-graph transitions and the engine harness:
  HUD code can request approval/cancellation but cannot transition a node or
  manufacture a receipt. Audit, policy, memory, and evaluation data remain
  provenance/evidence fields consumed by that graph, not execution gates.
- Required gates passed: `cargo fmt --all -- --check`; `cargo test -p arda-core
  -p arda-engine` (including all three language goldens and recovery suites);
  JavaScript SDK `node --test` (`4/4`); HUD `pnpm test` (`395/395`), `pnpm run
  build`, and `pnpm run lint` (zero errors, existing warnings only); the scoped
  documentation link check (`63` local links, `0` broken), release-ops help, and
  reliability-soak help gates also pass. The Tauri
  release build compiled the release binary and produced DEB/RPM candidates;
  the pre-existing linuxdeploy AppImage wrapper failure remains owned by U4 and
  Stage 5 packaging rather than creating a second U2 runtime path.
- Supplementary repository-wide checks remain honest baseline findings rather
  than U2 evidence: strict Clippy is blocked in the existing
  `arda-outpost-protocol::watchlist::new` argument-count lint. This U2 change
  introduces no Clippy finding; resolving that dependency-owned lint is outside
  the Workbench operational-loop boundary.

**Gate U2:** The installed candidate completes and explains all three supported adapter workflows, survives restart, and never reports success without project-native verification evidence.

### U3 — Close failure, observability, and recovery gaps

**Objective:** Make the unified workflow diagnosable and safe under realistic degradation.

**Files:**

- Modify: `crates/engine/src/observability.rs`
- Modify as required: existing domain receipt/telemetry projections
- Modify: `scripts/stage5_reliability_soak.py`
- Update evidence: `docs/evidence/stage-5-release-candidate/reliability/`
- Update: `docs/releases/stage-5-release-candidate-evidence.md`

**Work:**

- [x] Correlate semantic receipts with trace/span or equivalent runtime lineage IDs.
- [x] Define measured budgets for startup, idle resource use, UI latency, event projection, recovery, diagnostics, and bounded state growth.
- [x] Exercise provider loss, network loss, process kill, disk pressure, malformed/truncated state, model timeout, adapter crash/noisy output, cancellation, and operator rejection.
- [x] Preserve enough bounded diagnostics to assign every nonzero soak exit to a scenario and root cause.
- [x] Run one uninterrupted valid 24-hour Stage 5 matrix after capacity preflight.
- [x] Prove no silent mutation, false completion, lost terminal state, duplicate mutation, or unbounded state growth.

**U3 closeout evidence (reconciled 2026-08-10):**

- `crates/engine/src/observability.rs` now defines a machine-readable
  `RuntimeLineage`: the canonical run ID is the trace-equivalent lineage, and
  `node_id:event_sequence` is the span-equivalent position. The same module
  owns finite startup, idle RSS, UI interaction, event projection, recovery,
  diagnostic-bundle, and protected-state-growth budgets. The SSE regression in
  `crates/engine/tests/harness_runs.rs` also enforces the one-second event
  projection budget.
- The Stage 5 runner now rotates eleven exact scenarios, including explicit
  provider loss, external process kill/restart, adapter crash, and durable
  operator rejection. A successful Cargo exit that selected zero tests is now
  a typed `test_not_exercised` failure rather than a false pass.
- Every nonzero exit records its scenario, exact command, duration, output
  digest, explicit root-cause class, and a bounded redacted tail. Local roots,
  bearer credentials, API keys, tokens, passwords, secrets, and prompts are
  redacted; only the newest 20 failures and at most 4,096 bytes per diagnostic
  are retained.
- `u3-degradation-smoke-20260804.json` is a valid 11/11 all-scenario receipt:
  zero failures, unchanged source identity, zero protected-state growth, the
  64-GiB capacity floor preserved, and all command-latency ceilings preserved.
  `cargo test -p arda-engine` passes the full engine suite, including the new
  lineage, budget, provider-loss, process-kill, adapter-crash, rejection, and
  event-projection regressions. The reliability evaluator passes 11/11 unit
  tests.
- The earlier eight-scenario `20260804-v2` run was stopped without a receipt
  because it could not exercise the complete U3 matrix. Subsequent failed runs
  remain retained as failed evidence and are not reinterpreted. The final valid
  run from clean commit `efd118b5` completed 86,400 seconds on 2026-08-07 and
  passed 2,844/2,844 executions across all eleven scenarios with unchanged
  source identity, zero protected-state growth, every latency budget held, and
  the 64-GiB floor preserved. Its receipt and assessment are
  `docs/evidence/stage-5-release-candidate/reliability/soak-24h-final-efd118b5-20260807.json`
  and the adjacent `-assessment.md`. U3 and S5-R1 are closed. The later stopped
  `6616addd` elapsed run emitted no receipt and is not evidence; a new
  exact-candidate run is needed only if policy at the next actual release freeze
  requires it.

**Gate U3:** Stage 5 S5-R1 closes from a valid receipt, and every seeded failure is attributable from redacted diagnostics without source, prompts, secrets, or private state.

---

## 5. Track B — make the unified system usable by others

### U4 — One installation and first-run path

**Objective:** A supported user can install, configure, start, diagnose, recover, and remove Arda without repository knowledge.

**Files:**

- Modify: `apps/arda-launcher/src-tauri/src/onboarding/`
- Modify: `apps/arda-launcher/src-tauri/src/lib.rs`
- Modify: `scripts/arda_release_ops.py`
- Modify: `scripts/arda_beta_ops.py`
- Update: `docs/operator/private-beta-install-recovery.md`
- Update: Stage 5 packaging and support evidence

**Work:**

- [x] Consolidate detection, prerequisites, provider setup, service plan, readiness, and guided setup into one visible first-run sequence.
- [x] Fail unsupported profiles before partial installation.
- [x] Keep secret writes and consequential configuration changes explicitly approved and receipted.
- [x] Provide clear offline, provider-unavailable, degraded, and recovery guidance.
- [x] Prove install, upgrade, rollback, backup, restore, safe reset, and uninstall using final signed artifacts.
- [x] Ensure optional applications remain opt-in and cannot prevent Workbench startup.

**U4 execution evidence (2026-08-04):**

- `first_run_status` now returns one `arda.launcher-first-run.v1` projection from
  `apps/arda-launcher/src-tauri/src/onboarding/first_run.rs`. The launcher renders
  its six ordered phases, the exact supported-profile decision, prerequisite and
  provider state, approval-only mutation boundary, actionable diagnostics,
  guided setup, degradation/recovery guidance, and optional-service status.
  Native AT-SPI inspection confirmed all six phases and the complete recovery
  surface in the packaged UI. Optional product applications are absent from the
  required Workbench startup registry; engine tests prove a missing optional root
  service is dropped without blocking required services.
- Supported-profile detection is shared by first run and the private-beta
  lifecycle boundary. Unsupported fixtures fail before installation in
  `tests.test_arda_beta_ops`; launcher unit tests prove the corresponding first-run
  gate. Consequential onboarding steps remain human-gated and the existing
  approval/receipt regressions pass.
- `u4-lifecycle-local-20260804.json` proves fresh install, default native launch,
  upgrade, backup, diagnostics, rollback, terminal-run persistence, uninstall,
  post-uninstall state preservation, and unchanged source identity using an
  isolated Bluefin LTS operator home and the current locally built candidate.
  The default Wayland launch originally exited with protocol error 71; the
  launcher now applies the narrow NVIDIA/Wayland explicit-sync guard only when
  that host combination is detected and no operator override exists. Default
  launch then survived the native probe without an override.
- Frontend tests pass 8/8, launcher Rust tests pass 14/14, launcher Clippy passes
  with warnings denied, beta/release operations pass 20/20, engine optional-
  service isolation passes 5/5, and Tauri produces the current DEB and RPM.
  Tauri AppImage wrapping now passes through the normal package entry point with
  linuxdeploy's supported `NO_STRIP=true` control, avoiding its bundled old
  `strip` that rejects Bluefin LTS `.relr.dyn` sections. This does not substitute
  an unsigned local package for the final signed-artifact gate.
- The final signed-artifact checkbox and Gate U4 remain open. The published
  `v0.3.0-rc.0` checksum ledger and all six Sigstore bundles verify, but the
  release tag and manifest bind to source `28cde28b`, seven commits behind frozen
  final source `efd118b5`, and the manifest records
  `tracked_worktree_clean=false`. The fail-closed reconciliation receipt is
  `docs/evidence/stage-5-release-candidate/packaging/remote-signed-artifact-reconciliation-20260806.json`.
  U4 closes only after exact signed bytes from clean final source pass this
  lifecycle and the clean profile reaches a persisted Workbench change.

**U4/U5 prerequisite audit (2026-08-11):**

- Gates U0 through U3 retain their cited passing receipts; no missing U0-U3
  prerequisite was found in the live gate ledger.
- The U4 harness now accepts an identity-bound Sigstore bundle and fails closed
  when signature verification fails or a signed candidate cannot launch on the
  supported profile without a compatibility override.
- All six `v0.3.0-rc.0` release assets passed their checksum ledger and Sigstore
  identity checks. The exact signed AppImage then passed install, upgrade,
  backup, diagnostics, rollback, state preservation, and uninstall, but failed
  default Wayland startup and required `GDK_BACKEND=x11`. The fail-closed receipt
  is `docs/evidence/stage-5-release-candidate/reliability/u4-signed-v0.3.0-rc.0-lifecycle-20260811.json`.
- Clean source `8a5e3f75` produced all three `0.3.0-rc.1` Linux bundles. The
  fixed-epoch AppImage was byte-identical across two packages at
  `f332e34c39ae82674c002ab4d395522b580e02c91745fe983f62d5a345490bb3`
  and passed default native startup plus the complete isolated lifecycle without
  an override. The receipt is
  `docs/evidence/stage-5-release-candidate/reliability/u4-local-v0.3.0-rc.1-lifecycle-20260811.json`.
  This local unsigned run established the candidate baseline before signing.

**U4 closeout (2026-08-11):** `v0.3.0-rc.1` superseded the unsigned baseline.
The tag and clean manifest bind to source
`8a5e3f75db3867803d56c0b3568ec5fc51794349`; signing run `31543599410` passed
the exact six-asset checksum, identity, sign, verify, and upload gates. A fresh
download contained exactly six artifacts and six detached Sigstore bundles,
and all bundles verified against the tag-bound workflow identity. The
downloaded signed AppImage then passed default native launch, fresh install,
upgrade, backup, diagnostics, rollback/restore, terminal-truth preservation,
uninstall, post-uninstall state preservation, and source immutability without
a compatibility override. The authoritative receipts are
`docs/evidence/stage-5-release-candidate/packaging/remote-signed-artifact-reconciliation-v0.3.0-rc.1-20260811.json`
and
`docs/evidence/stage-5-release-candidate/reliability/u4-signed-v0.3.0-rc.1-lifecycle-20260811.json`.

**Gate U4 — passed 2026-08-11:** A clean supported profile reaches the first verified Workbench change from published instructions, and every induced setup failure gives an actionable recovery path.

### U5 — Accessibility, comprehension, and supportability

**Objective:** Make authority, evidence, runtime state, and recovery understandable without requiring knowledge of Arda's internal mythology or crate graph.

**Files:**

- Modify: `apps/arda-launcher/src/`
- Modify: `apps/arda-hud/src/`
- Update: `docs/operator/`
- Update: `docs/releases/stage-5-release-candidate-evidence.md`
- Create only at Stage 6 closeout: `docs/releases/1.0/` evidence files named by the Stage 6 plan

**Work:**

- [x] Preserve keyboard-complete, screen-reader-labelled, reduced-motion, forced-color, and high-contrast paths.
- [x] Present plain-language answers to “what happened?”, “why?”, “what can act?”, “what evidence is available?”, and “what should I do next?”.
- [x] Keep internal names as identity, but pair them with functional labels in operator-facing surfaces.
- [x] Validate redacted diagnostics against the seeded failure set.
- [x] Publish limitations, supported matrix, security/privacy boundaries, known issues, and support expectations without inflated claims.
- [x] Record the Stage 5 evaluator disposition against the final candidate. The
  operator selected the single-operator local release profile on 2026-08-11;
  no qualifying non-author evaluation was performed or represented by proxy.
  Stage 6 retains its explicit independent non-author evaluator gate.

**U5 implementation evidence (2026-08-05; remediated 2026-08-11):**

- `WorkbenchModule` now derives one labelled operator summary from the live run
  graph and event reasons. It answers what happened, why, which authority can
  act, what evidence is available, and the next operator action without requiring
  knowledge of node enums or receipt internals. Four accessibility tests cover
  initial/captured-objective axe scans, keyboard focus order, the five questions,
  and failed-run reason/authority/recovery guidance; the current HUD suite passes
  507/507.
- Workbench CSS preserves explicit focus indication plus reduced-motion,
  increased-contrast, forced-color, and responsive summary paths. Existing
  native AT-SPI/XTest evidence proves dialog naming, keyboard containment,
  Escape closure, focus restoration, and setup/recovery text exposure.
- `stage5_support_exercise.py` was rerun against the current launcher. All three
  seeded failures were diagnosed from redacted archives only, with no secret,
  credential filename, operator-home, or source-root leak; the largest bundle
  remained 1,530 bytes.
- `docs/operator/stage-5-support-policy.md` now pairs Manwe, Oromë, Mirromere,
  RELIC/CITADEL, and Workbench identities with functional labels, states the
  single supported profile, loopback/authentication and privacy boundaries, and
  current limitations. Stale license, signing-method, and Wayland-startup issues
  were removed or replaced with the actual final-artifact reconciliation blocker.
- HUD lint reports 105 existing warnings and zero errors; the production build
  passes. A reported non-author attempt could not identify current system state
  or approval authority, so it did not satisfy Gate U5 and is not represented as
  a pass receipt. The launcher now presents a top-level Operator orientation
  block naming current state, read-only actions, sole operator approval authority,
  evidence quality, execution blockers, and the exact next action. Launcher tests
  pass 11/11; focused Workbench tests pass 9/9; both frontend builds pass. The
  final signed candidate passed U4. The explicit operator release-profile
  decision closed the Stage 5 U5 profile gate without claiming a qualifying
  non-author session; it does not close Stage 6's independent-user gate.

**Gate U5 — passed for the selected Stage 5 release profile on 2026-08-11:**
Automated and native operator evidence demonstrates identifiable system state,
approval authority, evidence quality, and recovery action for the selected
Stage 5 profile. Independent non-author repetition remains explicitly
unperformed and release-gating in Stage 6.

### U6 — Release, closeout, and post-plan improvement mode

**Objective:** Close this finite release/integration estate and transition its
supported distribution to measured maintenance and improvement. Optional
capabilities may still advance only through the master plan's bounded gates;
this closeout does not erase them from Arda's architecture.

**Work:**

- [x] Close and archive Stage 5 only after its exact release gates and evidence packet pass. Closed 2026-08-11 under the documented single-operator local release profile.
- [ ] Execute Stage 6 as the release decision and proof phase, not a feature-expansion stage.
- [ ] Execute frontend/backend convergence one vertical workflow at a time after Stage 5 closes; preserve Rust authority and require durable restart recovery.
- [x] Accept all five upper monitor sessions and workstation continuity natively without changing World View into a workspace. On 2026-08-12 the native Tauri HUD recovered five distinct canonical sessions after a full process stop/restart; the clean overlay reported `sessions=5`, `owners=5`, and `handoffs=same_live_session`, with all five payloads rendered.
- [x] Classify every first-party application as supported, beta, preview, research, or not distributed. The product-plan suite is the sole live classification authority.
- [x] Archive every completed domain plan and repair all active references. Stage 5 is archived on the release line; the remaining execution plans retain unresolved work.
- [ ] Leave one live authority per domain: source/contracts, compact crate status, operator docs, and release evidence.
- [ ] Convert post-closeout work intake to defects, measured performance/usability failures, security findings, compatibility obligations, and user evidence.
- [ ] Reject post-closeout proposals that add a parallel authority or do not improve an existing supported workflow.

**Gate U6:** Stage 6 evidence is complete, the default documentation route matches runtime truth, `docs/plans/` contains no completed work, and future work is improvement of the released system rather than subsystem expansion.

**Current U6 frontier (reconciled 2026-08-12):** Stage 5 lifecycle and support
profile gates are closed, backend-owned HUD authority is implemented for system
health, Workbench, Research, Personal Operations, and monitor sessions, and the
five-monitor native lifecycle walkthrough has passed. Stage 6 execution has
started by making release source identity cover every production surface and by
rejecting dirty-source manifest generation. The Workbench verification-failure
vertical now exposes backend-owned recovery diagnostics through Tauri and React,
including durable failure owner, last valid receipt, safe recovery action, and a
post-recovery receipt proven across two fresh-process restarts. U6 remains open:
no final clean
`1.0.0` source or signed artifact bytes have been selected, and production
frontend/backend acceptance, final supported-matrix and recovery evidence,
security review, performance and native accessibility evidence, the final
release documentation/evidence packet, and post-closeout intake activation are
still required.

**U6 audit/preflight (2026-08-12):**
[`u6-audit-and-preflight-20260812.md`](../evidence/stage-6-1.0/u6-audit-and-preflight-20260812.md)
records the passing source, frontend, security, support, documentation, restart,
and Stage 5 signature revalidation gates. It also records the release blockers
that may not be closed by author or agent evidence: no final clean `1.0.0`
source or signed bytes, no qualifying independent non-author evaluator receipt,
no independent final release-critical security review, and open required
whole-system/phone proofs. U6 remains active and fail-closed.

---

## 6. Verification ladder

Run narrow gates after each owned change, then execute the release matrix at Stage 5/6 boundaries.

### Runtime and Workbench

```bash
cargo test --test root_daemon -- --test-threads=1
cargo test -p arda-engine --all-features -- --test-threads=1
cargo test -p manwe --all-features -- --test-threads=1
cargo run --bin arda -- --once
```

### Workspace and applications

```bash
cargo metadata --no-deps --format-version=1
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-features -- --test-threads=1
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo deny check
pnpm --dir apps/arda-hud test
pnpm --dir apps/arda-hud lint
pnpm --dir apps/arda-hud build
pnpm --dir apps/arda-launcher test
pnpm --dir apps/arda-launcher lint
pnpm --dir apps/arda-launcher build
```

### Documentation and release evidence

```bash
python3 scripts/hades_markdown_link_check.py --root docs --out /tmp/arda-unification-links.json
python3 scripts/arda_release_ops.py --help
python3 scripts/stage5_reliability_soak.py --help
```

Exact artifact signing, clean-install, upgrade/rollback, backup/restore, adapter conformance, fault-injection, soak, and security commands remain owned by Stage 5 and Stage 6. A source-only pass does not close an artifact or operational gate.

---

## 7. Program exit criteria

- [x] The active plan estate is finite, non-overlapping, and scope-locked; remaining active plans retain distinct unresolved release or product-proof gates.
- [x] One canonical runtime owns startup, supervision, health, shutdown, and recovery.
- [x] Manwë has one production runtime and no permanent static/adaptive split.
- [x] Rust, Python, and JavaScript/TypeScript Workbench workflows pass through the root-composed installed system.
- [x] Approval, denial, execution, verification, receipts, memory, and UI share traceable lineage without authority collapse.
- [x] Failure injection and the uninterrupted soak show no silent mutation, false completion, duplicate mutation, lost terminal state, or unbounded growth.
- [x] The final Stage 5 signed artifacts pass install, upgrade, rollback, backup, restore, diagnostics, and uninstall on the selected supported profile; Stage 6 still requires qualification of its final 1.0 bytes.
- [ ] Accessibility, plain-language recovery, security/privacy, known limitations, and support documentation pass their Stage 5/6 gates.
- [x] HUD health, Workbench, Research, and Personal Operations use backend-owned identity, state, error, receipt, and recovery contracts; final Stage 6 contract freeze and configured-identity acceptance remain release evidence gates.
- [x] All five upper monitor surfaces pass native concurrent-owner, full-aperture, same-session workstation, and restart-recovery lifecycle acceptance. Separate live-browser media proofs remain owned by the master/monitor plans and do not reopen this lifecycle result.
- [x] Other first-party applications are honestly classified and cannot block or destabilize Workbench.
- [ ] Completed plans are archived, stale paths are repaired, and the default documentation route describes the live system.
- [ ] Post-closeout development is limited to measurable improvement of existing supported workflows.

## 8. Definition of success

Arda is unified when another supported user can install one product, start one canonical runtime, attach a supported project, complete or reject a governed change, understand the evidence and authority path, recover from a seeded failure, and return later without the author repairing hidden state or explaining internal architecture.
