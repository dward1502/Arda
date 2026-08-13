# Arda Workbench Alpha Tester Handoff

This packet is for disclosed testing of the Arda 0.9 personal baseline. It is
not a claim that Arda 1.0 is complete or that 0.9 received independent flow
review. Personal Operations, payments, remote devices, and other optional
applications are not required for this Workbench alpha.

## What the facilitator supplies

- the exact `arda-source-0.9.0.tar.gz` source archive;
- one native launcher package for the tester's supported Linux environment;
- `release-bundle-manifest.json`, `release-sbom.json`, and `SHA256SUMS`;
- this guide and `templates/alpha-tester-record.json`;
- a disposable project fixture containing one bounded change and one declared
  verification command.

Do not send credentials, private runtime state, `core/state` changes from a live
operator profile, or monitor acceptance recordings.

## Supported alpha boundary

The declared package qualification profile is x86_64 Bluefin LTS 10. Testing on
another Linux profile is welcome compatibility feedback but cannot establish
the supported-profile release gate. The runtime is local and loopback-only; do
not expose its unauthenticated endpoints to a network.

The candidate still requires the supplied source tree as `ARDA_ROOT`. Unpack it
in a new directory and keep mutable state in the tester's normal XDG user paths.

## Integrity and installation

From the handoff directory:

```bash
sha256sum --check SHA256SUMS
tar -xzf arda-source-0.9.0.tar.gz
export ARDA_ROOT="$PWD/arda-source-0.9.0"
python3 "$ARDA_ROOT/scripts/arda_beta_ops.py" compatibility \
  --root "$ARDA_ROOT" --home "$HOME"
chmod +x "$PWD/arda-launcher_0.9.0_amd64.AppImage"
python3 "$ARDA_ROOT/scripts/arda_beta_ops.py" install-launcher \
  --root "$ARDA_ROOT" \
  --artifact "$PWD/arda-launcher_0.9.0_amd64.AppImage"
arda-launcher
```

The AppImage may instead be launched directly after checksum verification.
Record the exact package name and SHA-256 in the tester record.

## Alpha tasks

Without editing raw state or source files, ask the tester to:

1. install and start the native launcher;
2. explain the visible readiness state and next safe action;
3. attach the disposable project and identify its command, writable paths,
   network/secret scope, budget, and approval control;
4. approve or reject one bounded proposal and identify the resulting diff,
   verification result, provider/model provenance, and receipt status;
5. encounter one seeded recoverable failure and choose retry, reset, restore, or
   rollback from visible evidence;
6. restart the application and find the same run, approval, and evidence;
7. create redacted diagnostics and inspect the archive before sharing it;
8. uninstall the managed launcher while confirming state remains preserved.

Record confusion and defects rather than coaching around them. A failed task is
useful alpha evidence and should remain failed in the record.

## Evidence boundary

This alpha can validate installability and reveal defects now. Independent flow
review is intentionally not a 0.9 gate, so an alpha record is feedback rather
than independent qualification. A future final release must use its own exact
signed bytes and then-current evaluation policy; do not rewrite this record into
a final pass after the artifact identity changes.