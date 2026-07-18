---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-06-26"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-06-26

# Arda Root Protocol

> ∇ Operator-only canonical pointer file. Tells operators and agents where doctrine, runtime truth, and navigation indexes live — does not require upfront reads. Sanitize local live-surface details before using this file in public support or issue contexts.
> Updated 2026-06-26 PDT / 2026-06-27 UTC. Active `~/Annunimas` references are reference-architecture only; canonical authority now lives under this Arda workspace root unless otherwise marked.

---

## Current System Shape

| Surface | Current Posture |
|---------|-----------------|
| System | Arda autonomous CEO agent system |
| Workspace | 26 Rust workspace members; default member is `crates/engine` |
| Runtime supervisor | user systemd plus `scripts/agent_supervisor.sh`; policy lives in Prometheus |
| Router | Charon on port `5110`, with socket path `/run/user/1000/annunimas/charon.sock` when running under the standard service |
| Provider mesh | Local, free-cloud, paid-cloud, mixed-cloud, and LiteLLM-normalized lanes; runtime truth lives in `core/state/charon_router.json` |
| Queue posture | Read `core/state/queue_summary.json` before raw queue evidence |
| UI/device consumers | ARDA HUD and CITADEL avatar are private consumers outside the root Rust workspace |
| Human/operator docs | `human/` lives at the Arda root and is now part of the canonical tree |
| Reference architecture | `~/Annunimas` is reference-only unless `AGENTS.md` or this file marks a path as canonical there |

Use `ANNUNIMAS_SYSTEM_STATUS_REPORT.md` for the latest verified service/provider counts. This root protocol is the pointer map, not the live status ledger.

---

## Where Authority Lives

| Source | Contains |
|--------|----------|
| `AGENTS.md` | Operator/agent working rules for the current repository |
|| `core/realm/arda.toml` | Identity, laws, realms, Soterion basis |
| `core/realm/agents.toml` | Sovereign roster, agent roles, authority |
|| `core/realm/arda.toml` | Identity, laws, realms, Soterion basis |
|| `config/charon.providers.toml` | Charon provider/model routing config; live reload via CLI |
|| `config/model_route_matrix.toml` | Cross-provider model routing matrix |
| `config/fleet.toml` | Fleet node metadata and inference endpoints |
| `config/autonomy_operating_loop.toml` | Portable lane/config contract for the autonomous control plane |
| `core/state/queue_summary.json` | Compact task queue projection; read this before the raw ledger |
| `core/projects/tasks/queue.jsonl` | Active and historical append-only queued work; use for exact evidence or appends |
| `core/state/` | Machine-readable runtime projections and source maps |
| `data/` | Runtime receipts, ledgers, telemetry, service outputs |
| `docs/SAFETY_MODEL.md` | Human-readable safety and governance model |
| `docs/ANNUNIMAS_AUTONOMY_DOCTRINE.md` | North-star autonomy doctrine: goal-governed work, ATHENA knowledge sovereignty, and human escalation boundaries |
| `docs/ARCHITECTURE_OVERVIEW.md` | Architecture overview |
|| `docs/contracts/ARDA_ECOSYSTEM_STANDARD_INDEX.md` | Ecosystem standard index |
|| `docs/contracts/arda-ecosystem-standard-track-2-governance.md` | Governance/runtime contracts |

Read these **when you need architectural, identity, governance, or runtime context** — not on every turn.

---

## Navigation Indexes

| Surface | Path |
|---------|------|
| Operator entry | `AGENTS.md` |
| Low-token repository map | `docs/CODEMAP.md` |
| Machine-readable repository map | `docs/FILE_TREE.jsonl` |
| Documentation tree | `docs/DIRECTORY_INDEX.md`, `docs/FILE_TREE.jsonl` |
| Status snapshot | `ANNUNIMAS_SYSTEM_STATUS_REPORT.md` |
| Crate index | `core/INDEX.jsonl`, `crates/README.md` |
| Script/DX indexes | `config/INDEX.md`, `config/INDEX.jsonl` |

---

## Soterion Communication

| Sigil | Meaning |
|-------|---------|
| ∇ | Sovereignty / Command |
| ◈ | Evidence / Verification |
| ⚡ | JouleWork / Runtime cost |
| ♥ | Love Equation / Alignment |
| ↝ | Transition / Pivot |

---

## Live Surfaces

| Surface | Endpoint / Path |
|---------|-----------------|
| Charon router | standard service binds `0.0.0.0:5110`; operator-local probes usually use `http://127.0.0.1:5110` |
| Charon Unix socket | `/run/user/1000/annunimas/charon.sock` under the standard local service |
| Charon health scripts | `scripts/lib/charon_http.sh`, `scripts/check_charon_health.sh`, `scripts/check_charon_routes.sh`, `scripts/check_charon_inference.sh` |
| LiteLLM gateway / Anthropic bridge | operator-local gateway, commonly `127.0.0.1:4000` when enabled |
| Charon provider config | `config/charon.providers.toml` |
| Charon provider state projection | `core/state/charon_router.json` |
| Fleet config | `config/fleet.toml` |
| Runtime truth | `core/state/` |
| Autopilot/supervisor outputs | `data/prometheus/` |
| HADES automation outputs | `data/hades/` |
| Charon telemetry | `data/charon/` |
| Metrics exporter | local Annunimas metrics exporter commonly on `:9101` |
| Node exporter | local node exporter commonly on `:9100` |
| ARDA HUD | `/var/home/mythos/Eregion/arda-hud/`; final validation is native Tauri inside distrobox `lothlorien` |
| CITADEL avatar | `/var/home/mythos/Eregion/citadel-avatar/` |

Operational note: Charon's configured bind address and the URL used by health scripts are not the same concept. Use `scripts/lib/charon_http.sh` helpers for local probes so health checks can try the configured URL plus local bind aliases.

---

## If Asked "What Should the Next Agent Read First"

1. This file (`ARDA_ROOT_PROTOCOL.md`)
2. `AGENTS.md`
3. `docs/CODEMAP.md`
4. `ANNUNIMAS_SYSTEM_STATUS_REPORT.md`
5. `core/realm/arda.toml`
6. `core/realm/agents.toml`
7. `core/state/queue_summary.json`
8. `core/state/charon_router.json` when the question touches routing/provider health

For task queue questions, `core/state/queue_summary.json` comes before raw JSONL. For provider routing questions, `core/state/charon_router.json` is the first runtime posture source and static provider config is input only.

---

## Structural Notes

- `docs/governance/SOUL.md` and `docs/governance/AGENTS.md` are not present in the current tree; use `docs/SAFETY_MODEL.md`, `docs/contracts/`, and `human/` for current governance reading and human-facing doctrine.
- Charon provider health is runtime-owned. Do not hard-code `healthy = true/false` in `config/charon.providers.toml`; use `enabled = true/false` for routing participation.
- Systemd supervision policy belongs in `annunimas-prometheus`; execution is through user systemd and `scripts/agent_supervisor.sh`; `annunimas-systemd` is the typed query/control client.
- `active/exited` user-systemd units are usually successful one-shot/status surfaces, not failures. Timer-driven services normally appear `inactive/dead` between runs.
- Charon tool/code routes require tool-capable, non-visible-reasoning models at or above the configured context floor unless an explicit emergency low-context fallback flag is set.
- ARDA HUD native validation is Tauri inside distrobox `lothlorien`; host Vite/browser previews are not final proof for native WebKit/Tauri behavior.
- Queue ledger mutation must remain append-only. Before claiming a queue mutation is safe, run `scripts/check_task_queue_append_only.sh`.
