# Arda Private-Beta Install and Recovery

This runbook began as the Stage 4 operator path and now also documents the
release-candidate lifecycle. The launcher can be installed without root
privileges, but the current candidate still requires a complete Arda release
checkout as `ARDA_ROOT`. An alpha handoff must therefore include the exact
source archive identified by its manifest; the launcher package alone is not a
self-contained Arda distribution.

The commands below never write to or delete the source repository. Mutable state is restricted to the selected user's XDG directories.

## Supported state and install roots

| Purpose | Default path | Override |
|---|---|---|
| Runtime/source root (read only for beta operations) | current directory | `ARDA_ROOT` or `--root` |
| Private config | `~/.config/arda` | `ARDA_CONFIG_DIR` |
| Persistent data | `~/.local/share/arda` | `ARDA_DATA_DIR` |
| Launcher WebKit state | `~/.local/share/arda.launcher` | `ARDA_LAUNCHER_DATA_DIR` |
| Cache | `~/.cache/arda` | `ARDA_CACHE_DIR` |
| Runtime files | `${XDG_RUNTIME_DIR}/arda`, or `~/.local/run/arda` | `ARDA_RUNTIME_DIR` |
| Operation receipts/quarantine | `~/.local/state/arda` | `XDG_STATE_HOME` |
| Installed launcher | `~/.local/lib/arda/arda-launcher` | selected by `--home` |
| Launcher command | `~/.local/bin/arda-launcher` | selected by `--home` |

`arda_beta_ops.py` refuses a mutable state or install root that is the source root, contains it, is inside it, is outside the selected home, or resolves to `/` or the home directory itself.

## 1. Build and verify the launcher

From a complete release checkout:

```bash
cd /path/to/Arda
pnpm --dir apps/arda-launcher install --frozen-lockfile
pnpm --dir apps/arda-launcher test
pnpm --dir apps/arda-launcher lint
pnpm --dir apps/arda-launcher build
cargo test --manifest-path apps/arda-launcher/src-tauri/Cargo.toml
ARDA_LAUNCHER_SKIP_BUNDLE=1 cargo build \
  --manifest-path apps/arda-launcher/src-tauri/Cargo.toml \
  --release
```

Resolve Cargo's target directory and the user-installable artifact (this repo may configure a shared target cache):

```bash
TARGET_DIR="$(cargo metadata \
  --manifest-path apps/arda-launcher/src-tauri/Cargo.toml \
  --format-version 1 --no-deps \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
LAUNCHER_ARTIFACT="$TARGET_DIR/release/arda-launcher"
test -x "$LAUNCHER_ARTIFACT"
```

Record the checkout identity before handing it to an evaluator:

```bash
git rev-parse HEAD
git status --short
sha256sum "$LAUNCHER_ARTIFACT"
```

A dirty checkout is not a release artifact. Runtime state changes under `core/state/` must not be bundled or silently cleaned.

## 2. Reproduce a clean-profile install

Use a genuinely new OS user or VM for evaluator evidence. A temporary home is suitable only for testing the installer mechanics:

The current private-beta launcher is a dynamically linked Linux binary rather than a self-contained AppImage. Install its runtime libraries before installation:

```bash
# Fedora-family workstation
sudo dnf install gtk3 webkit2gtk4.1

# Ubuntu/Debian-family workstation
sudo apt install libgtk-3-0 libwebkit2gtk-4.1-0
```

Package names can vary by distribution release. `readiness` runs `ldd` against the installed artifact and reports the exact unresolved library names; do not treat an executable bit alone as launch readiness.

```bash
export ARDA_ROOT=/path/to/Arda
export BETA_HOME="$(mktemp -d)"
python3 scripts/arda_beta_ops.py install-launcher \
  --root "$ARDA_ROOT" \
  --home "$BETA_HOME" \
  --artifact "$LAUNCHER_ARTIFACT"

readlink "$BETA_HOME/.local/bin/arda-launcher"
sha256sum "$BETA_HOME/.local/lib/arda/arda-launcher"
```

On the evaluator account, omit `--home` and ensure `~/.local/bin` is on `PATH`:

```bash
python3 scripts/arda_beta_ops.py install-launcher \
  --root "$ARDA_ROOT" \
  --artifact "$LAUNCHER_ARTIFACT"
```

The install writes only:

- `~/.local/lib/arda/arda-launcher`;
- `~/.local/lib/arda/install-manifest.json`;
- `~/.local/bin/arda-launcher` as a symlink;
- `~/.local/share/applications/arda-launcher.desktop`.

## 2a. Stage 5 compatibility and release manifest

S5-RC0 supports only `bluefin-lts-10-x86_64`: Linux x86_64 with Bluefin LTS
10 (`ID=centos`, `VERSION_ID=10`). Check before packaging or installation:

```bash
python3 scripts/arda_beta_ops.py compatibility \
  --root "$ARDA_ROOT" --home "$HOME"

python3 scripts/arda_beta_ops.py release-manifest \
  --root "$ARDA_ROOT" --home "$HOME" \
  --artifact "$LAUNCHER_ARTIFACT" \
  --version 0.9.0 \
  --output /path/to/release-manifest.json \
  --checksums-output /path/to/SHA256SUMS
```

Unsupported profiles stop before installation paths are created. Use explicit
`--home` so inherited XDG roots from another account/container cannot escape
the selected operator home.

Upgrade an existing Stage 4 layout and preserve the returned rollback receipt:

```bash
python3 scripts/arda_beta_ops.py upgrade-launcher \
  --root "$ARDA_ROOT" --home "$HOME" \
  --artifact "$LAUNCHER_ARTIFACT" \
  --release-manifest /path/to/release-manifest.json \
  --backup-output "$HOME/arda-pre-upgrade.tar.gz"
```

After completing and backing up the candidate run, restore the prior launcher
and verified post-run state:

```bash
python3 scripts/arda_beta_ops.py rollback-launcher \
  --root "$ARDA_ROOT" --home "$HOME" \
  --upgrade-receipt /path/from/upgrade-receipt.json \
  --state-archive "$HOME/arda-post-run.tar.gz"
```

Default state archives omit secret-named files. Rollback preserves those files
in place while restoring verified non-secret config/data and records the backup
hash, run-truth comparison, and source-tree comparison in machine-readable
receipts.

Run `arda-launcher` inside the user's graphical session. The launcher evaluates
one first-run projection before claiming `Ready`. Select `REVIEW` to follow the
visible sequence in order:

1. supported-profile compatibility;
2. prerequisite checks;
3. provider setup state;
4. service plan;
5. readiness review; and
6. guided setup.

The panel is diagnostic and approval-gated: loading it performs no configuration
write. Secret writes and consequential service changes still require explicit
human approval and a receipt. A warning or failure does not claim the runtime can
begin. Every non-passing check includes its evidence, severity, and recovery
instruction. Offline routing, unavailable providers, degraded readiness, and
general recovery appear as separate plain-language conditions. Optional root runtime services are shown as opt-in and disabled. Optional
application products are not members of the required Workbench startup registry;
their absence cannot block the Workbench start decision.

## 3. Readiness diagnostics

The standalone readiness projection is useful before opening the GUI:

```bash
python3 scripts/arda_beta_ops.py readiness --root "$ARDA_ROOT"
```

Interpret `gate_status` literally:

- `pass`: all high-severity installation/provider checks passed;
- `warn`: the install can be inspected, but at least one medium-severity requirement is absent;
- `fail`: do not run a live-provider or evaluator golden run.

The launcher dynamic-library check is high severity. If it fails, install the runtime packages listed above and rerun readiness before attempting GUI startup.

The command does not probe model quality or claim a provider request succeeded. A configured Manwe endpoint is necessary, but a live-provider golden run remains a separate evidence gate.

## 4. Back up private-beta state

The default archive includes regular files under config and persistent data. It refuses symlinks, writes the archive with mode `0600`, records SHA-256 hashes, and omits secret-named files such as `.env`, `arda.env`, credentials, tokens, passwords, and private keys.

```bash
python3 scripts/arda_beta_ops.py backup \
  --root "$ARDA_ROOT" \
  --output "$HOME/arda-backup-$(date -u +%Y%m%dT%H%M%SZ).tar.gz"
```

The receipt reports `excluded_secret_count`. Provider credentials must normally be re-entered after restore.

Only when an operator has approved storing credentials in a protected archive:

```bash
python3 scripts/arda_beta_ops.py backup \
  --root "$ARDA_ROOT" \
  --include-secrets \
  --output /secure/offline/location/arda-full-backup.tar.gz
```

The archive is permission-restricted, not encrypted. Encrypt it before transport.

## 5. Restore and verify

Restore defaults to empty config/data targets. Archive members are allow-listed to `config/` and `data/`; absolute paths, traversal, links, malformed contracts, and hash mismatches are rejected.

```bash
python3 scripts/arda_beta_ops.py restore \
  --root "$ARDA_ROOT" \
  --archive "$HOME/arda-backup-YYYYMMDDTHHMMSSZ.tar.gz"
```

For a non-empty target, `--force` displaces the current config/data trees under `~/.local/state/arda/displaced/<timestamp>/` before installing the restored trees:

```bash
python3 scripts/arda_beta_ops.py restore \
  --root "$ARDA_ROOT" \
  --archive /path/to/verified-backup.tar.gz \
  --force
```

After restore:

```bash
python3 scripts/arda_beta_ops.py readiness --root "$ARDA_ROOT"
arda-launcher
```

If restore fails after displacement begins, the tool rolls the displaced state back into place.

## 6. Safe reset

Reset is backup-first and quarantine-only. It does not recursively delete state and never touches `ARDA_ROOT`:

```bash
python3 scripts/arda_beta_ops.py reset \
  --root "$ARDA_ROOT" \
  --backup-output "$HOME/arda-before-reset-$(date -u +%Y%m%dT%H%M%SZ).tar.gz"
```

Config, durable data, launcher WebKit state, cache, and runtime roots that exist are moved to `~/.local/state/arda/resets/<timestamp>/`. The launcher WebKit state is reset rather than backed up because it contains browser-engine cache/storage, not canonical Workbench run truth. Reconfigure or restore after confirming the fresh state behaves correctly. Delete quarantine only after human review.

## 7. Redacted diagnostics bundle

```bash
python3 scripts/arda_beta_ops.py diagnostics \
  --root "$ARDA_ROOT" \
  --output "$HOME/arda-diagnostics-$(date -u +%Y%m%dT%H%M%SZ).tar.gz"
```

The `0600` archive contains:

- OS and tool versions;
- standalone readiness results;
- source branch, short commit, and a bounded status summary when `.git` is present;
- text config files with secret-like assignment values, bearer tokens, URL credentials, home paths, and source paths redacted.

It omits secret-named config files, binary config files, files larger than 256 KiB, environment dumps, state payloads, source diffs, and provider responses. Inspect the archive before sharing:

```bash
tar -tzf "$HOME"/arda-diagnostics-*.tar.gz
tar -xOf "$HOME"/arda-diagnostics-*.tar.gz diagnostics.json | less
```

Redaction is defense in depth, not proof that arbitrary free-form user text is safe to disclose. An operator must review every external bundle.

## 8. Uninstall and recovery

Remove only installer-managed launcher files while preserving config, data, launcher WebKit state, and the source repository:

```bash
python3 scripts/arda_beta_ops.py uninstall-launcher --root "$ARDA_ROOT"
```

To remove state later, first create and inspect a backup, run safe reset, then manually delete the specific quarantine directory after approval. Do not use `rm -rf "$ARDA_ROOT"` as an uninstall step.

Recovery order:

1. Capture a redacted diagnostics bundle while the failure is present.
2. Back up state.
3. Run readiness and follow the exact failed-check recovery instructions.
4. If state is suspect, safe-reset it and retry with fresh state.
5. If fresh state works, restore the verified backup; use `--force` only when displacement is intended.
6. If the launcher artifact is suspect, uninstall and reinstall the launcher from the same verified checkout.
7. Preserve all receipts, archive hashes, commit identity, and observed failure text for the Stage 4 evidence record.

## Current release evidence boundary

This workflow proves install mechanics, truthful readiness reporting, state
backup/restore, quarantine reset, managed uninstall, and redacted diagnostics.
The local U4 lifecycle receipt at
`../evidence/stage-5-release-candidate/reliability/u4-lifecycle-local-20260804.json`
also proves default native startup, upgrade/rollback, terminal-run preservation,
post-uninstall state preservation, and unchanged source identity for the current
unsigned local candidate. It deliberately does not claim the final signed
artifact gate. This workflow does not by itself prove:

- sustained native HUD objective-to-review click-through;
- live HUD event streaming;
- a research-assisted run with cited evidence boundaries;
- a real paid/free provider request and review handoff;
- an invited external evaluator's independent completion.

S5-RC0 upgrade/rollback proof is recorded under
`docs/evidence/stage-5-release-candidate/s5-rc0/`. Signing, self-contained
distribution, full security/soak/accessibility gates, adapter conformance, and
support closeout remain separate Stage 5 work; do not infer them from this
runbook.
