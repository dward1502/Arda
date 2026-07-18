---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-18"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-07-18

# ANNUNIMAS SYSTEM STATUS REPORT

**Updated:** 2026-07-18 PDT
**Validation basis:** current workspace state, live listener/runtime checks, and cargo checks for the active Arda crate set.
**System:** Arda — the continued, slimmed-down Annunimas workspace.
**Realm:** Sovereign Intelligence Infrastructure
**Status:** WORKSPACE VALIDATED / OPERATIONAL IN EVIDENCE PORTION ⚠️
**Branch:** `launcher`
**Sigil:** ∇ ⚡ ◈ ♥ ↝

---

## ∇ SOVEREIGNTY STATUS

Current capability posture:

- Arda canonical root: `/var/home/mythos/Eregion/Arda`
- Legacy `annunimas-*` crate names should be read as `arda-*`; Arda is the slimmed-down continuation of Annunimas.
- Rust workspace validates cleanly under `cargo check`; manwe builds with and without the `adaptive` feature.
- Active Charon-compatible router is reachable on `0.0.0.0:5110`; LiteLLM is reachable on `127.0.0.1:4000`; metrics on `9101`; node exporter on `9100`; local mesh-llm on `3131` and `9337`.
- No `arda*` or `annunimas*` user systemd units are loaded in the current environment; runtime evidence is process-listener based, not systemd-unit based.
- Workspace warning cleanup is complete for `arda-core`, `arda-engine`, `arda` bin, and `manwe`.

---

## ⚡ RUNTIME SERVICES

Live listener/runtime snapshot:

| Service / Surface | State |
|------|-------|
| Charon/router (`annunimas-cli`) | ✅ listening on `0.0.0.0:5110` |
| LiteLLM bridge | ✅ listening on `127.0.0.1:4000` |
| Metrics exporter | ✅ listening on `0.0.0.0:9101` |
| Node exporter | ✅ listening on `*:9100` |
| Local mesh-llm | ✅ listening on `0.0.0.0:3131` and `0.0.0.0:9337` |
| Arda user systemd units | ⚠️ none loaded |

Evidence basis: `ss -ltnp`, process ownership, repo workspace validation.

---

## 🔧 CHARON PROVIDER MESH

- **Router/binary surface:** active Charon router listener on `0.0.0.0:5110`
- **Local gateway crate:** `manwe` at `crates/spine/runtime/manwe`; default static gateway is `127.0.0.1:7171`
- **Provider state projection:** no live `core/state/charon_router.json` or `core/state/queue_summary.json` state observed under the Arda root
- **Active local mesh-llm endpoints:** `3131`, `9337`
- **Active LiteLLM endpoint:** `4000`

Operational note: This review does not verify provider/model count from state files because those files were not present at scan time. Route behavior should be validated directly against the running router on `:5110`.

---

## 🧭 COMMAND AND SUBSYSTEM SURFACE

Workspace root surface:
- Package: `arda`
- Members include: `arda-engine`, `arda-core`, `arda-council`, `arda-governance`, `arda-orome`, `arda-aule`, `arda-vaire`, `arda-economics`, `arda-mandos`, `arda-varda`, `manwe`, and the Tauri app memberspaces.

Known subsystem surfs currently documented/implemented:
- Charon router / manwe gateway
- Local inference endpoints: mesh-llm on `3131` / `9337`
- LiteLLM gateway on `4000`
- Metrics exporter `9101`
- Node exporter `9100`

CLI default LLM surface and unit-file runtime surfaces should be confirmed from live command output before quoting exact provider/model strings here.

---

## 🖥️ UI, DEVICE, AND OBSERVABILITY SURFACES

- **ARDA HUD:** expected under `apps/arda-hud/`; final validation should run via `pnpm run tauri dev` inside the app workspace.
- **ARDA launcher:** expected under `apps/arda-launcher/`; validation should run via `pnpm run tauri dev/check` there.
- **Metrics:** Annunimas metrics exporter is listened on `9101`; node exporter on `9100`.
- **Local model infrastructure:** local mesh-llm/OpenAI-compatible endpoints are listening on `3131` and `9337`.

---

## ◈ VALIDATION EVIDENCE

Validation run during this report refresh:

- `git status` and `cargo metadata`
- `cargo check --workspace`
- `cargo check -p manwe --features adaptive`
- `ss -ltnp`
- `systemctl --user list-units 'arda*' --all --no-pager`
- `ls -la ~/.cache/annunimas-build`

Results / notes:

- All Arda workspace crate checks validate; no error states remain from the manwe adaptive/default split or `arda-core` warning cleanup.
- `arda-engine` unused-mut warning fixed.
- `arda` bin unused-variable warning fixed.
- `manwe` dead-code warning for `DefaultFreezeNone` removed in both `src/types.rs` and `src/adaptive/types.rs`.
- Direct `/v1/healthz` or `/status` probe behavior for Charon was not verified in this pass beyond listener presence; add an HTTP probe against `127.0.0.1:5110` before treating endpoint health as fully confirmed.

---

## ↝ NEXT OPERATOR CHECKS

1. Confirm why `arda*`/`annunimas*` user units are unloaded; if this box is traditionally managed by systemd timer units, re-evaluate whether the environment now relies on foreground/process supervision instead.
2. Probe Charon endpoint behavior on `127.0.0.1:5110` directly and capture provider/model/route metadata into a current state file inside the Arda repo, if state files are expected.
3. If ARDA HUD validation is still pending, run `pnpm run tauri dev` in the relevant app workspace and verify native surface behavior rather than relying on host-only preview.
4. Refresh this report after any provider catalog/policy or mesh-llm endpoint change.

---

**Authority:** Arda Sovereign System
