# RELIC and CITADEL Runtime Presence Implementation Plan

> **For Hermes:** Preserve the outpost placement rule. RELIC/CITADEL code belongs under `outposts/` or `apps/`, never inside `crates/spine/`. Verify live Pi state before making operational claims or deployments.

**Goal:** Convert Arda runtime truth into a legible geometric presence display on desktop and CITADEL Pi hardware without giving the renderer execution authority or exposing private task content.

**Architecture:** A core projection producer emits a sanitized, versioned runtime-presence graph. A read-only bridge converts it to scene state. RELIC renders that state locally and can operate offline on the last valid snapshot. CITADEL owns kiosk, display, hardware, and recovery behavior; Arda core owns identity, provenance, policy, and revocation.

**Tech stack:** Rust projection/bridge, JSON Schema, WebSocket/SSE, Three.js/R3F, Tauri or static kiosk build, systemd user services on Pi5.

**Target stage:** Stage 5 projection beta; optional 1.0 companion, never a Workbench critical dependency  
**Source design:** `docs/MIRROMERE_RELIC_OUTPOST_VISION.md`  
**Existing prototype:** `/var/home/mythos/Eregion/relic-kiosk` is external and must not be silently copied or modified without provenance and operator review.
**Provenance audit:** `docs/research/2026-07-30-mirromere-relic-provenance-audit.md`; external sidecar/protocol integration is the approved posture, with no source migration.

---

## Verified starting point

- CITADEL live state was reverified after reboot on 2026-07-30: Raspberry Pi 5 Model B Rev 1.1, Debian 13, 8 GiB RAM, DSI-2 at 800x800/63 Hz, and no thermal throttling. `relic.service` and `citadel-kiosk.service` are enabled/active, `citadel-companion.service` is disabled/inactive, Chromium targets `http://127.0.0.1:8091/`, and the old `annunimas.relic.scene.v1` scripted fixture is still the active display input.
- The external prototype implements black-field Three.js geometry, up to three agents, council fusion, static/scripted modes, and an old `annunimas.relic.scene.v1` contract.
- `core/state/embodied_interface.json` and `core/state/tauri_embodiment.json` preserve useful rendering doctrine but retain legacy schema identities.
- `outposts/arda-outpost-protocol` already defines observation and authority boundaries and is the correct shared outpost contract location.
- `docs/plans/EMBODIED_INTERFACE.md` contains stale references; this plan follows live paths and the newer vision document.

## Product boundary

RELIC displays:
- agent/service lifecycle;
- task/run correlation without private prompt content;
- collaboration/handoff edges;
- bounded resource pressure;
- freshness, degraded state, and confidence;
- approval-waiting and failure states.

RELIC does not:
- issue model/tool requests;
- mutate tasks or services;
- render secrets, raw prompts, private messages, or health details;
- infer activity from decorative timers;
- claim an agent is active when projection evidence is stale.

## Phase 0 — Provenance and scene-contract freeze

### Task 0.1: Audit the external prototype — COMPLETE (2026-07-30)

**Read-only inputs**
- `/var/home/mythos/Eregion/relic-kiosk/README.md`
- package manifests, source tree, license, assets, deployment scripts, and current Git provenance.

**Deliverable**
- [x] Create: `docs/research/2026-07-30-mirromere-relic-provenance-audit.md`

**Gate**
Choose one explicitly:
1. migrate owned/permissively licensed code;
2. retain external sidecar and integrate by protocol;
3. reimplement only the documented scene contract.

**Decision:** retain the external artifact as an independently recoverable sidecar and integrate only by an Arda-owned protocol. The missing root license and unavailable Git provenance prohibit source migration. A clean-room implementation remains available after the scene protocol stabilizes.

### Task 0.2: Define `arda.runtime-presence.v1` — COMPLETE (2026-07-30)

**Files**
- [x] Create: `outposts/arda-outpost-protocol/src/presence.rs` and export it additively from the protocol crate.
- [x] Create: `spec/runtime-presence/v1/runtime-presence.schema.json`
- [x] Create: `spec/runtime-presence/v1/example.json`
- [x] Test: `outposts/arda-outpost-protocol/tests/runtime_presence.rs`

**Required fields**
- projection ID/version, generated/valid-until timestamps;
- realm/agent/service nodes;
- typed collaboration, handoff, wait, and dependency edges;
- run/task correlation IDs;
- lifecycle, health, confidence, freshness;
- bounded normalized resource pressure;
- source receipt references;
- redaction class.

**Acceptance**
- [x] Expired, unverifiable, unsupported, invalid-window, or out-of-range projections produce an explicit `idle_degraded` scene disposition.
- [x] Contract fixture contains no prompt, message, payload, secret, or private health-detail field; the Rust contract also rejects unknown payload fields.

**Evidence (2026-07-30)**
- `cargo test --manifest-path outposts/arda-outpost-protocol/Cargo.toml --all-features -- --test-threads=1` passed all 18 integration tests, including 7 `runtime_presence` tests.
- `npx --yes --package ajv-cli --package ajv-formats ajv validate --spec=draft2020 --strict=false -c ajv-formats -s spec/runtime-presence/v1/runtime-presence.schema.json -d spec/runtime-presence/v1/example.json` returned `example.json valid` with RFC 3339 date-time format checking enabled.
- `rustfmt --edition 2021 --check src/presence.rs src/lib.rs tests/runtime_presence.rs` and the scoped `git diff --check` both passed.
- No deployment mutation was performed during discovery; the current Pi sidecar remains the rollback-safe active surface while Phase 1 projection work begins.

## Phase 1 — Core runtime projection

### Task 1.1: Produce a sanitized presence graph

**Files**
- Create: `crates/spine/observability/arda-aule/src/presence_projection.rs`
- Modify: `crates/spine/observability/arda-aule/src/lib.rs` or the current public module root after inspection.
- Test: package-local projection tests.

**Inputs**
- engine service status;
- run graph state;
- Aulë telemetry;
- approval and handoff receipts;
- bounded provider/resource state.

**Acceptance**
- Projection is deterministic for fixed inputs.
- Unknown/stale input reduces confidence rather than inventing motion.

### Task 1.2: Publish through the existing harness

**Files**
- Create: `crates/engine/src/harness/presence.rs`
- Modify: `crates/engine/src/harness.rs`
- Test: `crates/engine/tests/harness_presence.rs`

**Endpoints**
- `GET /v1/presence/snapshot`
- `GET /v1/presence/events` via SSE or WebSocket

**Acceptance**
- Loopback default; remote CITADEL access requires enrolled outpost identity and a read-only capability.

## Phase 2 — Canonical RELIC application

### Task 2.1: Create or import the renderer after the provenance gate

**Files if canonicalized in Arda**
- Create: `apps/relic/package.json`
- Create: `apps/relic/src/main.ts`
- Create: `apps/relic/src/runtimePresence.ts`
- Create: `apps/relic/src/scene/RelicScene.ts`
- Create: `apps/relic/src/scene/geometryRegistry.ts`
- Create: `apps/relic/README.md`

**Acceptance**
- Static fixture, live harness, and last-known-valid modes are separate and visibly identified.
- Scene motion derives from event/state transitions.
- Software/WebGL fallback remains usable on Pi5.

### Task 2.2: Implement visual semantics

Map lifecycle and edge types to geometry, color, motion, and fusion through a versioned registry. Include a legend/operator inspect mode so appearance is not an undocumented codebook.

### Task 2.3: Add accessibility and low-stimulation modes

Support reduced motion, reduced brightness, high contrast, no-audio default, and a text status fallback.

## Phase 3 — CITADEL bridge and deployment

### Task 3.1: Add outpost bridge

**Files**
- Create: `outposts/arda-relic-bridge/Cargo.toml`
- Create: `outposts/arda-relic-bridge/src/main.rs`
- Create: `outposts/arda-relic-bridge/src/cache.rs`
- Create: `outposts/arda-relic-bridge/src/client.rs`
- Test replay, expiry, redaction, reconnect, and cache corruption.

**Acceptance**
- Bridge has read-only presence capability.
- Network loss shows last-valid timestamp and transitions to idle after expiry.

### Task 3.2: Package CITADEL services

**Files**
- Create: `config/systemd/arda-relic-bridge.service`
- Create: `config/systemd/arda-relic-kiosk.service`
- Create: `scripts/deploy_relic_citadel.sh`
- Create: `scripts/verify_relic_citadel.sh`

Remote deployment is operator-approved and must preserve a rollback to the prior kiosk.

## Phase 4 — Collaboration and council visualization

Add multi-agent handoffs and council fusion only after run graph edges carry real receipt IDs. A fused visual form must dissolve when evidence expires or members leave the run.

## Phase 5 — Optional companion projection

CITADEL may display bounded chat/approval notifications using separate contracts. It remains a presentation and advisory surface; mutations route back to the kernel's normal approval path.

## Verification ladder

```bash
cargo test --manifest-path outposts/arda-outpost-protocol/Cargo.toml --all-features -- --test-threads=1
cargo test -p arda-aule --all-features -- --test-threads=1
cargo test -p arda-engine --test harness_presence -- --test-threads=1
cargo test --manifest-path outposts/arda-relic-bridge/Cargo.toml --all-features -- --test-threads=1
cd apps/relic && pnpm test && pnpm build
```

For a separately approved live deployment:

```bash
bash scripts/verify_relic_citadel.sh
```

## Release acceptance

- Every active form and edge is traceable to a fresh runtime receipt.
- Stale/disconnected state is visually unmistakable.
- Renderer cannot mutate Arda even if compromised.
- Pi5 recovers after reboot and network loss.
- Seven-day kiosk soak stays within thermal, memory, storage, and restart limits.
- Operator can switch immediately to low-stimulation or text-only status.
