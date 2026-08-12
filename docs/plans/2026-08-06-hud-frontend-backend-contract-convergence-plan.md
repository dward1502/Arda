# HUD Frontend–Backend Contract Convergence and 1.0 Closeout Plan

**Status:** Active implementation; Stage 5 is closed and Stage 6 acceptance remains open
**Adopted:** 2026-08-06
**Audit authority:** [HUD frontend/backend integration audit](../audits/2026-08-06-hud-frontend-backend-integration.md)

## Current execution state

The shared boundary is recorded in [`spec/hud-convergence/v1`](../../spec/hud-convergence/v1/README.md), with one schema and fixed passing/fail-closed fixtures. Rust now owns system health, Workbench, Research, Personal Operations, and monitor-session authority paths, and the five-monitor native lifecycle result is closed. On 2026-08-12 the root daemon became the fail-closed configured-operator authority: `ARDA_OPERATOR_ID`/`--operator-id` is required, `/v1/status` publishes the selected identity, the HUD resolves it without a frontend fallback, and Personal Operations rejects non-configured read and mutation identities. The same date also closed the Workbench HTTP/Tauri error-envelope boundary: shared harness errors emit versioned `arda.hud.error.v1` failures with stable code, message, and recovery action; Tauri accepts only that complete envelope and rejects legacy/unversioned failure bodies. Workbench restart acceptance now executes the production HTTP attach, plan, approve, and completion routes, verifies the journal receipt/checkpoint/review projection on disk, stops the harness process, starts a fresh harness against the same root, recovers the authoritative receipt and checkpoint, and proves the same idempotency key cannot append a second mutation. Stage 6 production acceptance remains open for the other required vertical workflows and final signed `1.0.0` artifact gates.

Stage 5 closed on `v0.3.0-rc.1` under the selected single-operator local release profile. Its signed-artifact lifecycle is historical candidate evidence, not qualification of final `1.0.0` bytes. Stage 6 now owns the remaining production acceptance and release decision without reopening completed five-monitor lifecycle acceptance.

## 1.0 definition

This plan owns the HUD/Workbench **release-qualification slice** of Arda 1.0;
the product doctrine and master convergence authority are
[`../architecture/ARDA_1_0_PERSONAL_AGENT_ECOSYSTEM.md`](../architecture/ARDA_1_0_PERSONAL_AGENT_ECOSYSTEM.md)
and
[`2026-08-08-arda-1.0-personal-agent-ecosystem-plan.md`](2026-08-08-arda-1.0-personal-agent-ecosystem-plan.md).
Within this bounded slice, supported Workbench/HUD release requires:

1. Stage 5 closed against the exact frozen candidate and signed artifact bytes;
2. backend contracts frozen and Rust authority preserved;
3. authoritative system health, Workbench, recovery/diagnostics, Research, and Personal Operations projections converged in the HUD;
4. five independently claimable upper HUD monitors with same-session workstation continuity accepted natively;
5. stale/loading/error/degraded states and operator recovery behavior accepted natively;
6. a measured performance baseline with release-blocking regressions corrected;
7. active plans reduced to finite, non-overlapping 1.0 authorities.

RELIC/CITADEL and Mirromere are outside this bounded HUD/Workbench release
slice and do not block it. They remain optional Arda-compatible capabilities
with their own implementation and acceptance gates.

## Development and release boundary

While Stage 5 release qualification remains open:

- proceed with C0 implementation and subsequent HUD convergence phases against the canonical branch;
- keep release evidence honest: the stopped `6616addd` run is not a completed soak;
- defer tag-bound signing, exact-byte lifecycle proof, and independent evaluation until the next actual release-candidate freeze;
- when release qualification resumes, choose a new exact clean source identity and regenerate only the evidence required by the then-current release policy.

## Phase C0 — Freeze frontend/backend contracts

### C0.1 Endpoint and identity inventory

Freeze:

- endpoint/Tauri command names and versioning;
- request/response schemas;
- stable project, run, node, event, receipt, question, watchlist, personal-event, and operator identifiers;
- authenticated operator identity and mutation attribution;
- error and degraded-state envelopes.

**Gate:** every operator-facing mutation names one Rust owner, one authenticated identity source, one idempotency rule, and one durable receipt.

**Implemented contract:** `arda-engine` owns authenticated operator sessions, policy decisions, authoritative IDs/timestamps, revisions, durable receipts, and versioned operator-facing failures. React supplies a typed intent plus a stable retry key; Tauri transports it without creating authority and renders only complete `arda.hud.error.v1` failures with recovery guidance. Legacy or malformed failure bodies are treated as contract violations rather than accepted operator truth. The current endpoint-to-target delta is retained in the integration audit and the shared contract README.

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

**Implemented fixture:** `spec/hud-convergence/v1/fixtures/valid-shared-contract.json` defines all seven load states, the versioned error envelope, backend cursor/reconnect semantics, durable recovery, and five independent monitor sessions with same-session workstation handoff. `tests/test_workbench_contract_fixtures.py` validates that fixture and rejects browser-created authority. Engine, Tauri, and React tests consume the same file directly; executable harness and Tauri tests additionally prove production error serialization, recovery guidance, and fail-closed rejection of unversioned failures.

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

**Accepted restart vertical (2026-08-12):** `completed_workbench_mutation_survives_process_restart_and_replays_once` drives the real harness HTTP boundary through project attachment, planning, approval, and execute-node completion. It requires the durable `events.jsonl` mutation receipt, `checkpoint.json`, and `result.json`; shuts down the serving process; launches a fresh harness with the same persistence root; reads the run through `GET /v1/runs/{id}`; and verifies state, output receipt, recovery token, checkpoint digest, and review evidence are authoritative after restart. Replaying the exact completion intent after restart leaves all three durable artifacts byte-identical, proving no duplicate mutation receipt. This closes the bounded Workbench mutation → receipt → restart-recovery frontier, not the remaining C1 workflows or Stage 6 release qualification.

### C1.3 Recovery and diagnostics

Expose exact failure owner, last valid state, safe recovery action, and post-recovery receipt. Never reduce missing evidence to success.

**Accepted Workbench recovery-diagnostics slice (2026-08-12):** the Rust
harness now derives a `recovery_diagnostics` projection from the durable run
graph, event ledger, review evidence, and receipts. Tauri transports that typed
projection without recreating authority, and React renders failure owner,
failure reason, last succeeded node/receipt, the backend-selected safe recovery
action, and an explicit missing or successful post-recovery receipt. The focused
`failed_verification_is_durable_and_blocks_review` harness test proves failed
verification remains blocked across process restart, then records a successful
retry receipt and recovers that receipt after a second fresh-process restart.
`WorkbenchModule.test.tsx` proves the HUD resumes and renders the backend-owned
diagnostics. This closes the bounded Workbench verification-failure diagnostic
path only; broader Stage 6 production error/recovery acceptance remains open.

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
