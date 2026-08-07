# HUD Frontend–Backend Contract Convergence and 1.0 Closeout Plan

**Status:** Active, audit/contract-preparation only while the `efd118b5` frozen-source Stage 5 soak and release gates remain open
**Adopted:** 2026-08-06
**Audit authority:** [HUD frontend/backend integration audit](../audits/2026-08-06-hud-frontend-backend-integration.md)

## Current execution state

Contract preparation is complete and implementation remains dependency-blocked. The shared draft boundary is recorded in [`spec/hud-convergence/v1`](../../spec/hud-convergence/v1/README.md), with one schema and fixed passing/fail-closed fixtures. Freeze-safe tests in `crates/engine/tests/hud_convergence_contract.rs`, `apps/arda-hud/src-tauri/tests/hud_convergence_contract.rs`, and `apps/arda-hud/src/lib/hudConvergenceContract.test.ts` now consume that same fixture and pin cross-layer authority, projection-state, event-stream, and five-monitor semantics. These are preparation tests, not evidence that production handlers or projections implement C0. No backend feature or runtime source was changed by this preparation slice.

Stage 5 is still open: the final-source soak receipt, qualifying independent evaluator receipt, and exact signed-artifact lifecycle remain unresolved in the release evidence. C0 implementation and all C1–C3 work therefore remain blocked by the frozen-source boundary below.

## 1.0 definition

Arda 1.0 is the supported Workbench product with:

1. Stage 5 closed against the exact frozen candidate and signed artifact bytes;
2. backend contracts frozen and Rust authority preserved;
3. authoritative system health, Workbench, recovery/diagnostics, Research, and Personal Operations projections converged in the HUD;
4. five independently claimable upper HUD monitors with same-session workstation continuity accepted natively;
5. stale/loading/error/degraded states and operator recovery behavior accepted natively;
6. a measured performance baseline with release-blocking regressions corrected;
7. active plans reduced to finite, non-overlapping 1.0 authorities.

RELIC/CITADEL and Mirromere are explicitly outside this 1.0 scope. Their retained records do not block release.

## Frozen-source boundary

Until Stage 5 closes:

- do not add backend features or alter the `efd118b5` candidate;
- allow the uninterrupted 24-hour receipt to finish;
- build reproducible artifacts from that exact clean source;
- publish and verify the tag-bound signed artifact set;
- run install, upgrade, backup, restore, rollback, reset, and uninstall against those exact bytes;
- obtain a qualifying independent non-author evaluation without agent/self-review substitution;
- permit only read-only integration audit, contract drafting, plan cleanup, and test design in this workstream.

## Phase C0 — Freeze frontend/backend contracts

### C0.1 Endpoint and identity inventory

Freeze:

- endpoint/Tauri command names and versioning;
- request/response schemas;
- stable project, run, node, event, receipt, question, watchlist, personal-event, and operator identifiers;
- authenticated operator identity and mutation attribution;
- error and degraded-state envelopes.

**Gate:** every operator-facing mutation names one Rust owner, one authenticated identity source, one idempotency rule, and one durable receipt.

**Prepared contract:** `arda-engine` owns authenticated operator sessions, policy decisions, authoritative IDs/timestamps, revisions, and durable receipts. React supplies a typed intent plus a stable retry key; Tauri transports it without creating authority. The current endpoint-to-target delta is retained in the integration audit and the shared contract README.

### C0.2 Remove frontend authority fabrication

Correct the current audit findings:

- React must not create Workbench run topology or authoritative IDs;
- React must not issue `policy_safe` approval decisions or approval timestamps;
- arbitrary client-provided receipt/evidence JSON must not become completion truth;
- Research mutations must not mint approval authority in the browser;
- Personal Operations must not default silently to `operator-0` as identity authority.

**Gate:** malformed, fabricated, expired, mismatched, replayed, or cross-operator authority fails closed in Rust and never renders as accepted.

**Prepared Workbench boundary:** planning accepts project/objective/session/idempotency intent and returns the Rust-created graph; approval accepts approve/reject intent and an authority reference; completion accepts target/session/idempotency intent. The browser does not send `policy_safe`, approval timestamps, graph topology, completion evidence, or receipt digests as authority.

### C0.3 Freeze event and state semantics

Define:

- `loading`, `healthy`, `stale`, `partial`, `degraded`, `unavailable`, and `failed`;
- source revision/time and recovery action;
- SSE sequence/cursor, gap detection, deduplication, reconnect/backoff, terminal closure, and multi-run ownership;
- durable restart recovery independent of browser local storage.

**Gate:** one shared contract test fixture drives Rust, Tauri, and React state handling.

**Prepared fixture:** `spec/hud-convergence/v1/fixtures/valid-shared-contract.json` defines all seven load states, backend cursor/reconnect semantics, durable recovery, and five independent monitor sessions with same-session workstation handoff. `tests/test_workbench_contract_fixtures.py` validates that fixture and rejects browser-created authority. The engine, Tauri, and React preparation tests named above consume the same file directly; production-path conformance remains open until C0 implementation is permitted.

## Phase C1 — Integrate vertical workflows

Complete one workflow before opening the next.

### C1.1 System/runtime health

Converge Hermes and Manwë projections without treating one successful subprocess, file, or endpoint read as aggregate health. Coordinate any change to the existing `:7171` consumers.

### C1.2 Workbench canonical loop

Prove:

```text
backend state → transport → frontend rendering → operator action
→ backend mutation → durable receipt → restart recovery
```

Cover objective, planning, approval/rejection, provider execution, verification failure, completion, cancellation, retry, event loss/reconnect, and resume.

### C1.3 Recovery and diagnostics

Expose exact failure owner, last valid state, safe recovery action, and post-recovery receipt. Never reduce missing evidence to success.

### C1.4 Research/evidence projection

Converge authenticated question/watchlist/brief lifecycle, citations, freshness, idempotency, pause/resume/retire, and restart recovery.

### C1.5 Personal Operations

Converge authenticated operator identity, revision-consistent snapshot loading, capture/classification/reminder receipts, export/delete boundaries, and restart recovery. Complete the plan's existing operator gates without fabricating dogfood evidence.

### C1.6 Five-monitor HUD acceptance

Use the active corrective monitor plan. Preserve authored geometry, all five independently claimable upper monitors, concurrent owners, full-aperture content, same live session in workstation windows, and display-only World View.

## Phase C2 — Design and interaction closeout

After live authority paths pass:

- settle window, panel, focus, navigation, and workstation behavior;
- remove duplicate controls and dead placeholders;
- make stale/loading/error/degraded states visually distinct and actionable;
- run native visual acceptance in the actual HUD scene and workstation windows;
- retain reduced-motion, keyboard, screen-reader, forced-color, and high-contrast paths.

## Phase C3 — Performance closeout

Record the baseline listed in the integration audit. Optimize only measured release blockers. Re-measure after each accepted change and retain before/after evidence.

## Verification gates

- focused Rust owner/contract tests;
- Tauri command tests;
- focused React component/client tests;
- full HUD test, lint, and build gates;
- relevant workspace checks;
- markdown link/document health checks;
- native HUD visual and interaction acceptance;
- installed-artifact restart/recovery proof.

## Archive gate

Archive this plan only when:

- Stage 5 and Stage 6 release gates are closed honestly;
- every matrix row is either accepted for 1.0 or explicitly unsupported with truthful UI;
- the five-monitor corrective plan is accepted by the operator;
- no React surface manufactures Rust-owned authority;
- the performance baseline and supported limitations are published;
- `docs/plans/` contains only unresolved, release-relevant work.
