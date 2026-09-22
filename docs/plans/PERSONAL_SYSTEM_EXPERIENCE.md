---
soterion:
  sigil: "SCROLL"
  role: "acceptance_plan"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-09-15"
---

> 🜏 Soterion: 📜 acceptance_plan | owner: PROMETHEUS | status: active | reviewed: 2026-09-15

# Personal System Experience — Media and Devices

## Outcome, scope and evidence

An agent handles genuine shared material through existing Hermes/Arda interfaces
and presents the appropriate original content on the five upper HUD monitors,
without making remote requests depend on operator-run workstation diagnostics.
Acquisition, reading/analysis, playback, display and retention are distinct intents.
This is one capability of the personal system, not Arda's entire identity.

The initial Discord-phone image flow was operator-confirmed after the gateway
executor repair. Preserve that accepted evidence; do not reopen it because older
paragraphs still said “retry required”. General media, pin/pending/restart lifecycle,
web restrictions and performance remain open. [Historical evidence](../archive/2026-09-15-plan-reconciliation/PERSONAL_SYSTEM_EXPERIENCE.md)
retains the original operator confirmation and component checks.

The [native adapter](../../apps/arda-hud/src-tauri/src/commands/monitor_surface/presentation.rs)
and [Hermes integration](../../integrations/hermes/arda-presentation/README.md) exist.
September 15 source inspection confirms image/PDF/text/video descriptor dispatch
and explicit rejection of web `play` and unsupported formats. This does not prove
native audio/office/YouTube playback. No new native media acceptance was run in this
documentation pass. A published descriptor is not a rendered/playing outcome.

## P1 — General media integration

P1 incorporates the former duplicate P1/P1a tasks. Reuse typed monitor registry,
owner/session/revision/lease authority, acquisition and renderer/browser paths.
Do not add another session registry, write session files directly, or use desktop
control as the primary publication transport.

- [ ] P1.1 — Complete the installed ingress→authorization→acquisition→actual MIME/
  capability→intent→session publication→renderer/player outcome→receipt trace.
  Retain the known image path; inspect only the missing family/lifecycle boundaries.
- [ ] P1.2 — Extend existing typed requests/results where needed: authenticated sender,
  stable replay ID, source/asset/provenance, media type, operation, privacy eligibility,
  project/topic, target preference, lifecycle and owner/session/revision. Separate
  acquired, published, rendered/playing, failed, withheld and deferred states.
- [ ] P1.3 — Preserve source scope and original-source access; distinguish knowledge
  retention from display and expose partial extraction/inaccessible sources. Retrieved
  content is untrusted data, never authority to run instructions.
- [ ] P1.4 — Bound file access, durable asset retention, remote fetching, MIME/size/codec
  validation, redirects, DNS resolution and subresources. Authenticate publishers
  independently of payload owner labels; enforce ambient privacy before publication.
- [ ] P1.5 — Honor explicit target or choose an eligible free slot; preserve occupied
  and operator-pinned content, defer when full. Retain durable pending intake while
  the HUD is closed and resume without duplicate claims/downloads/windows/playback.
- [ ] P1.6 — Connect outcome evidence per family and recovery for source loss, sender
  disconnect, lease expiry, replay/conflict and HUD crash/restart. Never corrupt
  another session or relabel a capture/discovery failure as a content failure.
- [ ] P1.7 — Add focused failing tests for missing family/lifecycle behavior, then repair
  only the actual boundary. Cover unsafe/mislabelled/missing/expired inputs, privacy,
  occupied/pinned/full displays, duplicate delivery and independent concurrent sessions.

## P2 — Native presentation and daily-use acceptance

Supported-format policy must be discoverable and extensible. Every input receives
supported/deferred/unsupported handling with a truthful reason; a universal decoder
is not implied. Do not substitute a summary, screenshot or downloaded still when
original/live content or playback was requested.

- [ ] P2.1 — Images/diagrams: original image, preserved aspect/fit, readable source and
  same-session deeper inspection. Preserve the accepted phone-image case; validate
  only changed integration and the remaining family/lifecycle coverage.
- [ ] P2.2 — PDF/text/Markdown and supported office documents: readable content,
  navigation and source access, with extraction limitations. A thumbnail is not reading.
- [ ] P2.3 — Audio: actual authorized playback/output routing and synchronized transport
  state; transcript only when requested, not a substitute for playback.
- [ ] P2.4 — Video/YouTube: actual playback and controls with buffering/error state;
  use real Chromium where required, not screenshots or static captures.
- [ ] P2.5 — Web/interactive/live sources: real source session, passive upper preview
  and input through its same-session workstation. A staged capture is not live access.
- [ ] P2.6 — Generated visualizations/artifacts: compatible renderer with provenance;
  unsupported/encrypted/inaccessible/unsafe input has an explicit limitation and
  useful permitted alternative, never silent loss or false success.
- [ ] P2.7 — Across all five canonical upper slots, verify explicit and agent placement,
  pin/ownership/full-slot deferral, HUD-closed intake, replay, expiry/release, restart,
  sender/source loss and privacy withholding with correlated native outcome evidence.
- [ ] P2.8 — Observe ordinary remote use without manual descriptor publication or
  workstation diagnostics; record operator readability/usefulness and baseline/after
  frame timing. Reject FPS regressions. Preserve lower WebGL apertures and passive
  World View; do not restart a visible HUD merely for capture convenience.

Exit gate: genuine ordinary requests across the media families reach their intended
native behavior, with source, privacy, ownership, replay/recovery and performance
evidence. Publication, tests, thumbnails or one successful photo are insufficient.
If native verification is unavailable, leave that acceptance item open.

## P3 — Reconcile and use real devices (existing placement/concurrency owners)

- [x] P3.1 — Operator scope retained: `annunimas-server` is intentionally powered off;
  keep it unavailable for placement, not a mandatory repair or wake/restart target.
  This is a scope decision, not fresh remote health evidence.
- [ ] P3.2 — Reconcile physical identity, configured service lanes, intended role,
  installed service, last successful observation, optional/offline policy, allowed
  work/data and capacity. Use read-only evidence before correcting inventory;
  multiple inference lanes are not multiple devices or independent capacity.
- [ ] P3.3 — When an intended remote device is reachable and authorized, correlate
  one useful task with actual node identity and canonical dispatch/recovery receipts.
  Offline Tailscale status does not establish power state or service failure.
- [ ] P3.4 — Present useful results through P2 and expose unavailable/degraded placement
  honestly. [Provider convergence](PROVIDER_WORKER_CONVERGENCE.md) owns routing;
  [M4](autonomous-task-completion/README.md#m4--real-multi-project-execution) owns
  real-project overlap/physical isolation. Cross-device proof is distinct from
  same-host concurrency and does not require waking every optional device.

[Configured fleet](../../config/fleet.toml), `core/state/fleet_nodes.json` and
[historical inventory](../../core/state/system_inventory.md) are comparison inputs,
not interchangeable live truth. Select the participating remote device only when
needed; previous hostname/port snapshots are retained in historical evidence.

Exit gate: intended versus observed device roles are truthful and approved useful
cross-device work has receipts, or the operator explicitly defers that capability.
A deferral does not become verified remote execution.

## P4 — Recurring care and broader personal interfaces

The [daily-loop owner](DAILY_RESEARCH_IMPROVEMENT_LOOP.md#rúmil-backed-apothecary)
owns Rúmil/Apothecary scheduling, governed repair and seven-cycle acceptance; it
consumes P2 for native briefs. No duplicate checklist or scheduler lives here.
Read-only care can proceed alongside runtime repairs; mutation uses existing gates.

Launcher remains entry/resume/connections/recovery; HUD workstations support deeper
personal work; Mirromere is separate ambient voice/persona/context and remains
[deferred](../archive/deferred/ambient-agent/03-mirromere-embodied-assistant.md).
Mail/social monitoring, multi-format knowledge acquisition and editable memory
remain broader capabilities under the whole-system owner, not hidden prerequisites
to displaying a share. Use mature permission-scoped connectors/parsers/players.

## Verification, decisions and retirement

Use current HUD package scripts, affected contract/registry/persistence/browser/
renderer tests, native monitor tests, TypeScript checks and supported Tauri builds
before installed acceptance. Existing entry points include `monitorSurfaceContract`,
`monitorSurfaceRegistryBridge`, `monitorSurfacePersistence` and
`universalMonitorAcceptance` tests under the HUD app.

Execution needs genuine non-sensitive input only when the path is ready, authorized
privacy/placement, and the operator's native verdict. The existing phone/Hermes
channel is already selected; do not demand new client setup. Daily cadence/budget/
destination decisions belong to the daily owner, not this media plan.

Retire this plan after P1–P3 owned gates pass or explicitly scoped deferrals are
accepted; link daily/provider/M4 receipts without falsely closing their programs.
Evidence belongs outside the active queue. Visual consistency belongs to the
[HUD visual pass](arda-hud-visual-pass/README.md).
