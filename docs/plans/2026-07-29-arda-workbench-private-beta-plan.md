# Arda Workbench Private Beta Implementation Plan

> **For Hermes:** Use `subagent-driven-development` to implement this plan task-by-task. Preserve user work, use TDD, and stop at each named gate.

**Goal:** Deliver Arda's first sellable vertical slice: attach an existing repository, submit an objective, approve a bounded run graph, execute and verify a real change, inspect receipts, and resume safely after restart.

**Architecture:** Extend the existing `arda` composition root and `arda-engine` harness rather than creating another daemon. Put cross-application project/run contracts in `arda-core`, execution/checkpoint ownership in `arda-engine`, and operator projection in the existing ARDA HUD. Agent implementations remain replaceable workers behind a versioned adapter protocol.

**Tech stack:** Rust, Tokio, Axum, Serde, JSON Schema, Tauri 2, React 19, TypeScript, Vitest, Cargo tests, JSON Lines/MCP adapters.

**Target stage:** Stage 4 — Private Beta  
**Priority:** P0; this is the product wedge  
**Depends on:** current root daemon, engine harness, governance, Manwe, Oromë, Vairë, Aulë, HUD  
**Does not depend on:** Mirromere hardware, fleet expansion, autonomous self-modification, or all first-party applications

---

## Execution status — 2026-07-31

- **Complete:** Task 0.1 core model, strict path/secret/version checks, Rust and Python fixtures, and JSON Schema.
- **Complete:** Task 0.2 canonical run graph, authority/provenance round-trip, DAG validation, approval-parent validation, duplicate idempotency rejection, and state-transition tests.
- **Complete (foundation):** Task 1.1 append-only per-run journal, atomic checkpoint/result writes, strict corruption detection, contiguous sequence validation, and restart idempotency recovery.
- **Complete:** Stage 4 contract-registry declaration/version wiring for `arda.project-contract.v1` and `arda.run-graph.v1`. The registry now rejects undeclared versions and its smoke tests pin both canonical contracts. Full S4-C1 remains open for a dedicated fixed-event replay proof and independently reproduced Python schema validation.
- **Complete:** Task 1.2 typed project/run harness API (`2470832`). The existing harness now validates and persists canonical project contracts, plans canonical run graphs through the existing run store, records approval/cancellation lifecycle events, exposes run/event reads, rejects unknown browser command fields, and gates serialized mutations behind loopback peer checks plus canonical Oromë approval and idempotency envelopes.
- **Started, not complete:** HUD objective composer and approval-first graph draft are integrated into the Planning surface. A read-only `validate_project_contract` Tauri command now validates through the canonical `arda-core` parser and projects identity, adapter, requested permissions, commands, and checks before attachment. The surface explicitly remains validation/draft-only and does not claim project attachment or execution.
- **Started, not complete:** Task 3.1 now has a dedicated `src-tauri/src/commands/workbench.rs` boundary with focused Rust tests. Its typed native attachment/run invocation remains unwired to the now-available harness endpoints.
- **Not yet complete:** live run-event streaming, adapter protocol/SDK, real execution/verification, native HUD attachment and receipts, golden paths, packaging, and external evaluation.

Stage 4 remains **in progress**. The implementation evidence above is a bounded vertical-slice foundation, not the private-beta exit gate.

Focused evidence: `cargo test -p arda-contract-registry -- --test-threads=1` passed 8 tests; `cargo test -p arda-core --test project_contract --test run_graph -- --test-threads=1` passed 12 tests; `cargo test -p arda-engine --test run_recovery --test harness_projects --test harness_runs -- --test-threads=1` passed 7 tests; `cargo test -p arda-engine -- --test-threads=1` passed 26 tests; `cargo check --workspace --all-targets --all-features` passed; and the final `cargo test --workspace --all-features -- --test-threads=1` rerun passed all workspace unit, integration, and doc-test targets. The workspace currently emits warnings from concurrent uncommitted presence-harness work; those warnings are not Task 1.2 evidence.

## Verified starting point

- `src/main.rs` is the single daemon entry point and supervises services from `services.toml`.
- Before Task 1.2, `crates/engine/src/harness.rs` exposed health/status/models and Warden proxy routes but no project or run graph API; the typed routes now extend that same harness rather than creating another daemon.
- `arda-core` already owns tasks, goals, plans, tools, messages, ledgers, governance gates, and loop primitives.
- The HUD already contains task, approval, source, operations, learning, business, and Hermes surfaces; its README understates the implemented UI.
- The Tauri backend is currently concentrated in `apps/arda-hud/src-tauri/src/lib.rs`; new Workbench commands must be placed in focused modules rather than enlarging that file.
- Hermes Agent can provide an initial execution adapter, but Workbench must not hard-wire itself to one agent vendor or model.

## Product contract

A successful run must preserve one correlation chain:

```text
project_id -> objective_id -> run_id -> graph node IDs
           -> approval IDs -> tool/action receipts -> verification receipts
           -> result/revision -> memory/checkpoint
```

Every mutating edge requires an explicit authority class, workspace boundary, budget, timeout, and idempotency key.

## Non-goals for the private beta

- No autonomous merge, push, deployment, purchasing, or external messaging.
- No arbitrary host filesystem access outside the attached project and declared artifact roots.
- No visual low-code DAG editor.
- No replacement of Hermes, Codex, OpenHands, or other agent implementations.
- No team tenancy or fleet scheduling before the single-operator path is reliable.

## Phase 0 — Freeze the product contract

### Task 0.1: Define `arda.project-contract.v1`

**Files**
- Create: `crates/spine/governance/arda-core/src/project_contract.rs`
- Modify: `crates/spine/governance/arda-core/src/lib.rs`
- Create: `spec/project-contract/v1/project-contract.schema.json`
- Create: `spec/project-contract/v1/examples/rust-project.json`
- Create: `spec/project-contract/v1/examples/python-project.json`
- Test: `crates/spine/governance/arda-core/tests/project_contract.rs`

**Steps**
1. Write failing tests for version parsing, canonical paths, denied parent traversal, command declarations, environment-name-only secrets, and authority defaults.
2. Implement typed project identity, workspace boundary, runtime adapter, commands, checks, artifacts, permissions, rollback, memory scope, and provenance.
3. Validate both examples against the JSON Schema.
4. Add additive migration rules; reject unsupported major versions with a typed error.

**Acceptance**
- A contract cannot embed secret values or escape its workspace root.
- Rust and Python examples deserialize to the same core model.
- Unknown authority defaults fail closed.

### Task 0.2: Define the canonical run graph

**Files**
- Create: `crates/spine/governance/arda-core/src/run_graph.rs`
- Modify: `crates/spine/governance/arda-core/src/lib.rs`
- Test: `crates/spine/governance/arda-core/tests/run_graph.rs`

**Required types**
- `RunId`, `NodeId`, `EdgeId`, `ObjectiveId`
- `RunGraph`, `RunNode`, `RunEdge`
- node states: pending, ready, blocked, running, succeeded, failed, cancelled, superseded
- node kinds: inspect, retrieve, research, plan, approval, execute, verify, review, compensate, close
- authority, budget, retry, timeout, input/output digest, parent receipts, and checkpoint metadata

**Acceptance**
- Invalid transitions, cycles in the initial executable DAG, missing approval parents, and duplicate idempotency keys are rejected.
- A serialized graph round-trips without losing authority or provenance.

## Phase 1 — Build durable execution and recovery

### Task 1.1: Add a run store and checkpoint journal

**Files**
- Create: `crates/engine/src/runs/mod.rs`
- Create: `crates/engine/src/runs/store.rs`
- Create: `crates/engine/src/runs/executor.rs`
- Create: `crates/engine/src/runs/recovery.rs`
- Modify: `crates/engine/src/lib.rs`
- Test: `crates/engine/tests/run_recovery.rs`

**Storage boundary**
- Canonical append-only events: `data/runs/<run_id>/events.jsonl`
- Checkpoint projection: `data/runs/<run_id>/checkpoint.json`
- Human-readable result bundle: `data/runs/<run_id>/result.json`

**Acceptance**
- Kill/restart after a tool action but before result projection does not repeat the mutation.
- Corrupt or truncated journal tails fail visibly and do not silently advance state.
- Cancellation terminates children and records a terminal receipt.

### Task 1.2: Add project and run APIs to the existing harness

**Files**
- Modify: `crates/engine/src/harness.rs`
- Create: `crates/engine/src/harness/projects.rs`
- Create: `crates/engine/src/harness/runs.rs`
- Test: `crates/engine/tests/harness_projects.rs`
- Test: `crates/engine/tests/harness_runs.rs`

**Endpoints**
- `POST /v1/projects/validate`
- `POST /v1/projects/attach`
- `GET /v1/projects`
- `POST /v1/runs/plan`
- `POST /v1/runs/{id}/approve`
- `POST /v1/runs/{id}/cancel`
- `GET /v1/runs/{id}`
- `GET /v1/runs/{id}/events`

**Acceptance**
- Mutating routes are loopback-only by default and require an approval/idempotency envelope.
- No endpoint proxies arbitrary shell strings directly from a browser payload.

**Status — 2026-07-31:** Complete for the endpoint and acceptance scope above. Focused integration tests cover typed validation/attachment/listing, plan/approve/cancel/read/event persistence, mutation idempotency, rejected missing or policy-blocked approval envelopes, path-safe run IDs, and rejection of unknown browser shell fields without execution. Live SSE/WebSocket run-event delivery remains a later Workstream 2/HUD integration item rather than an undocumented behavior of `GET /v1/runs/{id}/events`.

## Phase 2 — Implement the adapter boundary

### Task 2.1: Define an agent/project adapter protocol

**Files**
- Create: `spec/project-adapter/v1/protocol.md`
- Create: `spec/project-adapter/v1/messages.schema.json`
- Create: `sdk/python/arda_project_adapter/__init__.py`
- Create: `sdk/python/arda_project_adapter/server.py`
- Create: `sdk/python/tests/test_conformance.py`
- Create: `crates/engine/src/adapters/mod.rs`
- Create: `crates/engine/src/adapters/jsonl.rs`

**Protocol behavior**
- initialize, capability advertisement, health, bounded request, progress, result, cancellation, provenance, recovery token, and denied-capability response.
- JSON Lines over stdio is the reference path; MCP may be bridged but is not the sole protocol.

**Acceptance**
- Python adapter passes an offline conformance suite.
- Engine enforces process, timeout, cwd, environment allowlist, and cancellation boundaries.

### Task 2.2: Add the first Hermes execution adapter

**Files**
- Create: `crates/engine/src/adapters/hermes.rs`
- Create: `config/adapters/hermes-workbench.toml`
- Test: `crates/engine/tests/hermes_adapter_contract.rs`

**Acceptance**
- The adapter turns graph nodes into bounded Hermes jobs and translates actual tool/test evidence back into Arda receipts.
- Vendor-specific session data never becomes the canonical run state.

## Phase 3 — Build the Workbench operator surface

### Task 3.1: Split the HUD native command boundary before adding mutations

**Files**
- Create: `apps/arda-hud/src-tauri/src/commands/mod.rs`
- Create: `apps/arda-hud/src-tauri/src/commands/workbench.rs`
- Modify: `apps/arda-hud/src-tauri/src/lib.rs`
- Test: Rust unit tests beside `commands/workbench.rs`

**Acceptance**
- Existing Tauri commands remain compatible.
- Workbench invokes typed harness endpoints and never shells directly from frontend input.

### Task 3.2: Add project attachment and objective capture

**Files**
- Create: `apps/arda-hud/src/lib/workbench.ts`
- Create: `apps/arda-hud/src/components/arda/modules/WorkbenchModule.tsx`
- Create: `apps/arda-hud/src/components/arda/modules/WorkbenchModule.test.tsx`
- Modify: `apps/arda-hud/src/App.tsx`

**Acceptance**
- Operator sees validation failures, effective permissions, provider posture, and project checks before attachment.
- Objective capture supports text first; voice feeds the same objective contract later.

### Task 3.3: Add graph, approval, diff, test, and receipt views

**Files**
- Create: `apps/arda-hud/src/components/workbench/RunGraphView.tsx`
- Create: `apps/arda-hud/src/components/workbench/ApprovalPanel.tsx`
- Create: `apps/arda-hud/src/components/workbench/ChangeReview.tsx`
- Create: `apps/arda-hud/src/components/workbench/RunTimeline.tsx`
- Test each component with Vitest and Testing Library.

**Acceptance**
- First screen answers: what is happening, what is blocked, what needs approval, what changed, what tests ran, what it cost, and how to resume.
- Keyboard-only operation and reduced-motion behavior pass focused accessibility checks.

## Phase 4 — Prove the vertical slice

### Task 4.1: Rust repository golden test

Attach a clean fixture repository and complete one bounded, test-backed mutation. Record install-to-result time, interventions, model route, cost, failures, and recovery behavior.

### Task 4.2: Python repository golden test

Repeat through the Python reference adapter without importing the project into the Arda Cargo workspace.

### Task 4.3: Crash/restart and duplicate-mutation test

Terminate Arda at each graph boundary and verify exact-once observable mutation behavior or explicit compensation.

## Verification ladder

```bash
cargo test -p arda-core --test project_contract --test run_graph -- --test-threads=1
cargo test -p arda-engine --test run_recovery --test harness_projects --test harness_runs -- --test-threads=1
python3 -m pytest sdk/python/tests/test_conformance.py -q
cd apps/arda-hud && pnpm test && pnpm lint && pnpm build
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-features -- --test-threads=1
```

## Stage 4 exit criteria

- One Rust and one Python project complete objective → plan → approval → change → tests → review → close.
- One `run_id` correlates graph, model route, tools, evidence, costs, approvals, diff, tests, and memory.
- Restart resumes without duplicate mutation.
- A non-author can install, attach, and complete the workflow from documentation.
- No critical or high-severity security issue remains in the golden path.

## Commit boundaries

Commit by contract, run store, harness API, adapter protocol, Hermes adapter, HUD module, and each golden-path proof. Do not combine unrelated first-party applications into these commits.
