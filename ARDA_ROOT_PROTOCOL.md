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

# Arda Root Protocol

> ∇ Operator-only canonical pointer file. Tells operators and agents where doctrine, runtime truth, and navigation indexes live — does not require upfront reads. Sanitize local live-surface details before using this file in public support or issue contexts.
> Updated 2026-08-08 PDT. `~/Annunimas` references are reference-architecture only; canonical authority lives under this Arda workspace root unless otherwise marked.

---

## Current System Shape

| Surface | Current Posture |
|---------|-----------------|
| System | Arda personal agent ecosystem: one governed kernel with task-selected capabilities |
| Product doctrine | `docs/architecture/ARDA_1_0_PERSONAL_AGENT_ECOSYSTEM.md` |
| Runtime supervisor | canonical `arda` composition root, `services.toml`, and `arda-engine`; user systemd may supervise installed services |
| Router | Manwë is the optional model/provider-routing capability; the coordinated runtime contract currently uses `:7171` where enabled |
| Provider mesh | Hosted, local, and mixed lanes selected by task policy; use live Manwë/runtime probes rather than retired Charon projections |
| Queue posture | Read `core/state/queue_summary.json` before raw queue evidence |
| UI/device consumers | `apps/arda-hud` and `apps/arda-launcher` are canonical workspace applications; CITADEL remains an optional external presence consumer |
| Human/operator docs | `human/` lives at the Arda root and is now part of the canonical tree |
| Reference architecture | `~/Annunimas` is reference-only unless `AGENTS.md` or this file marks a path as canonical there |

Use `ARDA_SYSTEM_STATUS_REPORT.md` for the latest bounded workspace/runtime
snapshot. Re-probe live services before operational decisions; this root
protocol is a pointer map, not a live status ledger.

---

## Where Authority Lives

| Source | Contains |
|--------|----------|
| `AGENTS.md` | Operator/agent working rules for the current repository |
| `core/knowledge/realm/arda.toml` | Identity, laws, realms, Soterion basis |
| `core/knowledge/realm/agents.toml` | Sovereign roster, agent roles, authority |
| `config/manwe.providers.toml` | Manwë provider/model routing input; live runtime evidence still wins |
| `config/routing/model_route_matrix.toml` | Cross-provider model routing matrix |
| `config/fleet.toml` | Fleet node metadata and inference endpoints |
| `config/governance/autonomy_operating_loop.toml` | Draft portable lane/config contract for the autonomous operating loop; not live-execution proof |
| `core/state/queue_summary.json` | Compact task queue projection; read this before the raw ledger |
| `core/projects/tasks/queue.jsonl` | Active and historical append-only queued work; use for exact evidence or appends |
| `core/state/` | Machine-readable runtime projections and source maps |
| `data/` | Runtime receipts, ledgers, telemetry, service outputs |
| `docs/architecture/ARDA_1_0_PERSONAL_AGENT_ECOSYSTEM.md` | Arda 1.0 product doctrine, capability composition, personal-system and optional-capability boundaries |
| `docs/releases/0.9/BASELINE.md` | Current whole-system capability, evidence, limitation, and release baseline |
| `docs/contracts/ARDA_ECOSYSTEM_STANDARD_INDEX.md` | Ecosystem standard index |
| `docs/contracts/arda-ecosystem-standard-track-2-governance.md` | Governance/runtime contracts |

Read these **when you need architectural, identity, governance, or runtime context** — not on every turn.

---

## Navigation Indexes

| Surface | Path |
|---------|------|
| Operator entry | `AGENTS.md` |
| Low-token repository map | `docs/CODEMAP.md` |
| Machine-readable core index | `core/INDEX.jsonl` |
| Documentation inventory | live `docs/` tree plus active-plan links |
| Historical generated indexes | `docs/archive/DIRECTORY_INDEX.md`, `docs/archive/FILE_TREE.jsonl` |
| Status snapshot | `ARDA_SYSTEM_STATUS_REPORT.md` |
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
| Manwë runtime | coordinated service contract uses `:7171` when enabled; verify the current bind before probing or changing consumers |
| Manwë provider config | `config/manwe.providers.toml` |
| Manwë health/inference checks | `scripts/check_manwe_inference.sh`, `scripts/smoke_manwe_production.py` |
| Hosted-agent worker | Hermes execution adapter under `crates/engine/src/adapters/hermes.rs`; runtime configuration in `config/adapters/hermes-workbench.toml` |
| Fleet config | `config/fleet.toml` |
| Runtime truth | `core/state/` |
| Autopilot/supervisor outputs | `data/prometheus/` |
| HADES automation outputs | `data/hades/` |
| Model-routing telemetry | live Manwë/Aulë outputs; do not use retired Charon labels as current truth |
| Metrics exporter | local Annunimas metrics exporter commonly on `:9101` |
| Node exporter | local node exporter commonly on `:9100` |
| ARDA HUD | `apps/arda-hud`; final visual acceptance is native Tauri, not browser preview |
| CITADEL avatar | `/var/home/mythos/Eregion/citadel-avatar/` |

Operational note: preserve coordinated `:7171` consumer behavior when changing Manwë bind or transport settings. Probe live configuration and all consumers before altering that contract.

---

## If Asked "What Should the Next Agent Read First"

1. This file (`ARDA_ROOT_PROTOCOL.md`)
2. `AGENTS.md`
3. `docs/architecture/ARDA_1_0_PERSONAL_AGENT_ECOSYSTEM.md`
4. `docs/releases/0.9/BASELINE.md`
5. `docs/CODEMAP.md`
6. `core/knowledge/realm/arda.toml`
7. `core/knowledge/realm/agents.toml`
8. `core/state/queue_summary.json`
9. Live Manwë/Aulë evidence when the question touches routing/provider health

For task queue questions, `core/state/queue_summary.json` comes before raw JSONL. For provider-routing questions, live Manwë/Aulë evidence comes before generated projections; static provider configuration is input only.

---

## Structural Notes

- `docs/governance/SOUL.md`, `docs/governance/AGENTS.md`, and the former
  top-level safety/architecture pointers are not present in the current tree;
  use the product doctrine, `docs/contracts/`, and `human/` as applicable.
- Manwë provider health is runtime-owned. Do not present static provider configuration as current health.
- Runtime composition belongs to the root `arda` binary and `arda-engine`;
  installed user-systemd units are deployment supervision, not a separate
  product or execution authority.
- `active/exited` user-systemd units are usually successful one-shot/status surfaces, not failures. Timer-driven services normally appear `inactive/dead` between runs.
- Tool/code routes require tool-capable models at or above the configured context floor unless an explicit bounded fallback policy applies.
- ARDA HUD native validation is Tauri inside distrobox `lothlorien`; host Vite/browser previews are not final proof for native WebKit/Tauri behavior.
- Queue ledger mutation must remain append-only. Before claiming a queue mutation is safe, run `scripts/check_task_queue_append_only.sh`.
- Revenue/x402, council, local inference, health-device ingestion, and external
  adapters are task-selected capabilities. Their absence does not redefine or
  fail the base personal-agent ecosystem.
