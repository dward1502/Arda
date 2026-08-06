# AppImage RELR-capable packaging preflight — 2026-08-05

Lane: `stage5/appimage-relr`
Worktree: `/var/home/mythos/Eregion/Arda-worktrees/appimage-relr`
Baseline: `f646245ce5dde26490dfbacb209571ce8f064fc6`

## Implemented in this lane

- `scripts/arda_appimage.py` fetches and verifies exact AppImage assembly
  inputs, rejects hash drift, requires a valid AppDir, supplies a fixed
  `SOURCE_DATE_EPOCH`, and rejects non-type-2 outputs.
- `scripts/arda_release_ops.py` now requires and records the separately pinned
  runtime SHA-256 in the release manifest inputs.
- The release packaging runbook contains the exact two-build reproducibility
  procedure.
- `tests/test_arda_appimage.py` covers cache reuse, fail-closed tool identity,
  fixed epoch/runtime invocation, and type-2 output validation.

Pinned x86_64 inputs:

- appimagetool 1.9.1 release asset SHA-256:
  `ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0`
- type-2 runtime SHA-256:
  `1cc49bcf1e2ccd593c379adb17c9f85a36d619088296504de95b1d06215aebbf`

The runtime's upstream `continuous` URL is mutable. The recorded checksum is
therefore authoritative and the fetch fails closed if upstream changes it.

## Executed proof

Command gate:

`python3 -m unittest tests.test_arda_appimage tests.test_arda_release_ops`

Result: 6 tests passed.

The pinned toolchain packaged the existing populated Launcher AppDir twice with
one fixed epoch. Both outputs were real type-2 x86_64 AppImages and were
byte-identical:

`7ce0900826d1e099879f6c76c0bed104123eca38bb0dd8cb6ccff85ff1d025bf`

The embedded runtime reported commit `75849dc`. The produced file was
approximately 92.42 MiB and `file` identified it as a static PIE x86-64 ELF.
The proof outputs are retained outside the repository at:

`/tmp/arda-appimage-relr-proof/`

## Acceptance boundary

This proves that the pinned replacement assembly path handles the populated
RELR-era AppDir and creates reproducible actual AppImages. It is not final
release evidence: the AppDir came from the current main working tree, not from
a post-security-migration frozen candidate. After integration, rebuild the
AppDir, DEB, and RPM from the exact final source, repeat the two-build proof on
Bluefin LTS, run native startup/lifecycle checks, and only then generate the
manifest/checksums/signatures.

AppDir output alone is explicitly not accepted as an AppImage substitute.
