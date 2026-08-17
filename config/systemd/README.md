---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "organization_index"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
---

> 🜏 Soterion: 📜 organization_index | owner: HADES | status: active | reviewed: 2026-05-21

# systemd

Purpose: repository-owned templates for Arda user-systemd runtime authority.

## Session lifecycle

- `arda-session.target` pulls in the required backend units only:
  `arda.service` and `hermes-gateway.service`.
- `arda-hud.service` is a static graphical-session unit. It starts the packaged
  native HUD from `%h/.local/lib/arda/hud/arda_hud` and cannot start through a
  Vite/browser preview path.
- The HUD unit wants the session target, but the session target never wants the
  HUD. Stopping or closing HUD therefore does not stop backend lifetime.
- The launcher will start `arda-hud.service` explicitly after health gates in a
  later task; the HUD unit intentionally has no `[Install]` section.

Install the units and one verified native HUD binary with:

```bash
scripts/install_arda_user_units.sh /path/to/arda_hud
```

The installer atomically replaces the native binary and unit files, restores
their prior versions if verification or manager reload fails, imports only the
known graphical-session environment names into the user manager, reloads user
systemd, and verifies unit discovery. It does not enable or start either unit.

Run the static verifier independently with:

```bash
scripts/verify_arda_user_units.sh
```

## Contents

See `INDEX.md` for deterministic child listing.
