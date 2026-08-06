# Tauri/GTK security migration preflight — 2026-08-05

Lane: `stage5/tauri-gtk-security`
Worktree: `/var/home/mythos/Eregion/Arda-worktrees/tauri-gtk-security`
Baseline: `f646245ce5dde26490dfbacb209571ce8f064fc6`

## Verified baseline graph

The baseline Launcher resolved `tauri 2.11.5`, `wry 0.55.1`,
`webkit2gtk 2.0.2`, `gtk 0.18.2`, and `glib 0.18.5`.
`cargo tree -p arda-launcher` proved that `glib 0.18.5` was reachable through
Tauri/Wry's GTK3/WebKitGTK stack. RustSec `RUSTSEC-2024-0429` affects
`glib >=0.15.0,<0.20.0`; the affected `VariantStrIter` iterator methods can
produce undefined behavior and optimized-build null dereferences.

## Upstream migration boundary

The current crates.io releases do not provide the requested migration:

- `wry 0.56.0` still depends on `gtk 0.18` and `webkit2gtk 2.0.2`.
- `gtk 0.18.2` is the unmaintained GTK3 binding and cannot resolve to
  `glib >=0.20`.
- Tauri issue `tauri-apps/tauri#12048` is closed as upstream-blocked/not
  planned for the GTK3 line.

The active GTK4/WebKitGTK 6 migration is `tauri-apps/tauri#14684`. At commit
`402ebddca9c2891594341edc7f5967ed1f6ded54`, it uses `gtk4 0.10` and
`webkit6 0.5`, but also uses four moving Git branch dependencies for Tao, Wry,
Muda, and tray-icon. The pull request is open and its Linux, Windows, macOS,
Android, rustfmt, and clippy checks currently fail. Wry pull request #1530 is
open as a conflicting draft. The upstream branches themselves are not
acceptable unpinned release inputs.

## Pinned candidate implemented in this lane

The lane now implements the maintained-fork option as a review candidate. All
five source repositories are pinned to exact commits in both Cargo workspaces:

- Tauri: `402ebddca9c2891594341edc7f5967ed1f6ded54`
- Tao: `dfa8a322eef7f5b8a6a8a9dcbe7924dd3be0cdeb`
- Wry: `7222c92af3c91998ee37e8ee1635fb3830a13c82`
- Muda: `350b9bda3eabd7327e1a2592019cd33e7f559d44`
- tray-icon: `b2f085e8a02cc8651cfd8884678a2d438ab50370`

Launcher and HUD now resolve `tauri 2.9.5`, `tauri-runtime-wry 2.9.3`,
`wry 0.53.5`, `tao 0.34.5`, `gtk4 0.10.3`, `webkit6 0.5.0`, and
`glib 0.21.5`. Neither workspace resolves `gtk`, `webkit2gtk`, or any
`glib <0.20` package. The GTK3 `RUSTSEC-2024-0429` path is therefore absent.

HUD also declared seven native plugins which had no Rust initialization and no
frontend imports. They were unreachable product surface, but still expanded
the lockfile and initially reintroduced GTK3 through `tauri-plugin-dialog`.
Those unused declarations were removed; `tauri-plugin-opener`, which is
initialized by HUD, remains pinned to the compatible `2.5.3` release.

The host lacks GTK4/WebKitGTK 6 development packages, so native verification
used a Fedora 44 container with `gtk4-devel 4.22.4` and
`webkitgtk6.0-devel 2.52.5`. Real results:

- `cargo check -p arda-launcher`: passed.
- `cargo test -p arda-launcher`: 18 passed, 0 failed.
- HUD `cargo check`: passed.
- HUD `cargo test`: 36 passed, 0 failed.
- Launcher frontend: 7 Vitest tests passed, lint reported 0 warnings/errors,
  and the production Vite build passed.
- HUD frontend: 386 Vitest tests passed across 97 files, lint completed with
  103 pre-existing warnings and 0 errors, and the production Vite build passed.
- `cargo build -p arda-launcher --release`: passed.
- HUD `cargo build --release`: passed.
- Both release binaries remained alive for the complete 15-second Xvfb/DBus
  startup smoke window (`timeout` exit 124). Container-only Vulkan, portal, and
  bubblewrap warnings were observed; neither application exited.
- Launcher `cargo audit`: 0 vulnerabilities, exit 0.
- HUD `cargo audit`: 0 vulnerabilities, exit 0.
- Both audit reports retain informational unmaintained/unsound warnings,
  including `atty 0.2.14` and build-time `rand 0.7.3`; neither report contains
  a vulnerability or the removed GLib advisory.

The HUD's standalone dependency-policy invocation is not green. Running the
canonical root `deny.toml` against the HUD lock reports the pinned Git sources
as unlisted warnings, rejects existing MPL-2.0 transitive packages, and treats
upstream unmaintained advisories as errors. No advisory or source exception was
added to make the candidate appear compliant.

## Independent fork review

Independent source review compared the exact candidate commits with their
upstream merge bases. It found release-blocking behavior and coverage gaps:

- Tauri commit `402ebddc` differs from merge base `1b0e335d` by 20 files,
  2,460 insertions, and 1,891 deletions. The migration changes runtime,
  webview, monitor, undecorated-resizing, bundler, and dependency behavior, but
  adds no Linux GTK4 integration suite; only the mock runtime test file changed.
- Tao commit `dfa8a322` differs from merge base `e196538f` by 19 files. Its
  event loop was substantially rewritten around blocking channel sends from GTK
  callbacks. `monitor_from_point` now always returns `None`, cursor position is
  reported as `(0, 0)`, multiple window operations regress, and raw X11/Wayland
  handle paths add unwrap/FFI panic risk. No focused GTK4 behavioral tests cover
  these changes.
- Wry commit `7222c92a` differs from merge base `51d06d0c` by 20 files. Raw
  X11 child embedding is explicitly unsupported, a new raw signal trampoline
  uses `connect_raw`, `transmute`, and manual closure lifetime management, and
  IPC, drag/drop URI parsing, network-session, persistence, proxy, download,
  and cookie behavior all changed without focused tests.
- Muda commit `350b9bda` differs from merge base `f67371e4` by 28 files. It
  introduces `unsafe impl Send/Sync for PlatformIcon`, unsafe GTK object-data
  access, panic-prone native UI paths, and an untested GTK4/KSNI compatibility
  layer.
- The configured tray-icon commit
  `b2f085e8a02cc8651cfd8884678a2d438ab50370` is not available from the named
  repository. Both a direct Git fetch and GitHub's commit API reject it. It did
  not enter the resolved candidate graph, so successful builds did not prove
  that source was retrievable.

The review evidence therefore rejects maintenance ownership of this fork chain
for the Stage 5 candidate. Compile, unit-test, release-build, and startup-smoke
success do not close these native event-loop, unsafe-lifetime, IPC, persistence,
display-backend, or source-availability risks.

## Release decision

No safe coordinated crates.io-only upgrade to `glib >=0.20` exists today. The
pinned candidate proves that the complete GTK4 path compiles, passes the native
and frontend suites, builds in release mode, and reaches a live application
event loop without an advisory ignore. Independent source review nevertheless
rejects it as a release input. It must not be integrated, packaged as the final
candidate, signed, or used to replace the frozen-source soak candidate.

Remaining choices are therefore:

1. Keep Stage 5 blocked until the upstream GTK4/WebKitGTK 6 chain is released,
   then upgrade Launcher and HUD together and run their complete native gates.
2. Build and maintain an Arda-owned fork with fixes plus focused Linux
   GTK4/Wayland/X11 security and behavior tests; the reviewed third-party fork
   commits are not an acceptable starting release input without that work.
3. Replace the Linux webview shell with a framework whose maintained dependency
   graph already resolves `glib >=0.20`; treat this as a product migration.

A one-line local backport to `glib 0.18.5` may remove the known defective code,
but it does not meet the stated `glib >=0.20` migration criterion and would
require a documented advisory exception. It is not implemented in this lane.

## Candidate disposition

- Candidate implementation commit: `fd58583`
- Disposition: rejected by independent source review
- Integration status: isolated; not merged into `manwe`
- Signing/package status: prohibited for final release use
- Stage 5 security gate: blocked pending a maintained GTK4 source chain or a
  separately approved product/framework migration

This lane remains isolated from the running frozen-source soak.
