---
soterion:
  sigil: "SCROLL"
  glyph: "🧠"
  code_point: "U+1F9E0"
  role: "memory_architecture"
  owner: "HADES"
  status: "draft"
  last_reviewed: "2026-07-17"
---

# Vairë Implementation Plan

Memory tiering, folder architecture, and migration roadmap for Arda.

Status: planning only. No code changes, no file moves until this plan is approved.

## Current state

- `core/state/` is a flat namespace with ~200 JSON files.
- `core/metrics/` duplicates some state under `by_crate/` plus timestamped `history/` snapshots.
- `queue/`, `knowledge/`, `projects/`, `edge/`, `clients/`, `personal/`, `realm/` are adjacent domains with unclear ownership.
- No databases. No tiering. No explicit read/write contracts.
- `arda-vaire` crate exists at `crates/memory/arda-vaire`.

## Desired architecture

### Memory-owned paths

```
core/
├── state/
│   ├── runtime/                    # Tier 1 hot state
│   │   ├── settings.json
│   │   ├── topology.json
│   │   ├── budget_policy.json
│   │   ├── admission_receipts.json
│   │   └── ...
│   ├── operational/                # Tier 2 warm state
│   │   ├── task_lifecycle.jsonl
│   │   ├── queue/                  # queue domain files
│   │   ├── plans/                  # only active/live plans
│   │   ├── ledger/                 # ledger data
│   │   └── outcomes/               # ephemeral runtime outcome records
│   ├── knowledge/                  # Tier 3 cool state
│   │   ├── triage_registry.jsonl
│   │   ├── source_inventory.jsonl
│   │   ├── athena/                 # athena-specific indexes
│   │   └── graphs/                  # Graphthulhu-style graph indexes
│   └── vaults/                     # Tier 4 cold/external
│       ├── clients/
│       └── secure/
├── metrics/                        # separate from memory tiers
│   ├── current/
│   └── history/
└── README.md

archive/
└── core_state/                     # retired state from core/state/_archive and planned cleanup
```

### App/config-owned paths

```
config/
├── runtime/
├── routing/
├── governance/
├── business/
├── integrations/
├── monitoring/
├── systemd/
└── env/

apps/
├── arda-launcher/
└── arda-hud/
    └── settings.json               # app-local config, not repo root config/
```

### Crate-owned paths

```
crates/
├── memory/
│   └── arda-vaire/                 # memory abstraction layer
├── spine/                          # runtime crates
│   ├── arda-core/
│   ├── arda-engine/
│   ├── arda-mandos/
│   └── ...
└── apps/
```

### Data/artifacts

```
data/
├── charon/
├── prometheus/
├── assets/
│   ├── generated/
│   ├── temp/
│   └── cache/
└── ...
```

## Vairë crate responsibilities

`crates/memory/arda-vaire` owns:
- `MemoryTier` enum: `Working`, `Operational`, `Knowledge`, `Archive`
- `MemoryPath` contract: canonical paths for each tier/domain
- `MemoryStore` trait: load/save/append/compact with tier-aware semantics
- Tier metadata for each known state file so consumers can ask “is this hot?”
- Default JSON/JSONL backend that matches current filesystem layout exactly
- Future backend swap capability without changing consumer code

Non-responsibilities:
- Vairë does not own `metrics/`, `config/`, `queue/queue.jsonl` semantics, or app-local settings.
- Vairë does not split `core/state/` directories in phase 1; it just maps existing files to tiers.

## Migration principles

1. Consumer-first: every existing consumer must keep working during migration.
2. Layout-first, backend-later: enforce tier boundaries with directories and structs before introducing any database.
3. No duplication: one canonical copy per state entity; retire legacy copies explicitly into `archive/`.
4. Incremental: each phase is a complete, reviewable slice; consumers can adopt Vairë APIs one domain at a time.
5. Preserve history: old state files are moved, not deleted; history is queryable.

## Phases

### Phase 1 — Map and document

- Vairë defines `MemoryTier` + `MemoryPath` manifest for every current file under `core/state/`.
- Every file gets a tier assignment and a “canonical path” target.
- Produce the architecture map above as a living doc; update as decisions change.
- No file moves. No Rust API changes beyond Vairë internal types.

### Phase 2 — Vairë trait + JSON backend

- Implement `MemoryStore` trait with local JSON/JSONL backend.
- Vairë APIs used by new code paths first; existing direct-file paths remain untouched.
- Add tier metadata to Vairë manifest; tooling can validate “this path is hot, load eagerly” vs “this path is cool, load on demand.”

### Phase 3 — Domain migration

- Migrate one domain at a time into Vairë-backed paths:
  1. `runtime_settings.json`, `runtime_topology.json`, `runtime_budget_policy.json`
  2. `fleet_health.json`, `provider_intelligence.json`, `queue_summary.json`
  3. `task_lifecycle_runtime.json`, `runtime_admission_receipts.json`
  4. `queue/*.jsonl`
  5. `plans/` cleanup: retire failed automation-loop artifacts, keep only live plans
- Each domain migration is a separate PR with tests and BREAKDOWN updates.

### Phase 4 — Retire noise and archive

- Remove `state/tick_output_*.txt`.
- Deduplicate or compress `metrics/history/*`.
- Archive `state/_archive/*` into `archive/core_state/`.
- Retire stale Annunimas-era duplicate batches after confirming live consumers.

### Phase 5 — Backend swap readiness

- Once all primary domains use `MemoryStore`, new backends can be introduced without consumer changes:
  - Redis/Dragonfly for Tier 1 hot cache with TTL
  - SQLite or sled for Tier 2 operational JSONL if append-only guarantees are needed
  - External vault bindings for Tier 4 client data
- This phase is optional and future; the Rust trait should already support it.

## Consumer impact summary

- `arda-engine`, `arda-mandos`, `arda-hud`, `manwe`, launcher, and scripts all read/write under `core/state/` today.
- Phase 1 is invisible to consumers; it is only Vairë internal mapping.
- Phase 2 adds new APIs alongside old ones; no breakage.
- Phase 3 is where consumers start switching; each domain is independent.
- Phase 4/5 are cleanup and optional optimization.

## Verification

- `cargo check -p arda-vaire` green after each phase.
- Existing app workflows still load; no hard failures in HUD/engine/mandos.
- `BREAKDOWN.md` updated at each domain boundary after migration.
