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

## Governed work cycles

- Enable `arda-aule-autopilot.timer` for the production governed admission
  cycle. It may admit only governance-authorized reversible work; consequential
  actions still require operator approval lineage.
- `arda-aule-autopilot-read-only.timer` is the diagnostic alternative. Do not
  enable it alongside the governed timer because both units inspect and project
  the same operating-loop state.
- Enable `arda-workbench-queue-executor.timer` to consume eligible canonical
  queue work through the bounded Workbench adapter once per minute.

Install the source-current CLI and all six automation unit templates atomically,
then activate the governed admission and Workbench timers with:

```bash
scripts/install_arda_automation_units.sh
```

The installer records a timestamped rollback bundle under
`~/.local/state/arda/rollback/`, disables the superseded read-only timer, and
enables the governed admission and Workbench execution timers.

## Contents

See `INDEX.md` for deterministic child listing.
