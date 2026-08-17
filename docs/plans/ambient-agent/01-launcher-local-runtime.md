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

# Phase 1: Launcher and Local Runtime Implementation Plan

> **For Hermes:** Use the `subagent-driven-development` skill to implement this plan task-by-task.

**Goal:** After a real login/restart, one desktop click opens Arda Launcher, starts or reconnects to an allowlisted deterministic runtime, reports honest health, and opens the native Arda HUD without tying runtime lifetime to the launcher process.

**Architecture:** A user-systemd target owns backend lifetime. Launcher is a thin Tauri control/status client over typed lifecycle commands. HUD starts as its own user unit only after required health gates pass. Desktop packaging installs one `.desktop` entry and icon.

**Tech stack:** Rust, Tauri 2, React/TypeScript/Vitest, `arda-core` systemd client, user systemd, Freedesktop desktop entry, existing `scripts/launch_arda_hud.sh`.

---

## Current source baseline

- `apps/arda-launcher/src-tauri/src/lib.rs` exposes registry/readiness/service-plan inspection commands but does not expose `apply_service_plan` or `launch_console`.
- `apps/arda-launcher/src-tauri/src/onboarding/service_plan.rs` already implements an approval-aware service-plan application path; reuse or narrow it rather than introducing arbitrary shell execution.
- `config/systemd/arda.service` owns the canonical root runtime and is enabled through `default.target`; no Arda session target currently exists under `config/systemd/`.
- `scripts/launch_arda_hud.sh` locates the newest native HUD/AppImage and falls back to a browser preview. Phase 1 must reject preview fallback for native acceptance.
- No tracked `.desktop` file currently exists in the repository.
- `apps/arda-launcher/package.json` defines `test`, `lint`, `build`, and Tauri commands.

## Runtime contract

Create `arda.system-lifecycle.v1` with:

- aggregate state: `stopped | starting | healthy | degraded | failed | stopping | unknown`;
- component id, required/optional classification, owning unit, enablement, active/sub state;
- protocol-level health state and last checked time;
- source and freshness for every observation;
- bounded diagnostic code/message;
- allowed recovery action id, never an arbitrary command;
- HUD native availability and running state;
- Hermes gateway availability as an independently observed component;
- no secret-bearing environment, command output, or journal body.

Process-active is not protocol-healthy. Unknown and stale remain explicit.

## Task 1: Freeze lifecycle types and state reduction

**Files:**
- Create: `apps/arda-launcher/src-tauri/src/lifecycle/types.rs`
- Create: `apps/arda-launcher/src-tauri/src/lifecycle/mod.rs`
- Test: inline Rust unit tests in those modules

**Steps:**
1. Write failing table tests for aggregate state reduction, including stopped, starting, required failure, optional degradation, stale observation, and all-healthy cases.
2. Run `cargo test -p arda-launcher lifecycle -- --nocapture`; expect missing module/type failure.
3. Implement strict Serde types and a pure reducer.
4. Repeat the focused test; expect all lifecycle tests to pass.
5. Commit only lifecycle types/tests: `feat(launcher): define system lifecycle contract`.

**Task 1 evidence (2026-08-17):** **Implemented** and **tested** in
[`lifecycle/types.rs`](../../../apps/arda-launcher/src-tauri/src/lifecycle/types.rs)
and [`lifecycle/mod.rs`](../../../apps/arda-launcher/src-tauri/src/lifecycle/mod.rs).
The focused RED command failed on the expected missing lifecycle symbols; GREEN
passed 5 lifecycle tests. Strict Clippy and the launcher binary check also pass.
`lib.rs` exports the module for compilation only; no lifecycle command, systemd
observer, or runtime path is wired yet. Task 1 does not make Phase 1 proven.

## Task 2: Build bounded systemd and health observations

**Files:**
- Create: `apps/arda-launcher/src-tauri/src/lifecycle/systemd.rs`
- Create: `apps/arda-launcher/src-tauri/src/lifecycle/health.rs`
- Modify: `apps/arda-launcher/src-tauri/src/lifecycle/mod.rs`
- Test: module unit tests with fixture command/HTTP adapters

**Steps:**
1. Add failing tests for missing unit, inactive unit, failed unit, active-but-unhealthy endpoint, timeout, malformed payload, and healthy response.
2. Inspect and reuse the public `arda-core` systemd abstraction where it supplies typed calls. If it cannot represent the required user-unit state, add the narrow adapter locally; do not expose generic `systemctl` arguments to the frontend.
3. Implement hard timeouts and output bounds.
4. Define the first required-component allowlist in one backend-owned constant/config path. Initial candidates must be verified against installed reality before freezing: Arda root runtime, Hermes Gateway, and any service needed for continuity. Varda/RELIC remain optional until their phase.
5. Run focused Rust tests and `cargo clippy -p arda-launcher --all-targets --all-features -- -D warnings`.
6. Commit: `feat(launcher): observe bounded runtime health`.

**Task 2 evidence (2026-08-17):** **Implemented** and **tested** in
[`lifecycle/systemd.rs`](../../../apps/arda-launcher/src-tauri/src/lifecycle/systemd.rs),
[`lifecycle/health.rs`](../../../apps/arda-launcher/src-tauri/src/lifecycle/health.rs),
and [`lifecycle/mod.rs`](../../../apps/arda-launcher/src-tauri/src/lifecycle/mod.rs).
The focused RED command failed on the expected missing observer APIs; GREEN
passes 20 lifecycle tests and all 34 launcher library tests. The adapters enforce
fixed allowlists, two-second timeouts, bounded output, and fixed diagnostics
without retaining command, HTTP, or journal bodies. Live discovery confirmed
`arda.service` and `hermes-gateway.service` are installed, enabled, and active;
the allowlisted Manwë `/healthz` contract returned `ok=true`. No Hermes protocol
endpoint was verified, so gateway protocol health remains explicitly unavailable
rather than inferred from its active process. `arda-core`'s typed systemd client
was inspected but does not expose `UnitFileState` or bounded `show` calls, so the
launcher uses a narrower fixed-argument adapter. Strict Clippy, launcher binary
check, formatting, and diff checks pass. These observers are not yet exposed by
Tauri commands, so Task 2 does not make Phase 1 wired, running, or proven.

## Task 3: Add the user-systemd session and HUD units

**Files:**
- Create: `config/systemd/arda-session.target`
- Create: `config/systemd/arda-hud.service`
- Modify: `config/systemd/README.md`
- Modify or create the existing canonical installer under `scripts/` after locating it; do not add a second competing installer
- Test: shell/systemd verification script next to the canonical installer

**Steps:**
1. Write a failing verifier that checks unit syntax, dependency direction, executable paths, and absence of repository build paths in installed units.
2. Define `arda-session.target` for backend lifecycle only. Closing Launcher or HUD must not stop this target.
3. Define `arda-hud.service` as a separate graphical-session unit invoking the packaged native HUD path. Do not use Vite preview for this unit.
4. Ensure installation imports required graphical-session environment without embedding session-specific values in tracked files.
5. Run `systemd-analyze --user verify` on staged units in a temporary install root where supported.
6. Install to the user unit directory, run `systemctl --user daemon-reload`, and verify unit discovery.
7. Commit: `feat(runtime): add Arda session and HUD user units`.

**Task 3 evidence (2026-08-17):** **Implemented** and **tested** in
[`arda-session.target`](../../../config/systemd/arda-session.target),
[`arda-hud.service`](../../../config/systemd/arda-hud.service), and the
[`user-unit installer`](../../../scripts/install_arda_user_units.sh) with its
[`verifier`](../../../scripts/verify_arda_user_units.sh). The RED verifier failed
on the expected missing session target. GREEN passes source, temporary-install,
and installed-unit verification plus negative fixtures for repository build paths,
reversed HUD dependency, and transactional rollback after manager failure. A
canonical no-bundle HUD build was installed at
`%h/.local/lib/arda/hud/arda_hud`; source and installed SHA-256 matched. The
installer imported only the named graphical-session environment variables,
reloaded the live user manager, and verified both units are discovered. Live
state remains intentionally non-started: `arda-session.target` is disabled and
inactive, while `arda-hud.service` is static and inactive. Closing or stopping
HUD cannot stop the backend target, and no browser-preview path is present.
This establishes installed unit mechanics, not a wired launcher command, native
HUD launch acceptance, restart proof, or Phase 1 proven status.

## Task 4: Expose narrow lifecycle Tauri commands

**Files:**
- Create: `apps/arda-launcher/src-tauri/src/lifecycle/commands.rs`
- Modify: `apps/arda-launcher/src-tauri/src/lib.rs`
- Test: command-helper unit tests; no frontend mocks for backend policy

Commands:

- `lifecycle_status()` — read only;
- `start_arda_session()` — starts only `arda-session.target`;
- `stop_arda_session()` — explicit confirmation token and only the Arda target;
- `recover_component(action_id)` — backend allowlist only;
- `launch_native_hud()` — allowed only after required health and native binary checks;
- `hud_status()` — read only.

**Steps:**
1. Write failing authorization/allowlist tests.
2. Implement command helpers with typed errors and bounded polling.
3. Register only the narrow handlers in `tauri::generate_handler!`.
4. Test that arbitrary unit names and action ids are rejected.
5. Run Rust tests and Clippy.
6. Commit: `feat(launcher): expose governed lifecycle commands`.

**Task 4 evidence (2026-08-17):** **Implemented** and **tested** in
[`lifecycle/commands.rs`](../../../apps/arda-launcher/src-tauri/src/lifecycle/commands.rs)
and registered through the launcher [`lib.rs`](../../../apps/arda-launcher/src-tauri/src/lib.rs).
RED failed on the expected missing command/control APIs. GREEN passes 5 focused
command-policy tests and 25 lifecycle tests. The backend accepts only fixed unit
and recovery identities, requires the exact session-stop confirmation, suppresses
command output, applies command and polling timeouts, and refuses native HUD
launch unless required aggregate health is healthy and the installed binary is
present. Strict Clippy, locked launcher check, formatting, and diff checks pass.
No frontend consumer or native launch acceptance is claimed by this task.

## Task 5: Replace readiness claims with source-truth lifecycle UI

**Files:**
- Modify: `apps/arda-launcher/src/App.tsx`
- Modify/create components under `apps/arda-launcher/src/components/`
- Modify/create typed API helpers under `apps/arda-launcher/src/lib/`
- Test: colocated `*.test.tsx` and helper tests

**Steps:**
1. Write failing Vitest cases for stopped, starting, healthy, degraded, failed, stale, and command-error states.
2. Render one primary action based on lifecycle state: Start, Open HUD, Retry bounded recovery, or Inspect failure.
3. Show required component truth without a generic dashboard card wall: component, state, freshness, and concise recovery meaning.
4. Preserve launcher as ignition/status, not orchestration or task management.
5. Add a preference: close after HUD opens or remain open. The preference changes window behavior only, never service lifetime.
6. Run `pnpm test`, `pnpm run lint`, and `pnpm run build` in `apps/arda-launcher`.
7. Commit: `feat(launcher): present source-truth startup flow`.

## Task 6: Package the desktop icon and installation path

**Files:**
- Create: `apps/arda-launcher/packaging/linux/io.arda.Launcher.desktop`
- Add icons under the existing Tauri icon path only after inspecting `tauri.conf.json`
- Modify: canonical package/install scripts and `apps/arda-launcher/README.md`
- Test: desktop-entry validator and install smoke

**Steps:**
1. Add a failing check that `Exec` points to the installed launcher binary, not the repository or a terminal wrapper.
2. Define name, comment, icon, application categories, startup notification, and single-instance behavior.
3. Validate with `desktop-file-validate` where available.
4. Package/install the launcher using the repository's Tauri flow.
5. Verify the desktop environment indexes the application and its icon.
6. Commit: `feat(launcher): install Arda desktop entry`.

## Task 7: Implement health-gated HUD launch and recovery

**Files:**
- Modify: lifecycle modules and launcher UI from Tasks 2–5
- Test: Rust integration tests plus frontend state tests

**Steps:**
1. Write a failing test proving HUD cannot launch while required health is failed or stale.
2. Add bounded polling with visible elapsed state and cancellation.
3. Start the HUD unit only once; repeated clicks must focus/reuse or report already running.
4. Verify Launcher exit leaves both `arda-session.target` and HUD service running.
5. Verify HUD failure does not stop backend services and Launcher can retry it.
6. Commit: `feat(launcher): gate native HUD on runtime health`.

## Task 8: Native restart acceptance

**Evidence artifact:** create an operations acceptance record only after execution under `docs/operations/` using the existing documentation convention.

**Run:**
1. Build/package with `pnpm run tauri build` from `apps/arda-launcher` and the canonical HUD package command.
2. Install both packages and user units.
3. Log out/in or perform a real restart.
4. Confirm the desktop icon is visible without opening a terminal.
5. Click it; capture launcher transitions from stopped/starting to healthy or an honest degraded/failed state.
6. Confirm the native HUD window appears only after required health.
7. Close Launcher; verify Hermes/Arda runtime and HUD remain available.
8. Reopen Launcher; verify it reconnects instead of duplicating services.
9. Stop one required service; verify degraded/failed truth and bounded recovery.
10. Restart once more and repeat the happy path.

## Phase gate

Phase 1 is **proven** only when the real post-restart click path passes. Builds, unit tests, a `.desktop` file, or a manually launched dev server are insufficient. The acceptance record must identify installed artifact versions, observed units, health probes, native windows, failure injection, and date. Do not mark the phase supported until install/upgrade/rollback is also reproduced.
