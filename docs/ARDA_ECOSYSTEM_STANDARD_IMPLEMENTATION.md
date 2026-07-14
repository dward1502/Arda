---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  reviewed: "2026-07-13"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-07-13

# Arda — Ecosystem Standard Implementation

> Purpose: concrete, operator-style rollout from current Annunimas layout.
> Scope: Arda/Annunimas as an identity-first autonomous control plane with governance + observability.

---

## 1. Observability

Goal: standard telemetry transport + Arda-specific formatted views on top.

Tracks
- Add OTel as a transport layer: tracing, structured logging, metrics instrumentation points in CLI + service crates.
- Canonical event names: `agent.<crate>.command`, `llm.call`, `governance.triad`, `router.route`, `system.supervisor`, `queue.event`.
- Keep current views: `queue_observability_snapshot`, `format_operations_briefing_text`, `build_governance_observation`.
- Persist evidence bundles for receipts in `data/telemetry/`, gated so telemetry writes do not affect command latency.

Arda touchpoints
- `crates/annunimas-cli/src/observability.rs`
- `core/metrics/`, `data/prometheus/`, `config/monitoring-setup/`
- Receipt witnesses in `annunimas-cli` policy/observability surfaces
- Per-receipt review gates in `annunimas-warden`

Stop condition
- Logs and dashboards reflect the same event schema as in-process receipts and persisted bundles.

---

## 2. Governance

Goal: versioned policy-as-code, still expressible through your triad/resonance model.

Tracks
- Convert policy grants from prose+toml to typed contracts: actor, capabilities, clearance, resource actions.
- Verdict engine emits `allow|deny|redirect` with `reasons[]`, `actor`, `rule_id`, `timestamp`, `hash`.
- Signature/resonance remains as trust metadata, not programmatic truth.
- Human override remains first-class: immutable operator root with auditable intent + provenance.

Arda touchpoints
- `core/realm/annunimas.toml`, `core/realm/agents.toml`, `core/realm/boot.toml`
- `docs/contracts/`, `crates/annunimas-governance/`, `crates/annunimas-oracle`
- Machine-readable INDEX/state snapshots for runtime reads

Stop condition
- Every gatepass or denial writes an immutable contract receipt tied to an exact policy version and actor.

---

## 3. Agent Runtime / Tooling

Goal: shared typed runtime with bounded lifecycle, less process sprawl.

Tracks
- Supervisor manages bounded async workers, not an unbounded process list.
- Each service crate exposes a standard service trait for IPC.
- Stable tool manifest per agent; execution via MCP or `annunimas-tool-harness`.
- Personality, capability, and clearance are identity metadata; runtime identity is typed `crate + agent_id`.

Arda touchpoints
- `crates/annunimas-prometheus` + `crates/annunimas-systemd`
- `scripts/agent_supervisor.sh`
- `crates/annunimas-mcp`, `crates/annunimas-tool-harness`
- `core/realm/agents.toml` capability blocks

Stop condition
- One CLI/binary dashboard; new service joins by schema, not systemd coupling.

---

## 4. Evaluation / Learning Loop

Goal: standardized eval tasks with immutable receipts instead of one-off customs.

Tracks
- First-class eval harness for smoke and task-quality checks.
- Learning loop emits receipts: observation, reward, next policy delta, provenance.
- Human approval remains explicit when divergence exceeds bounded thresholds.
- Receipts feed treasury/reward models rather than ad hoc bonuses.

Arda touchpoints
- `docs/contracts/autonomous-operating-loop-contract.md`
- `crates/annunimas-core` task/ledger contracts
- `data/prometheus/`, `core/projects/tasks/queue.jsonl`
- lanes: HADES lifecycle, ATHENA source ledgers, Oracle validation, Hermes confirmation, Plutus JouleWork

Stop condition
- Loop run is reproducible from receipt ledger alone.

---

## Minimal execution order

1. Observability transport + shared event schema
2. Governance contract schema + one `policy.rs` reader in `annunimas-core`
3. Service trait boundary for one non-CLI crate
4. Eval receipt format + one pilot loop run with uploaded receipts

---
---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "implementation_guide"
  owner: "HADES"
  status: "active"
  reviewed: "2026-07-13"
---
