---
soterion:
  sigil: "REPAIR"
  role: "implementation_guide"
  owner: "HERMES"
  status: "active"
  reviewed: "2026-08-20"
---

> 🜏 Soterion: REPAIR implementation_guide | owner: HERMES | status: active | reviewed: 2026-08-20

# ARDA Mirromere

Standalone Tauri application for the governed Mirromere physical display. It consumes `arda-mirromere` and `@arda/mirromere-ui`; ARDA HUD remains a separate passive aperture consumer and cannot create this window.

## Build and package

From the repository root:

```text
scripts/package_arda_mirromere.sh
```

The packaging receipt at `data/prometheus/arda_mirromere_package_last.json` records the release binary, AppImage, and SHA-256 values.

## Install and lifecycle

```text
scripts/install_arda_mirromere.sh
scripts/launch_arda_mirromere.sh
scripts/uninstall_arda_mirromere.sh
```

The binary installs to `~/.local/lib/arda/mirromere/arda_mirromere`. The static user unit is explicit-only: it is not wanted by `arda-session.target`, has `Restart=no`, and a normal close leaves it inactive.

## Display ownership

The application enumerates displays itself and requires an explicit non-primary selection. It persists only a stable display id under the operator configuration directory, re-resolves current geometry before projection, and fails closed to a privacy veil when selection is absent, primary, ambiguous, or disconnected.

## Verification

```text
pnpm test
pnpm run build
cargo test --manifest-path src-tauri/Cargo.toml
scripts/verify_arda_mirromere_unit.sh
scripts/verify_arda_user_units.sh
```

These gates prove buildable contracts and package identity. Physical second-monitor placement, disconnect/reconnect, operator interaction, and frame-time acceptance remain native Task 7 evidence and cannot be replaced by browser or fixture proof.
