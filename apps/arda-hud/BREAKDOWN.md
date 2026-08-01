
## Frontend data / lib layer

- `src/lib/ardaSource.ts`: core state source/bundle creation + readers
- `src/lib/ardaSurfaces.ts`: surface derivation for queue, governance, operators,
  sources, storage, autonomy readiness, escalation, runtime, etc.
- `src/lib/operatingSurfaceDerivation.ts`: operating-surface reports + knowledge map
- `src/lib/reviewGateDerivation.ts`: review-gate items, queue write requests,
  human augmentation, plan shelf, task/command/CEO council runtime
- `src/lib/charonLive.ts`, `src/lib/hermesDashboardLauncher.ts`: live Manwe
  snapshot + Hermes runtime window orchestration
- `src/lib/bundleDerivation.ts`: provenance/coverage/tagging helpers
- `src/lib/surfaceAdapterManifests.ts`: adapter/workstation manifest derivation
- `src/lib/settingsLayout.ts`, `src/lib/worldSurfaceSettings.ts`,
  `src/lib/boardroomSlotSettings.ts`, `src/lib/orderedProjection.ts`: layout/settings
- `src/lib/ingest/`: parser/registry/types/sources for external data ingestion
- `src/lib/endpointConfig.ts`, `src/lib/providerRouting.ts`, `src/lib/avatarPersona.ts`,
  `src/lib/weathertop.ts`, `src/lib/configWalkthrough.ts`: endpoint/provider/auth config
- `src/lib/systemActionBus.ts`, `src/lib/automationStatus.ts`: action descriptors + state
- `src/lib/tauriGuard.ts`, `src/lib/operatorStore.ts`, `src/lib/ardaLiveListener.ts`:
  Tauri guardrails + runtime listener

## Frontend testing

- `vitest`-configured test suite across:
  - `src/components/arda/modules/` module tests
  - `src/lib/` source/surface/projection tests
  - `src/scene/` scene/boardroom/world derivation tests
  - `src/utils/` multi-window helper tests

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
5. Replace placeholder/local-only Hermes runtime assumptions with configurable
   endpoints sourced from `EnvironmentProfile`
6. Add CI gate for frontend tests + Rust check from the standalone package
7. Add explicit offline/failure UI for when Manwe/Hermes endpoints are
   unreachable instead of silent worst-case behavior
8. Consider `arda-hud` standalone vs workspace membership: if it stays standalone,
   document required run order clearly; if it joins workspace, remove redundant
   copies of shared types
