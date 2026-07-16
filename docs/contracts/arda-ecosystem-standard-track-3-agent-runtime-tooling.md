---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "contract"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-07-13"
---

> 🜏 Soterion: 📜 contract | owner: PROMETHEUS | status: active | reviewed: 2026-07-13

# Agent Runtime and Tooling Contract

This contract defines Arda agent runtime behavior and tool execution boundaries.

## Scope

Arda remains one CLI binary with typed subcommands.
Personality and capability are identity metadata.
Runtime identity is `crate + agent_id`, not process group.

## Service boundary

Each service crate exposes a standard trait for IPC.
Supervisor manages bounded async workers, not an unbounded process list.
New services join by schema, not by bespoke systemd coupling.

## Tool manifest

Each agent has a stable tool manifest.
Tools are executed via MCP or `arda-tool-harness`.
Manifests must declare:
- `tool_id`
- `agent_id`
- `capabilities[]`
- `clearance`
- `timeout_ms`
- `retry_policy`
- `schema_version`

## Entity schemas

Service registration record:
- `service_id`
- `crate`
- `agent_id`
- `status`
- `started_at`
- `last_heartbeat`
- `supervisor_policy`

Tool manifest record:
- `manifest_id`
- `agent_id`
- `crate`
- `tools[]`
- `policy_version`
- `hash`

## Evidence classes

| evidence class | meaning |
|---------------|---------|
| `documentation` | trait schema and manifest schema documented |
| `local_heuristic` | one non-CLI crate implements service trait |
| `source_metadata` | supervisor emits typed service registration records |
| `runtime_receipts` | `data/prometheus/`, `core/state/` contain durable service/worker state |
| `policy_enforcement` | worker lifecycle is gated by supervisor policy, not ad hoc restarts |
| `independent_review_receipts` | independent checks validate service graph/health |
| `scoped_autonomy_policy` | named agent scope has explicit manifest and runtime policy |

## Current default projection

Expected current level: `runtime_receipted` for existing supervisor output; system-wide `policy_enforced` after one bounded-worker crate migration.

## CLI behavior

- `arda service graph` prints typed service graph, cache recent snapshot without network I/O.
- `arda tool manifest <agent_id>` prints accepted tool manifest.
- `arda runtime receipt <run_id>` prints service/tool execution receipt bundle.

## Stop condition

New service joins by schema, not systemd or shell script changes.

---
---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "contract"
  owner: "PLUTUS"
  status: "active"
  reviewed: "2026-07-13"
---
