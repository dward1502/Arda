# Arda System Unification and Usability Plan

> **For Hermes:** Execute this plan as a scope-frozen integration program. Use existing domain plans as implementation authorities; do not create new agents, applications, queues, memory systems, policy layers, or speculative capability domains.

**Status:** Active
**Adopted:** 2026-08-02
**Owner:** Arda system/runtime maintainers
**Goal:** Finish the capabilities already committed under `docs/` and `docs/plans/`, compose them into one dependable Arda system, and mature that system into a functional, supportable product for other users without expanding its conceptual scope.

**Architecture:** The root `arda` daemon, `services.toml`, `arda-engine`, Manwë, the launcher, and the HUD form the canonical runtime and operator path. Existing domain crates remain the owners of their contracts and state; this plan coordinates composition, end-to-end proof, simplification, release readiness, and plan closeout rather than cloning domain work.

**Primary release authorities:**

- [Stage 5 — Release Candidate](2026-07-29-stage-5-release-candidate-plan.md)
- [Stage 6 — Legitimate 1.0](2026-07-29-stage-6-legitimate-1.0-plan.md)
- [Arda Product Plan Suite](../ARDA_PRODUCT_PLAN_SUITE.md)

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
| Workbench release candidate | `2026-07-29-stage-5-release-candidate-plan.md` | Coordinate remaining signed-artifact, security, and soak gates; do not duplicate them. |
| Workbench 1.0 | `2026-07-29-stage-6-legitimate-1.0-plan.md` | Enter only after Stage 5 closes; preserve its release evidence requirements. |
| Product/application classification | `docs/ARDA_PRODUCT_PLAN_SUITE.md` and application plans | Keep Workbench release-critical; classify all other applications honestly. |
| Root process composition | `src/main.rs`, `services.toml`, `crates/engine/src/registry.rs`, `crates/engine/src/supervisor.rs` | Establish one canonical startup, health, shutdown, and recovery path. |
| Routing/runtime convergence | `MANWE_SINGLE_RUNTIME_CONVERGENCE.md` | Consume that plan's result; do not create another routing runtime. |
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

- Modify: `docs/ARDA_PRODUCT_PLAN_SUITE.md`
- Modify as evidence warrants: `docs/plans/*.md`
- Modify on closeout: `docs/archive/README.md`

**Work:**

- [ ] Classify every active plan as release-critical, optional but bounded, dependency-blocked, or stale/superseded.
- [ ] Map every unchecked requirement to exactly one implementation authority.
- [ ] Remove stale duplicate requirements and repair links without deleting unique acceptance criteria.
- [ ] Keep optional applications out of the Stage 5/6 critical path.
- [ ] Archive each completed plan immediately after its live gates pass.
- [ ] Record the finite Stage 5 blockers as the first release-critical queue: final signed artifact reconciliation, security closeout, and a valid uninterrupted 24-hour reliability receipt.

**Gate U0:** Every open plan item has one owner, one acceptance surface, and one release classification; no completed plan remains in `docs/plans/`.

### U1 — Establish one canonical runtime topology

**Objective:** Make one root invocation own the supported Arda runtime topology and expose truthful service health.

**Files:**

- Modify: `services.toml`
- Modify: `src/main.rs`
- Modify: `crates/engine/src/registry.rs`
- Modify: `crates/engine/src/supervisor.rs`
- Modify: `tests/root_daemon.rs`
- Coordinate with: `docs/plans/MANWE_SINGLE_RUNTIME_CONVERGENCE.md`

**TDD sequence:**

1. Add or tighten a failing `tests/root_daemon.rs` fixture for the exact supported service set, required/optional policy, health state, and clean shutdown.
2. Run `cargo test --test root_daemon -- --test-threads=1` and retain the expected failure.
3. Make the smallest registry/supervisor/root-daemon change.
4. Re-run the focused test and the Manwë process/integration smoke named by its convergence plan.
5. Remove only the parallel runtime/config path whose parity and consumer independence are proven.

**Work:**

- [ ] Declare the canonical root command and supported headless/UI profiles.
- [ ] Reconcile repository supervision with any independently managed user-systemd services so one process owner exists per service.
- [ ] Complete Manwë single-runtime convergence without changing the coordinated `:7171` contract before consumer verification.
- [ ] Expose required, optional, starting, healthy, degraded, restarting, and stopped states through one operator-readable status surface.
- [ ] Prove bounded restart backoff, clean shutdown, and no duplicate service ownership.
- [ ] Remove dual-runtime language and stale launch instructions only after cutover evidence passes.

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
- Test: `crates/engine/tests/workbench_boundary_recovery.rs`

**Required chain:**

`objective → evidence/context → plan/run graph → authority preview → approval or rejection → bounded execution → project-native verification → receipt/evidence projection → restart-safe terminal state`

**Work:**

- [ ] Pin one Rust and one Python golden workflow through the root-composed runtime.
- [ ] Add the JavaScript/TypeScript workflow already required by Stage 5/6 adapter scope without creating another execution authority.
- [ ] Prove accepted, rejected, revised, cancelled, failed, and resumed outcomes.
- [ ] Correlate audit, policy, execution, verification, memory, and HUD projections with one stable lineage identifier.
- [ ] Ensure every HUD claim is backed by live receipt/state and visibly distinguishes stale, partial, unavailable, and failed evidence.
- [ ] Prove that no audit, evaluation, memory, or UI projection can authorize execution.

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

- [ ] Correlate semantic receipts with trace/span or equivalent runtime lineage IDs.
- [ ] Define measured budgets for startup, idle resource use, UI latency, event projection, recovery, diagnostics, and bounded state growth.
- [ ] Exercise provider loss, network loss, process kill, disk pressure, malformed/truncated state, model timeout, adapter crash/noisy output, cancellation, and operator rejection.
- [ ] Preserve enough bounded diagnostics to assign every nonzero soak exit to a scenario and root cause.
- [ ] Run one uninterrupted valid 24-hour Stage 5 matrix after capacity preflight.
- [ ] Prove no silent mutation, false completion, lost terminal state, duplicate mutation, or unbounded state growth.

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

- [ ] Consolidate detection, prerequisites, provider setup, service plan, readiness, and guided setup into one visible first-run sequence.
- [ ] Fail unsupported profiles before partial installation.
- [ ] Keep secret writes and consequential configuration changes explicitly approved and receipted.
- [ ] Provide clear offline, provider-unavailable, degraded, and recovery guidance.
- [ ] Prove install, upgrade, rollback, backup, restore, safe reset, and uninstall using final signed artifacts.
- [ ] Ensure optional applications remain opt-in and cannot prevent Workbench startup.

**Gate U4:** A clean supported profile reaches the first verified Workbench change from published instructions, and every induced setup failure gives an actionable recovery path.

### U5 — Accessibility, comprehension, and supportability

**Objective:** Make authority, evidence, runtime state, and recovery understandable without requiring knowledge of Arda's internal mythology or crate graph.

**Files:**

- Modify: `apps/arda-launcher/src/`
- Modify: `apps/arda-hud/src/`
- Update: `docs/operator/`
- Update: `docs/releases/stage-5-release-candidate-evidence.md`
- Create only at Stage 6 closeout: `docs/releases/1.0/` evidence files named by the Stage 6 plan

**Work:**

- [ ] Preserve keyboard-complete, screen-reader-labelled, reduced-motion, forced-color, and high-contrast paths.
- [ ] Present plain-language answers to “what happened?”, “why?”, “what can act?”, and “what should I do next?”.
- [ ] Keep internal names as identity, but pair them with functional labels in operator-facing surfaces.
- [ ] Validate redacted diagnostics against the seeded failure set.
- [ ] Publish limitations, supported matrix, security/privacy boundaries, known issues, and support expectations without inflated claims.
- [ ] Run the Stage 6 independent-user evidence gate when clean external evaluators are available; never manufacture proxy evidence.

**Gate U5:** A non-author operator can identify system state, approval authority, evidence quality, and the next recovery action without inspecting source or raw state files.

### U6 — Release, closeout, and post-plan improvement mode

**Objective:** Close the finite plan estate and transition from architecture expansion to measured maintenance and improvement.

**Work:**

- [ ] Close and archive Stage 5 only after its exact release gates and evidence packet pass.
- [ ] Execute Stage 6 as the release decision and proof phase, not a feature-expansion stage.
- [ ] Classify every first-party application as supported, beta, preview, research, or not distributed.
- [ ] Archive every completed domain plan and repair all active references.
- [ ] Leave one live authority per domain: source/contracts, compact crate status, operator docs, and release evidence.
- [ ] Convert post-closeout work intake to defects, measured performance/usability failures, security findings, compatibility obligations, and user evidence.
- [ ] Reject post-closeout proposals that add a parallel authority or do not improve an existing supported workflow.

**Gate U6:** Stage 6 evidence is complete, the default documentation route matches runtime truth, `docs/plans/` contains no completed work, and future work is improvement of the released system rather than subsystem expansion.

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

- [ ] The active plan estate is finite, non-overlapping, and scope-locked.
- [ ] One canonical runtime owns startup, supervision, health, shutdown, and recovery.
- [ ] Manwë has one production runtime and no permanent static/adaptive split.
- [ ] Rust, Python, and JavaScript/TypeScript Workbench workflows pass through the root-composed installed system.
- [ ] Approval, denial, execution, verification, receipts, memory, and UI share traceable lineage without authority collapse.
- [ ] Failure injection and the uninterrupted soak show no silent mutation, false completion, duplicate mutation, lost terminal state, or unbounded growth.
- [ ] Final signed artifacts pass install, upgrade, rollback, backup, restore, diagnostics, and uninstall on the supported matrix.
- [ ] Accessibility, plain-language recovery, security/privacy, known limitations, and support documentation pass their Stage 5/6 gates.
- [ ] Other first-party applications are honestly classified and cannot block or destabilize Workbench.
- [ ] Completed plans are archived, stale paths are repaired, and the default documentation route describes the live system.
- [ ] Post-closeout development is limited to measurable improvement of existing supported workflows.

## 8. Definition of success

Arda is unified when another supported user can install one product, start one canonical runtime, attach a supported project, complete or reject a governed change, understand the evidence and authority path, recover from a seeded failure, and return later without the author repairing hidden state or explaining internal architecture.
