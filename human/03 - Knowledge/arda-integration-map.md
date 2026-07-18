---
sigil: SCROLL
soterion:
  id: arda-integration-map
  version: 1.0.0
  classification: general-document
  author: Aulendil
  created: 2026-03-20
  last_edited: 2026-05-03
  status: active
  domain: general
  tags:
    - documentation
    - general
  mnemosyne:
    lineage: arda-integration-map-doc
    memory_type: general-knowledge
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

<!-- sigil: SCROLL -->
# ARDA Integration Map

ARDA_HUD should not read arbitrary files across the repo.

Primary rule:
- Prefer `/core/state/*.json` projections as the first read-model.
- Use `data/*.jsonl` feeds only for timelines, logs, and drill-down panels.
- Avoid binding UI sections directly to crate-local `data/` or `human/` mirrors.

Primary backend entrypoints for the future ARDA pass:
- `core/state/arda_source_map.json`
- `core/state/arda_snapshot.json`

Current section map:

## Sovereign World
- Primary:
  - `core/state/world.json`
  - `core/state/system_manifest.json`
- Purpose:
  - 3D world posture
  - system identity
  - realm and agent baseline

## Governance And Guardhouse
- Primary:
  - `core/state/warden_guardhouse.json`
  - `core/state/warden_policy_authority.json`
  - `core/state/warden_edge_contract.json`
  - `core/state/warden_nightly_doctrine.json`
- Supplemental:
  - `data/prometheus/gate_matrix_last.json`
  - `data/prometheus/gate_metrics_last.json`
- Purpose:
  - WARDEN edge visibility
  - policy posture
  - governance and validity indicators

## Routing And Communications
- Primary:
  - `core/state/charon_router.json`
  - `core/state/hermes_command.json`
- Supplemental:
  - `data/hermes/boardroom.jsonl`
  - `data/hermes/interruptions.jsonl`
- Purpose:
  - board room
  - routing health
  - operator interrupt flows

## Lifecycle Execution Economics
- Primary:
  - `core/state/hades_lifecycle.json`
  - `core/state/apollo_runtime.json`
  - `core/state/plutus_runtime.json`
- Supplemental:
  - `data/prometheus/health_workflow_last.json`
  - `data/prometheus/pressure_guard_last.json`
  - `data/hades/joulework.jsonl`
- Purpose:
  - maintenance
  - execution
  - JouleWork and budget state

## Knowledge And Reasoning
- Primary:
  - `core/state/oracle_runtime.json`
- Supplemental:
  - `data/athena/digest.jsonl`
  - `data/athena/deep_graph.jsonl`
  - `data/knowledge/athena/index/sources.jsonl`
  - `data/athena/policy_readiness.jsonl`
- Current gap:
  - ATHENA does not yet have a first-class `/core/state` runtime snapshot

## Memory And Continuity
- Primary:
  - `core/state/mnemosyne_continuity.json`
- Supplemental:
  - `data/mnemosyne/obsidian_index.jsonl`
  - `data/mnemosyne/noise.jsonl`

## Planning And Queue
- Primary:
  - `core/state/queue_summary.json`
- Supplemental:
  - `core/projects/tasks/queue.jsonl`
  - `core/projects/tasks/queue.jsonl`
  - `core/projects/Plans/`
  - `human/plans/`
  - `data/prometheus/orders.jsonl`
  - `data/prometheus/escalations.jsonl`

## Human Business Personal
- Primary:
  - `core/state/human_context.json`
- Supplemental:
  - `docs/`
  - `human/index.md`
  - `human/onboard.md`
  - `human/company_view.md`
  - `human/Notes/`
  - `human/arandur/`
  - `config/business.toml`
  - `data/business/soterion-business.json`
  - `data/personal/`
  - `core/personal/`

Backend work still needed before the ARDA pass:
- ATHENA `/core/state` snapshot

Readable plan root for ARDA and graph-oriented operator views:
- `human/plans/README.md`
- richer human/business/personal semantics once those sections are fleshed out


## See Also
- [agent-routing-contract.md](agent-routing-contract.md) - Related documentation
