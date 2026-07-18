---
soterion:
  sigil: "BOOKMARK"
  glyph: "🚀"
  code_point: "U+1F680"
  role: "operator_desktop"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-17"
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

Frontend is intentionally lightweight for now — atmospheric background, logo,
phase-animated CTA — and the backend is the real work. The launcher is
expected to grow into the bootstrapper that hands the operator off to live
ARDA services.

## Where it lives

- App root: `/var/home/mythos/Eregion/Arda/apps/arda-launcher`
- Rust backend: `apps/arda-launcher/src-tauri/`
- Frontend: `apps/arda-launcher/src/`
- Build/config: `src-tauri/tauri.conf.json`, `vite.config.ts`, `package.json`

## Verification status

- `cargo check -p arda-launcher`: OK
- Build output: `Finished "dev" profile` in ~2s
- Warnings: only upstream `arda-core` unused imports/warnings, none in
  `arda-launcher` itself
- Tests: no unit/doc tests in this crate build; tests exist under
  `src-tauri/src/onboarding/tests.rs` but are not auto-discovered by default
  Tauri build layout

## Binary / runtime

- Desktop entry: `src-tauri/src/main.rs` calls `arda_launcher_lib::run()`
- `src-tauri/src/lib.rs`: thin Tauri builder surface; currently registers one
  sample command `greet`; the real surface is the `onboarding` moduleIndex +
  Tauri commands produced by `build_guided_session()`
- Tauri commands are exposed to the frontend via `#[tauri::command]` handlers
- `src-tauri/build.rs`: Tauri codegen hook
- Window config: `tauri.conf.json`

## Frontend

- `src/main.tsx`: React root, mounts `<App/>`
- `src/App.tsx`: phase-animated composition; `ParticleSmoke`, `WorldTree`,
  `OnboardingText`, `ArdaLogo`, `Background` layered under a `<Canvas>`
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

- `arda-engine`: uses `arda-launcher` types for operator-side bootstrap state
- `arda-hud`: reads launcher/projections when surfacing readiness state
- `arda-launcher`: itself depends on `arda-core` for task, contract, and
  service-registry primitives
- `manwe`: launcher assumes manwe on `:7171` when computing endpoints

## Improvement ideas

1. Remove orphaned `src/App.css` and the unused `greet` sample command from
   `lib.rs`; replace with real Tauri commands that drive the onboarding flow
2. Wire `build_guided_session()` and the onboarding outputs into actual
   frontend panels instead of the current placeholder phase animation only
3. Add `onboarding/tests.rs` coverage to CI and fix any compilation gaps
4. Move hardcoded `127.0.0.1:7171` assumptions to configurable endpoints
   discovered via `EnvironmentProfile`
5. Add `sysinfo`-driven resource checks (RAM/disk) to prerequisites before
   recommending local assistant / local model routes
6. Consider extracting the onboarding Rust module into a workspace crate if
   `arda-engine` or other crates need to reuse it outside Tauri context
