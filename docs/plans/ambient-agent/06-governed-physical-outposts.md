---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "implementation_plan"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-08-17"
---

> 🜏 Soterion: 📜 implementation_plan | owner: PROMETHEUS | status: active | reviewed: 2026-08-17

# Phase 6: Governed Physical Outposts Implementation Plan

> **For Hermes:** Use the `subagent-driven-development` skill to implement this plan task-by-task. Prove the complete protocol with a simulator before connecting a real actuator.

**Goal:** Let Hermes propose and Arda govern typed physical-device work while each outpost enforces identity, capability, exactly-once execution, expiry, local safety, cancellation, revocation, and terminal receipts.

**Architecture:** Devices enroll through strict manifests and advertise capabilities. Hermes requests outcomes, never arbitrary shell/device commands. Arda classifies authority and produces an approved intent bound to exact artifact/parameter digests. The outpost validates locally, executes once, and returns a terminal observed receipt. Simulation closes the protocol before the first real 3D printer adapter.

**Tech stack:** Rust, `arda-outpost-protocol`, Serde/JSON schemas, authenticated transport, governance/approval contracts, SHA-256 artifact identity, simulator, device-local safety adapters.

---

## Current source baseline

- `outposts/arda-outpost-protocol/src/lib.rs` is the shared outpost boundary.
- `outposts/arda-outpost-protocol/src/authority.rs` contains current authority types.
- `presence.rs` is for read-only runtime presence and must not be stretched into actuation.
- Existing transport/contracts are foundations, not proof of safe device execution.

## Contract set

### `arda.outpost.manifest.v1`

- cryptographic outpost identity and enrollment state;
- software/protocol versions;
- capability ids and parameter schemas;
- observation/proposal/execution authority classes;
- health, calibration, local safety/interlock state;
- data retention/egress policy;
- revocation and manual-override information.

### `arda.outpost.intent.v1`

- objective and stable lineage;
- exact capability id/version;
- artifact digest/size/media type where applicable;
- canonical parameter digest and bounded parameters;
- risk/authority class and approval receipt reference;
- issuing operator/agent identity references;
- issued, not-before, expiry times;
- exactly-once key;
- cancel/compensation policy;
- no arbitrary shell, G-code passthrough, URL, or unvalidated free-form command.

### `arda.outpost.execution-receipt.v1`

- accepted/rejected/prepared/executing/cancelled/failed/completed;
- outpost and capability identity;
- intent and exactly-once key;
- observed start/end state;
- artifact and parameter digests actually used;
- local interlock/manual override state;
- bounded error and compensation result;
- terminal timestamp/signature/authentication evidence.

## Task 1: Define manifest and enrollment state machine

**Files:**
- Create: `outposts/arda-outpost-protocol/src/manifest.rs`
- Create: `outposts/arda-outpost-protocol/src/enrollment.rs`
- Modify: `outposts/arda-outpost-protocol/src/lib.rs`
- Test: strict fixtures and state-transition tests

**Steps:**
1. Write failing tests for valid manifest, unknown capability, duplicate identity, revoked device, incompatible version, stale health, invalid parameter schema, and authority escalation.
2. Implement strict bounded types and `unseen → pending → enrolled → suspended → revoked` transitions.
3. Bind enrollment to operator-reviewed identity material without committing secrets.
4. Make revocation fail closed immediately.
5. Run protocol tests/Clippy and commit: `feat(outpost): define manifest and enrollment`.

## Task 2: Define intent, approval binding, and execution receipt

**Files:**
- Create: `outposts/arda-outpost-protocol/src/intent.rs`
- Create: `outposts/arda-outpost-protocol/src/execution_receipt.rs`
- Modify: `authority.rs` only where one canonical authority type must be reused
- Test: strict transition/digest/replay fixtures

**Steps:**
1. Write failing tests for altered artifact, altered parameter, wrong outpost, expired approval, missing approval, duplicate execution key, cancellation, and terminal replay.
2. Implement canonical digest input and strict state transitions.
3. Require exact scoped approval for consequential capabilities.
4. Ensure model/council confidence cannot lower the authority class.
5. Run tests and commit: `feat(outpost): bind governed intents to receipts`.

## Task 3: Build authenticated bounded transport

**Files:**
- Add transport module/crate under the existing outpost protocol layout after tracing current transport conventions
- Test: local integration harness

**Steps:**
1. Write failing tests for unauthenticated peer, revoked certificate/key, replay, oversized body, wrong version, timeout, disconnect, and duplicate terminal response.
2. Use mutually authenticated transport suitable for LAN/mesh deployment; bind transport identity to enrolled outpost identity.
3. Add strict request/response size and time limits.
4. Persist intent/terminal receipt around execution boundaries for restart safety.
5. Do not expose a generic remote command endpoint.
6. Commit: `feat(outpost): add authenticated intent transport`.

## Task 4: Build a deterministic printer simulator

**Files:**
- Create crate: `outposts/arda-printer-simulator/`
- Modify root workspace manifests in a serialized packet
- Test: simulator unit and end-to-end integration tests

Capabilities:

- `printer.observe.v1` — read state;
- `printer.prepare.v1` — validate artifact/parameters and estimate bounded metadata;
- `printer.start.v1` — consequential, approval required;
- `printer.cancel.v1` — safety/recovery path.

**Steps:**
1. Write failing tests for prepare, approve/start, duplicate start, expiry, cancellation, simulated jam, restart during execution, and manual override.
2. Implement a finite-state simulator with persisted exactly-once keys.
3. Validate artifact digest and bounded print parameters; never execute G-code.
4. Emit terminal receipts and health.
5. Run tests/Clippy and commit: `feat(outpost): add governed printer simulator`.

## Task 5: Wire Hermes proposal and Arda approval flow

**Files:**
- Add a Hermes tool/plugin surface through the audited Phase 2 extension path
- Modify canonical governance/approval adapter only after tracing existing approval authority
- Add HUD/Launcher proposal status through backend projections
- Test full proposal/approval/deny/replay paths

**Steps:**
1. Write failing end-to-end test: Hermes proposes, simulator remains idle until exact approval.
2. Expose typed high-level inputs only: artifact reference, material/profile id, copies, and requested outcome.
3. Produce an inspectable proposal with risk, digest, parameters, device health, and cancel behavior.
4. Reuse existing append-only approval/recommendation authority; do not add a device-specific approval ledger.
5. After approval, emit exactly one intent and terminal receipt.
6. Test deny, alter-after-approval, expiry, duplicate, and cancellation.
7. Commit: `feat(outpost): connect proposal approval execution`.

## Task 6: Close the simulator vertical slice

**Run:**
1. Enroll the simulator as an outpost.
2. Ask Hermes to prepare a specific test artifact.
3. Inspect proposal and exact digest/parameters.
4. Approve the scoped start.
5. Verify one execution and terminal receipt.
6. Replay the same intent; verify no second execution.
7. Alter parameters/artifact; verify approval invalidation.
8. Simulate disconnect/restart/jam/manual override and verify safe recovery.
9. Revoke the outpost and verify further intents fail.

Only after this gate passes may a real actuator branch begin.

## Task 7: Integrate one real 3D printer

**Files:**
- Create one device-specific adapter crate under `outposts/`, named for the actual supported protocol/device class
- Add device-local setup/rollback documentation
- Test protocol adapter with fake server before hardware

**Steps:**
1. Identify the actual printer/protocol and read its official safety/API documentation.
2. Map only the four proven capability ids; no generic passthrough.
3. Require printer-local interlocks and physical cancel/override.
4. Run prepare-only against hardware first.
5. Run a low-risk approved test print while attended.
6. Test cancellation, disconnect, restart, wrong artifact, and revocation.
7. Record exact hardware/firmware/protocol support limits.
8. Commit adapter separately; do not claim robots or other printers are supported.

## Future device rule

Every additional robot, fabrication tool, sensor, relay, or environmental control receives its own adapter branch and safety case. Read-only sensors may use lower authority, but physical motion, access control, heat, sharp tools, chemicals, medical implications, financial effects, or human proximity require stronger local interlocks and explicit operator review.

## Phase gate

Phase 6 is **proven** when the simulator completes proposal → exact approval → one execution → terminal receipt through restart/replay/failure tests. Real-device support is **proven** separately only after an attended hardware run with local safety and cancellation. Success with one printer never generalizes to robots or unrestricted physical autonomy.
