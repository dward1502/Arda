# Pi5 Outpost Integration Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Make Warden and CITADEL first-class Arda outposts for bounded scouting, avatar/chat presentation, ARDA HUD companion projection, and advisory participation in autonomous council flows.

**Architecture:** Arda/Manwe remains the authority and tool-execution boundary. Warden contributes local inference and receipted scout evidence; CITADEL presents chat, RELIC, and a Pi-safe HUD projection while dispatching requests through governed APIs. Neither Pi gains independent approval authority or unrestricted internet shell access.

**Tech Stack:** OpenSSH over Tailscale, user systemd, Manwe OpenAI-compatible API, Arda scout/council JSONL contracts, RELIC/Three.js, ARDA HUD React companion-state export.

---

## Verified baseline — 2026-07-29

- `ssh warden` → `numenor@100.110.85.37`: operational with non-interactive key auth.
- Warden `llama-server.service`: active.
- Warden `arda-warden-scout.service`: active/enabled and bound to `100.110.85.37:8092`.
- Scout `/health`: `status=ok`, `source=node-pi5-warden`, `authority=advisory`.
- One live scout request returned two HTTP(S) sources and was recalled with one Vairë memory receipt and advisory authority.
- `ssh citadel` and `ssh raspberrypi` → `citadel@100.119.130.127`: operational with non-interactive key auth.
- CITADEL `relic.service` and `citadel-kiosk.service`: active.
- CITADEL `arda-citadel-bridge.service`: inactive.
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

**Status:** Partial. Observation/authority contracts are first-class; manifest, dispatch, health, chatbot, and presence envelopes remain absent.

**Live files:**
- Existing: `outposts/arda-outpost-protocol/Cargo.toml`
- Existing: `outposts/arda-outpost-protocol/src/{lib,observation,authority}.rs`
- Existing: `outposts/arda-outpost-protocol/tests/observation_authority.rs`
- Planned/absent: manifest, dispatch, health, chatbot, and presence modules
- Reference: `core/state/embodied_interface.json`

**Steps:**
1. [ ] Define and test the absent shared envelopes only when a live producer/consumer requires them.
2. [x] Assert canonical observation classification and authority wire values.
3. [x] Prove every observation authority class prohibits execution.
4. [x] Pass focused protocol tests and strict crate gates.

### Task 2: Build the governed Warden scout worker

**Objective:** Let Warden accept bounded scout requests, use its local model or Manwe, and return source-bearing findings.

**Status:** Partial/operational. The local crate contract and live service are proven. The deployed binary predates the 2026-07-29 policy/expiry repair, so current-source AArch64 rebuild/redeploy remains open.

**Live files:**
- Existing: `outposts/arda-outpost-scout/Cargo.toml`
- Existing: `outposts/arda-outpost-scout/src/{main,research,runtime,memory}.rs`
- Existing: `outposts/arda-outpost-scout/tests/{research_fixtures,runtime_api,memory_fixtures}.rs`
- Existing: `config/systemd/arda-warden-scout.service`
- Existing: `config/outposts/warden/research-topics.json`
- Deliberately absent: model client and general tool-execution module

**Steps:**
1. [x] Reject missing/unallowlisted source policy and expired or overlong-validity requests before network access.
2. [x] Require valid HTTP(S) result URLs/provenance and emit advisory observations with no approval field.
3. [x] Run an HTTP service on the Warden Tailscale interface.
4. [ ] Route model work through Warden inference; the current bounded scout intentionally has no model client.
5. [x] Persist append-only Vairë observations and return canonical memory receipt IDs.
6. [ ] Build the 2026-07-29 source for AArch64 Linux; the local Rust toolchain does not have that target installed.
7. [ ] Redeploy the current source. The live active/enabled service and real source-cited receipt are proven, but the binary predates this Packet's repair.

### Task 3: Wire scout evidence into Varda and council

**Objective:** Make Pi findings visible to autonomous flows without bypassing governance.

**Status:** Partial. Root HTTP consumption, Vairë receipts, and the ARDA HUD evidence projection are proven; no live producer owns the Varda scout ledgers/runtime projection.

**Live files:**
- Scout producer: `outposts/arda-outpost-scout/src/{runtime,research,memory}.rs`
- Root HTTP consumer: `crates/engine/src/harness.rs`
- Planned projection stores: `data/athena/scout_requests.jsonl`, `data/athena/scout_findings.jsonl`, `core/state/scout_runtime.json`
- Existing read-only projection consumer: `apps/arda-hud/src/lib/{ardaSource,reviewGateDerivation}.ts`
- Varda/council projection producer owner: unresolved; no source writer was found live

**Steps:**
1. [ ] Add durable Varda scout request/finding producers.
2. [x] Require source policy and expiry at the scout boundary; runtime state supplies node identity and emits provenance/advisory authority.
3. [ ] Produce accepted findings into `core/state/scout_runtime.json`.
4. [x] Project existing scout rows in ARDA HUD as evidence/review state without approval receipts or automatic promotion.
5. [ ] Carry real Warden receipt linkage into the Athena projection; current receipts terminate in Vairë.

### Task 4: Build the CITADEL chatbot bridge

**Objective:** Give the avatar a governed conversational backend without storing cloud API keys on the Pi.

**Status:** Not started in this repository. The proposed bridge crate is absent and the live `arda-citadel-bridge.service` is inactive.

**Planned live-root files:**
- Create: `outposts/arda-citadel-bridge/Cargo.toml`
- Create: `outposts/arda-citadel-bridge/src/main.rs`
- Create: `outposts/arda-citadel-bridge/tests/chat_contract.rs`
- Create: `config/systemd/arda-citadel-bridge.service`
- Modify: `/var/home/mythos/Eregion/relic-kiosk` state adapter after inspecting its live schema

**Steps:**
1. Test chat input limits, timeout behavior, safe output shape, and no embedded provider credential requirement.
2. Implement a local bridge that calls Manwe over Tailscale and emits sanitized avatar scene events.
3. Keep RELIC read-only for runtime state; place chat input/output in a separate bounded contract.
4. Deploy and verify voice/text failure modes leave the display usable.

### Task 5: Add the Pi-safe ARDA HUD companion projection

**Objective:** Show fleet, scout, council, and health summaries on CITADEL without running the full desktop Tauri cockpit.

**Status:** Partial operational baseline only. RELIC and the kiosk are active on CITADEL; the dedicated companion export and bridge are absent.

**Live/planned files:**
- Create: `apps/arda-hud/scripts/export-companion-state.mjs`
- Create or modify: `apps/arda-hud/src/lib/ardaPresenceSchema.ts`
- Create: `apps/arda-hud/src/lib/piOutpostCompanion.ts`
- Create: `apps/arda-hud/src/lib/piOutpostCompanion.test.ts`
- Modify: `/var/home/mythos/Eregion/relic-kiosk` companion scene after inspection

**Steps:**
1. Add fixtures for Warden/CITADEL connectivity, scout activity, council state, and stale telemetry.
2. Export only bounded display-safe state; exclude prompts, credentials, and private reasoning.
3. Add a round-display layout with predictable low-density transitions.
4. Preserve RELIC idle/stale behavior when the companion bundle is unavailable.
5. Run focused Vitest tests and `pnpm run build` in `apps/arda-hud`.
6. Deploy static assets to CITADEL and verify with `curl :8091` plus kiosk/display evidence.

### Task 6: Add advisory Pi seats to council flows

**Objective:** Represent Warden and CITADEL as explicit advisory participants with no approval authority.

**Status:** Not started. Packet 6 proves scout findings are advisory and do not write queue/approval state, but it does not register council seats.

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

**Status:** Partial. Canonical aliases and user services work; the proposed checked-in status/restart helpers remain absent.

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

- [x] Both aliases connect non-interactively.
- [x] Warden completes a bounded source-cited scout request and produces a durable Vairë receipt.
- [ ] An Varda producer carries that receipt into scout request/finding ledgers and `scout_runtime.json`.
- [ ] CITADEL completes a Manwe-backed chat turn and renders a corresponding safe scene.
- [ ] ARDA HUD exports a Pi-safe state bundle consumed by CITADEL.
- [ ] A council flow records Warden/CITADEL advisory evidence without granting either approval authority.
- [x] Focused inspection proves the relevant live user units are active/enabled as documented.
- [ ] Current-source AArch64 build, deploy, and reboot-path smoke automation is reproducible from this repository.
- [x] No credential-bearing deployment/runtime code was introduced by Packet 6.
