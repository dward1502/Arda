# Mirromere Ambient Avatar Implementation Plan

> **For Hermes:** Use `subagent-driven-development`. Implement monitor-first, local-first, and capability-by-capability; do not activate camera, microphone, identity, or physical action in a single broad packet.

**Goal:** Deliver a JARVIS-like first-party ambient interface that can speak, listen, display Arda state, capture personal operations input, and embody the orchestrator while remaining safe, quiet, and useful when sensors, models, or the network fail.

**Architecture:** Mirromere is a separately deployable Tauri application and outpost client. The Arda kernel owns goals, memory, routing, policy, and run graphs. Mirromere owns scenes, avatar rendering, local media pipelines, interaction state, and hardware-specific privacy behavior. The avatar is a projection and dialogue surface, not a new authority or duplicate orchestrator.

**Tech stack:** Tauri 2, Rust, React, React Three Fiber, Three.js/WebGPU with WebGL fallback, VRM, local STT/TTS sidecars, Manwe, Oromë, Vairë, Personal Operations API.

**Target stage:** Stage 5 monitor alpha; physical mirror beta after Stage 6 Workbench 1.0  
**Source design:** `docs/plans/Mirromere_PRD.md` and `docs/plans/MIRROMERE_RELIC_OUTPOST_VISION.md`
**Critical correction:** camera/face presence may provide context but never sole identity, authority, or clinical evidence.
**Provenance audit:** `docs/research/2026-07-30-mirromere-relic-provenance-audit.md`; no executable Mirromere prototype was found, so this plan remains design-only and clean-room.

---

## Verified starting point

- No `apps/mirromere/` application exists in the current workspace.
- `docs/plans/Mirromere_PRD.md` defines scenes, VRM direction, local speech, and a Rust-owned state machine, but includes aspirational names and authority that must be reconciled with current Manwe/Oromë/governance boundaries.
- `docs/plans/MIRROMERE_RELIC_OUTPOST_VISION.md` establishes current outpost, privacy, wellness, and evidence-class boundaries.
- ARDA HUD already has world, boardroom, presence, scene transition, and runtime source infrastructure that can be referenced but must not be copied wholesale.
- Voice, camera, wake-word, and physical action are not active default capabilities.
- No executable Mirromere prototype, dependency inventory, asset-rights record, or runtime recovery path exists in the inspected `/var/home/mythos/Eregion` tree.

## Product boundary

Mirromere may:
- render sanitized Arda state;
- accept text/voice capture;
- converse through governed model routing;
- request low-risk scene transitions;
- present approvals and reminders;
- emit receipted interaction and acknowledgement events.

Mirromere may not:
- independently approve system mutations;
- bypass Workbench or Personal Operations authority;
- infer medical status or change medication;
- silently identify, record, or upload people;
- expose prompts, secrets, or private task content in the ambient scene;
- command unrestricted shell, service, home, or device actions.

## Phase 0 — Reconcile and freeze contracts

### Task 0.1: Define Mirromere capability and consent manifests

**Files**
- Create: `spec/mirromere/v1/mirromere-manifest.schema.json`
- Create: `spec/mirromere/v1/interaction-events.schema.json`
- Create: `config/mirromere/default.toml`
- Create: `outposts/arda-outpost-protocol/src/presence.rs`
- Modify: `outposts/arda-outpost-protocol/src/lib.rs`
- Test: `outposts/arda-outpost-protocol/tests/presence_contract.rs`

**Required capabilities**
- display, speaker, microphone, camera features, wake input, RFID/NFC, text input, scene control, notification, and physical action;
- each classified as disabled, read-only, advisory, confirmation-gated, or prohibited;
- retention, remote projection, expiry, freshness, and physical mute evidence.

**Acceptance**
- Missing consent or stale sensor state fails closed.
- Raw audio/video cannot be represented as ordinary durable memory by default.

### Task 0.2: Define scene and avatar command contracts

**Files**
- Create: `spec/mirromere/v1/scene-contract.md`
- Create: `outposts/arda-outpost-protocol/src/scene.rs`
- Test: `outposts/arda-outpost-protocol/tests/scene_contract.rs`

**Scenes for first alpha**
- `Passive`
- `AvatarPresence`
- `ArdaStatus`
- `PersonalBrief`
- `Offline`
- `PrivacyMuted`

Dynamic agent-created scenes are deferred.

## Phase 1 — Monitor-first application shell

### Task 1.1: Scaffold the independent application

**Files**
- Create: `apps/mirromere/package.json`
- Create: `apps/mirromere/src-tauri/Cargo.toml`
- Create: `apps/mirromere/src-tauri/src/main.rs`
- Create: `apps/mirromere/src-tauri/src/lib.rs`
- Create: `apps/mirromere/src/main.tsx`
- Create: `apps/mirromere/src/App.tsx`
- Create: `apps/mirromere/README.md`
- Modify only after validation: root `Cargo.toml`, `apps/README.md`, `services.toml`

**Acceptance**
- App builds independently before workspace/supervisor registration.
- Headless daemon operation remains unaffected when Mirromere is absent.

### Task 1.2: Implement Rust-owned scene state

**Files**
- Create: `apps/mirromere/src-tauri/src/scene.rs`
- Create: `apps/mirromere/src/lib/scene.ts`
- Create tests on both sides of the Tauri serialization boundary.

**Acceptance**
- Invalid, expired, unauthorized, or unsupported transitions are rejected with operator-readable reasons.
- Offline and privacy-muted states are always reachable locally.

### Task 1.3: Render a basic VRM avatar and graceful fallback

**Files**
- Create: `apps/mirromere/src/scenes/AvatarPresenceScene.tsx`
- Create: `apps/mirromere/src/avatar/AvatarController.tsx`
- Create: `apps/mirromere/src/avatar/FallbackPresence.tsx`
- Create visual/performance smoke tests.

**Acceptance**
- A non-humanoid fallback renders if VRM/WebGPU fails.
- Reduced-motion mode removes materialization and idle micro-motion.

## Phase 2 — Text dialogue and Arda projection

### Task 2.1: Connect to the Arda harness, not internal service ports

**Files**
- Create: `apps/mirromere/src-tauri/src/arda_client.rs`
- Extend the engine harness with a sanitized ambient projection endpoint under `crates/engine/src/harness/`.
- Test stale, unavailable, and redacted projections.

**Acceptance**
- Mirromere receives current system health, active public-safe presence, next operator attention item, and Personal Operations brief without raw prompts or secrets.

### Task 2.2: Add typed text dialogue

Dialogue requests route through the kernel and Manwe. Responses return text, optional speech intent, expression, gesture, and source/receipt references. Avatar expression never substitutes for governance state.

## Phase 3 — Local voice pipeline

### Task 3.1: Integrate a supervised voice sidecar

**Files**
- Create: `adapters/mirromere-voice/arda_adapter.py`
- Create: `adapters/mirromere-voice/tests/`
- Create: `config/adapters/mirromere-voice.toml.example`
- Create: `apps/mirromere/src-tauri/src/voice.rs`

**Protocol**
- local VAD -> streaming STT -> typed dialogue -> TTS -> phoneme/viseme timing;
- raw audio ring buffer is ephemeral;
- operator can review transcript before governed action;
- hardware mute overrides software state.

**Acceptance**
- Mute state is visible, testable, and cannot be overridden remotely.
- Voice failure falls back to text without losing a capture.
- Latency is measured, not claimed: first alpha target is <1.5 seconds voice-to-first-audio on the primary PC.

### Task 3.2: Add lip-sync and interruption handling

Support barge-in, cancel, replay, transcript, and quiet mode. Spoken output stops immediately on local mute/stop.

## Phase 4 — Presence sensing, one capability at a time

### Task 4.1: Add non-identifying presence detection

Start with local person-present/absent features and freshness only. Do not persist frames. Do not enable face recognition.

### Task 4.2: Add RFID/NFC identity as a separate reviewed adapter

RFID/NFC may select an operator profile but still cannot confer high-risk system authority without kernel authentication/approval.

### Task 4.3: Evaluate face recognition only after privacy review

Default decision is defer. If implemented, it remains convenience context, opt-in, local, revocable, and never sole authentication.

## Phase 5 — Personal Operations and ambient routines

- Morning brief consumes `DailyBrief` projection.
- Universal capture writes to Personal Operations inbox.
- Reminders acknowledge/defer through Oromë receipts.
- “What was I doing?” uses the context resume card.
- Quiet windows and sensory intensity profiles are locally enforceable.

## Phase 6 — Physical mirror deployment

Only after monitor soak:

- optical calibration;
- kiosk recovery;
- thermal/resource monitoring;
- local hardware mute and shutdown;
- clear offline scene;
- signed update/rollback;
- physical privacy indicator verification.

## Verification ladder

```bash
cargo test -p arda-outpost-protocol --all-features -- --test-threads=1
cd apps/mirromere && pnpm test && pnpm lint && pnpm build
cargo test --manifest-path apps/mirromere/src-tauri/Cargo.toml --all-features -- --test-threads=1
python3 -m pytest adapters/mirromere-voice/tests -q
pnpm run tauri build
```

## Release acceptance

### Monitor alpha
- Five fixed scenes work with keyboard/text control.
- Dialogue, transcript, mute, offline, and recovery paths are understandable.
- No raw camera/audio persists by default.
- Seven-day desktop soak has no unbounded resource growth.

### Physical beta
- Local privacy controls remain effective when the network and Arda core are offline.
- Sensor state, consent, scene state, model route, and spoken/action receipts are inspectable.
- Operator finds morning brief, capture, and context recovery useful—not merely visually impressive.

## Sequencing guardrail

Do not let avatar polish delay Workbench Stage 4. The first Mirromere implementation may begin after Workbench contracts stabilize, but its physical deployment is not a 1.0 release dependency.
