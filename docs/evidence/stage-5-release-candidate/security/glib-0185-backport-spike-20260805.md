# glib 0.18.5 security backport spike — 2026-08-05

## Decision

**Result:** technically viable with explicit local-fork controls.

The maintained Tauri 2 Linux graph still resolves `glib 0.18.5`. Arda therefore
vendors that exact crates.io package and applies only the upstream gtk-rs-core
fix for RUSTSEC-2024-0429 while waiting for a maintained Tauri GTK4 line.

This is a bounded Stage 5 exception, not a claim that `glib 0.18.5` became an
upstream-fixed release. The exception must be removed when the maintained Tauri
line reaches a fixed gtk-rs-core generation.

## Provenance and patch boundary

- crates.io package: `glib 0.18.5`
- crates.io archive SHA-256:
  `233daaf6e83ae6a12a52055f568f9d7cf4671dabb78ff9560ab6da230ce00ee5`
- upstream fix: gtk-rs-core pull request 1343, commit
  `e2f5aefcc60492b7f51a2ddcf1b649ef73f54bf4`
- source delta: `src/variant_iter.rs`, changing the `VariantStrIter::next`
  out-argument from immutable `&p` to mutable `&mut p`
- local provenance note: `vendor/glib-0.18.5/ARDA_PATCH.md`
- deterministic verifier: `scripts/verify_glib_0185_backport.py`
- network-free contract test: `tests/test_glib_backport.py`

The verifier downloaded the pinned archive, validated its SHA-256, compared 121
files, and accepted only the declared `variant_iter.rs` transformation plus the
Arda provenance note.

## Red/green evidence

Unpatched optimized regression:

```text
cargo test --manifest-path /tmp/glib0185-red/Cargo.toml --release test_variant_str_iter_nth -- --nocapture
red_rc=101
2 tests selected; first passed; process terminated with SIGSEGV (signal 11)
```

Patched optimized regression:

```text
cargo test --manifest-path /tmp/glib0185-green/Cargo.toml --release test_variant_str_iter -- --nocapture
4 passed; 0 failed
```

## Consumer and package gates

Passed:

- `python3 -m unittest scripts.tests.test_glib_backport -v` — 3 passed.
- `python3 scripts/verify_glib_0185_backport.py` — 121 files verified.
- `cargo deny check advisories` — advisories ok.
- `cargo check -p arda-launcher --all-targets`.
- `cargo test -p arda-launcher` — 14 passed after integration into the active source.
- launcher `pnpm test` — 8 passed after integration into the active source.
- `pnpm run tauri build --bundles deb,rpm` — DEB and RPM produced.
- HUD `cargo check --all-targets` and `cargo test` — 38 passed.
- HUD `pnpm test` — 397 passed after integration into the active source.
- HUD `pnpm run tauri:build:stable` — release binary produced.
- manual AppDir packaging with local `appimagetool` — valid x86-64 AppImage,
  SHA-256 `3523b0de15540827da1e9c6141a43af89635510307bcb001fc1938620fcd7e05`.
- Launcher and HUD production `pnpm audit` — zero advisories after both
  applications advanced to `postcss 8.5.25`, closing the newly reported
  `GHSA-fxqj-rqcc-2cmp` follow-up advisory.

The default launcher `pnpm run tauri build` compiled the release binary and
produced DEB/RPM but failed in Tauri's cached `linuxdeploy` while its bundled old
`strip` rejected Fedora `.relr.dyn` sections. This is the previously classified
host-packager incompatibility, not a glib compilation or runtime regression.
The generated AppDir was successfully wrapped by the local appimagetool. Final
release artifacts still require the normalized, reproducible production path.

## Scanner caveat and compensating control

`cargo audit` and `cargo deny` do not match RustSec version advisories against a
path dependency, so their clean output alone does **not** prove this backport.
The pinned-archive verifier and source contract test are mandatory alongside the
normal scanners. Removing either verifier while the path patch exists reopens
SEC-GLIB-001.

## Acceptance

Accept this bounded local backport for the Stage 5 source candidate with these
conditions:

1. both Launcher and HUD remain pinned to the same vendored path;
2. the archive verifier and optimized `VariantStrIter` tests remain green;
3. the final artifact build reruns all ordinary scanners plus these controls;
4. upstream Tauri/gtk-rs status remains a tracked post-release maintenance item;
5. no final-release claim is made until the active reliability soak and exact
   final-artifact lifecycle gates pass.
