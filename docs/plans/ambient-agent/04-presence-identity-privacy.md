---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "implementation_plan"
  owner: "HERMES"
  status: "active"
  reviewed: "2026-08-20"
---

> 🜏 Soterion: 📜 implementation_plan | owner: HERMES | status: active | reviewed: 2026-08-20

# Phase 4: Presence, Identity, and Privacy Implementation Plan

> **Planning-only hold:** Do not implement until core Arda and the monitor-first Mirromere avatar/voice experience are accepted by the operator. Do not activate camera or biometric collection without separate explicit operator setup and visible local disable control.

**Goal:** Let the already-working Mirromere avatar move from passive/private behavior to a privacy-appropriate greeting and personalization when the operator arrives, while preserving strict separation between presence, identity confidence, data visibility, and consequential authorization.

**Architecture:** A local outpost sidecar combines enrolled, revocable signals into short-lived operator-presence claims. Raw sensor material remains local. Arda consumes only typed claims with factor classes, confidence, privacy ceiling, expiry, and provenance. Hermes uses the claim to prepare context; explicit handoff and action gates remain separate.

**Tech stack:** Rust, `arda-outpost-protocol`, local BLE/NFC providers, optional local camera/liveness provider, Tauri privacy controls, Vairë policy references, systemd user service.

---

## Safety model

Three distinct decisions must remain separate:

| Decision | Meaning | Example consequence |
|---|---|---|
| Presence | likely person/device in a room | wake a blank/ambient surface |
| Identity | bounded confidence about enrolled operator | permit a private greeting or safe context preparation |
| Authorization | explicit/scoped permission for an action | expose sensitive data or execute a consequential operation |

No single RFID UID, BLE advertisement, face match, voice match, or camera observation grants all three. Presence claims are expiring evidence, not permanent login tokens.

## Current source baseline

- `outposts/arda-outpost-protocol/src/presence.rs` defines runtime-presence semantics used for operational/RELIC projection. Do not repurpose it for biometric/operator identity.
- `outposts/arda-outpost-protocol/src/authority.rs` already models bounded authority concepts; extend through explicit public contracts instead of embedding policy in providers.
- Phase 2 defines authenticated session/operator lineage; Phase 3 defines display privacy classes and veil behavior.

## Operator-presence contract

Add `arda.operator-presence.v1` with:

- claim id, outpost/room id, enrolled subject reference;
- factor classes, never raw secret/template material;
- factor observations with issued/observed/expiry times and provider identity;
- confidence and deterministic policy disposition;
- visibility ceiling and authorization ceiling;
- `present | departed | uncertain | revoked | unavailable`;
- privacy mode and local hardware-disable state;
- provenance/receipt links;
- no face embedding, raw frame/audio, RFID static UID, or precise unnecessary location history.

## Task 1: Define strict presence claims and policy reduction

**Files:**
- Create: `outposts/arda-outpost-protocol/src/operator_presence.rs`
- Modify: `outposts/arda-outpost-protocol/src/lib.rs`
- Test: module unit tests and strict JSON fixtures

**Steps:**
1. Write failing tests for single weak factor, two-factor agreement, disagreement, expired factor, revoked enrollment, disabled sensor, unknown field, replay, and visibility ceiling.
2. Implement bounded Serde types and a pure policy reducer.
3. Make confidence descriptive evidence; policy disposition is deterministic and separately represented.
4. Prohibit any claim from carrying a direct consequential-action grant.
5. Run `cargo test -p arda-outpost-protocol operator_presence` and Clippy.
6. Commit: `feat(outpost): define operator presence claims`.

## Task 2: Build a simulator before hardware providers

**Files:**
- Create crate: `outposts/arda-presence-sidecar/` using workspace conventions
- Modify: root `Cargo.toml` and `Cargo.lock` in a serialized integration packet
- Test: crate tests and integration fixtures

**Steps:**
1. Add failing tests for arrive, depart, signal expiry, factor disagreement, provider restart, and revocation.
2. Implement an in-memory/file-fixture provider available only in explicit test/dev mode.
3. Emit claims through the real contract and transport path; fixture mode must be tagged and rejected by live acceptance.
4. Add bounded local status endpoint with no raw factors.
5. Run tests/Clippy and commit: `feat(presence): add deterministic claim simulator`.

## Task 3: Implement enrollment, revocation, and local privacy authority

**Files:**
- Add modules under `outposts/arda-presence-sidecar/src/`
- Add configuration schema under the repository's existing `config/` convention
- Test: encrypted/permissioned storage, revocation, and restart tests

**Steps:**
1. Write failing tests proving an unenrolled, revoked, copied, or expired credential cannot identify the operator.
2. Store enrollment references with `0600`-equivalent permissions and no tracked secrets.
3. Add a local privacy mode that immediately stops sensor intake, expires claims, and drives Mirromere veil state.
4. Add retention limits and explicit delete/re-enroll flow.
5. Receipt enrollment/revocation without logging raw identifiers.
6. Commit: `feat(presence): govern enrollment and privacy`.

## Task 4: Add the first real non-biometric provider

**Preferred order:** authenticated phone/BLE or cryptographic BLE/NFC token. A basic static RFID tag is presence-only and cannot identify or authorize.

**Files:**
- Add provider module under `outposts/arda-presence-sidecar/src/providers/`
- Add provider tests with captured/redacted protocol fixtures
- Update operator setup documentation only after implementation

**Steps:**
1. Choose hardware/protocol with challenge-response or OS-authenticated identity support.
2. Write failing tests for replay, cloned/static id, signal loss, rotation, and out-of-range recovery.
3. Implement provider with no raw identifier in logs or upstream claims.
4. Map it to a bounded factor class and TTL.
5. Test physical arrival/departure repeatedly before assigning any private visibility.
6. Commit: `feat(presence): add enrolled proximity provider`.

## Task 5: Add optional local camera/liveness as a separate provider

**Prerequisite:** explicit operator opt-in, physical camera-disable path, threat/privacy review, and Phase 4 Tasks 1–4 proven.

**Files:**
- Separate provider process/module under `outposts/arda-presence-sidecar/`
- Local-only model/config assets outside Git where required
- Tests using consented synthetic/local fixtures only

**Steps:**
1. Write failing tests for camera disabled, no face, multiple people, spoof attempt, low confidence, model unavailable, and template revocation.
2. Process frames locally and discard promptly; emit only the bounded factor result.
3. Require liveness and another enrolled factor before private personalization.
4. Expose a persistent visible camera-active/privacy indicator on Mirromere.
5. Document false-positive/negative limits; never frame this as safety-critical authentication.
6. Commit separately: `feat(presence): add opt-in local visual factor`.

## Task 6: Wire presence into Mirromere and continuity

**Files:**
- Modify Phase 3 backend Mirromere projection
- Modify Phase 2 handoff policy path
- Add read-only status to Launcher/HUD where appropriate
- Test cross-phase privacy scenarios

**Steps:**
1. Write failing tests: weak presence wakes only `PassiveMirror`; sufficient identity may enter a privacy-safe `AvatarPresence` greeting; shared-room signal veils private context; expiry returns to passive/private behavior.
2. Add context preparation by references only; do not expose transcript content before policy allows.
3. Presence may wake or personalize the avatar but cannot expose a transcript, approve an action, or create a new conversation identity.
4. Ensure business/private domain boundaries survive the same physical presence.
5. Commit: `feat(mirromere): apply bounded presence policy`.

## Task 7: Deploy and recover the local sidecar

**Files:**
- Create: `config/systemd/arda-presence-sidecar.service`
- Update canonical installer and `config/systemd/README.md`
- Test: unit verification and restart/disconnect soak

**Steps:**
1. Run as an unprivileged user with minimum device permissions.
2. Add hardening and bounded restart behavior.
3. Make absence optional/degraded for the broader Arda runtime; it must not prevent manual use.
4. Verify sensor/device removal, sidecar crash, malformed input, and restart all expire claims safely.
5. Commit: `feat(presence): supervise local presence sidecar`.

## Phase gate

Phase 4 is **proven** when the operator-approved Mirromere avatar already works, a real enrolled non-biometric signal moves it from passive/private behavior to a privacy-appropriate greeting, and expiry/departure returns it safely. Presence must not expose prior conversation, authorize action, or substitute for the Phase 3 voice/dialogue system. Camera-based recognition is optional and cannot be required to close this phase.
