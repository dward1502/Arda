---
soterion:
  sigil: "BOOKMARK"
  glyph: "🚀"
  code_point: "U+1F680"
  role: "operator_desktop"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-29"
---

# arda-launcher

Operator desktop launcher for the ARDA ecosystem — the title screen of the
world. A Tauri 2 desktop application with a React + TypeScript + Tailwind v4
frontend that renders a calm atmospheric onboarding experience: a starfield, a
slowly growing world-tree, and a single **Begin** call to action.

Owner: hades | Sigil: 🜏 BOOKMARK | Status: active

## Summary

`arda-launcher` is the front door Arda presents to an operator. It is a
cross-platform desktop app built with Tauri 2, but it is also structurally
the operator onboarding surface: prerequisite detection, environment profiling,
provider checklisting, guided session creation, service-plan generation, and
private-config staging all happen in the Rust side under `onboarding/`.

The frontend now consumes the read-only registry, readiness, and service-plan
commands through one tested TypeScript contract and displays onboarding output
without acquiring service mutation or approval authority.

## Where it lives

- App root: `/var/home/mythos/Eregion/Arda/apps/arda-launcher`
- Rust backend: `apps/arda-launcher/src-tauri/`
- Frontend: `apps/arda-launcher/src/`
- Build/config: `src-tauri/tauri.conf.json`, `vite.config.ts`, `package.json`

## Verification status

- `cargo test -p arda-launcher --all-features`: 8 passed
- `cargo fmt -p arda-launcher -- --check`: passed
- `cargo clippy -p arda-launcher --all-targets --all-features -- -D warnings`:
  passed
- `pnpm test`: 2 command-contract tests passed
- `pnpm run lint`: 0 warnings and 0 errors
- `pnpm run build`: passed
- Frontend package, Cargo package, and Tauri bundle versions: aligned at `0.3.0-rc.0`
- `pnpm run tauri build`: v0.2 release binary, DEB, and RPM produced; AppImage
  stage reaches a populated AppDir but Tauri's cached `linuxdeploy` fails on
  CentOS 10 `.relr.dyn` sections
- Direct `appimagetool` fallback: v0.2 AppImage assembled and extracted with
  `AppRun`, desktop entry, icon, and launcher binary present

## Binary / runtime

- Desktop entry: `src-tauri/src/main.rs` calls `arda_launcher_lib::run()`
- `src-tauri/src/lib.rs`: registers the typed `registry_status`,
  `readiness_status`, `service_plan_status`, and intrinsic `release_identity`
  commands; no `greet` command
- `src-tauri/src/onboarding/mod.rs`: includes `tests.rs` through
  `#[cfg(test)] mod tests`, so the onboarding suite is compiled normally
- Commands expose read-only registry/readiness/service-plan projections; the
  apply/private-config functions are not registered as frontend commands
- `src-tauri/build.rs`: Tauri codegen hook
- Window config: `tauri.conf.json`

## Frontend

- `src/main.tsx`: React root, mounts `<App/>`
- `src/App.tsx`: registry-gated scene that loads a typed onboarding snapshot
- `src/components/OnboardingPanel.tsx`: accessible readiness/service-plan panel
- `src/lib/tauri-core-compat.ts`: typed Tauri command contract and snapshot load
- `src/lib/tauri-core-compat.test.ts`: exact command/argument/payload tests
- `src/components/ArdaLogo.tsx`: ARDA badge/mark
- `src/components/ParticleSmoke.tsx`: `<canvas>`-based starfield / smoke
- `src/scenes/components/WorldTree.tsx`: animated SVG world-tree
- `src/scenes/state/OnboardingText.tsx`: title + subtitle + Begin CTA
- `src/scenes/Background.tsx`: full-bleed animated container
- `src/styles/colors.ts`: shared palette
- `src/.oxlintrc.json`: lint config
- `src/vite-env.d.ts`: ambient type declarations

## Operator onboarding Rust modules

| Module | Role |
|--------|------|
| `types.rs` | Shared serializable types: `OperatorAnswers`, `GuidedSession`,
`EnvironmentProfile`, `PathValue`, `UrlValue`, `ServicePlan`, `ApprovalReceipt`,
`ApplyResult`, `ProviderChecklist`, `DeviceScan`, `PrerequisiteReport`,
`PrivateConfigStage`, `ReadinessProjection` |
| `guided.rs` | `build_guided_session()`: assembles a multi-step onboarding
session from environment profile, operator answers, device scan, prerequisite
report, and provider checklist; returns `GuidedSession` with ordered steps
| `constants.rs` | Contract version strings for all onboarding artifacts:
`arda.onboarding.*.v1` shape |
| `prerequisites.rs` | `build_prerequisite_report()`: detects platform/tools
state and returns structured checks |
| `environment.rs` | `build_environment_profile()` and `workspace_root()`:
detects paths, endpoints, systemd, safety posture |
| `provider.rs` | `provider_checklist()`: inspects configured providers,
resolves keys/models, builds provider guidance |
| `device.rs` | `device_scan()`: host/architecture/container/Tailscale/runtime
capability snapshot |
| `private_config.rs` | Build/propose/write/parse operator private config
and environment baseline; `OperatorAnswers` template |
| `service_plan.rs` | `build_service_plan()`, `apply_service_plan()`,
`parse_approval_receipt()`: human-gated apply workflow |
| `readiness.rs` | `build_readiness_projection()` and
`l3_readiness_onboarding_checklist()`: L3-level handoff gating |
| `io.rs` | JSON read/write helpers, run directory resolution, receipt
writing, profile/readiness persistence |
| `helpers.rs` | `now_utc()` helper |
| `console.rs` | `launch_console()`: exposes First Light / local console
open behavior |
| `tests.rs` | In-module tests for onboarding behavior |

## Consumer wiring

- `arda-launcher` depends on `arda-core` for governance primitives and
  `arda-contract-registry` for registry resolution.
- `EnvironmentProfile` discovers Manwe through `MANWE_BASE_URL` or
  `ARDA_MANWE_BASE_URL`; launcher production code has no `:7171` literal.
- Manwe, engine, root daemon, service registry, scripts, and tests still share
  the workspace's `:7171` compatibility default. Changing it requires a
  coordinated fleet/consumer migration, not a launcher-local replacement.
- No direct Rust package consumes `arda-launcher`; the frontend is its direct
  command consumer.

## Improvement ideas

1. Add an explicit user-confirmed command only when service-plan application is
   ready; preserve the existing backend human-gate receipts.
2. Generate frontend bindings from Rust contracts if command growth makes the
   hand-maintained TypeScript mirror unsafe.
3. Coordinate any `:7171` default migration across Manwe, engine, daemon,
   registry, scripts, and fleet configuration before changing compatibility.
4. Repair or replace Tauri's AppImage `linuxdeploy` toolchain for modern
   `.relr.dyn` binaries; direct `appimagetool` assembly is the verified fallback.
5. Add `sysinfo`-driven resource checks (RAM/disk) to prerequisites before
   recommending local assistant / local model routes
6. Consider extracting the onboarding Rust module into a workspace crate if
   `arda-engine` or other crates need to reuse it outside Tauri context
