---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "implementation_plan"
  owner: "HADES"
  status: "active"
  reviewed: "2026-07-18"
---

# Arda Observability + Telemetry + Contracts Implementation Plan

> Goal: turn the ecosystem standard contracts into real wiring, starting from
> the closest live source modules, and adopt a maintainable OpenTelemetry + gRPC
> substrate on the existing Beelink observability stack.

## 1. Contracts at a glance

| Track | Doc | Contract intent | Closest live module | Current level | Gap to next level |
|---|---|---|---|---|---|
| T1 Observability | `arda-ecosystem-standard-track-1-observability.md` | Unified telemetry event schema + durable receipts | `crates/spine/observability/arda-aule/src/cli/export_surface/runtime_exports.rs` | `local_heuristic.local` / `runtime_receipted` for admission receipts | No global `arda telemetry` CLI verbs; no single event bus; no schema registry |
| T2 Governance | `arda-ecosystem-standard-track-2-governance.md` | Policy-as-code + typed verdict receipts | `crates/spine/governance/arda-governance/src/triad.rs`, `arda-core/src/loop_engine.rs` | `policy_enforced` for configured lanes; `runtime_receipted` elsewhere | No `arda governance policy/receipt` verbs exposed top-level; policy version/hash/provenance not emitted everywhere |
| T3 Agent Runtime/Tooling | `arda-ecosystem-standard-track-3-agent-runtime-tooling.md` | Service registration, tool manifests, supervisor policy | `crates/engine` (supervisor), `crates/spine/executors/arda-varda` | `runtime_receipted` for some supervisor state; `policy_enforced` missing | No service/tool manifest schema; no `arda service graph/tool manifest/runtime receipt` verbs |
| T4 Evaluation/Learning | `arda-ecosystem-standard-track-4-evaluation-learning.md` | Learning loop receipts + approval gates | `crates/spine/executors/arda-varda` docs/plans + `arda-core/src/loop_engine.rs` | `runtime_receipted` for lane receipts | No `arda eval/learning delta` verbs; explicit approval gate not implemented |

## 2. What "connected" means here

These markdown track docs are **not wired into runtime code today** outside of
partial scaffolding. They describe the desired surface, but:

- No crate imports or verifies them at build time.
- Evidence classes are documented as aspirational ladders, not enforced.
- CLI verbs referenced in the docs are not all implemented.

Therefore the plan is **incremental realization**, starting from what already
exists, then adding missing structure.

## 3. Telemetry substrate from grok research

`human/telemtry.md` proposes:

- **OpenTelemetry + OTLP** via `opentelemetry`, `opentelemetry-otlp` (`tonic`),
  `tracing-opentelemetry`.
- **Tonic (gRPC)** for clean typed request/response and streaming between agents.
- **Grafana Tempo + Loki + Prometheus** on Beelink for traces/logs/metrics.

Why this fits Arda:

- Arda already uses `tracing` / `tracing-subscriber`; OTel bridge plugs in.
- Manwe and arda-aule both have HTTP/SSE/IPC surfaces; gRPC gives a typed
  control plane for service-to-service calls and tool execution.
- Existing Beelink Grafana/Prometheus stack becomes a single observability stack
  with minimal new infra.

Recommended concrete additions:

- `opentelemetry`, `opentelemetry-otlp` with `tonic`, `tracing-opentelemetry`
  to workspace deps.
- Emit `gen_ai.*` semantic conventions for LLM/tool calls in:
  - `crates/spine/runtime/manwe/src/` proxy handlers
  - `crates/spine/runtime/arda-economics/src/service.rs`
  - `crates/spine/executors/arda-varda/src/ingest.rs`
- Service mesh for internal typed calls:
  - Proto schemas for `ManweRouteRequest`, `PlutusStatusRequest`,
    `GovernanceVerdictRequest`.
  - Generated Rust stubs in `crates/spine/interface/arda-orome` or a new
    `arda-transport` crate.
- Backends:
  - Prometheus: already running on Beelink.
  - Loki: add for aggregate logs/trace tails.
  - Tempo: add for Jaeger-compatible trace UI.

## 4. Implementation plan

### Phase A — Contract-to-module reconciliation

1. For each track doc, declare the exact source modules and evidence fields in
   a small registry file, e.g. `core/state/contract_registry.json`.
   Fields: `track_id`, `title`, `owner`, `status`, `source_modules[]`,
   `evidence_class_current`, `evidence_class_target`, `cli_verbs[]`.
2. Update each track doc to point to that registry as the live source of truth.
3. Add compile/runtime checks that at least one source module exists per track
   and that the expected schema version string is present.

### Phase B — Telemetry wiring

1. Add OTel deps to workspace.
2. Create `crates/spine/observability/arda-aule/src/telemetry/` with:
   - tracer initialization from env/config.
   - event emission helpers for:
     - `agent.<crate>.command`
     - `llm.call`
     - `governance.triad`
     - `router.route`
     - `system.supervisor`
     - `queue.event`
3. Instrument manwe, plutus, mandos, vaire, varda, auge, aule.
4. Export OTLP/gRPC to Beelink Tempo; logs to Loki.

### Phase C — CLI surface

Implement these in engine or auge CLI:

- `arda telemetry schema`
- `arda telemetry receipt <run_id>`
- `arda governance policy <policy_id>`
- `arda governance receipt <receipt_id>`
- `arda service graph`
- `arda tool manifest <agent_id>`
- `arda runtime receipt <run_id>`
- `arda eval run <task_id>`
- `arda learning delta <run_id>`

Each reads from the same durable receipt stores, so no data duplication.

### Phase D — gRPC typed surfaces

1. Define `.proto` files for service-to-service contracts.
2. Generate Rust stubs into `arda-orome` or a dedicated transport crate.
3. Wire manwe and supervisor to expose typed health/model endpoints over gRPC
   in addition to HTTP.
4. Replace ad-hoc JSON-over-HTTP internal paths with typed RPC where latency
   and schema stability matter.

### Phase E — Fleet-backed provider catalog (manwe)

Revisit `crates/spine/runtime/manwe/src/provider.rs`:

- Source of truth: `config/fleet.toml` nodes.
- Generate `ProviderDefinition` entries for each active node with fields:
  `id = charon_provider_id`, `base_url`, `model_id/runtime_model_alias`,
  `transport`, `capabilities`, `health_url`, `models_url`.
- Add `/providers/candidates` reconciliation against `/v1/models` for each node.
- Persist state to `data/prometheus/runtime_admission_shed_receipts.jsonl` and
  `core/state/fleet_bootstrap.json`.

## 5. Evidence rules

- Every schema/contract change must include:
  - doc update in `docs/contracts/`
  - at least one source module reference
  - one receipt-producing write path
  - one independent review receipt or smoke test

## 6. Stop conditions

- T1: logs/dashboards/receipts all show the same `arda.telemetry` schema.
- T2: every gatepass/denial writes an immutable receipt with policy version,
  policy hash, and actor.
- T3: new services register by schema, not systemd or bespoke scripts.
- T4: learning loop run is reproducible from receipt ledger alone.
