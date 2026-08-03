# RELIC and CITADEL Runtime Presence Plan

> **For Hermes:** This is the canonical presence and presentation authority. Keep renderer, bridge, kiosk, scene semantics, accessibility, display-safe companion state, and CITADEL application recovery here. Do not place these tasks in the Pi5, governed-learning, or Warden Research plans.

**Status:** Active optional projection plan; reconciled on 2026-07-31
**Target:** Optional Stage 5 projection beta; never a Workbench release-candidate dependency
**Source design:** `docs/MIRROMERE_RELIC_OUTPOST_VISION.md`  
**Provenance decision:** external sidecar/protocol integration only; no source migration from `/var/home/mythos/Eregion/relic-kiosk`
**Shared node operations:** [Pi5 deployment/fleet/recovery](../archive/2026-07-23-pi5-outpost-integration-plan.md)

## Goal and boundary

Arda produces a sanitized, versioned runtime-presence graph. A read-only bridge converts fresh snapshots into scene state. RELIC renders locally and falls back to an explicit last-valid/idle-degraded state. CITADEL owns kiosk/display/application recovery; Arda core owns identity, provenance, policy, and revocation.

RELIC may display lifecycle, health, run/task correlation, handoffs, bounded resource pressure, freshness, confidence, approval waiting, and failure. It may not issue model/tool requests, mutate work, expose prompts/messages/secrets/private health data, or invent activity from decorative motion.

## Stage 4/live-source audit

### Complete foundations

- [x] External prototype provenance was audited in `docs/research/2026-07-30-mirromere-relic-provenance-audit.md`; missing root license/Git provenance requires protocol-sidecar integration.
- [x] `arda.runtime-presence.v1` exists in `outposts/arda-outpost-protocol/src/presence.rs` with schema/example fixtures under `spec/runtime-presence/v1/` and focused tests.
- [x] Invalid, expired, unsupported, or unverifiable projections resolve to `idle_degraded`; unknown payload fields are rejected.

### Partial implementation discovered by reconciliation

- `crates/spine/observability/arda-aule/src/presence_projection.rs` now builds a deterministic sanitized projection and has stale/unknown-input tests.
- `crates/engine/src/harness/presence.rs` now exposes `GET /v1/presence/snapshot` and `GET /v1/presence/events`; `crates/engine/tests/harness_presence.rs` covers loopback and a capability-shaped remote path.
- RC-1 is now partially closed: projection IDs are derived from canonicalized inputs plus an injected clock; `HarnessPresenceState` stores the latest bounded inputs; and snapshot/SSE routes publish that state. Remote enrollment remains hard-coded and is still an explicit fail-closed follow-up.
- No repository-owned RELIC bridge crate, RELIC systemd unit, deploy/verify helper, or canonical `apps/relic` renderer exists.
- The external CITADEL sidecar remains independently recoverable and still consumes the legacy scripted scene contract; its operational state must be reverified immediately before deployment work.

Stage 4 Workbench acceptance does not imply RELIC/CITADEL completion. Presence work remained optional and isolated throughout Stage 4.

## Ownership boundary after reconciliation

| Concern | Exclusive owner |
|---|---|
| runtime-presence schema, producer, publication, bridge, scene semantics, renderer, kiosk, display accessibility, application recovery/soak | this plan |
| generic Pi fleet inventory, SSH reachability, node-level restart/reboot helpers | Pi5 plan |
| Warden/Varda backend receipts and learning authority | governed-learning plan |
| research questions, watchlists, briefs, research HUD | Warden Research plan |

## Open presence/presentation tasks — exclusive ownership

### RC-1 — Connect the presence projection to live runtime truth

**Files:** `arda-aule/src/presence_projection.rs`, `arda-engine/src/harness/presence.rs`, harness state/identity integration, and focused tests

- [x] Replace fixed time/ID generation with deterministic IDs derived from canonical input receipts plus an injected/testable clock.
- [x] Feed bounded engine service, run graph, approval/handoff, provider, and resource inputs into stored `HarnessPresenceState`.
- [x] Make `update_inputs` persist the latest bounded snapshot and make snapshot/SSE routes publish that state rather than empty fixtures.
- [ ] Load enrolled outpost identity, `presence.read` capability, revocation, and allowed network posture from canonical contracts; remove hard-coded bearer authority.
- [x] Prove stale/missing inputs reduce confidence and scene state instead of inventing nodes, edges, or motion.

**Acceptance:** fixed receipts produce a deterministic projection; live harness input changes produce fresh snapshots/events; revoked, stale, malformed, or unauthorized callers fail closed.

**Evidence (2026-08-03):** `cargo test -p arda-aule presence_projection --all-features -- --test-threads=1` passed 7 tests; `cargo test -p arda-engine --test harness_presence -- --test-threads=1` passed 6 tests, including `presence_snapshot_publishes_updated_live_inputs`. Identity enrollment/revocation loading remains open.

### RC-2 — Add the Arda-owned read-only RELIC bridge

**Files**

- Create: `outposts/arda-relic-bridge/Cargo.toml`
- Create bridge client/cache/scene-adapter modules and focused replay/expiry/redaction/reconnect/corruption tests.

**Open work**

- [x] Consume only `arda.runtime-presence.v1` through `presence.read`.
- [x] Validate schema, time window, redaction class, source receipts, and monotonic snapshot sequence.
- [x] Cache the last valid sanitized snapshot atomically and expose its age.
- [x] On network loss, show last-valid timestamp and transition to idle-degraded after expiry.
- [ ] Emit the external sidecar's scene adapter as a protocol boundary without copying or modifying unlicensed source.

**Evidence (2026-08-03):** Added `outposts/arda-relic-bridge`. `cargo test -p arda-relic-bridge --all-features -- --test-threads=1` passed 3 tests covering valid receipt acceptance/age, monotonic sequence rejection, expiry to `idle_degraded`, strict unknown-field rejection, and unverifiable receipt rejection. Renderer/sidecar adapter emission remains open.

### RC-3 — Canonical presentation semantics

**Depends on:** RC-1 and RC-2

- [ ] Define a versioned registry from lifecycle/health/edge/freshness states to geometry, color, motion, fusion, and text fallback.
- [ ] Provide legend/operator inspection so appearance is not an undocumented codebook.
- [ ] Derive all motion from state/event transitions, never decorative activity timers.
- [ ] Support reduced motion, reduced brightness, high contrast, no-audio default, and text-only status.
- [ ] Keep static fixture, live harness, cached last-valid, and idle-degraded modes visibly distinct.

The separately maintained sidecar remains the default renderer for the first projection beta. A clean-room `apps/relic` implementation is a later decision after the protocol and semantics stabilize; it is not an open release task now.

### RC-4 — Package, deploy, recover, and soak CITADEL presence

**Depends on:** RC-1 through RC-3 and Pi5 PI5-2/PI5-3 shared node checks

**Files**

- Create: `config/systemd/arda-relic-bridge.service`
- Create: `config/systemd/arda-relic-kiosk.service` only if replacing the existing external unit is explicitly approved.
- Create: `scripts/deploy_relic_citadel.sh`
- Create: `scripts/verify_relic_citadel.sh`

**Open work**

- [ ] Build/checksum the bridge, preserve the prior sidecar and units, and require explicit operator deployment approval.
- [ ] Verify bridge, renderer, kiosk, display, network-loss, cache-corruption, reboot, rollback, and stale-scene behavior independently.
- [ ] Prove renderer compromise cannot mutate Arda and no private content crosses the projection.
- [ ] Complete a seven-day kiosk soak within thermal, memory, storage, and restart budgets.
- [ ] Verify immediate low-stimulation and text-only switching on the physical display.

### RC-5 — Optional companion and collaboration expansion

**Blocked until:** RC-1 through RC-4 projection beta acceptance

- [ ] Add council fusion only when run/council edges carry real receipt IDs and dissolve when evidence expires.
- [ ] Add chat or approval notifications only through separate bounded contracts that route mutations back through normal kernel approval.
- [ ] Add any broader HUD companion state only as sanitized presentation data with independent stale/offline behavior.

RC-5 is optional expansion. Do not begin it during Stage 5 reconciliation or before the base presence beta is accepted.

## Stage 5 dependency

- RELIC/CITADEL is feature-flagged and independently recoverable.
- No RC task blocks the Workbench release candidate, Warden backend, or Warden Research beta.
- If the projection beta is selected, RC-1 through RC-4 become its own release gate; RC-5 remains deferred.
- Generic node checks/restart mechanics come from the Pi5 plan, but application-specific bridge/kiosk recovery and soak remain here.

## Verification

```bash
cargo test --manifest-path outposts/arda-outpost-protocol/Cargo.toml --all-features -- --test-threads=1
cargo test -p arda-aule presence_projection --all-features -- --test-threads=1
cargo test -p arda-engine --test harness_presence -- --test-threads=1
cargo test --manifest-path outposts/arda-relic-bridge/Cargo.toml --all-features -- --test-threads=1
```

Run bridge/application/deployment commands only after their files exist. A separately approved physical deployment must also pass `scripts/verify_relic_citadel.sh` and record rollback plus soak evidence.

## Projection beta acceptance

- [x] Presence schema and degraded-state contract are versioned and tested.
- [ ] Every rendered active form/edge traces to fresh runtime receipts from live harness state.
- [ ] Stale, disconnected, cached, and fixture modes are unmistakable.
- [ ] Remote identity/capability/revocation is canonical and fail-closed.
- [ ] Renderer/bridge cannot mutate Arda or expose private content.
- [ ] CITADEL recovers after process failure, network loss, reboot, and rollback.
- [ ] Seven-day soak and physical accessibility checks pass.

External-person evaluation is optional supplementary confidence while no separate evaluator or clean machine is available; protocol fixtures, operator display acceptance, and recovery/soak evidence are the active gates.