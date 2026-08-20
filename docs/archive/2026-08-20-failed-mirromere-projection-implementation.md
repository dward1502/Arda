---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "implementation_plan"
  owner: "HERMES"
  status: "archived"
  reviewed: "2026-08-20"
---

> 🜏 Soterion: 📜 implementation_plan | owner: HERMES | status: archived | reviewed: 2026-08-20

# Failed Mirromere Projection Implementation Record

> **Historical warning:** This record implemented a standalone shell, display lifecycle, and decorative status projection, then briefly replaced that projection with the Hermes dashboard. Neither behavior was Mirromere. This file has no active implementation authority; it is retained to prevent the same substitution from being repeated.

> **For Hermes:** Implement this plan directly. Product behavior is accepted by the human operator; tests and accessibility inspection do not substitute for useful behavior.

**Goal:** Make the standalone second-display Mirromere useful: it must host a real application or continue a real Hermes conversation. Decorative status scenes, radar/vector pictures, labels, and backend projection alone do not satisfy this plan.

**Corrected architecture:** A backend-owned `arda.mirromere.surface.v1` projection describes scene/application state. The in-world HUD aperture is only a simulation/design lens. Operational Mirromere must be a standalone application and process with its own Tauri package, binary, lifecycle, display ownership, close behavior, and packaging. It may reuse shared contract/renderer code, but it must not be a child `WebviewWindow` owned, reopened, or repositioned by `arda_hud`.

**Tech stack:** React 19, TypeScript, Vitest, Three.js/react-three-fiber, separate Tauri 2 application packages with monitor APIs, Rust source projection, existing ARDA workstation and boardroom components.

---

## Product boundary

**Reopened after operator rejection (2026-08-20):** The archived closeout was false at the product level. The standalone app displayed decorative scene projections but did not host an application or provide a usable Hermes conversation. Phase 3 is active again. The first corrective implementation replaces the radar default with the live Hermes dashboard; avatar presence, local voice, and operator acceptance remain open.

The boardroom/HUD remains the operator's desktop embodiment. The HUD World View is display-only and intentionally sparse; it is not acceptance for application workflows. Mirromere is a calm display/application outpost. The useful two-for-one is to reuse the same Mirromere surface contract in:

1. one HUD screen/aperture as a simulation and design lens; and
2. one standalone native Mirromere application on the second physical monitor as operational acceptance.

Do not duplicate application logic in a Three.js texture and a second React page. Both consumers use the same typed model and interaction ids, but their process and lifecycle ownership remain separate.

## Architecture correction and HUD regression (2026-08-20)

The Task 5 experiment implemented Mirromere as a borderless `arda-mirromere` child window inside the `arda_hud` process. That interpretation is rejected as the product architecture.

The experiment also introduced a concrete HUD regression:

- ARDA HUD persisted the selected display and called `open_mirromere_window` immediately and every two seconds.
- Closing the child window therefore did not mean “stay closed”; HUD recreated it on the next poll.
- The child window was created with `focused(false)`, so its page-level Escape handler could not reliably receive keyboard input.
- A full-display borderless child could consequently remain over the operator's dashboard and appear impossible to close.

Immediate containment removes the HUD auto-reopen loop and the HUD control that opens the child display. The acceptance process used during Task 7 was terminated. The committed child-window experiment remains migration/retirement input only; it is not accepted Mirromere architecture and must not be enabled as a HUD lifecycle feature.

Required correction before Phase 3 can continue:

1. Create a standalone application boundary (provisional path `apps/arda-mirromere`, subject to repository ownership review) with its own Tauri package and binary.
2. Move or share only the strict surface contract, bounded renderer, and backend client needed by that application.
3. Remove the child-window route, display-selection persistence, polling, and native-window commands from `arda_hud` after the standalone consumer is proven.
4. Require ordinary application close semantics: Escape or an explicit close action closes Mirromere and it stays closed unless the operator launches it again.
5. Repeat native display, disconnect/reconnect, accessibility, and performance acceptance against the standalone binary, not `arda_hud`.

## Current source baseline

- `apps/arda-hud/src/components/arda/core/SceneWorkstation.tsx` and `PanelWorkspace.tsx` implement existing workstation presentation paths.
- `apps/arda-hud/src-tauri/src/lib.rs` already contains native window creation machinery and workstation request/result types.
- `apps/arda-hud/src-tauri/tauri.conf.json` currently declares one fullscreen transparent native window.
- `apps/arda-hud/src/lib/ardaSource.ts` owns backend-sourced HUD projections.
- Existing source tests distinguish fixture/fallback/runtime modes; preserve that honesty.
- The current visual branch is the correct implementation lineage for HUD changes; isolate shared schemas before parallel renderer work.

## Mirromere surface contract

Create `arda.mirromere.surface.v1` with:

- surface/outpost id and intended display role;
- scene/application id, version, and human-readable purpose;
- slots with typed content: status, text, media reference, vector/radar/wave field, conversational presence, or registered app view;
- data source and evidence references;
- freshness/expiry and explicit unavailable state;
- privacy class and visibility ceiling;
- interaction ids from a backend allowlist;
- accessibility description, reduced-motion behavior, and urgency;
- transition policy and attention budget;
- no arbitrary HTML, JavaScript, URL, shell command, or unsanitized remote media.

First registered scenes:

- `ambient.idle`;
- `system.starting` / `system.degraded`;
- `conversation.presence`;
- `continuity.handoff-ready`;
- `research.focus` using Varda provenance;
- `privacy.veil`;
- `offline.local`.

## Task 1: Inventory and freeze the HUD integration seam

**Files:**
- Read/trace: `apps/arda-hud/src/App.tsx`
- Read/trace: `apps/arda-hud/src/components/arda/core/BoardroomStage.tsx`
- Read/trace: `SceneWorkstation.tsx`, `PanelWorkspace.tsx`, `types.ts`
- Read/trace: `apps/arda-hud/src/lib/ardaSource.ts`
- Read/trace: `apps/arda-hud/src/utils/multiWindow.ts`
- Create: `apps/arda-hud/src/features/mirromere/INTEGRATION.md`

**Steps:**
1. Identify the exact upper-monitor aperture that can host a display-only preview without replacing World View acceptance.
2. Identify the existing native window query/routing convention.
3. Record current source-mode, interaction, monitor, and reduced-motion contracts.
4. Select one existing scene/workstation path; do not add a parallel window manager.
5. Run existing focused tests before edits and record the baseline command/results.
6. Commit the seam inventory separately.

## Task 2: Define strict shared surface types and fixtures

**Files:**
- Create schema/types in the existing cross-boundary contract location selected after source trace
- Create: `apps/arda-hud/src/features/mirromere/types.ts`
- Create: `apps/arda-hud/src/features/mirromere/fixtures.ts` for tests only
- Test: Rust strict-deserialization and TypeScript parser/type-guard tests

**Steps:**
1. Write failing cases for valid idle/degraded/handoff scenes, unknown field, expired scene, privacy escalation, unknown interaction, arbitrary URL/HTML, oversized slot, and missing evidence source.
2. Implement strict backend types and bounded frontend decoding.
3. Ensure fixture mode is visibly and structurally distinct from live runtime mode.
4. Generate or validate TypeScript against the canonical schema rather than hand-maintaining divergent enums.
5. Run focused Rust and Vitest suites.
6. Commit: `feat(hud): define Mirromere surface contract`.

## Task 3: Add backend-owned Mirromere projection

**Files:**
- Extend the current HUD backend/source projection path; exact module chosen from Task 1 trace
- Modify: `apps/arda-hud/src/lib/ardaSource.ts`
- Test: backend projection and source-mode tests

**Steps:**
1. Write failing tests for runtime, stale, unavailable, privacy veil, and fixture isolation.
2. Compose surface state from Phase 1 lifecycle and Phase 2 continuity references; do not read random files directly from React.
3. Add a bounded projection endpoint/Tauri command following the existing source pattern.
4. Preserve source timestamps and evidence links.
5. Run backend tests and direct consumer checks.
6. Commit: `feat(hud): project governed Mirromere scenes`.

## Task 4: Render the in-world HUD aperture

**Files:**
- Create: `apps/arda-hud/src/features/mirromere/MirromereAperture.tsx`
- Modify: the selected boardroom monitor component from Task 1
- Add styles/shaders only under the existing HUD feature/style convention
- Test: `MirromereAperture.test.tsx` plus affected boardroom tests

**Steps:**
1. Write failing tests for idle, lifecycle degraded, handoff-ready, privacy veil, stale, and reduced-motion views.
2. Build a nearly textless vector/radar/wave presentation consistent with the lower/upper screen role being used; avoid generic dashboard cards and repeated labels.
3. Make provenance/state inspectable through the existing workstation or detail path without filling the scene with text.
4. Route interactions through existing activation callbacks and backend allowlisted ids.
5. Verify no polling/render-loop FPS regression.
6. Run focused tests, full HUD tests, lint, and build.
7. Commit: `feat(hud): render Mirromere proving aperture`.

## Task 5: Rejected experiment — HUD-owned second-monitor window

**Files:**
- Modify: `apps/arda-hud/src-tauri/src/lib.rs`
- Modify: `apps/arda-hud/src/utils/multiWindow.ts`
- Create: native Mirromere window route/component under existing app routing convention
- Test: Rust monitor-selection unit tests and frontend route tests

**Steps:**
1. Write failing pure tests for requested display id present, absent, disconnected, primary-display fallback disabled, and geometry change.
2. Enumerate monitors through Tauri; persist a stable operator selection separately from transient coordinates.
3. Open one borderless native Mirromere window on the selected second monitor. Do not steal primary-display focus after initial operator configuration.
4. On disconnect, close or veil the surface and report unavailable; never silently move private content to the primary display.
5. Reuse the exact `MirromereSurface` renderer/model from Task 4 with an environment adapter for native geometry.
6. Run Rust tests, Vitest, lint, build, then `pnpm run tauri build`.
7. Commit: `feat(hud): project Mirromere to selected display`.

**Implementation evidence (2026-08-19; invalidated as product acceptance on 2026-08-20):**

- Added Tauri-owned stable display enumeration and fail-closed resolution. The native command rejects absent, disconnected, ambiguous, and primary-only selections, closes an existing Mirromere window when the selected display becomes unavailable, and updates physical geometry without focusing the window.
- Added a persisted stable-id frontend bridge, a two-second topology refresh that re-reads the current operator selection without rewriting storage, and a standalone native route that reuses the in-world aperture's visual model, motion policy, and canvas renderer.
- RED: the focused Vitest run failed because `MirromereNativeSurface` did not exist. GREEN: the focused frontend matrix passed 13 tests; `cargo test --lib mirromere_window_tests` passed 5 tests; the full HUD suite passed 604 tests; the full Rust library suite passed 74 tests with 2 explicitly ignored Chromium-dependent tests.
- `pnpm run lint` exited 0 with pre-existing repository warnings. `pnpm run build` completed successfully. `rustfmt --edition 2021 --config skip_children=true --check src/lib.rs` and scoped `git diff --check` passed.
- `pnpm run tauri build` produced the release binary, DEB, and RPM, then exposed a host AppImage packaging incompatibility: linuxdeploy's bundled `strip` cannot read Fedora `.relr.dyn` sections. The source-current AppImage succeeded through the validated distrobox path: `distrobox enter lothlorien -- bash -lc 'cd /var/home/mythos/Eregion/Arda/apps/arda-hud && pnpm exec tauri build --bundles appimage --verbose'`.
- Final artifact SHA-256: release binary `19b27dfbee61b9578de7500c3254bf4f68d22cc59ad4778160f038814fb95bb7`; DEB `6cc8691d6839e8b0331ad9e571d675bcf8ad94076f3e7aa984bbe15ec673e2f1`; RPM `c67e93fe7760495cf699f29f60918b4d03d53ab55927d9b8272c7e3287912d5a`; AppImage `0b50377f35e26031bb0db293cd3aa3f95110afaf2befa31e8031d764bb422fa1`.
- This records an implemented experiment only. It does not close the standalone Mirromere application requirement and must not be used as Phase 3 product acceptance.

## Task 6: Add scene registration and guarded interaction

**Files:**
- Create: `apps/arda-hud/src/features/mirromere/sceneRegistry.ts`
- Modify backend projection/commands selected earlier
- Test: registry and interaction-policy tests

**Steps:**
1. Write failing tests rejecting unknown scene ids, unregistered interactions, privacy mismatch, and expired action.
2. Register only the seven initial scenes listed above.
3. Keep read-only scene switching automatic when low risk; require explicit operator action for conversation handoff or any mutation.
4. Record bounded receipts for interaction requests; UI does not mint success.
5. Commit: `feat(mirromere): register bounded ambient scenes`.

**Implementation evidence (2026-08-19):**

- Added seven logical scene registrations covering the eight typed scene ids; `system.starting` and `system.degraded` share the single `system.lifecycle` registration. Only `inspect_provenance` is automatic. `continue_handoff` and `dismiss_attention` require explicit operator action.
- Added strict backend interaction requests and a managed 128-entry receipt ring. Accepted requests are recorded as `status: requested`; rejected requests are recorded as `status: rejected`; neither path mints completion or success.
- Backend acceptance is bound to the exact latest projection issued by `get_mirromere_surface`. Client-authored or superseded surfaces receive a `surface_not_current` rejection receipt before policy evaluation.
- Both the in-world aperture and accessibility inspection path now request a backend receipt and open provenance only after an `accepted` / `requested` response. Backend errors fail closed without opening the workstation.
- RED: focused Vitest failed because `sceneRegistry.ts` was absent; focused Rust compilation failed because the interaction request, evaluator, receipt state, and receipt enums were absent. GREEN: focused frontend registry/wiring coverage passed 9 tests; focused Rust interaction coverage passed 5 tests; full HUD coverage passed 611 tests; full Rust library coverage passed 79 tests with 2 explicitly ignored Chromium-dependent tests.
- Spec review passed after replacing a broad automatic-read-only flag with explicit automatic interaction ids. Quality review found and then approved the fix binding receipts to backend-issued current surfaces.
- `pnpm run lint` exited 0 with pre-existing warnings, `pnpm run build` passed, Rust formatting and scoped diff checks passed. The source-current release binary, DEB, and RPM built through `pnpm run tauri build`; the known host linuxdeploy `.relr.dyn` incompatibility remained isolated to AppImage, which succeeded through the validated `lothlorien` distrobox path.
- Final artifact SHA-256: release binary `7b27fe993637d3b755afe3b12c08e0fc2da9d5acd6b60d3180c5bdcf3e2d1768`; DEB `a719ebf06368e60d7aa6a8948dea0c40e432f044ff1a47d5b68cca34ea0ea937`; RPM `08f8c3ecd37eea1c8245c90d28495cf0d893bba69f130cb613feac5d2898e7fb`; AppImage `b29d60d08adb49e2923e5639c0943fff65bd0034910eef717a0990548a3b224d`.
- This closes Task 6 implementation/build evidence. Task 7 physical second-monitor, reconnect, semantic-equivalence, performance, and operator-session acceptance remains open.

## Corrective workstream: standalone Mirromere application

**Status:** Complete — Tasks C1-C7 are implemented and verified; physical display acceptance remains exclusively in Task 7.

### Verified connection topology to preserve

The repository already contains most of the Mirromere-to-Arda connection contract. The extraction must reuse these authorities rather than making Mirromere scrape or remote-control ARDA HUD:

- `outposts/arda-outpost-protocol/src/mirromere.rs` owns the Rust `arda.mirromere.surface.v1` contract and validation bounds.
- `apps/arda-hud/src-tauri/src/mirromere.rs` currently projects lifecycle and continuity state, calls the existing continuity endpoint at `/v1/continuity/projection`, and enforces backend interaction receipts.
- `apps/arda-hud/src/features/mirromere/types.ts` strictly parses the frontend contract; `source.ts` invokes `get_mirromere_surface`; `sceneRegistry.ts` invokes the governed interaction command.
- `MirromereAperture.tsx` remains the passive HUD consumer. `MirromereNativeSurface.tsx` is migration input for the real application renderer, not evidence that a second application exists.
- `apps/arda-launcher/src-tauri/src/lifecycle/commands.rs` already owns explicit native-application lifecycle commands for ARDA HUD and is the convention to extend for an explicit Mirromere launch/status action.

The intended connection is therefore **shared backend authority, not hidden app-to-app ownership**: ARDA HUD and the standalone Mirromere application consume the same governed projection and interaction policy. HUD does not spawn, poll, recreate, focus, position, or close Mirromere. The launcher may expose an explicit operator launch action, but closing Mirromere exits its process and it stays closed.

### Task C1: Extract the backend projection from the HUD app boundary

**Objective:** Make projection and interaction policy reusable without depending on the `arda_hud` Tauri process.

**Files:**
- Create: `crates/spine/interface/arda-mirromere/Cargo.toml`
- Create: `crates/spine/interface/arda-mirromere/src/lib.rs`
- Move/refactor from: `apps/arda-hud/src-tauri/src/mirromere.rs`
- Modify: root `Cargo.toml`
- Modify: `apps/arda-hud/src-tauri/Cargo.toml`
- Test: `crates/spine/interface/arda-mirromere/tests/projection.rs`

**Steps:**
1. Write contract tests for lifecycle, continuity, privacy, stale/offline, and interaction-receipt behavior before moving code.
2. Move only pure projection, source loading, validation, and receipt-policy code. Keep Tauri command wrappers inside each application.
3. Depend on the existing `arda-outpost-protocol` Mirromere types; do not create a second Rust schema.
4. Make both current HUD commands delegate to the extracted library and rerun the existing Rust projection matrix.
5. Commit the extraction separately from application scaffolding.

**Gate:** Existing HUD aperture semantics and receipt rejection tests remain green with no native child-window behavior required.

**Implementation evidence (2026-08-20):**

- Added the workspace-owned `arda-mirromere` interface crate. It reuses `arda-outpost-protocol` as the sole Rust surface schema and now owns projection, continuity source loading, validation, privacy/freshness handling, issued-surface tracking, and interaction receipt policy.
- Reduced the HUD backend module to app-local lifecycle mapping plus the two Tauri command wrappers; both commands delegate projection/source/receipt behavior to `arda-mirromere` and retain no native child-window dependency.
- The exact staged tree passed 9 extracted-crate tests: the five-test receipt-policy suite and four external contract tests covering lifecycle/continuity scenes, public-display privacy veiling, stale/offline fail-closed behavior, current-surface authority, and explicit mutation requirements.
- The same isolated staged tree passed the committed eight-case HUD projection matrix. The source-current working tree also passed `cargo check --manifest-path apps/arda-hud/src-tauri/Cargo.toml` and scoped whitespace checks with pre-existing vendor/dead-code warnings only.
- The source-current working tree passed `pnpm run tauri build --no-bundle` and produced the native HUD binary at `/var/home/mythos/.cache/arda-build/target/release/arda_hud`; this build is not standalone Mirromere acceptance.
- This closes Task C1's extracted-backend implementation gate only. It does not prove or package the standalone Mirromere application required by Tasks C2-C7 and the Phase 3 gate.

### Task C2: Establish one shared frontend contract/renderer package

**Objective:** Let the HUD aperture and standalone app share strict types and bounded rendering without one app importing source from the other.

**Files:**
- Create: `packages/arda-mirromere-ui/package.json`
- Create: `packages/arda-mirromere-ui/src/index.ts`
- Move/refactor from: `apps/arda-hud/src/features/mirromere/types.ts`
- Move/refactor from: `apps/arda-hud/src/features/mirromere/sceneRegistry.ts`
- Move/refactor the renderer/model portions of: `MirromereAperture.tsx` and `MirromereNativeSurface.tsx`
- Modify: `apps/arda-hud/package.json`
- Test: package contract, model, privacy, reduced-motion, and renderer tests

**Steps:**
1. Use a local `file:` dependency from each application; do not silently introduce a repository-wide JavaScript workspace or merge the existing per-app lockfiles.
2. Keep Tauri IPC adapters app-local. The shared package receives typed surfaces and emits typed interaction requests; it does not own window lifecycle.
3. Preserve strict parsing, fixture/runtime distinction, source freshness, privacy ceilings, and bounded vector/text limits.
4. Prove the HUD aperture still renders through the package before deleting its original local copies.

**Gate:** There is one frontend contract/parser and one reusable visual model, with separate HUD-aperture and native-app adapters.

**Implementation evidence (2026-08-20):**

- `packages/arda-mirromere-ui` owns strict parsing, scene registration, the reusable visual model/renderer, interaction requests, privacy/freshness policy, and reduced-motion resolution.
- The HUD imports the shared package through a local `file:` dependency while retaining an app-local Tauri adapter; the shared package suite passes 11 tests and type checking.

### Task C3: Scaffold `apps/arda-mirromere` as a real Tauri application

**Objective:** Produce a distinct `arda_mirromere` process and package.

**Files:**
- Create: `apps/arda-mirromere/package.json`
- Create: `apps/arda-mirromere/index.html`
- Create: `apps/arda-mirromere/src/main.tsx`
- Create: `apps/arda-mirromere/src/App.tsx`
- Create: `apps/arda-mirromere/src/runtime.ts`
- Create: `apps/arda-mirromere/src-tauri/Cargo.toml`
- Create: `apps/arda-mirromere/src-tauri/src/main.rs`
- Create: `apps/arda-mirromere/src-tauri/src/lib.rs`
- Create: `apps/arda-mirromere/src-tauri/build.rs`
- Create: `apps/arda-mirromere/src-tauri/tauri.conf.json`
- Create: `apps/arda-mirromere/src-tauri/capabilities/default.json`

**Steps:**
1. Start with one ordinary decorated/resizable window so native close behavior is observable and reversible. Borderless presentation is a later accepted mode, not the scaffold default.
2. Give it a unique identifier such as `com.arda.mirromere`, a unique window label, and its own binary/package names.
3. Register app-local `get_mirromere_surface` and `request_mirromere_interaction` wrappers backed by `arda-mirromere`.
4. Render the shared native presentation with explicit loading, stale, offline, privacy, and error states.
5. Add Escape handling at the Tauri window layer or a focus-proven frontend path; verify Escape closes once and no process recreates the app.
6. Build with `pnpm run tauri build --no-bundle` and verify both `arda_hud` and `arda_mirromere` binaries exist simultaneously.

**Gate:** Process inspection proves two independent binaries. Exiting Mirromere does not exit HUD, and exiting HUD does not exit or relaunch Mirromere.

**Implementation evidence (2026-08-20):**

- `apps/arda-mirromere` is a standalone decorated/resizable Tauri application with identifier `com.arda.mirromere`, binary `arda_mirromere`, and app-local projection/interaction wrappers backed by `arda-mirromere`.
- Its frontend build and Rust format/test gates pass. The standalone Rust suite passes 3 display-policy tests, and the release package produces a process/binary distinct from `arda_hud`.

### Task C4: Add explicit display ownership and fail-closed recovery

**Objective:** Let Mirromere own its selected non-primary display without HUD lifecycle involvement.

**Files:**
- Create: `apps/arda-mirromere/src-tauri/src/display.rs`
- Create: `apps/arda-mirromere/src/display.ts`
- Test: Rust display-selection and frontend unavailable/veil tests

**Steps:**
1. Port the useful stable-display observation and primary-display rejection logic from the rejected HUD child-window experiment.
2. Persist selection in the Mirromere application's own data/config scope, never HUD local storage.
3. Require explicit operator selection before entering full-display presentation.
4. On disconnect or ambiguous identity, veil/close and report unavailable; never fall back to the primary display.
5. On reconnect, require safe identity resolution and do not steal focus from HUD.

**Gate:** A real cable disconnect/reconnect proves no private frame appears on the primary display.

**Implementation evidence (2026-08-20):**

- The standalone application owns persisted stable display selection, rejects the primary display, fails closed when selection is absent/ambiguous/unavailable, and renders a recovery veil that offers only non-primary targets.
- The standalone frontend suite passes 6 tests covering missing display state, missing surface, blocked projection, recovery veil, and primary-display exclusion. The physical cable proof remains intentionally open under Task 7.

### Task C5: Wire explicit launcher and service lifecycle

**Objective:** Reuse ARDA's existing native-app lifecycle connection while preserving “closed means closed.”

**Files:**
- Modify: `apps/arda-launcher/src-tauri/src/lifecycle/commands.rs`
- Modify: `apps/arda-launcher/src-tauri/src/lifecycle/types.rs`
- Modify: launcher frontend lifecycle controls/tests selected from live source
- Create: `config/systemd/arda-mirromere.service`
- Modify: `scripts/install_arda_user_units.sh`
- Modify: `scripts/verify_arda_user_units.sh`
- Add packaging/launch scripts following `scripts/package_arda_hud.sh` and `scripts/launch_arda_hud.sh`

**Steps:**
1. Add explicit `launch_native_mirromere` and `mirromere_status` paths rather than coupling Mirromere to `launch_native_hud`.
2. Keep `arda-mirromere.service` out of automatic `Wants=` membership for `arda-session.target` unless the operator later requests ambient autostart.
3. Use clean-exit semantics and at most `Restart=on-failure`; a successful operator close must not trigger restart.
4. Install to a distinct path such as `~/.local/lib/arda/mirromere/arda_mirromere` and verify package identity independently from HUD.

**Gate:** Launcher can start Mirromere explicitly and report status; closing it leaves the service inactive while HUD remains usable.

**Implementation evidence (2026-08-20):**

- Launcher lifecycle commands and controls are distinct from HUD lifecycle; `arda-mirromere.service` is explicit-only, has `Restart=no`, and has no `[Install]` membership or session-target autostart path.
- Package/install/uninstall/launch/unit-verification scripts install the runtime at `~/.local/lib/arda/mirromere/arda_mirromere`; aggregate user-unit install and verification include the service.
- A live service probe installed the final packaged binary, launched PID `1063151`, stopped to `inactive`, explicitly relaunched PID `1063162`, and stopped to final `inactive`; HUD service state was unchanged.

### Task C6: Retire the rejected HUD child-window path

**Objective:** Remove every path that lets ARDA HUD own a physical Mirromere window.

**Files:**
- Modify: `apps/arda-hud/src-tauri/src/lib.rs`
- Modify: `apps/arda-hud/src/utils/multiWindow.ts`
- Modify: `apps/arda-hud/src/App.tsx`
- Remove/migrate: `apps/arda-hud/src/features/mirromere/MirromereNativeSurface.tsx`
- Update: `apps/arda-hud/src/features/mirromere/INTEGRATION.md`
- Remove/update associated child-window tests

**Steps:**
1. Remove `list_mirromere_displays` and `open_mirromere_window` from HUD command registration and source.
2. Remove HUD-owned selected-display persistence, `__view=mirromere`, the `arda-mirromere` child label, and all reopen/reposition behavior.
3. Keep only the passive `monitor_3` aperture and governed inspect/interaction path.
4. Add a source-level regression test forbidding HUD references to the retired physical-window command and route.

**Gate:** Searching and tests prove `arda_hud` cannot create or manage the standalone Mirromere window.

**Implementation evidence (2026-08-20):**

- HUD command registration and source no longer contain display-list/open/close Mirromere commands, child-window construction, the native Mirromere query route, selected-display persistence, or reopen/reposition behavior.
- The passive in-world aperture remains. Focused HUD regression tests, the 149-file / 606-test frontend suite, the 9-test HUD Rust projection suite, frontend production build, and Rust check pass.

### Task C7: Package, document, and hand off to native acceptance

**Objective:** Make the application reproducible before resuming Task 7.

**Files:**
- Create: `apps/arda-mirromere/README.md`
- Create: Mirromere package/install/launch verification scripts
- Update: this plan and relevant launcher/runtime documentation

**Steps:**
1. Build the standalone binary and supported Linux bundles through the repository's validated packaging path.
2. Record checksums and exact install locations separately from ARDA HUD artifacts.
3. Run focused tests for both consumers, Rust projection/receipt tests, lint, frontend builds, Tauri builds, systemd verification, and scoped diff checks.
4. Only then resume Task 7 against the standalone binary.

**Gate:** A fresh session can install, launch, close, relaunch, and identify Mirromere without starting or modifying HUD.

**Implementation evidence (2026-08-20):**

- `apps/arda-mirromere/README.md` documents ownership, explicit display selection, fail-closed recovery, packaging, install, launch, stop, uninstall, and state-preservation behavior.
- `scripts/package_arda_mirromere.sh` uses the validated `lothlorien` AppImage path and emitted `Mirromere_0.1.0_amd64.AppImage` with SHA-256 `7d71ff6fc68306c1699313c4da35e4a126bc3980179611d35e24394afcbff34e`.
- The packaged release binary SHA-256 is `b66f39f2ff443a2cc84cbd19245b8363ae6ce7988e24cf32428fad13fdb31dbb`; the installed runtime matched it byte-for-byte during the live lifecycle probe.
- Mirromere and aggregate systemd-unit verification, shell syntax checks, focused builds/tests, scoped whitespace checks, and the HADES active-plan link/completion-language gate pass.
- Task C is closed at packaged/runtime-proven maturity. Cable disconnect/reconnect, native physical placement, and the five-minute disconnected HUD observation remain unclaimed Task 7 gates.

## Task 7: Visual, native, and performance acceptance

**Status:** Closed with explicitly recorded exceptions. The implementation and live native walkthrough are complete. Physical cable disconnect/reconnect and the five-minute observation were waived by the operator and were not performed. Escape handling is implemented and release-build verified, but native Escape input was not accepted because the Wayland session did not permit deterministic compositor focus; it is not claimed as observed.

**Run:**
1. Launch the packaged native HUD and the separately packaged Mirromere application through Phase 1.
2. Verify the selected HUD aperture renders the live Mirromere scene and is clearly in-world/display-only.
3. Launch the standalone native Mirromere application on the physical second monitor.
4. Drive identical idle, starting, degraded, handoff-ready, privacy, stale, and offline states through the backend contract.
5. Compare both consumers for semantic equivalence, not pixel identity.
6. Disconnect/reconnect the second monitor and verify safe veil/recovery.
7. Exercise reduced motion and keyboard/escape control.
8. Run the existing HUD performance acceptance; reject any meaningful FPS/frame-time degradation.
9. Confirm browser preview or static screenshots are not used as native proof.
10. Record a real operator session using the second monitor for conversation presence and one Varda research visualization.

**Operator evidence (2026-08-20):**

- Installed release binaries ran as independent `arda_hud` and `arda_mirromere` processes. Mirromere selected explicit non-primary connector `DP-3` at `1920,0 1920x1080`; invalid selection failed closed rather than falling back to the primary display.
- Backend-authoritative native semantic walkthroughs covered `ambient.idle`, `system.starting`, `system.degraded`, `conversation.presence`, `continuity.handoff-ready`, `research.focus`, `privacy.veil`, stale continuity, and `offline.local`. HUD and standalone Mirromere exposed equivalent scene purpose, freshness, privacy, and availability semantics through AT-SPI.
- The starting-state walkthrough used a temporary bounded systemd `ExecStartPre` delay. Systemd reported `activating/start-pre`, launcher lifecycle authority reported `aggregate_state: starting`, and both native consumers exposed `system.starting`. The drop-in was removed and `/healthz` was healthy afterward.
- GNOME's real `enable-animations=false` preference caused standalone Mirromere to expose `motion reduced`; the preference was restored to `true`. Shared renderer tests prove reduced mode disables animation.
- Ordinary close/remain-closed/explicit-relaunch service semantics were exercised: `arda-mirromere.service` uses `Restart=no`, has no install/autostart membership, remains inactive after an explicit stop, and starts only through explicit launch. HUD remains active independently.
- Frontend Escape handling closes the current Tauri window and passed TypeScript and release Tauri builds. A native synthesized Escape attempt was rejected as acceptance evidence because AT-SPI could not acquire Wayland compositor focus; no native Escape pass is claimed.
- Physical cable disconnect/reconnect and the five-minute observation were operator-waived and not performed. Screenshot capture was denied by GNOME, so no screenshot claim is made; native evidence used backend receipts, systemd state, process identity, display geometry, and AT-SPI semantics.

## Phase gate

Phase 3 remains open. The standalone app now opens the real Hermes dashboard on the selected second display instead of rendering the radar scene. It is not complete until the human operator accepts Mirromere as useful. Remaining product work includes avatar presence and local voice behavior from the original Mirromere requirements. A HUD-owned child window remains rejected architecture.

## Closeout

This plan is active. Earlier scene-contract, packaging, and display-placement results remain implementation history only; they are not product acceptance.
