---
soterion:
  sigil: "📝"
  glyph: "📝"
  code_point: "U+1F4DD"
  role: "migration_notes"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-18"
---

# Config Migration Notes

Root: `/var/home/mythos/Eregion/Arda/config`

## Completed
- Normalized `annunimas` → `arda` in configured documents and schemas with minimal blast radius under `config/`.
- Retired legacy `annunimas.*` templates/env into `config/archive/legacy-annunimas/`.
- Retired generated artifacts to `config/archive/generated`.
- Migrated retirement paths into clean directories: governance, business, integrations/hermes, integrations/litellm, integrations/chronos, monitoring, runtime, routing, systemd, env.
- Refreshed `INDEX.md` to match current tree.
- Retired stale `INDEX.jsonl`; regenerate when metadata is needed.

## Deferred
- `.env` / `.env.example` details retained in `config/env/`; local secret content remains untouched. If moved elsewhere, update consumers first.
- Hard-wired paths remain intentionally present where consumers still resolve `~/Annunimas` or `annunimas-server`; those are left in place unless this is actively migrating endpoints.
- `gen_keys.rs` retired, but if tooling/maintenance depends on it, it should be relocated inside `tools/` and updated explicitly.
- Systemd `.service`/`.timer`, monitoring dashboards/rules JSON, and monitoring markdown renamed only in metadata; runtime jobs are unchanged. Update actual unit names/alert identities separately if desired.
- `arda_hud.settings.json` relocated to apps-relative target outside this config pass unless confirmed.

## Index/index recovery
- `INDEX.jsonl` was stale. Delete-on-archive path: `config/archive/index/INDEX.jsonl`.
- `config/INDEX.md` restored to tree-real state. Do not re-add ghost entries until the files exist.
