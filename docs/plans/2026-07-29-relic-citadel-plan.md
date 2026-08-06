# RELIC and CITADEL Runtime Presence Plan

> **For Hermes:** This is the canonical presence and presentation authority. Keep renderer, bridge, kiosk, scene semantics, accessibility, display-safe companion state, and CITADEL application recovery here. Do not place these tasks in the Pi5, governed-learning, or Warden Research plans.

## Status

**Status:** Active final soak gate; implementation/recovery reconciled on 2026-08-04

**Live connection:** RELIC/CITADEL transport boundary implemented and verified as of 2026-08-03. The Arda harness now listens on `127.0.0.1:7878`, the `arda-relic-presence-sync` bridge service is active, and CITADEL's `relic.service` (port 8091) and `citadel-kiosk.service` are running. The three-shape geometric visual now responds to live Arda presence via the `arda.relic.scene-adapter.v1` contract instead of the external scripted renderer.
**Source design:** `docs/plans/MIRROMERE_RELIC_OUTPOST_VISION.md`
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

### Implemented runtime state after reconciliation

- `crates/spine/observability/arda-aule/src/presence_projection.rs` now builds a deterministic sanitized projection and has stale/unknown-input tests.
- `crates/engine/src/harness/presence.rs` now exposes `GET /v1/presence/snapshot` and `GET /v1/presence/events`; `crates/engine/tests/harness_presence.rs` covers loopback and a capability-shaped remote path.
- RC-1's transport and authorization boundary is closed: projection IDs are derived from canonicalized inputs plus an injected clock; `HarnessPresenceState` stores the latest bounded inputs; snapshot/SSE routes publish that state; and remote access loads the strict `arda.outpost-access.v1` contract from `config/outposts/access.toml`. Bearer values stay outside Git in the contract-named environment variable, and absent secrets, revocation, wrong capabilities, disallowed network posture, malformed contracts, or spoofed forwarding posture fail closed.
- The repository-owned `arda-relic-bridge` crate, systemd unit, deployment/verification helpers, and seven-day soak collector exist. The external renderer remains outside Arda under the protocol-sidecar provenance decision.
- The CITADEL sidecar consumes `arda.relic.scene-adapter.v1`; live, expired, disconnected, and unverifiable inputs resolve through the same receipt-validating bridge rather than the legacy scripted activity path.

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
- [x] Load enrolled outpost identity, `presence.read` capability, revocation, and allowed network posture from canonical contracts; remove hard-coded bearer authority.
- [x] Prove stale/missing inputs reduce confidence and scene state instead of inventing nodes, edges, or motion.

**Acceptance:** fixed receipts produce a deterministic projection; live harness input changes produce fresh snapshots/events; revoked, stale, malformed, or unauthorized callers fail closed.

**Evidence (2026-08-04):** `cargo test -p arda-aule presence_projection --all-features -- --test-threads=1` passes 7 tests. `cargo test -p arda-engine --test harness_presence -- --test-threads=1` passes 8 tests, including live-input publication plus no-enrollment, valid capability, wrong capability, revocation, and disallowed-network cases. `cargo test -p arda-outpost-protocol access -- --test-threads=1` passes 3 strict contract tests. The release `arda` binary was rebuilt, installed, restarted, and returned `ok` from `127.0.0.1:7878/health`; unresolved bearer environment variables intentionally leave remote access disabled.

### RC-2 — Add the Arda-owned read-only RELIC bridge

**Files**

- Create: `outposts/arda-relic-bridge/Cargo.toml`
- Create bridge client/cache/scene-adapter modules and focused replay/expiry/redaction/reconnect/corruption tests.

**Open work**

- [x] Consume only `arda.runtime-presence.v1` through `presence.read`.
- [x] Validate schema, time window, redaction class, source receipts, and monotonic snapshot sequence.
- [x] Cache the last valid sanitized snapshot atomically and expose its age.
- [x] On network loss, show last-valid timestamp and transition to idle-degraded after expiry.
- [x] Emit the external sidecar's scene adapter as a protocol boundary without copying or modifying unlicensed source.

The bridge exposes a clean-room `arda.relic.scene-adapter.v1` presentation
state in `scene_adapter.rs`. `arda-relic-presence-sync` fetches the harness
snapshot, applies the bridge's fail-closed validation, and atomically writes
the adapter state. The external RELIC sidecar now consumes that adapter while
retaining the existing renderer and generated Three.js payload outside Arda.

**Evidence (2026-08-03):** `cargo test -p arda-relic-bridge --all-features -- --test-threads=1` passed 5 tests; the release sync binary produced an active adapter with 3 receipt-backed forms from a fresh snapshot and produced `idle_degraded` for an expired/network-failed snapshot; external `npm run validate` passed 5 tests covering adapter consumption and no-invention degraded behavior.

### RC-3 — Canonical presentation semantics

**Depends on:** RC-1 and RC-2

- [x] Define a versioned registry from lifecycle/health/edge/freshness states to geometry, color, motion, fusion, and text fallback.
- [x] Provide legend/operator inspection so appearance is not an undocumented codebook.
- [x] Derive all motion from state/event transitions, never decorative activity timers.
- [x] Support reduced motion, reduced brightness, high contrast, no-audio default, and text-only status.
- [x] Keep static fixture, live harness, cached last-valid, and idle-degraded modes visibly distinct.

**Evidence (2026-08-03):** `outposts/arda-relic-bridge/src/scene_adapter.rs`
adds the versioned renderer boundary, receipt-carrying forms, explicit legend,
state-derived motion, degraded text, and accessibility profile. Focused bridge
tests pass; the renderer's `normalizeRelicSceneState` maps `active` adapter
state to `mode=static` with live forms, maps `idle_degraded` to an empty
`active_agents` list with `degraded=true` and `brightness=0.55`, and retains the
kiosk-safe scripted fixture only as the schema-mismatch/missing-file fallback.
Live transport/renderer consumption exercised end-to-end (see RC-4 evidence).

The separately maintained sidecar remains the default renderer for the first projection beta. A clean-room `apps/relic` implementation is a later decision after the protocol and semantics stabilize; it is not an open release task now.

### RC-4 — Package, deploy, recover, and soak CITADEL presence

**Depends on:** RC-1 through RC-3 and Pi5 PI5-2/PI5-3 shared node checks

**Files**

- Create: `config/systemd/arda-relic-bridge.service`
- Create: `config/systemd/arda-relic-kiosk.service` only if replacing the existing external unit is explicitly approved.
- Create: `scripts/deploy_relic_citadel.sh`
- Create: `scripts/verify_relic_citadel.sh`

The deploy helper is explicit-approval gated: it defaults to `--dry-run` and
only mutates the local bridge service or CITADEL when invoked with `--apply`.

**Open work**

- [x] Build/checksum the bridge, preserve the prior sidecar and units, and require explicit operator deployment approval (`--apply`).
- [x] Verify bridge, renderer, kiosk, display, network-loss, cache-corruption, reboot, rollback, and stale-scene behavior independently.
- [x] Prove renderer compromise cannot mutate Arda and no private content crosses the projection.
- [ ] Complete a seven-day kiosk soak within thermal, memory, storage, and restart budgets.
- [x] Verify immediate low-stimulation and text-only switching on the physical display.

**Evidence (2026-08-03):** The deploy helper now validates the sidecar,
builds the bridge binary, runs a live presence preflight against
`127.0.0.1:7878/v1/presence/snapshot` (rejecting schema/`sequence` mismatches),
backs up the prior remote sidecar into `.relic-backup/`, installs the binary and
unit, stages sidecar files atomically via `scp` + `install -m 0644`, refreshes
the kiosk, and is verified by `scripts/verify_relic_citadel.sh`. The bridge
unit depends on `arda.service` so it never starts before the harness is ready;
SSH copy failures retain prior state silently.

Verification run of `scripts/verify_relic_citadel.sh`:

```text
1. Local build: presence snapshot preflight — presence snapshot ok: seq=97 nodes=0
2. Bridge service: arda-relic-bridge.service — active
3. Local bridge runtime state — adapter schema ok: state=idle_degraded forms=0
4. Remote sidecar: scene.json schema — arda.relic.scene-adapter.v1
5. Remote services — relic.service active, citadel-kiosk.service active
6. Local sidecar validation — sidecar validate ok
7. Bridge crate tests — 5 passed
relic_citadel_verification=pass
```

**CITADEL hardware (Pi5, 2026-08-03):** disk 7.6G/117G (7%), RAM 903M/8G used,
temp ~84.5 C, `throttled=0xe0008` (soft temperature limit active, bit 3 only —
no under-voltage or hard throttle). Load average 1.72 under idle-rendered scene.
The soft thermal throttle is a known-environment ambient-heat condition; the
kiosk remains active and the scene state stays `idle_degraded` with `forms=[]`,
`brightness=0.55`. A seven-day soak is still required to confirm sustained
thermal and restart budgets. `scripts/relic_citadel_soak.py sample` collects
the local/remote scene schemas and sizes, service state/restarts/memory, Pi
temperature/throttle flags, and disk headroom; `evaluate` enforces the window.

**Recovery correction and new soak baseline (2026-08-04):** live inspection found
the installed bridge unit incorrectly launched the one-shot sync binary without
arguments, causing 383 failed restart attempts. The unit now runs the continuous
`relic_presence_sync.sh` transport, and deployment installs that script alongside
the binary. A forced bridge-process failure incremented `NRestarts` from 0 to 1
and recovered to `active`; an unreachable endpoint plus a corrupted local scene
was atomically replaced by a valid empty `idle_degraded` adapter. CITADEL was then
rebooted: `relic.service` and `citadel-kiosk.service` returned active, and the
remote adapter resumed. `scripts/verify_relic_citadel.sh` passed all seven gates.

The seven-day gate was reset after those recovery exercises. Its first sample is
`~/.local/state/arda/relic-soak/sample-20260804T162511Z.json`; daily samples are
scheduled through 2026-08-11, followed by `scripts/relic_citadel_soak.py evaluate`.
The current evaluator fails only the required elapsed-window/sample-count checks.

### RC-5 — Post-plan optional companion and collaboration backlog

**Blocked until:** RC-1 through RC-4 projection beta acceptance

- [ ] Add council fusion only when run/council edges carry real receipt IDs and dissolve when evidence expires.
- [ ] Add chat or approval notifications only through separate bounded contracts that route mutations back through normal kernel approval.
- [ ] Add any broader HUD companion state only as sanitized presentation data with independent stale/offline behavior.

RC-5 is optional expansion and is not a closure gate for this plan. Do not begin it during Stage 5 reconciliation or before the base presence beta is accepted.

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
- [x] Transport/sidecar consumption boundary is wired and verified (`arda.relic.scene-adapter.v1`).
- [x] Arda harness listens on `127.0.0.1:7878`; `arda-relic-presence-sync` bridge is active.
- [x] Every rendered active form/edge traces to fresh runtime receipts from live harness state.
- [x] Stale, disconnected, cached, and fixture modes are unmistakable.
- [x] Remote identity/capability/revocation is canonical and fail-closed.
- [x] Renderer/bridge cannot mutate Arda or expose private content.
- [x] CITADEL recovers after process failure, network loss, reboot, and rollback.
- [ ] Seven-day soak passes (physical accessibility switching already passed).

External-person evaluation is optional supplementary confidence while no separate evaluator or clean machine is available; protocol fixtures, operator display acceptance, and recovery/soak evidence are the active gates.