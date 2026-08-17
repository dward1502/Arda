---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "implementation_plan"
  owner: "HERMES"
  status: "active"
  reviewed: "2026-08-17"
---

> 🜏 Soterion: 📜 implementation_plan | owner: HERMES | status: active | reviewed: 2026-08-17

# Phase 5: RELIC Runtime Embodiment Implementation Plan

> **For Hermes:** Use the `subagent-driven-development` skill to implement this plan task-by-task. RELIC is a read-only projection; it never becomes runtime or receipt authority.

**Goal:** Make a physical RELIC/CITADEL display truthfully visualize fresh agent execution, delegation, waits, approvals, failures, and handoffs from authenticated Arda runtime events.

**Architecture:** Existing runtime-presence contracts and `arda-relic-bridge` remain the transport boundary. The bridge reduces receipt-backed runtime state into bounded scene updates. The external renderer acknowledges scene application but cannot mint execution truth. Disconnect, stale input, and bridge failure visibly degrade.

**Tech stack:** Rust, `arda-outpost-protocol`, `arda-relic-bridge`, systemd, authenticated local/mesh transport, external CITADEL renderer, receipt/runtime presence projection.

---

## Current source baseline

- `outposts/arda-outpost-protocol/src/presence.rs` implements runtime-presence types.
- `outposts/arda-relic-bridge/src/lib.rs` and `scene_adapter.rs` implement the bridge/scene mapping.
- `outposts/arda-relic-bridge/src/bin/arda-relic-presence-sync.rs` is the sync binary.
- `config/systemd/arda-relic-bridge.service` defines a tracked service unit.
- Existing source is implementation evidence only; this phase begins by proving whether it is wired, deployed, reachable, fresh, and renderer-visible.

## Visual semantics

RELIC may represent:

- agent identity/role and active/idle/waiting/failed state;
- delegation edge and handoff direction;
- approval gate and blocked work;
- evidence freshness and degraded/disconnected state;
- bounded resource pressure;
- convergence/disagreement when source receipts support it.

RELIC may not infer emotion, certainty, success, or work from decorative motion. No synthetic timer-driven activity is accepted in live mode.

## Task 1: Reconcile contract, producer, bridge, service, and renderer

**Files:**
- Read/trace exact call paths in the source baseline above
- Create: `docs/operations/relic-live-path-audit.md`
- Modify implementation only after the audit identifies a concrete gap

**Steps:**
1. Trace each `arda.runtime-presence.v1` field to its producer and receipt/state authority.
2. Trace bridge startup, endpoint/transport, authentication, retry, freshness, and renderer acknowledgement.
3. Inspect installed unit state, process, listener, bridge journal, and current renderer state.
4. Classify every link as documented, implemented, tested, wired, running, or proven.
5. Record exact missing links without representing authored fixtures as live.
6. Commit the audit: `docs(relic): map live runtime presence path`.

## Task 2: Harden runtime-presence freshness and reduction

**Files:**
- Modify: `outposts/arda-outpost-protocol/src/presence.rs`
- Modify: producer modules found in Task 1
- Test: strict contract and reduction tests

**Steps:**
1. Write failing tests for unknown agent, duplicate event, out-of-order event, expired presence, producer restart, missing receipt, and impossible transition.
2. Require source observation time, sequence/revision, and explicit expiry.
3. Preserve last known state only as visibly stale; do not keep animated activity after expiry.
4. Verify cancellation, failure, and approval-wait states map distinctly.
5. Run protocol/producer tests and Clippy.
6. Commit: `fix(relic): make runtime presence freshness explicit`.

## Task 3: Harden bridge transport and scene acknowledgement

**Files:**
- Modify: `outposts/arda-relic-bridge/src/lib.rs`
- Modify: `outposts/arda-relic-bridge/src/scene_adapter.rs`
- Modify: `outposts/arda-relic-bridge/src/bin/arda-relic-presence-sync.rs`
- Test: bridge integration tests with a bounded fake renderer

**Steps:**
1. Write failing tests for renderer offline, timeout, stale acknowledgement, duplicate scene, oversized payload, auth failure, and reconnect.
2. Add bounded queue/coalescing so old visual updates cannot overwhelm or replay after reconnection.
3. Require renderer acknowledgement correlated to scene revision.
4. On bridge/source failure, send or retain only the explicit degraded/idle scene allowed by the contract.
5. Keep renderer commands allowlisted and data-only.
6. Run `cargo test -p arda-relic-bridge` and Clippy.
7. Commit: `fix(relic): harden scene delivery and recovery`.

## Task 4: Freeze truthful visual mapping

**Files:**
- Modify: `outposts/arda-relic-bridge/src/scene_adapter.rs`
- Modify external CITADEL renderer in its own repository/branch only if explicitly in scope
- Create mapping fixture/golden tests in the owning repositories

**Steps:**
1. Define one visual primitive for active work, delegation, waiting, approval, failure, and stale/disconnected.
2. Write golden mapping tests from runtime-presence fixtures to scene documents.
3. Include color-independent and reduced-motion distinctions.
4. Ensure high activity remains legible and bounded rather than becoming noise.
5. Preserve provenance: each visible active entity is traceable to a current receipt/runtime event.
6. Commit Arda and external-renderer changes separately with cross-referenced contract version.

## Task 5: Integrate RELIC lifecycle into Phase 1

**Files:**
- Modify: `config/systemd/arda-relic-bridge.service`
- Modify Phase 1 lifecycle component config/projection
- Update canonical installation docs/scripts
- Test: service hardening and optional-component degradation

**Steps:**
1. Verify the unit runs unprivileged with minimum network/filesystem access.
2. Mark RELIC optional: failure degrades embodiment but does not take down Hermes/Arda continuity.
3. Expose bridge and renderer health/freshness to Launcher without generic dashboard clutter.
4. Add bounded Start/Retry action ids only.
5. Verify restart and renderer disconnect recovery.
6. Commit: `feat(launcher): report RELIC embodiment health`.

## Task 6: Physical runtime acceptance and soak

**Run:**
1. Start Arda through Launcher.
2. Start one genuine Hermes task that delegates to at least one subagent and reaches a wait/approval/failure or completion boundary.
3. Observe corresponding RELIC entities and transitions on the physical display.
4. Correlate visible scene revisions to runtime receipts.
5. Stop the producer; confirm activity expires to stale/degraded.
6. Disconnect renderer network/power; confirm Launcher degradation and bounded bridge recovery.
7. Restart bridge and renderer; confirm no old activity replays as current.
8. Run an extended normal-use soak and measure dropped/coalesced updates, CPU, memory, and renderer frame health.

## Phase gate

Phase 5 is **proven** only when the physical RELIC display tracks a genuine Hermes/Arda delegation flow, each active visualization is traceable to fresh runtime evidence, and disconnect/restart produces an honest degraded state. A local fixture renderer, screenshot, or decorative animation is not physical acceptance.
