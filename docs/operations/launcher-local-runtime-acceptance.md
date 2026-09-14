---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "operations_acceptance"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-08-17"
---

> 🜏 Soterion: 📜 operations_acceptance | owner: PROMETHEUS | status: active | reviewed: 2026-08-17

# Launcher and Local Runtime Acceptance

**Disposition:** Phase 1 accepted on the declared `bluefin-lts-10-x86_64` local profile on 2026-08-17. This record proves the unsigned local launcher/runtime lifecycle; it does not claim public signing, an independent evaluator, or support beyond the declared profile.

## Accepted source and artifacts

- Source commit: `8c86dc2475232a5d7792afef809b0e97b2f99313`.
- Build-time tree drift was limited to the tracked TypeScript build-info refresh and generated queue projections; neither changes launcher/HUD runtime source. The queue projections remain unstaged and are not release inputs.
- Launcher version: `0.9.0`.
- Installed launcher SHA-256: `c9247fc37ee24d902d2efea5b70449ae7b327312f9a7cee6f402a0ee56c3717f`.
- Launcher AppImage SHA-256: `d78b299e33beddebea327ecd57238e133fe3383ac4d40fb845c54e93bc7b58da`.
- Launcher DEB SHA-256: `ca11c16096658981283c41a648af2b059d2dc3f64b5478ff204cba7c00b9b59c`.
- Launcher RPM SHA-256: `6e7ae7cf3f921371b51b90406d4879a6188c5c859edfeed9fe77acb37c7e801c`.
- Installed HUD SHA-256: `7ac6c30b23483accb36c6fe7251e3a7434d863808d30c1f291f5c8a8a827e71e`.
- Release-manifest identity: `sha256:aef742874d12165584af31c1ecaa1271bb60fb7ce3e8176ec335b7f0e5a43f06`.

`pnpm run tauri build` completed through the normal launcher entry point and produced AppImage, DEB, and RPM bundles. The AppImage extracted successfully with executable `AppRun` and launcher payload plus desktop/icon metadata. RPM metadata and payload paths passed native inspection; the DEB archive contained the expected Debian, control, and data members. The canonical HUD package command completed `ready` in `tauri-no-bundle` mode with no blockers.

## Post-restart native path

The workstation booted at 2026-08-17 12:23 PDT. GNOME launched the installed desktop application without a terminal. The launcher then observed the independently managed runtime and opened the native HUD only after the required components became healthy.

Observed sequence:

1. `arda.service` became active/running at 12:23:54.
2. Manwë listened on loopback and `/healthz` returned `ok=true` and `service=manwe`.
3. `hermes-gateway.service` reached active/running with a fresh systemd watchdog at 12:24:14.
4. `arda-session.target` became active at 12:24:14.
5. The installed launcher started from the desktop entry at 12:24:19.
6. `arda-hud.service` started after required health and rendered the native HUD. The final installed candidate is active/running with zero restarts.
7. Closing/replacing the launcher left `arda-session.target`, `arda.service`, `hermes-gateway.service`, and `arda-hud.service` active. Reopening the launcher produced one installed launcher process and reconnected without duplicating backend or HUD services.

GNOME's compositor denied programmatic window enumeration and the desktop-control accessibility bridge exposed no windows. Native-window presence is therefore operator-observed and corroborated by the installed Tauri processes, their WebKit child processes, Wayland display environment, D-Bus registrations, and independent systemd ownership; it is not represented as screenshot evidence.

## Failure injection and recovery

At 12:48:15 the required `hermes-gateway.service` was deliberately stopped. Its shutdown currently reports `failed` because the gateway exits non-zero after systemd SIGTERM; the launcher/systemd observer therefore had a real required-component failure rather than a synthetic fixture. During the outage:

- `arda.service` remained active;
- `arda-session.target` remained active;
- `arda-hud.service` remained active.

Bounded recovery restarted the gateway. It returned to active/running at 12:48:33 with a fresh watchdog timestamp. The failure did not terminate the backend session or HUD.

## Install, upgrade, and rollback

The canonical private-beta operations path was exercised against the exact launcher candidate:

- upgrade from installed SHA-256 `5ecb81411772a878c5de2b1b2364ae4545689dda657531c9b696e5ccb9d8a376` to candidate SHA-256 `c9247fc37ee24d902d2efea5b70449ae7b327312f9a7cee6f402a0ee56c3717f`: `ok`;
- backup-first rollback to the prior launcher: `ok`;
- terminal run truth preserved across rollback: `true`;
- source repository touched by rollback: `false`;
- final upgrade back to the accepted candidate: `ok`;
- installed launcher hash equals the accepted candidate hash;
- HUD user-unit installer installed the accepted HUD candidate transactionally, reloaded the user manager through the verified host transport, and restarted the static HUD unit successfully.

State archives remain operator-local with mode `0600` and are not committed. Only bounded hashes and dispositions appear in this record.

## Verification gates

- Launcher frontend: 20/20 tests, lint, TypeScript, and production Vite build passed.
- Launcher Rust lifecycle: focused tests and strict gates passed in the implementation tasks; final all-target gates were repeated at closeout.
- Private-beta operations: 17/17 tests passed.
- HUD canonical package: `ready`, native candidate produced, no blockers.
- User units: source and installed verification passed.
- Desktop entry: installed path resolves to the managed launcher binary; validation has only the existing non-fatal multiple-main-category hint.
- Live final state: launcher, HUD, Arda runtime, session target, and Hermes Gateway active; Manwë protocol health true; Hermes watchdog fresh.

## Evidence boundary

This closes the Phase 1 ignition contract: real restart, desktop launch, source-truth health, native HUD gating, independent lifetimes, reconnect, real failure observation, bounded recovery, packaging, install, upgrade, and rollback. Public signing, broader distro support, independent evaluation, and final `1.0.0` qualification remain separate authorities.