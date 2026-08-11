
## Frontend data / lib layer

- `src/lib/ardaSource.ts`: core state source/bundle creation + readers
- `src/lib/ardaSurfaces.ts`: surface derivation for queue, governance, operators,
  sources, storage, autonomy readiness, escalation, runtime, etc.
- `src/lib/operatingSurfaceDerivation.ts`: operating-surface reports + knowledge map
- `src/lib/reviewGateDerivation.ts`: review-gate items, queue write requests,
  human augmentation, plan shelf, task/command/CEO council runtime
- `src/lib/charonLive.ts`: legacy-named compatibility reader for the live Manwë
  snapshot; it is not Charon ownership authority
- `src/lib/hermesDashboardLauncher.ts`: legacy dashboard/CLI compatibility
  launcher pending explicit retirement; it is not monitor-session authority
- `src/lib/bundleDerivation.ts`: provenance/coverage/tagging helpers
- `src/lib/surfaceAdapterManifests.ts`: adapter/workstation manifest derivation
- `src/lib/settingsLayout.ts`, `src/lib/worldSurfaceSettings.ts`,
  `src/lib/boardroomSlotSettings.ts`, `src/lib/orderedProjection.ts`: layout/settings
- `src/lib/ingest/`: parser/registry/types/sources for external data ingestion
- `src/lib/endpointConfig.ts`, `src/lib/providerRouting.ts`, `src/lib/avatarPersona.ts`,
  `src/lib/weathertop.ts`, `src/lib/configWalkthrough.ts`: endpoint/provider/auth config
- `src/lib/statefulPersona.ts`: validated read-only consumer for canonical
  `persona.*` identity projections; static avatar persona helpers remain
  rendering skins rather than identity authority
- `src/lib/systemActionBus.ts`, `src/lib/automationStatus.ts`: action descriptors + state
- `src/lib/tauriGuard.ts`, `src/lib/operatorStore.ts`, `src/lib/ardaLiveListener.ts`:
  Tauri guardrails + runtime listener
- `src/lib/monitorSurfaceContract.ts`, `src/lib/boardroomSessionRegistry.ts`,
  `src/lib/monitorSurfaceRegistryBridge.ts`: canonical five-slot session schema,
  persisted registry projection, and native event bridge
- `src/lib/boardroomRenderContent.ts`: resolves typed session/claim content before
  any ambient fallback

## Frontend testing

- `vitest`-configured test suite across:
  - `src/components/arda/modules/` module tests
  - `src/lib/` source/surface/projection tests
  - `src/scene/` scene/boardroom/world derivation tests
  - `src/utils/` multi-window helper tests

## Boardroom presence

- `src/scene/systems/presenceState.ts`: canonical phase/scenario/urgency state
  derivation and pure visual projection, including bounded optional persona
  influence with alert precedence
- `src/scene/boardroom/AvatarPresenceLayer.tsx`: live emitter-bound orchestrator
  and canonical persona projection loader
- `src/scene/boardroom/ParticleOrb.tsx`: preallocated particle renderer with
  neutral fallback and hard-paused dismissed state
- `src/scene/boardroom/particlePresence.ts`: deterministic materialize,
  dematerialize, density, and motion transitions

## Boardroom display system

- `BoardroomApertureSurface.tsx`: full-aperture CanvasTexture/WebGL display host
- `UpperAmbientMonitorScreen.tsx`: five unique idle upper-monitor identities;
  ambient screens are non-interactive until occupied
- `MonitorSessionWorkstation.tsx`: focused window bound to the exact typed
  `surfaceSessionId`
- `MonitorOwnershipRail.tsx`: external lease/owner signal that never overlays
  agent content
- `LowerInstrumentScreen.tsx` and `CommandCoreInstrumentScreen.tsx`: distinct,
  nearly textless state-driven desk instruments
- `renderers/`: typed image, video, web, document, terminal, component, and
  remote-session monitor renderers

## Consumer wiring

- `arda-engine`: HUD consumes engine/manwe status via Manwe live snapshot
- `manwe`: HUD reads `/v1/models`, `/health`, `/status`, provider candidates
- `arda-launcher`: conceptually upstream of HUD; launcher hands off operator to HUD
- `arda-core`: shared governance/task/contract primitives consumed by HUD lib derivations

## Improvement ideas

1. Separate the HUD backend into smaller crates/files; `lib.rs` at ~2,718 lines
   is a maintenance bottleneck
2. Extract shared `lib/` derivation logic into a workspace crate if `arda-engine`
   or monorepo services need it
3. Move PTY spawner (`tools/ptyspawn`) into a reusable runtime crate if not already
4. Standardize Manwe access paths/config; current hardcoded allowed-path allowlist
   should match manwe adaptive catalog
5. Retire the active generic-panel Hermes dashboard/CLI compatibility surface
   and migrate any still-required capability to explicit Manwë/Aulë ownership;
   do not cosmetically rename the localhost `:9119` embed
6. Add CI gate for frontend tests + Rust check from the standalone package
7. Add explicit offline/failure UI for unavailable Manwë/Aulë projections and
   any temporarily retained compatibility endpoint
8. Consider `arda-hud` standalone vs workspace membership: if it stays standalone,
   document required run order clearly; if it joins workspace, remove redundant
   copies of shared types
