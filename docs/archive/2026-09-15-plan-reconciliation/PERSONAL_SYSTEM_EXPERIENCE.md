---
soterion:
  sigil: "SCROLL"
  role: "acceptance_plan"
  owner: "PROMETHEUS"
  status: "archived"
  reviewed: "2026-09-07"
---

> 🜏 Soterion: 📜 acceptance_plan | owner: PROMETHEUS | status: archived | reviewed: 2026-09-07

# Personal System Experience — Assessment and Action Plan

## Goal and authority

Make Arda useful through its existing interfaces and real devices: share material and have an agent present it on the five upper HUD monitors; understand and use device-specific services; receive useful Rúmil-backed recurring improvement outcomes.

This plan owns share-to-display integration and its native acceptance. It does not replace the [whole-system program](ARDA_WHOLE_SYSTEM_COMPLETION_PROGRAM.md) with a short demonstration. Concurrent execution remains owned by [Milestone 4](autonomous-task-completion/04-real-multi-project-execution.md), placement by [provider convergence](PROVIDER_WORKER_CONVERGENCE.md), and Apothecary by the [daily improvement plan](DAILY_RESEARCH_IMPROVEMENT_LOOP.md#rúmil-backed-apothecary). Those workstreams consume the presentation path; do not create duplicate queues or backlogs.

**Status:** implementation in progress; native publication verified and the initial Discord-phone-to-HUD flow operator-confirmed. General-media acceptance remains open. The original assessment below is historical. The operator subsequently authorized implementation; source, installed HUD, and default-profile Hermes presentation configuration changed. No schedules or remote device power state changed.

### P1/P1a implementation evidence — 2026-09-07

- [Native presentation adapter](../../../apps/arda-hud/src-tauri/src/commands/monitor_surface/presentation.rs) and [same-user socket](../../../apps/arda-hud/src-tauri/src/commands/monitor_surface/presentation_socket.rs) reuse the typed monitor authority. Explicit ambient permission, active-slot refusal, request replay/conflict checks, canonical asset bounds, and startup readiness are enforced. Publication is not rendering.
- [Hermes plugin and installation guide](../../../integrations/hermes/arda-presentation/README.md) provide `arda_present`, enabled for the existing Discord ingress. Fresh-process platform tool resolution finds it; tool-search deferral explains its absence from the initial top-level schema list. The gateway was restarted externally and Discord reconnected. A new phone-originated tool call is still required.
- RED/GREEN checks: native monitor suite **50 passed, 2 ignored**; Python adapter **4 passed**; frontend persistence **6 passed**. `pnpm build` and the packaged native release build passed. These are component evidence, not visual acceptance.
- The first raw Cargo release omitted `tauri/custom-protocol`; its process started but never reached native readiness. The previous installed binary was restored and hash-verified before rebuilding with the required feature. The corrected installed build reached `ready: true`. A historical native registry file contained only expired August leases and was not rewritten.
- The operator's original cached photograph was published to `monitor_1` as `presentation-discord-photo-821851822ce9` and independently read back with matching session/content. Raw local result: `/tmp/arda-photo-presentation-result.json`. This was an agent-side replay of the genuine attachment, **not** a new phone-originated acceptance run.
- Operator subsequently confirmed the original photograph was visible on `monitor_1`. This establishes visible acceptance for that image only, not general media or a successful new phone-originated flow. Native capture itself was unavailable; the temporary inspection daemon was stopped.
- The next genuine phone request reached `arda_present` but failed before handler execution: `DaemonThreadPoolExecutor` had no `_initializer`. The installed gateway uses `.venv` Python 3.14, unlike the earlier `venv` Python 3.11 checks. Hermes's pool now selects the matching CPython worker-context signature. Five pool checks pass on both interpreters; the Python 3.14 focused pool/executor suite passes nine tests. A real threaded `arda_present status` call returned `ok: true, ready: true`; only the gateway was restarted to load the repair. A fresh phone retry is still required.
- Subsequent operator acceptance supersedes the pending-phone-retry statements above: after the executor repair, the operator reported, “awesome it worked from discord on my phone.” The initial genuine Discord-phone-to-HUD flow is operator-confirmed. This does not establish every media type, lifecycle, or performance gate.
- Open gates: general media/playback; pinned-screen handling; HUD-closed durable pending intake; crash/restart replay; source/privacy lifecycle; DNS/redirect/subresource restrictions for web content; FPS impact. No P1a/P2 completion claim, commit, or push.
- September 7 follow-up: live gateway executable and `.venv` both resolve to Python 3.14.7; all four existing plugin tests passed under that interpreter. `arda_present status` returned `ready: true` and the original monitor_1 plus subsequent monitor_2 image-session records without republishing. Source `presentation.rs` explicitly rejects audio/office formats and web `play`, publishes web capture-stream descriptors, and only dispatches the listed image/document/video forms. Do not claim working audio or YouTube playback from the broader monitor contract. Pin-aware admission and durable HUD-closed intake remain unresolved, not proven absent throughout the ecosystem.

## Evidence assessment — 2026-09-07

### 1. Sharing and five upper monitors

- [Monitor contract](../../../apps/arda-hud/src/lib/monitorSurfaceContract.ts) names exactly `monitor_1` through `monitor_5`. It already represents web, YouTube, video, image, PDF/Markdown/text documents, terminal, component, remote session, and explicit fallback content. Descriptor support is not proof every media path works natively.
- [Native typed commands](../../../apps/arda-hud/src-tauri/src/commands/monitor_surface/typed.rs) implement claim/release/refresh, owner checks, leases, revision-bound playback, and registry events. Reuse this ownership model.
- [Frontend bridge](../../../apps/arda-hud/src/lib/boardroomSlotSettings.ts) invokes `claim_monitor_surface`; [App.tsx](../../../apps/arda-hud/src/App.tsx) rehydrates claims and subscribes to registry/expiry updates. Inspected callers include native acceptance and browser session paths. An unattended authenticated share-to-agent-to-monitor path was **not established** by this inspection; do not call it absent solely from symbol searches.
- [Phone/HUD continuity evidence](../../operations/hermes-continuity-handoff-acceptance.md) records genuine phone-originated Hermes lineage and native handoff acceptance on August 17. This is reusable historical evidence, not present proof of arbitrary link/file presentation.
- Local `systemctl --user show` reported `arda.service` and `hermes-gateway.service` active/running, but `arda-hud.service` inactive/dead. This checks that unit, not every possible manually started HUD process. The HUD was not started for this planning task.

**Assessment:** substantial presentation substrate; everyday ingress, agent publication, readable source-aware presentation, and installed end-to-end acceptance remain the integration work to establish.

### 2. Device roles and availability

Declared roles below come from [fleet.toml](../../../config/fleet.toml), not fresh endpoint probes:

| Physical device | Declared purpose/services | Observation in this assessment |
|---|---|---|
| annunimas-core | Home authority/workstation; core inference `:9337`; local Arda and Hermes gateway | Tailscale self online; Arda and gateway user units running |
| Warden Pi5 | Bounded scout/research `:8092`, light inference `:1234`, local SearXNG | Tailscale peer offline; installed remote services unverified |
| Beelink SER9 | Efficient bounded worker, Prism inference `:9337`; separate `:1234` lane intentionally disabled in configuration | Peer at configured address offline; Tailscale hostname appears as `bluefin` |
| Backbone server | Bonsai review/research `:8097`, Nanbeige coding/tools `:8098` | Peer at configured address offline; Tailscale hostname appears as `bluefin` |
| Laptop | Optional portable operator/voice surface, not a required inference host | Tailscale peer offline |
| CITADEL / raspberrypi | Embodied outpost/RELIC `:8091`; configuration does not claim deployed scout worker | Tailscale peer offline |

The operator identified `annunimas-server` as deliberately powered off. Leave it off and unavailable for placement. **Offline in Tailscale does not prove powered off, service failure, or retirement.** No remote wake, restart, deployment, or repeated connection attempt was made. Multiple configured lanes on one device are not multiple physical devices.

[System inventory](../../../core/state/system_inventory.md) still lists backbone ports `:8093/:8094`, while fleet configuration declares `:8097/:8098`; Warden context values also differ. Neither source alone proves current deployment. Historical verification notes must not become live health labels.

**Assessment:** roles are documented, but a reconciled intended-role/last-observed-service/availability view is missing from the evidence checked. Multi-project concurrency on one host and real multi-device execution require separate receipts.

### 3. Rúmil / Apothecary

- [Rúmil README](../../../crates/spine/runtime/arda-rumil/README.md) and [status](../../../crates/spine/runtime/arda-rumil/STATUS.md) describe reusable bounded inventory, allowlisted providers, comparisons, organization proposals, Warden audit/follow-up receipts, and Vairë/Mandos/Varda/HUD consumers.
- [Profile implementation](../../../crates/spine/runtime/arda-rumil/src/profile.rs) exposes `audit_with_profile`; [engine research](../../../crates/engine/src/harness/research.rs) carries Rúmil packet identities/digests into research evidence. This is substantive scaffolding, not a replacement audit engine to build.
- Local timers include `arda-aule-autopilot.timer` and inference probes. The last inspected governed autopilot service exited successfully; that does **not** establish useful Rúmil execution or repair. No Rúmil-specific recurring unit was identified in the local timer inspection. Remote Warden schedules were not inspected because peers were offline.
- Existing daily-loop documentation has stale wording about an installed read-only autopilot; current inspection identifies the governed `prometheus autopilot once` command. Neither observation proves the complete improvement cycle.

**Assessment:** audit/evidence capability is implemented in source; recurring invocation, finding-to-objective promotion, evaluated improvement, and useful native briefing remain unproven here. Rúmil must stay advisory, not become its own mutation authority.

## Action sequence

### P1 — Trace one real share and define presentation policy

- [ ] Trace authenticated Hermes link/file ingress through existing Oromë/engine/Vairë boundaries and the HUD claim path; record the exact callable boundary and installed artifact identity. Reuse existing tools/adapters before adding a narrow bridge.
- [ ] Preserve source identity, sender/surface scope, media type, acquisition state, project/topic reference, and replay identity. Treat retrieved content as untrusted data, not executable instructions.
- [ ] Separate acquisition/knowledge retention from display: a displayed URL is not evidence it was ingested or understood. Show partial extraction and inaccessible source states.
- [ ] Define an agent presentation request against the existing five-slot registry: honor explicit target, select an eligible unoccupied slot otherwise, preserve operator-pinned content, and defer when all slots are occupied. Do not dedicate five slots to five permanent dashboards.
- [ ] Authenticate the publisher independently of a payload's owner label. Bound local-file access, remote fetching, redirects, media sizes, leases, and private content eligibility for ambient display.

Likely integration files: `apps/arda-hud/src/lib/{monitorSurfaceContract,boardroomSlotSettings,browserMonitorSession}.ts`, `apps/arda-hud/src-tauri/src/commands/monitor_surface/`, `crates/engine/src/harness/`, and the discovered installed Hermes/Oromë bridge. Identify that bridge before editing; do not invent an endpoint or a second session authority.

### P1a — Restore general agent-controlled media handling

**Status: open; full intended system behavior remains unverified.** The failed Discord photograph request is evidence of an end-to-end capability gap, not a single-use image feature. This work owns general media handling across existing Arda/Hermes interfaces and agent-controlled HUD surfaces. Do not close it by making one cached image appear or adding an image-only publication script. The operator subsequently authorized implementation; the evidence above records the bounded native adapter and outstanding acceptance gates.

**Product contract:** given media or a reference to it and an operator intent, Arda must acquire it when authorized, identify its actual type, determine the appropriate handling, route it through the existing owner/session boundaries, and verify the requested outcome. Handle attachments, local assets, remote links, agent-generated artifacts, and live sources. Presentation, playback, reading, analysis, and retention are distinct intents; do not automatically ingest everything displayed, display private material, or replace requested original content with a summary.

**Content-appropriate behavior:**

- Images and diagrams: render the original with preserved aspect ratio and appropriate fit; offer deeper inspection through the same-session workstation.
- PDFs, text, Markdown, and supported office documents: readable document presentation, source access, navigation, and honest extraction limitations. A thumbnail is not document-reading acceptance.
- Audio: authorized playback with transport state, output routing, and optional requested transcript; do not confuse a transcript with having played the audio.
- Video and YouTube: actual playback with synchronized controls and explicit buffering/error state; use the real Chromium path where required. A downloaded still or screenshot is not playback.
- Web pages and interactive/live sources: the real source/session, with passive upper-monitor preview and input in its same-session workstation. Do not substitute a staged capture for a live session.
- Generated visualizations and other supported artifacts: select the existing compatible renderer and preserve source/provenance. Unsupported, inaccessible, encrypted, or unsafe formats must receive an explicit reason and useful alternative, never a false success or silent drop.

The supported-format policy must be discoverable and extensible. “Any media handled appropriately” requires classification and a truthful supported/deferred/unsupported outcome for every input; it does not mean claiming a universal decoder exists.

**Observed failure, not an established root cause:**

- Hermes loaded the attachment at `/var/home/mythos/.hermes/cache/images/img_821851822ce9.jpeg` and detected a running installed HUD process. Neither proved presentation.
- Desktop capture returned zero windows/elements and a 0×0 image. A later source-search command failed from unmatched shell quoting. Those were discovery and agent-execution failures, not proof that all publication paths were absent.
- No authorized claim, publication readback, or native rendering evidence was obtained. Hermes then incorrectly made remote operation depend on the operator running workstation diagnostics.
- The operator can be away from the workstation while seeing its output. Routine media handling must not require the operator to return to a terminal or manually publish a descriptor.

**Architecture:** reuse the existing typed monitor registry and owner/session authority, media renderers, acquisition services, and authenticated Hermes/Oromë integration. Discover the actual missing boundary before adding a narrow general-purpose adapter. Desktop control is not the primary media-publication transport. Do not write session files directly, add a parallel ownership registry, or turn World View into a workflow UI.

**Remaining execution tasks:**

1. [ ] Trace ingress → authorization → acquisition → type/capability resolution → intent routing → session publication → renderer/player outcome → operator receipt across installed components. Inspect `apps/arda-hud/src/lib/{monitorSurfaceContract,boardroomSlotSettings,monitorSurfaceRegistryBridge,browserMonitorSession}.ts`, `apps/arda-hud/src/scene/boardroom/renderers/`, `apps/arda-hud/src-tauri/src/commands/monitor_surface/typed.rs`, engine harness routes, and the discovered installed Hermes integration. Record callable boundaries and build identities; distinguish missing integration from failed discovery.
2. [ ] Specify one typed media-handling request/result contract using existing authorities. Carry authenticated sender, stable request ID, source/asset identity, actual MIME, intended operation, privacy eligibility, target preference, lifecycle, and owner/session/revision. Expose acquisition, publication, rendering/playback, failure, and deferred states separately. Define which actions need further approval.
3. [ ] Add failing contract and adapter tests across the media families above, not just images. Include mislabelled MIME, missing/expired sources, bounded file access, redirects and unsafe fetches, unsupported formats/codecs, authorization failures, privacy withholding, duplicate requests, unavailable HUD, occupied/pinned monitors, and disconnected sender.
4. [ ] Implement only missing integration and capability dispatch. Reuse durable asset storage rather than treating transient attachment caches as permanent. Preserve live URLs/session bindings where appropriate instead of converting every input to a file. Select an eligible free slot or report deferred; never steal another owner's display. Replay must not duplicate claims, downloads, windows, or playback.
5. [ ] Connect type-specific outcome evidence to the request: decoded image, readable document/page, actual audio/video playback state, live browser session, or visualization render. Read back authoritative slot/session identity and correlate content provenance. Accepted publication must not be reported as rendered or playing without downstream evidence. Preserve source interaction through same-session workstations.
6. [ ] Recover from media failures, expired leases, HUD restart, source loss, and unsupported content without corrupting other sessions. Diagnose desktop discovery agent-side when needed; do not require operator terminal commands or restart a visible HUD merely for capture convenience. Keep capture failure separate from media-handling failure.
7. [ ] Run focused contract/registry/persistence, media-lifecycle, renderer-dispatch, and adapter tests using RED→GREEN, followed by TypeScript and native Rust checks. Record actual test commands and results. Exercise independent concurrent media sessions, replay, release, and recovery, preserving privacy and frame timing.
8. [ ] With fresh execution authorization, perform installed end-to-end acceptance using genuine inputs spanning image, document, audio, video/YouTube, web/live content, and generated artifacts, plus an unsupported input. Test both agent-selected and explicit placement across all five upper slots. Include the original photograph when still available as one regression case, not the completion gate for the system.

**Acceptance:** ordinary remote requests produce the appropriate real content behavior without operator-side diagnostics, manual descriptor publication, or custom one-off scripts. For each media family record intent, source, handling decision, authoritative session, downstream outcome, and native evidence appropriate to the medium; audio requires playback evidence, not a screenshot alone. Preserve existing owners, same-session interaction, privacy, restart continuity, and baseline FPS. A green test, receipt, process, thumbnail, or one successful photo does not close general media handling. Unknown/unsupported cases must explain the limitation and next available action. If native verification is unavailable, retain that explicit open gate rather than claiming completion. Retire this subsection with P1/P2 only after system-level acceptance, keeping evidence outside the active plan queue.

### P2 — Native presentation and daily-use acceptance

- [ ] Implement only missing integration from P1. Retain a durable pending result when the HUD is closed; reopening must not lose the share or launch an uncontrolled set of windows.
- [ ] Present a readable title/source, useful content or honest summary, acquisition status, and reason for placement. Preserve original-source access and same-session workstation focus for deeper reading or interaction.
- [ ] Execute the general-media acceptance matrix in P1a: images, readable documents, audio, video/YouTube, web/live sources, generated artifacts, and unsupported inputs. Verify behavior against operator intent, not merely descriptor support or a single successful attachment.
- [ ] Demonstrate agent selection and explicit placement across all five canonical upper slots, occupied/pinned-slot handling, independent owners, expiry/release, duplicate delivery, HUD restart, disconnected sender, and unavailable source.
- [ ] Verify privacy withholding on ambient/shared surfaces. World View stays display-only; lower WebGL apertures remain unchanged. Native Chromium remains the YouTube path. Record baseline/after frame timing and reject FPS regressions.
- [ ] Repeat during ordinary use: the operator does not manually publish a descriptor or return to a terminal to make each share appear. Record operator judgment of readability and usefulness, not just screenshots.

Focused existing test targets: `monitorSurfaceContract.test.ts`, `monitorSurfaceRegistryBridge.test.ts`, `monitorSurfacePersistence.test.ts`, native `monitor_surface/{typed_tests,contract_tests,registry_tests}.rs`, and `universalMonitorAcceptance.test.ts`. Use the current HUD package scripts, Rust checks, and installed native acceptance after implementation; tests alone do not close this gate.

### P3 — Reconcile and use real devices (existing placement/concurrency owners)

- [ ] Reconcile physical-device identity, configured lanes, intended role, installed service, last successful probe, optional/offline policy, allowed work/data, capacity, and evidence timestamp. Correct inventory only after read-only validation; do not rewrite runtime configuration to match stale prose.
- [x] Operator identified `annunimas-server` as deliberately powered off. Keep it unavailable for placement and preserve continuation without wake/restart automation. This is an operator scope decision, not remote service verification.
- [ ] When an intended remote device is reachable, inspect its actual services and correlate one useful task with node identity and canonical execution receipts. Report degraded local-only availability honestly in the meantime.
- [ ] Use existing Milestone 4 for safe overlapping real-project work, same-root exclusion, and receipt-backed joining. Separately prove cross-device dispatch/recovery. Shared physical inference capacity must not be double-counted across configured lanes.
- [ ] Display useful work/results through P2, rather than exposing only infrastructure health.

Owner paths: `config/fleet.toml`, `core/state/fleet_nodes.json`, `core/state/system_inventory.md`, provider configuration, [provider plan](PROVIDER_WORKER_CONVERGENCE.md), and [Milestone 4](autonomous-task-completion/04-real-multi-project-execution.md). These checklist entries describe dependency acceptance, not a second execution backlog.

### P4 — Rúmil-backed recurring care (daily-loop owner)

Execute the [Apothecary integration sequence](DAILY_RESEARCH_IMPROVEMENT_LOOP.md#rúmil-backed-apothecary) using P2 for readable outcomes. Read-only scheduled audits may be established without waiting for arbitrary autonomous mutation; repair promotion/execution still depends on the existing runtime acceptance gates. Do not reduce Apothecary to activating a timer.

## Broader interfaces and reuse

The five-monitor share flow is the first delivery, not the definition of Arda. Launcher remains entry/resume/connections/recovery; HUD workstations support deeper personal information and concurrent work; Mirromere remains ambient voice/persona/context. Carry material summaries, source links, visibility policy, and continuity across those different layouts. Reuse the [visual pass](arda-hud-visual-pass/README.md) and [Mirromere plan](../deferred/ambient-agent/03-mirromere-embodied-assistant.md) for their existing responsibilities; this assessment does not authorize a new visual redesign or unhold physical embodiment.

Mail/social monitoring, multi-format knowledge acquisition, and editable operator memory remain part of the broader experience. Select existing connectors/parsers/players through Arda's bounded integration contracts; do not rebuild them as prerequisites to P1. Permission-scoped access and explicit operator corrections take precedence over inferred personality traits.

## Decisions needed only at execution boundaries

- Operator confirmed `annunimas-server` is deliberately powered off. Leave it off and unavailable for placement; do not diagnose its intentional absence as a failure. First participating remote execution device remains to be selected when needed.
- Operator selected the existing phone/Hermes channel for the first genuine share. No new client or channel setup is required. Request a non-sensitive real link only when the installed share-to-display path is ready for native verification; sending it beforehand is not evidence it will appear on a monitor.
- Before enabling Apothecary: confirm local nightly window, resource budget, project scope, and notification destination. No schedule or notification delivery was created by this plan.

## Completion and retirement

Close this plan only after ordinary native share-to-display use is operator-accepted with source, privacy, replay, ownership, and performance evidence. Cross-link dependent fleet/concurrency and daily-loop receipts without claiming their gates closed. Move completed evidence out of `docs/plans/` and retire this active plan when its owned work is finished; do not retain it as a duplicate planning library.
