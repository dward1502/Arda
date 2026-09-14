---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "execution_map"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-08-21"
---

> 🜏 Soterion: 📜 execution_map | owner: PROMETHEUS | status: active | reviewed: 2026-08-21

# Digital Organism Workstreams and Integration Order

## Execution rule

Stages execute sequentially. Within a stage, parallel work is allowed only after shared contracts and exclusive file ownership are frozen. Direct implementation is the default; do not delegate coding unless the operator explicitly asks.

## Ownership lanes

| Lane | Existing owner | Primary paths | Must not own |
|---|---|---|---|
| Organism identity/tasks/contracts | `arda-core` | `crates/spine/governance/arda-core/**` | transport implementations, UI state |
| Semantic messaging/operator bridge | Oromë | `crates/spine/interface/arda-orome/**` | model placement, platform credentials |
| A2A transport/runtime adapter | Hermes plugin/A2A integration | repository adapter source + Hermes plugin install | Arda semantic authority, canonical queue |
| Node/fleet/outpost identity | engine/outpost contracts | `crates/engine/**`, `outposts/**`, `config/fleet.toml` | conversational sessions |
| Provider/model routing | Manwë | `crates/spine/runtime/manwe/**`, provider config | task approval, node identity |
| Durable worker execution | engine | `crates/engine/**` | planning/council conclusions |
| Memory/context | Vairë | `crates/spine/memory/arda-vaire/**` | evidence truth, second memory store |
| Evidence/research | Varda | `crates/spine/executors/arda-varda/**` | commitments and approvals |
| Governance | `arda-governance` | `crates/spine/governance/arda-governance/**` | platform transport, model voting authority |
| Executive composition | Prometheus/Arandur via Aulë | `crates/spine/observability/arda-aule/**` | parallel queue/memory/router |
| Projections | Aulë/HUD/RELIC | `arda-aule`, `apps/arda-hud/**`, `outposts/arda-relic-bridge/**` | minting health, work, approval, or receipts |

## Shared-file serialization

Only one active packet may edit each of these at a time:

- root `Cargo.toml` and `Cargo.lock`;
- `src/main.rs` and `services.toml`;
- shared contract registry/state schemas;
- `config/fleet.toml` and provider catalogs;
- canonical queue ledger and approval contracts;
- Hermes plugin manifest/hook registrations;
- active portfolio and master program docs.

Generated projections under `core/state/queue_active.json` and `core/state/queue_summary.json` remain unstaged. Current research/runtime ledgers and unrelated HUD edits are preserved.

## Stage integration gates

1. Specification review: task matches the stage’s literal behavior and authority boundaries.
2. TDD/focused tests: contract and failure behavior proven before implementation expansion.
3. Root composition review: feature appears in the actual default/production closure.
4. Installed/runtime review: owning artifact and process are current.
5. Cross-boundary proof: real transport/process/node path, not fixtures alone.
6. Operational-truth review: UI/status/receipts agree and stale states degrade.
7. Operator acceptance where the stage requires human-visible usefulness.

## Branch and commit discipline

- One stage branch at a time from the current canonical branch unless the operator directs otherwise.
- Use worktrees only for explicitly approved parallel packets with disjoint paths.
- Commit focused source/tests/docs; never `git add .` in the dirty canonical worktree.
- No push unless explicitly requested.
- Every task closeout names changed paths, tests, runtime proof, remaining maturity state, and next stage gate.

## Queue strategy

The canonical task ledger should contain ordered stage-level tasks with `plan_path`, `sequence`, `depends_on`, and acceptance metadata. Later-stage tasks remain dependency-gated and non-executable until their predecessor has an accepted terminal receipt. Detailed sub-packets are derived from the stage plan when that stage opens, preventing a large stale queue from becoming false active backlog.

## Stop conditions

Stop and re-plan when:

- a proposed contract duplicates a live owner;
- only tests/docs can demonstrate the result;
- a task requires manually copying state between nodes;
- the implementation bypasses Hermes/Arda public integration surfaces;
- a later-stage feature is being used to compensate for an unproven earlier stage;
- a model, Bot, council, or UI is about to become canonical authority;
- worktree contamination makes a focused commit unverifiable.
