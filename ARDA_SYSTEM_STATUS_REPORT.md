---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-08-08"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-08-08

# ARDA SYSTEM STATUS REPORT

**Updated:** 2026-08-08 PDT
**Validation basis:** live Cargo metadata, listener inventory, and user-systemd
unit state captured during P0.1 doctrine reconciliation. This is a bounded
snapshot, not release qualification or proof that current source was deployed.
**System:** Arda 1.0 personal agent ecosystem.
**Realm:** Sovereign Intelligence Infrastructure
**Status:** `specified` product doctrine / current workspace metadata resolved /
selected installed services active
**Branch:** `visual/hud-boardroom-convergence`
**Sigil:** ∇ ⚡ ◈ ♥ ↝

---

## ∇ SOVEREIGNTY STATUS

Current capability posture:

- Arda canonical root: `/var/home/mythos/Eregion/Arda`
- `cargo metadata --no-deps --format-version 1` resolves 18 packages and 16
  default members.
- The root package/binary is `arda`; `src/main.rs` remains composition authority.
- The workspace contains no standalone council package; council behavior is
  owned by current governance, Oromë, and Aulë surfaces.
- Optional revenue/x402, council, local-inference, external-adapter, and health
  capabilities are not universal product goals or default release gates.

---

## ⚡ RUNTIME SERVICES

Live listener/runtime snapshot:

| Service / Surface | State |
|------|-------|
| `arda.service` | active/running; started 2026-08-07 21:58:59 PDT |
| `arda-manwe.service` | active/running; started 2026-08-07 21:58:59 PDT |
| `arda-metrics-exporter.service` | active/running; started 2026-08-07 21:58:59 PDT |
| `arda-varda.service` | active/running; started 2026-08-07 21:58:59 PDT |
| `arda-relic-bridge.service` | active/running; started 2026-08-07 21:58:59 PDT |
| Listener inventory | `:5110`, `:9101`, `:9100`, and `:9337` were listening; no ownership or API-health claim is inferred from listener presence alone |

Evidence basis: `systemctl --user is-active/show` and `ss -ltn`. Active units
prove process supervision only, not current-source deployment or workflow health.

---

## 🔧 MODEL/PROVIDER ROUTING

- Canonical package: `manwe` at `crates/spine/runtime/manwe`.
- Canonical configuration input: `config/manwe.providers.toml`.
- Coordinated consumer assumptions around `:7171` must be audited before any
  bind change; this snapshot does not change that contract.
- The legacy `:5110` listener remains visible, but listener presence does not
  identify package version, provider health, or route eligibility.

---

## 🧭 COMMAND AND SUBSYSTEM SURFACE

Workspace packages are listed from live metadata in `docs/CODEMAP.md`. The
current command binaries are `arda`, `arda-cli`, `manwe`, `arda-launcher`,
`arda-varda-server`, `arda-varda-benchmark`, `arda-outpost-scout`, and
`arda-relic-presence-sync`.

Package names and source presence establish `compile_active` candidates only.
They do not establish `root_composed`, `workflow_proven`, `operator_accepted`,
or `release_supported` maturity without their specific gates.

---

## 🖥️ UI, DEVICE, AND OBSERVABILITY SURFACES

- **ARDA HUD:** canonical at `apps/arda-hud/`; final visual acceptance requires
  native Tauri and is not claimed by this report.
- **ARDA launcher:** canonical at `apps/arda-launcher/`; package presence is not
  install or operator acceptance.
- **RELIC/CITADEL:** the checked-in bridge is read-only. External display or
  Mirromere behavior remains optional and separately gated.

---

## ◈ VALIDATION EVIDENCE

Validation run during this report refresh:

- `cargo metadata --no-deps --format-version 1`
- metadata package/target extraction (18 packages, 16 default members)
- `ss -ltn`
- `systemctl --user list-units 'arda*' 'annunimas*' --all --no-pager --plain`
- `systemctl --user is-active/show` for the five active services above

No workspace compile/test, external endpoint, phone, native HUD, restart,
privacy, packaging, or release-support claim is made by this P0.1 refresh.

---

## ↝ NEXT OPERATOR CHECKS

1. Complete P0.2 active-plan authority reconciliation.
2. Add P0.3 completion-language checks through the existing HADES Markdown
   health path.
3. Re-probe and record capability-specific runtime evidence only in the plan
   task that owns that acceptance gate.

---

**Authority:** Arda Sovereign System
