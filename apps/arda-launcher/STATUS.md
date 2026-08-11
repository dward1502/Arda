# arda-launcher status

App: `apps/arda-launcher`
Current state: S5-RC0 candidate lifecycle verified locally on 2026-07-31
Branch: `manwe`
Documentation: `README.md`, `INDEX.md`, `BREAKDOWN.md`, `STATUS.md`, `OWNERSHIP.md`

Current signature: a Tauri 2 operator front door that performs registry gating,
loads environment-derived readiness and human-gated service-plan projections,
and renders those projections through a tested React command boundary without
service mutation or approval authority.

## Command surface

- `registry_status`: resolves the workspace registry and returns load/gate status.
- `readiness_status`: builds the current readiness projection.
- `service_plan_status`: builds proposed service actions while preserving each
  action's `requires_human_gate` field.
- `release_identity`: reports the native binary's compiled package version and
  declared Linux support profile.
- No sample `greet` command is registered.
- Apply, private-config write, receipt creation, and console launch functions are
  not exposed to the frontend.

## Frontend integration

- `App.tsx` gates **Begin** on successful registry discovery.
- **Begin** invokes a typed readiness/service-plan snapshot and opens
  `OnboardingPanel`.
- The panel is read-only, labels human-gated actions, uses an ARIA live region,
  and presents invocation failures instead of discarding them.
- Vitest pins exact Tauri command names, root arguments, and serialized payload
  shapes.

## Endpoint compatibility

Launcher production code contains no `:7171` literal. `EnvironmentProfile`
reads `MANWE_BASE_URL` first and `ARDA_MANWE_BASE_URL` second. The existing
`:7171` default remains in Manwe, engine, root-daemon, registry, script, test,
and fleet consumers. It is intentionally preserved until those owners execute a
coordinated environment/fleet migration.

## Verification

- Rust launcher tests: 14 passed.
- Rustfmt: passed.
- Strict all-target/all-feature Clippy with `-D warnings`: passed.
- Frontend contract/orientation tests: 11 passed.
- Oxlint: 0 warnings and 0 errors.
- TypeScript/Vite production build: passed.
- Tauri release binary: produced.
- Frontend, Cargo, and Tauri bundle versions: aligned at `0.3.0-rc.1`.
- S5-RC0 compatibility, deterministic manifest/checksum, isolated upgrade,
  exact-once Workbench recovery, diagnostics, and rollback proof: passed under
  `docs/evidence/stage-5-release-candidate/s5-rc0/`.
- DEB: `target/release/bundle/deb/arda-launcher_0.2.0_amd64.deb`.
- RPM: `target/release/bundle/rpm/arda-launcher-0.2.0-1.x86_64.rpm`.
- Manual AppImage: `target/release/bundle/appimage/arda-launcher_0.2.0_amd64-manual.AppImage`.

## Packaging compatibility

Tauri's cached `linuxdeploy` bundles an old `strip` that rejects modern ELF
`.relr.dyn` sections as unknown section type `0x13`. The launcher package script
sets linuxdeploy's supported `NO_STRIP=true` control, preserving those already
stripped system libraries instead of passing them through the incompatible
tool. It also puts the distro's GNU coreutils before Homebrew uutils because the
cached GTK plugin relies on GNU `cp --parents` behavior. The normal
`pnpm run tauri build` entry point now produces the AppImage, DEB, and RPM;
`tests.test_arda_appimage` keeps these compatibility controls from regressing.