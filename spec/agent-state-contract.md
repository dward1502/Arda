---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# Agent State Contract — v0.1

**Status:** Draft (v0.1, frozen for Phase 1)
**Owner:** arda-core
**Tracked in:** `crates/spine/governance/arda-core/src/contract/`
**Sigil:** ∇ ◈ ↝

---

## 1. Purpose

This document defines the canonical, versioned data shapes that every
Arda agent reads from and writes to. It is the single source of
truth Phase 1 (the autonomy loop) depends on. Without it, Planner,
Dispatcher, and Reflector cannot exchange state deterministically.

The contract is intentionally **minimal** for v0.1. Per the PRD's own
guidance: start with the smallest field set the loop needs. Extensions
go in v0.2 behind a version bump — never as silent additions.

A `CONTRACT_VERSION = "0.1.0"` constant lives in
`arda-core::contract` and is written into every persisted record
under the `contract_version` key. Readers MUST refuse records whose
major/minor version they do not recognize; a future v0.2 reader MAY
read v0.1 records by promoting absent fields to defaults.

## 2. Out of scope for v0.1

- Federation / multi-operator records (Decision §6.3 in the PRD).
- Provider-mesh telemetry shapes (lives in `arda-plutus` for now).
- The philosopher-corpus output taxonomy beyond what the Decision record
  needs to capture a Triad outcome — full corpus shapes land in Phase 2b.

## 3. Type catalogue

| Type            | Purpose                                                 | Persisted at                           |
|-----------------|---------------------------------------------------------|----------------------------------------|
| `Goal`          | Standing objective the planner derives plans from       | `core/state/goals/<id>.json`           |
| `Plan`          | Decomposition of a goal into ordered task intents       | `core/state/plans/<id>.json`           |
| `Task`          | Unit of work the dispatcher routes to an agent          | `core/projects/tasks/queue.jsonl`      |
| `Decision`      | Dispatch / governance decision with Triad + Love score  | `core/state/ledger/decisions.jsonl`    |
| `Reflection`    | Post-completion scoring of a task against plan intent   | `core/state/<task_id>.reflection.json`|
| `LedgerEntry`   | Append-only typed envelope around any of the above      | `core/state/ledger/<YYYY-MM-DD>.jsonl` |
| `MemoryRecord`  | Episodic or semantic memory unit (Mnemosyne)            | `core/state/memory/<kind>/<id>.json`   |

All timestamps are RFC3339 UTC. All ids are stable strings (UUIDv4 by
default; agents MAY use deterministic ids prefixed with their realm).

## 4. Schemas

### 4.1 Goal

```jsonc
{
  "contract_version": "0.1.0",
  "id": "goal_provider_mesh_health",
  "title": "Keep provider mesh healthy",
  "intent": "Detect and recover from provider failures within one tick.",
  "owner_agent": "prometheus",
  "status": "active",          // active | paused | achieved | abandoned
  "priority": "high",          // low | medium | high | critical
  "joule_budget_per_day": 200.0, // optional; null = unbounded for v0.1
  "created_at": "2026-05-03T12:00:00Z",
  "updated_at": "2026-05-03T12:00:00Z"
}
```

### 4.2 Plan

```jsonc
{
  "contract_version": "0.1.0",
  "id": "plan_20260503_mesh_health_001",
  "goal_id": "goal_provider_mesh_health",
  "summary": "Probe each provider, retire any failing two consecutive checks.",
  "steps": [
    { "intent": "probe_provider", "params": { "tier": "free" } },
    { "intent": "probe_provider", "params": { "tier": "paid" } },
    { "intent": "retire_failing", "params": {} }
  ],
  "status": "ready",            // draft | ready | dispatched | done | abandoned
  "lessons_consulted": ["lesson_edge_laptop_evening_unhealthy"],
  "created_at": "2026-05-03T12:01:00Z",
  "updated_at": "2026-05-03T12:01:00Z"
}
```

### 4.3 Task

The `Task` type already exists in `arda-core::task`. v0.1 of the
contract **re-exports** it unchanged from `contract::task` to minimise
churn. The tradeoff and migration path are recorded in
`docs/plans/FILE_LAYOUT.md`.

### 4.4 Decision

```jsonc
{
  "contract_version": "0.1.0",
  "id": "dec_20260503_dispatch_0001",
  "decided_at": "2026-05-03T12:02:00Z",
  "decision_class": "dispatch",     // dispatch | governance | budget | retire
  "subject_id": "tsk_...",          // task / plan / agent id this is about
  "options_considered": ["edge_core", "openrouter_free", "anthropic"],
  "chosen": "edge_core",
  "rationale": "Lowest joule estimate among Triad-passed options.",
  "triad": {
    "verdict": "pass",              // pass | conditional | fail
    "aurelius": { "verdict": "pass", "reason": null },
    "bacon":    { "verdict": "pass", "reason": null },
    "sun_tzu":  { "verdict": "pass", "reason": null }
  },
  "love_score": 0.71,
  "resonance":  0.83,
  "joule_estimate": 0.4
}
```

`triad.*.verdict` matches the existing `arda-governance::triad`
output. The Decision record is what makes the PRD's "every dispatch
must consult governance and ledger the consultation" requirement
auditable.

### 4.5 Reflection

```jsonc
{
  "contract_version": "0.1.0",
  "id": "ref_20260503_tsk_0001",
  "task_id": "tsk_...",
  "plan_id": "plan_...",
  "completed_at": "2026-05-03T12:05:30Z",
  "outcome": "success",             // success | partial | failure
  "score": 0.88,                    // 0..1, plan-intent alignment
  "narrative": "Probe completed; one provider retired as expected.",
  "joule_estimated": 0.4,
  "joule_actual": 0.37,
  "lessons_emitted": ["lesson_edge_core_cheap_for_probes"]
}
```

`joule_estimated` vs `joule_actual` is the input the existing
`joulework::honesty_ratio` consumes. Reflections are the canonical
place that ratio is recorded.

### 4.6 LedgerEntry

```jsonc
{
  "contract_version": "0.1.0",
  "id": "led_...",
  "ts": "2026-05-03T12:02:00Z",
  "agent": "supervisor",
  "kind": "decision",               // goal | plan | task | decision | reflection | memory | other
  "payload": { /* one of the above shapes, or arbitrary for `other` */ }
}
```

`LedgerEntry` is an envelope — the existing `arda-core::ledger`
writer already accepts `Serialize` values; the contract type is what
Phase 1 code SHOULD wrap structured records in so a reader can route
on `kind` without sniffing fields.

### 4.7 MemoryRecord

```jsonc
{
  "contract_version": "0.1.0",
  "id": "mem_lesson_edge_laptop_evening",
  "kind": "semantic",               // episodic | semantic
  "agent": "mnemosyne",
  "content": "edge_laptop unhealthy after 18:00 local — prefer edge_core.",
  "salience": 0.6,                  // 0..1
  "evidence_count": 3,
  "state": "active",                // active | decayed | revoked | promoted
  "created_at": "2026-04-29T19:00:00Z",
  "last_seen_at": "2026-05-02T19:14:00Z"
}
```

Episodic vs semantic split is a Phase 4 concern; v0.1 records the
distinction so we don't have to migrate the field later.

## 5. Versioning

- `CONTRACT_VERSION` follows semver. Field additions with safe defaults
  bump the minor; renames or removals bump the major.
- Every persisted record carries `contract_version`. Code reading state
  MUST check it; mismatched-major reads are an error, mismatched-minor
  reads are tolerated with defaults.
- `arda-cli state validate` (Phase 0 follow-up) walks the on-disk
  state and exits non-zero if any record fails its declared version's
  schema.

## 6. Compatibility shims

Existing per-agent runtime dirs (`data/charon/`, `data/hermes/`, …)
are NOT moved by v0.1. The migration to `data/state/` with per-agent
indices is the second Phase 0 deliverable, after this contract is
frozen and the core types compile against it.

## 7. v0.2 backlog — Human Projections

Out of scope for v0.1. Tracked here so the next contract bump has the
design in writing rather than re-inventing it ad-hoc.

### 7.1 Motivation

The system holds two audiences for the same facts:

1. **Agents** — read structured records (`Goal`, `Plan`, `Decision`,
   `MemoryRecord`, …) at machine cadence. Need exact fields, stable
   shape, append-only or single-record-per-file layout.
2. **The operator** — needs the *same* facts in a condensed,
   scannable form: an at-a-glance markdown table of active goals, a
   one-line digest per recent decision, a weekly summary of memory
   formation. The full machine record is too noisy to read directly.

The earlier ad-hoc mirrors (`data/knowledge/triage_registry.jsonl`
1:1 copy; `data/knowledge/arda_knowledge_map.json` flat snapshot)
tried to fill this gap and failed for the same two reasons every time:
no structure (each crate invented its own mirror format) and no
regenerator (the "mirror" drifted from canonical the first time the
canonical was rewritten by anyone other than the original author).

### 7.2 Proposed shape — `Projection`

A `Projection` is **not** a new persisted contract type. It is a
declared *derivation* attached to an existing record type, with three
required attributes:

```jsonc
{
  "source_record_type": "Decision",      // one of the §3 types
  "view_path": "human/decisions/<YYYY-MM-DD>.md",  // where the human view lives
  "format": "markdown_table",            // markdown_table | markdown_digest | jsonc_pretty | csv
  "regenerator": "arda-hermes::projection::decisions",  // module path of the writer
  "regenerated_on": "every_write"        // every_write | hourly | daily
}
```

Three **invariants** the contract enforces:

- **Same writer.** The crate that writes the canonical record is the
  crate that regenerates the projection. No separate sync job, no
  "occasional refresh" cron. If you can't write the canonical without
  also writing the view, drift is impossible.
- **One direction.** Agents NEVER read from `view_path`. Projections
  are write-only from the system's perspective, read-only from the
  human's. An agent that needs the data reads the canonical record.
- **Declared, not discovered.** Every projection is registered in
  `core/state/projections.json`. `arda-cli state validate`
  walks the registry and verifies (a) every declared regenerator
  exists, (b) every `view_path` is reachable, (c) no two projections
  declare the same `view_path`.

### 7.3 Where views live

- All projections write under `human/` (the existing human-facing
  tree), never under `data/<agent>/` or `core/state/`.
- One subdirectory per source type:
  `human/goals/`, `human/decisions/`, `human/memory/`, `human/plans/`.
- Naming convention: dated digests (`<YYYY-MM-DD>.md`) for
  append-style records (Decision, LedgerEntry, MemoryRecord),
  one-file-per-record (`<id>.md`) for record-style (Goal, Plan).

### 7.4 What this is *not*

- Not an inverse for the contract. The view does not need to be
  parseable back into the canonical record. It can drop fields,
  re-order, group by date, summarize across records.
- Not federation. ARDA-HUD and similar UIs continue to read the
  canonical records directly; projections are for plain-text human
  consumption (editor, terminal, obsidian) without a UI in the loop.
- Not a replacement for ARDA-HUD. The HUD is the rich/interactive
  read surface; projections are the boring/durable one.

### 7.5 Migration of existing ad-hoc mirrors

When v0.2 lands, any surviving ad-hoc mirror MUST either:
- be re-modeled as a declared `Projection` with a registered
  regenerator, OR
- be deleted as a flat duplicate.

The Phase 0 audit (`docs/plans/PHASE_0_DATA_AUDIT.md`) is the
inventory; v0.2 closes the loop.

### 7.6 v0.2 — `Decision.gate_used: GovernanceGate`

Phase 2 (philosopher-chain refactor) introduces the `GovernanceCorpus`
trait and the `GovernanceGate` enum. The Phase 0 stub lives at
`crates/spine/runtime/governance/arda-core/src/governance/corpus.rs` (trait surface,
six-variant enum: `None | Regex | Single | Triad | Chain | Corporate`).

When v0.2 lands, `Decision` gains a required field:

```rust
pub gate_used: GovernanceGate,
```

This makes the validation cost model legible at the record level: a
`Decision` produced by a regex gate is materially cheaper than one
produced by the full `Chain`, and the `GateSelector` (Phase 3)
needs that history to learn its routing thresholds.

Pairs with: `human/03-Knowledge/plans/rust-tips-incorporation-plan.md`
§3 (GateSelector + Corporate Corpus pattern) and §5.2 (the Phase 0
stub commit that names the types).

### 7.7 v0.2 forward-compat — `extensions` already in v0.1

The six v0.1 record types (`Goal`, `Plan`, `Decision`, `Reflection`,
`MemoryRecord`, `LedgerEntry`) carry an `extensions:
HashMap<String, serde_json::Value>` field flattened via
`#[serde(flatten)]`. Unknown fields on read land in `extensions`
rather than failing deserialization; serializers preserve them.

This is not a v0.2 backlog item — it is a v0.1 forward-compat
mechanism intended to keep v0.2 additions from triggering schema
churn. It does **not** change v0.1's contract: readers MUST still
reject mismatched majors. `extensions` is a soft channel for
ride-along data inside the same major.

## 8. Change log

- **v0.1.0 — 2026-05-03:** initial draft. Frozen for Phase 1 wiring.
- **v0.1.0 — 2026-05-03 (addendum):** §7 v0.2 backlog added —
  `Projection` pattern for canonical-machine + condensed-human view,
  written as a response to the ad-hoc mirror failures discovered
  during the Phase 0 data audit. No code or schema change in v0.1.
- **v0.1.0 — 2026-05-06 (addendum):** §7.6 added — `Decision.gate_used`
  with the `GovernanceCorpus` / `GovernanceGate` Phase 0 stub at
  `crates/spine/governance/arda-core/src/governance/corpus.rs`. §7.7 added —
  `extensions` flatten field on the six record types (forward-compat
  inside v0.1; not a contract bump).
