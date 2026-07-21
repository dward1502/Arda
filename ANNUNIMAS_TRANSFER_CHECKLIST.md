---
soterion:
  sigil: "BOOKMARK"
  glyph: "🚀"
  code_point: "U+1F680"
  role: "annunimas_transfer"
  owner: "HADES"
  status: "draft"
  last_reviewed: "2026-07-17"
---

# Annunimas → Arda Transfer Checklist

Source: `~/Annunimas/*`
Destination: `/var/home/mythos/Eregion/Arda/*`

## Immediately removable / junk

- `output.txt`
- `override.txt`
- `test.log`
- `hermes-verify-mintest-qwBNZz.txt`
- `ingest_human_notes.py`
- `__pycache__/`
- `.pytest_cache/`
- `.opencode/`
- `text_multi-agent_orchestration_open_source_2026_20260619_061629.json`

## Transfer — high confidence

| Source | Destination | Why |
|--------|-------------|-----|
| `ANNUNIMAS_ROOT_PROTOCOL.md` | `docs/ANNUNIMAS_ROOT_PROTOCOL.md` | Canonical repo-root protocol doc |
| `ANNUNIMAS_SYSTEM_STATUS_REPORT.md` | `docs/ANNUNIMAS_SYSTEM_STATUS_REPORT.md` | Status artifact |
| `ARDA_HUD_PUBLIC_PRODUCT_STRATEGY.md` | `apps/arda-hud/ARDA_HUD_PUBLIC_PRODUCT_STRATEGY.md` | HUD strategy |
| `CODEMAP.md` | `docs/CODEMAP.md` | Code map |
| `DIRECTORY_INDEX.md` | `docs/DIRECTORY_INDEX.md` | Legacy index |
| `FILE_TREE.jsonl` | `docs/FILE_TREE.jsonl` | Metadata artifact |
| `TRACT_INDEX.json` | `docs/TRACT_INDEX.json` | Metadata artifact |
| `TRACT_MAP.txt` | `docs/TRACT_MAP.txt` | Metadata artifact |
| `meta/soterion_sigils.yaml` | `meta/soterion_sigils.yaml` | Shared metadata |
| `spec/agent-state-contract.md` | `spec/agent-state-contract.md` | Spec doc |
| `human/company_view.md` | `human/company_view.md` | Human corpus |
| `human/thoughts.md` | `human/thoughts.md` | Human corpus |
| `human/onboard.md` | `human/onboard.md` | Human corpus |
| `human/overrides.md` | `human/overrides.md` | Human corpus |
| `human/index.md` | `human/index.md` | Human corpus |
| `human/README.md` | `human/README.md` | Human corpus |
| `audit/upper_monitor_2_preview_2026-06-02.png` | `audit/upper_monitor_2_preview_2026-06-02.png` | Audit artifact |
| `data/charon_route_smoke_history.jsonl` | `data/charon_route_smoke_history.jsonl` | Data artifact |

## Review before transfer — likely duplicate

- `config/*` ↔ `Eregion/Arda/config/*`: diff first; prefer Arda canonical copies, archive Annunimas duplicates
- `core/*` ↔ `Eregion/Arda/core/*`: many files already moved; treat `~/Annunimas/core/` as legacy view
- `scripts/*` ↔ `Eregion/Arda/scripts/*`: prefer Arda versions if present
- `tests/*`: Python tests may still run from this tree; check CI before moving

## Likely junk / archive-only

- `AGENTS.md`, `README.md`, `CONTRIBUTING.md`
- `registry.toml`, `Cargo.toml`, `Cargo.lock`, `.cargo/config.toml`
- `.env.example`
- `tests/INDEX.jsonl`, `scripts/INDEX.jsonl`, `docs/INDEX.jsonl`
- `core/core_audit.json`, `core/INDEX.jsonl`

## Suggested transfer order

1. **Docs/metadata first** — low risk: `docs/*`, `meta/*`, `spec/*`, top-level `*.md` except `README.md`/`AGENTS.md`
2. **Configs** — diff `~/Annunimas/config/` against `Eregion/Arda/config/`; transfer only missing or newer files
3. **Scripts** — diff `scripts/` and transfer only missing/updated scripts; retire duplicates
4. **Tests** — verify CI still runs from `~/Annunimas/tests/`; if Arda has equivalents, archive Annunimas tests
5. **Data/audit artifacts** — transfer if missing from `Eregion/Arda/data/` and `audit/`
