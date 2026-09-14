---
soterion:
  sigil: "ANCHOR"
  role: "phase_0_baseline"
  owner: "HERMES"
  status: "complete_with_recorded_native_limitations"
  reviewed: "2026-08-16"
---

> 🜏 Soterion: ⚓ HUD convergence Phase 0 baseline | owner: HERMES | status: complete with recorded native limitations

# Phase 0 — Verified HUD Baseline

Captured at `2026-08-16T10:35:27Z` before application implementation began.

## Source boundary

- Repository: `/var/home/mythos/Eregion/Arda`
- Branch: `visual/hud-boardroom-convergence`
- Source commit: `b2eabb87d3667079fc27579c3e963a3515a79a1a`
- User-owned modified files, deliberately untouched:
  - `core/state/queue_active.json`
  - `core/state/queue_summary.json`
- Assistant changes at this point are confined to `docs/plans/arda-hud-lower-workstations/`.
- No application source or runtime-state file was changed during Phase 0.

## Automated baseline

| Gate | Result | Evidence |
|---|---|---|
| HUD Vitest suite | PASS — 134 files, 526 tests | [`evidence/phase0-vitest-20260816.log`](evidence/phase0-vitest-20260816.log) |
| TypeScript + Vite production build | PASS — 2,613 modules transformed | [`evidence/phase0-build-20260816.log`](evidence/phase0-build-20260816.log) |
| Diff whitespace check | PASS | `git diff --check` returned no output |

The production build retains one known Vite warning: `boardroomSlotSettings.ts` is both statically and dynamically imported, so its dynamic import does not create a separate chunk. This is a composition-authority concern for Phase 1, not a build failure.

## Runtime and native acceptance baseline

The existing native development runtime was attached rather than restarted:

- `pnpm run tauri dev` active;
- Vite listening on `[::1]:1421`;
- native binary `/var/home/mythos/.cache/arda-build/target/debug/arda_hud` active;
- AT-SPI application `arda_hud` exposed a `1920 × 1080` `ARDA HUD` frame.

The native semantic capture found:

- 96 total AT-SPI nodes;
- 42 named buttons;
- 36 meaningful semantic controls after excluding window chrome;
- zero duplicate meaningful control names;
- `Open settings`, `Open Hermes CLI`, and `Open Hermes dashboard` all present.

Evidence: [`evidence/phase0-native-atspi-20260816.json`](evidence/phase0-native-atspi-20260816.json).

Direct screenshot capture was not available from the automation session:

- `computer_use` returned no capturable windows even though AT-SPI exposed the native application;
- GNOME Shell rejected non-interactive `ScreenshotArea` with `org.freedesktop.DBus.Error.AccessDenied`.

The AT-SPI artifact is therefore the current native acceptance artifact. No screenshot success is claimed.

## Native development-process profile

The repository profiler sampled the already-running development process tree for 60.057 seconds at 10-second intervals. It used the non-secret `XDG_SESSION_TYPE=wayland` environment marker to attach to the existing `arda_hud` process because the runtime had not been launched with an `ARDA_HUD_PROFILE_TOKEN`.

Evidence: [`evidence/phase0-native-profile-20260816.json`](evidence/phase0-native-profile-20260816.json).

| Metric | Development runtime | Qualified release limit | Disposition |
|---|---:|---:|---|
| aggregate CPU, one-core percent | 105.47% | 40.0% | outside limit |
| PSS peak | 912.17 MiB | 550.0 MiB | outside limit |
| PSS growth | 41.90 MiB | 16.0 MiB | outside limit |
| RSS peak | 1,033.43 MiB | 700.0 MiB | outside limit |

This is an explicit pre-existing development-runtime failure, not release qualification and not a measured frame-rate value. The attached process had been running for more than three hours and includes Vite development behavior. The current release baseline remains the separately recorded host-qualified Wayland profile in `docs/evidence/0.9/hud-native-performance-accessibility-baseline-20260814.json`; later implementation phases must compare like-for-like release measurements and must not use this development profile to claim qualification.

## Source and interaction contract snapshot

### Lower workstation assignment authority currently persisted

From `core/state/arda_boardroom_slots.json`:

| Slot | Zone | Modules | Focus mode |
|---|---|---|---|
| `view_desk_l` | `governance_guardhouse` | `governance_controls`, `section_focus` | native window |
| `view_desk_control_panel` | `fleet_and_backbone` | `systems`, `operations_and_packages` | in-scene workstation |
| `view_desk_r` | `routing_and_comms` | `systems`, `operations_and_packages` | native window |
| `view_desk_aux` | `human_business_personal` | `human_realm`, `business` | native window |

This confirms the known Fleet/Routing module duplication and the omission of Personal from the Human/Business/Personal assignment.

### First-level output contract

`apps/arda-hud/src/lib/firstLevelTerminalContracts.ts` currently resolves five first-level surfaces:

1. `governance_guardhouse`
2. `fleet_and_backbone`
3. `routing_and_comms`
4. `human_business_personal`
5. `command_core_now`

`command_core_now` is fixed to `boardroom.control.center`; the four desk workstations use persisted or fallback assignments.

### Physical action contract

`BOARDROOM_PHYSICAL_CONTROL_ACTIONS` currently exposes these IDs in order:

1. `service_health_status`
2. `open_settings`
3. `open_hermes_cli`
4. `open_hermes_dashboard`
5. `open_approval_queue`
6. `open_command_core`
7. `open_emergency_stop`
8. `open_route_selector`
9. `enter_world`

The utility actions already retain typed target zones, authority classes, and verification paths. Phase 2 must relocate these existing actions rather than create replacements.

### Native accessible utility contract

`BoardroomAccessibilityControls.tsx` currently exposes semantic controls for:

- `Open Hermes dashboard`
- `Open Hermes CLI`
- `Open settings`

The Phase 0 AT-SPI artifact confirms all three are present in the running native WebKit tree.

## Phase 0 disposition

Phase 0 exit criteria are met with explicit limitations:

- current automated behavior is reproducible;
- native semantics are reproducible through AT-SPI;
- screenshot capture and a current frame-rate measurement are unavailable and are not claimed;
- the development process profile is isolated as outside the release performance contract;
- user-owned queue projections are identified and remain untouched.

Phase 1 may proceed. Any visual implementation phase must still obtain native visual acceptance through a functioning capture path, and Phase 10 must run a like-for-like release profile before performance qualification.
