# Arda Workbench release-candidate packaging and signing

This runbook covers the declared Stage 5 Linux profile and the `0.3.0-rc.1`
Workbench launcher. The Linux release set includes the x86_64 AppImage, DEB,
and RPM packages.

## Build inputs

Build from the repository root with the lockfiles unchanged. Set
`SOURCE_DATE_EPOCH` to the source commit timestamp. The packaging manifest
records:

- source commit and release-relevant source-tree SHA-256;
- Cargo and pnpm lockfile SHA-256 values;
- Rust, Cargo, Node, pnpm, and AppImage tool versions;
- the AppImage tool binary SHA-256;
- the separately pinned AppImage runtime SHA-256;
- supported profile and schema/rollback compatibility.

The reproducible AppImage proof uses upstream `appimagetool` 1.9.1 x86_64,
pinned to SHA-256
`ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0`.
The type-2 x86_64 runtime is fetched separately and pinned to SHA-256
`1cc49bcf1e2ccd593c379adb17c9f85a36d619088296504de95b1d06215aebbf`.
The runtime release URL is mutable; the checksum is the immutable authority and
the fetch fails closed if upstream replaces those bytes. The older AppImageKit
build `5735cc5` is not suitable: it injects the current time and conflicts with
`SOURCE_DATE_EPOCH`.

## Build and verify

Build the frontend and native launcher, then create AppImage/DEB/RPM bundles
with Tauri. The launcher package script sets linuxdeploy's supported
`NO_STRIP=true` control because its bundled old `strip` cannot read modern
`.relr.dyn` sections; this does not skip Rust release optimization. Repackage
the complete AppDir twice with the pinned `appimagetool`, pinned runtime, and
fixed epoch for the byte-reproducibility proof:

```text
python3 scripts/arda_appimage.py fetch --cache-dir <tool-cache>
export APPIMAGETOOL=<tool-cache>/appimagetool-1.9.1-x86_64.AppImage
export APPIMAGE_RUNTIME=<tool-cache>/runtime-x86_64-1cc49bcf1e2c
export SOURCE_DATE_EPOCH="$(git show -s --format=%ct HEAD)"

python3 scripts/arda_appimage.py package \
  --appdir <Arda.AppDir> --output <first>/arda-launcher.AppImage \
  --appimagetool "$APPIMAGETOOL" --runtime "$APPIMAGE_RUNTIME" \
  --source-date-epoch "$SOURCE_DATE_EPOCH"
python3 scripts/arda_appimage.py package \
  --appdir <Arda.AppDir> --output <second>/arda-launcher.AppImage \
  --appimagetool "$APPIMAGETOOL" --runtime "$APPIMAGE_RUNTIME" \
  --source-date-epoch "$SOURCE_DATE_EPOCH"
cmp <first>/arda-launcher.AppImage <second>/arda-launcher.AppImage
```

`arda_appimage.py` rejects unpinned tool/runtime bytes, missing AppDir metadata,
and outputs that are not real type-2 AppImages. AppDir output alone does not
satisfy the release gate.

Tauri 2.9.4 writes wall-clock metadata into DEB tar members and into the RPM
`BUILDTIME`/`FILEMTIMES` tags. Normalize both package formats before hashing,
manifest generation, or signing. In-place normalization is supported:

```text
python3 scripts/arda_reproducible_packages.py normalize-deb \
  --input <DEB> --output <DEB> --epoch "$SOURCE_DATE_EPOCH"

python3 scripts/arda_reproducible_packages.py normalize-rpm \
  --input <RPM> --output <RPM> --epoch "$SOURCE_DATE_EPOCH"
```

The RPM normalization updates only timestamp tags and the dependent header
SHA-256 field; payload bytes are preserved. Verify the result with `rpm -Kv`
and compare two independently built/normalized outputs with `cmp`.

Generate and verify the release ledger:

```text
python3 scripts/arda_release_ops.py bundle-manifest \
  --root "$PWD" --version 0.3.0-rc.1 \
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
642 components and no missing package metadata. The repository includes the
top-level MIT `LICENSE` text required by its Cargo package metadata.

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

Production artifacts use keyless Sigstore bundles created by the tag-bound
GitHub Actions workflow. Verify them with the exact release tag identity:

```text
cosign verify-blob <artifact> \
  --bundle <artifact>.sigstore.json \
  --certificate-identity \
    "https://github.com/dward1502/Arda/.github/workflows/release-sign.yml@refs/tags/<tag>" \
  --certificate-oidc-issuer "https://token.actions.githubusercontent.com"
```

## Key ownership and production boundary

The local Stage 5 proof used a newly generated, password-encrypted ephemeral RC
key. Its private half was deleted after all candidate files were signed. Only
the public key and detached verification bundles are retained. These signatures
prove integrity against that candidate public key; they do not establish
production publisher identity.

Production release signing uses `.github/workflows/release-sign.yml` with
GitHub OIDC, Fulcio short-lived certificates, and Rekor transparency logging.
No long-lived production private key is retained. Before enabling releases:

1. create the `production-release` GitHub environment with a required
   maintainer reviewer;
2. protect `main` and require review for changes to the signing workflow;
3. publish only from a reviewed `v*` tag whose commit matches the release
   manifest;
4. do not announce or distribute a release until every detached bundle has
   uploaded and passed identity-bound verification.

The workflow downloads an exact six-file allowlist, verifies the manifest
version/source commit and `SHA256SUMS`, signs each file, verifies each generated
bundle, and only then uploads the bundles. A workflow path, repository owner,
or OIDC issuer change is a publisher-identity rotation and requires a documented
transition. Unexpected Rekor entries for this workflow identity are a signing
incident.

## License policy and exceptions

Shipped dependencies must identify an SPDX-compatible license in the generated
inventory. Missing, unknown, source-only, or non-redistributable licensing
blocks publication. An exception requires a named owner, affected component and
version, rationale, distribution impact, expiry/review date, and maintainer
approval in the Stage 5 risk ledger. There are no implicit exceptions.

## Current reproducibility result

- AppImage: byte-identical across two fixed-epoch builds.
- Release manifest and `SHA256SUMS`: byte-identical from the same inputs.
- DEB: byte-identical after fixed-epoch normalization.
- RPM: byte-identical after fixed-epoch normalization; header and payload
  digests pass `rpm -Kv`.

The exact hashes, signature status, SBOM count, and blockers are recorded in
`docs/evidence/stage-5-release-candidate/packaging/packaging-summary.json` and
`docs/evidence/stage-5-release-candidate/packaging/linux-package-reproducibility.json`.
