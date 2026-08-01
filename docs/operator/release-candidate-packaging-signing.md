# Arda Workbench release-candidate packaging and signing

This runbook covers the declared Stage 5 Linux profile and the `0.3.0-rc.0`
Workbench launcher. The canonical release artifact is the x86_64 AppImage. DEB
and RPM packages are supplemental until their timestamp-bearing output is made
byte-reproducible.

## Build inputs

Build from the repository root with the lockfiles unchanged. Set
`SOURCE_DATE_EPOCH` to the source commit timestamp. The packaging manifest
records:

- source commit and release-relevant source-tree SHA-256;
- Cargo and pnpm lockfile SHA-256 values;
- Rust, Cargo, Node, pnpm, and AppImage tool versions;
- the AppImage tool binary SHA-256;
- supported profile and schema/rollback compatibility.

The reproducible AppImage proof uses upstream `appimagetool` 1.9.1, pinned by
SHA-256. The older AppImageKit build `5735cc5` is not suitable: it injects the
current time and conflicts with `SOURCE_DATE_EPOCH`.

## Build and verify

Build the frontend and native launcher, then create DEB/RPM bundles with Tauri.
Tauri's AppImage `linuxdeploy` phase currently fails after producing a complete
AppDir. Package that AppDir with the pinned `appimagetool` and a fixed
`SOURCE_DATE_EPOCH`.

Generate and verify the release ledger:

```text
python3 scripts/arda_release_ops.py bundle-manifest \
  --root "$PWD" --version 0.3.0-rc.0 \
  --artifact <AppImage> --artifact <DEB> --artifact <RPM> \
  --artifact <release-sbom.json> \
  --output <release-dir>/release-bundle-manifest.json \
  --checksums-output <release-dir>/SHA256SUMS

python3 scripts/arda_release_ops.py verify-checksums \
  --checksums <release-dir>/SHA256SUMS \
  --artifact-dir <release-dir>
```

Generate the launcher-reachable Cargo and frontend inventory:

```text
python3 scripts/arda_release_ops.py sbom \
  --root "$PWD" \
  --output <release-dir>/release-sbom.json
```

A nonzero missing-license count blocks publication. The current inventory has
642 components and no missing package metadata. The repository still requires
a top-level license text before public publication; Cargo's workspace
`license = "MIT"` declaration is metadata, not a substitute for the license
notice.

## Signature verification

Stage 5 candidate artifacts use Cosign blob bundles. Verify a file with the
published candidate public key and its matching bundle:

```text
cosign verify-blob \
  --key arda-stage5-rc-cosign.pub \
  --bundle <artifact>.sigstore.json \
  <artifact>
```

Every AppImage, DEB, RPM, SBOM, release manifest, and checksum ledger must pass
that command before distribution.

## Key ownership and production boundary

The local Stage 5 proof used a newly generated, password-encrypted ephemeral RC
key. Its private half was deleted after all candidate files were signed. Only
the public key and detached verification bundles are retained. These signatures
prove integrity against that candidate public key; they do not establish
production publisher identity.

Production release signing remains blocked until the operator configures one of:

1. a hardware-backed or KMS-held Cosign key with documented owner and recovery;
2. a keyless Sigstore identity tied to protected release automation.

Production private keys and passwords must never be stored in the repository,
release directory, diagnostics bundle, shell history, or support archive. Key
rotation requires publication of the replacement trust root, a transition
period in which old releases remain verifiable, and an incident entry if the
rotation follows suspected compromise.

## License policy and exceptions

Shipped dependencies must identify an SPDX-compatible license in the generated
inventory. Missing, unknown, source-only, or non-redistributable licensing
blocks publication. An exception requires a named owner, affected component and
version, rationale, distribution impact, expiry/review date, and maintainer
approval in the Stage 5 risk ledger. There are no implicit exceptions.

## Current reproducibility result

- AppImage: byte-identical across two fixed-epoch builds.
- Release manifest and `SHA256SUMS`: byte-identical from the same inputs.
- DEB: not byte-identical across two fixed-epoch builds.
- RPM: not byte-identical across two fixed-epoch builds.

The exact hashes, signature status, SBOM count, and blockers are recorded in
`docs/evidence/stage-5-release-candidate/packaging/packaging-summary.json`.
