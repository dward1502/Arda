# Pi5 Outpost Integration Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Make Warden and CITADEL first-class Arda outposts for bounded scouting, avatar/chat presentation, ARDA HUD companion projection, and advisory participation in autonomous council flows.

**Architecture:** Arda/Manwe remains the authority and tool-execution boundary. Warden contributes local inference and receipted scout evidence; CITADEL presents chat, RELIC, and a Pi-safe HUD projection while dispatching requests through governed APIs. Neither Pi gains independent approval authority or unrestricted internet shell access.

**Tech Stack:** OpenSSH over Tailscale, user systemd, Manwe OpenAI-compatible API, Arda scout/council JSONL contracts, RELIC/Three.js, ARDA HUD React companion-state export.

---

## Verified baseline — 2026-07-23

- `ssh warden` → `numenor@100.110.85.37`: operational with key auth, passwordless sudo, and linger.
- Warden `llama-server.service`: active/enabled on `0.0.0.0:1234`; model `Qwen3.5-4B-Q4_K_M.gguf`.
- `ssh citadel` and `ssh raspberrypi` → `citadel@100.119.130.127`: operational with key auth, passwordless sudo, and linger.
- CITADEL `relic.service`: active/enabled on `0.0.0.0:8091`.
- CITADEL `citadel-kiosk.service`: active/enabled and opens `http://127.0.0.1:8091/`.
- CITADEL has no deployed scout worker, chatbot bridge, council worker, or ARDA HUD companion service.
- Canonical role contract: `core/state/embodied_interface.json`.
- Canonical node connectivity: `config/fleet.toml`.

## Authority boundaries

- Scout nodes may fetch only through allowlisted Hermes/Arda web tools and must emit source links and receipts.
- Scout findings are evidence, not queue mutation or execution approval.
- Pi council participation is advisory. Final policy and approval remain in Arda governance.
- RELIC and the Pi HUD projection are presentation surfaces; rendering a decision does not authorize it.
- CITADEL should call Manwe rather than carry cloud-provider credentials.

### Task 1: Add a shared outpost envelope and fixtures

**Objective:** Define node identity, request, response, health, and receipt payloads used by both Pi services.

**Files:**
- Create: `crates/outposts/arda-outpost-protocol/Cargo.toml`
- Create: `crates/outposts/arda-outpost-protocol/src/lib.rs`
- Create: `crates/outposts/arda-outpost-protocol/tests/fixtures.rs`
- Modify: `Cargo.toml`
- Reference: `core/state/embodied_interface.json`

**Steps:**
1. Write fixture tests for `OutpostManifest`, `ScoutDispatch`, `ScoutFinding`, `CouncilEvidence`, `ChatTurn`, and `OutpostHealth` JSON round trips.
2. Assert authority fields distinguish `advisory`, `presentation`, and `execution_prohibited`.
3. Implement only the shared serde types and schema constants needed by the fixtures.
4. Run `cargo test -p arda-outpost-protocol` and expect all fixtures to pass.

### Task 2: Build the governed Warden scout worker

**Objective:** Let Warden accept bounded scout requests, use its local model or Manwe, and return source-bearing findings.

**Files:**
- Create: `crates/outposts/arda-warden-scout/Cargo.toml`
- Create: `crates/outposts/arda-warden-scout/src/main.rs`
- Create: `crates/outposts/arda-warden-scout/tests/dispatch_contract.rs`
- Create: `deploy/pi5/arda-warden-scout.service`
- Modify: `Cargo.toml`

**Steps:**
1. Write tests proving requests without an allowlisted source policy are rejected.
2. Write tests proving findings require URLs/provenance and cannot encode approval.
3. Implement an HTTP service bound to the Tailscale interface or loopback-forwarded surface.
4. Route model work to `http://100.110.85.37:1234/v1` for local summaries and Manwe for tool-backed internet research.
5. Persist append-only local receipts and return their IDs to Arda.
6. Build for `aarch64-unknown-linux-gnu` or build natively on Warden.
7. Deploy with `rsync`, install the user unit, and verify active/enabled plus a real source-cited scout result.

### Task 3: Wire scout evidence into Athena and council

**Objective:** Make Pi findings visible to autonomous flows without bypassing governance.

**Files:**
- Modify: the existing producer that owns `data/athena/scout_requests.jsonl`
- Modify: the existing producer that owns `data/athena/scout_findings.jsonl`
- Modify: the existing council conversation producer for `data/council/agent_conversations.jsonl`
- Test: adjacent producer/integration tests discovered before implementation

**Steps:**
1. Add a failing integration fixture for a Warden request/finding lifecycle.
2. Require node ID, source policy, expiry, provenance, and advisory authority.
3. Project accepted findings into `core/state/scout_runtime.json`.
4. Permit council consumption as evidence; prohibit direct task-queue writes.
5. Run focused producer tests and verify a real Warden receipt reaches the projection.

### Task 4: Build the CITADEL chatbot bridge

**Objective:** Give the avatar a governed conversational backend without storing cloud API keys on the Pi.

**Files:**
- Create: `crates/outposts/arda-citadel-bridge/Cargo.toml`
- Create: `crates/outposts/arda-citadel-bridge/src/main.rs`
- Create: `crates/outposts/arda-citadel-bridge/tests/chat_contract.rs`
- Create: `deploy/pi5/arda-citadel-bridge.service`
- Modify: `/var/home/mythos/Eregion/relic-kiosk` state adapter after inspecting its live schema

**Steps:**
1. Test chat input limits, timeout behavior, safe output shape, and no embedded provider credential requirement.
2. Implement a local bridge that calls Manwe over Tailscale and emits sanitized avatar scene events.
3. Keep RELIC read-only for runtime state; place chat input/output in a separate bounded contract.
4. Deploy and verify voice/text failure modes leave the display usable.

### Task 5: Add the Pi-safe ARDA HUD companion projection

**Objective:** Show fleet, scout, council, and health summaries on CITADEL without running the full desktop Tauri cockpit.

**Files:**
- Modify: `/var/home/mythos/Eregion/Arda-HUD/scripts/export-companion-state.mjs`
- Modify: `/var/home/mythos/Eregion/Arda-HUD/src/lib/ardaPresenceSchema.ts`
- Create: `/var/home/mythos/Eregion/Arda-HUD/src/lib/piOutpostCompanion.ts`
- Create: `/var/home/mythos/Eregion/Arda-HUD/src/lib/piOutpostCompanion.test.ts`
- Modify: `/var/home/mythos/Eregion/relic-kiosk` companion scene after inspection

**Steps:**
1. Add fixtures for Warden/CITADEL connectivity, scout activity, council state, and stale telemetry.
2. Export only bounded display-safe state; exclude prompts, credentials, and private reasoning.
3. Add a round-display layout with predictable low-density transitions.
4. Preserve RELIC idle/stale behavior when the companion bundle is unavailable.
5. Run focused Vitest tests and `pnpm run build` in `Arda-HUD`.
6. Deploy static assets to CITADEL and verify with `curl :8091` plus kiosk/display evidence.

### Task 6: Add advisory Pi seats to council flows

**Objective:** Represent Warden and CITADEL as explicit advisory participants with no approval authority.

**Files:**
- Modify: `config/governance/autonomy_operating_loop.toml`
- Modify: the canonical council seat contract discovered during implementation
- Modify: `core/state/embodied_interface.json`
- Add tests beside the council seat parser/evaluator

**Steps:**
1. Add failing tests that advisory seats can contribute evidence but cannot satisfy approval thresholds.
2. Register Warden as `scout_evidence` and CITADEL as `presentation_and_advisory`.
3. Require freshness and receipt links before their evidence affects a deliberation.
4. Verify an autonomous decision flow records their participation while retaining the normal human/risk gates.

### Task 7: Operationalize SSH and service recovery

**Objective:** Make both outposts recoverable with stable commands and no passwords in scripts.

**Files:**
- Modify: `config/fleet.toml`
- Create: `scripts/pi5_outpost_status.sh`
- Create: `scripts/pi5_outpost_restart.sh`
- Modify: `docs/MIRROMERE_RELIC_OUTPOST_VISION.md`

**Required commands:**

```bash
ssh warden
ssh citadel
ssh raspberrypi
ssh warden 'systemctl --user status llama-server.service --no-pager'
ssh citadel 'systemctl --user status relic.service citadel-kiosk.service --no-pager'
```

**Steps:**
1. Make status helpers use `BatchMode=yes`, canonical SSH aliases, and finite timeouts.
2. Never use `sshpass` or embedded passwords.
3. Keep restart actions explicit per node and verify service health after each restart.
4. Treat Tailscale reachability, TCP/22, SSH authentication, user-systemd health, and app health as separate gates.

## Final acceptance

- Both aliases connect non-interactively.
- Warden completes a bounded source-cited scout request and produces an Athena-compatible receipt.
- CITADEL completes a Manwe-backed chat turn and renders a corresponding safe scene.
- ARDA HUD exports a Pi-safe state bundle consumed by CITADEL.
- A council flow records Warden/CITADEL advisory evidence without granting either approval authority.
- Reboot-path checks prove user units enabled, linger active, and services healthy.
- No credentials or plaintext SSH passwords exist in deployment/runtime scripts.
