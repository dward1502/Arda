\
---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-21"
crate: openfang
owner: prometheus
status: active
reviewed: "2026-07-21"
---

> Arda OpenFang: 📜 comparative-architecture extraction | owner: prometheus | status: active | reviewed: 2026-07-21

# OpenFang Plan Narrative

`OPENFANG` is the current Arda comparative-architecture extraction surface.
Historic narration is preserved at
`docs/plans/OPENFANG.md`. This document merges
the prior operator narrative with the current Arda surface names so old detail is
retained without stale crate assumptions. The active queue packet remains
`tsk_20260619_queue_openfang_plan_review` / `Queue: OPENFANG plan review`.

## Overview

OPENFANG is the Arda comparative-architecture plan surface for extracting useful OpenFang patterns into sovereign Arda contracts without adopting OpenFang as a replacement system.

## Core Runtime Surfaces

The reviewed OPENFANG contract is represented by these primary surfaces:

 - `docs/plans/OPENFANG.md` — operator-facing narrative and quick reference
 - `core/state/openfang_alignment.json` — policy-ready OpenFang pattern extraction and Arda adaptation map
 - `core/state/crate_spawn_contract.json` — crate-spawn contract derived from OpenFang-style capability packaging
 - `core/state/crate_spawn_blueprint_contract.json` — crate-spawn blueprint contract and required sovereign hooks
 - `core/state/network_native_node_onboarding_contract.json` — network-native node onboarding contract aligned with OpenFang onboarding/mutual-auth patterns
 - `data/athena/books/src_cebb6abe.jsonl` — ATHENA ingest evidence for `https://github.com/RightNow-AI/openfang`
 - `core/state/queue.jsonl` — append-only task ledger and closeout target

## Current Contract

OPENFANG currently owns:

1. **Comparative architecture extraction**: treat OpenFang as a source of useful architecture patterns, not as a replacement control plane.
2. **Crate-native decomposition**: adapt autonomous "Hands" / capability packages into Arda crate-spawn templates with sovereign hooks from first boot.
3. **Network-native onboarding**: carry forward onboarding and mutual-auth ideas while preserving Arda local socket authority and Tailscale-as-internal-mesh doctrine.
4. **Security-by-default runtime posture**: map signing, taint tracking, sandboxing, audit trails, receipts, and signatures onto WARDEN, HADES, `.aipkg`, and control-plane lockdown surfaces.
5. **Marketplace boundary discipline**: keep reusable package law and `.aipkg` open-standard contracts separate from marketplace economics and out of core truth/hot paths.
6. **Embodiment pattern reuse**: carry Tauri/native desktop embodiment forward into ARDA embodied interface and Pepper's Ghost controller surfaces where it supports operator experience.

## Observed Runtime / Plan State

The inspected surfaces show OPENFANG is present and policy-ready as a pattern-extraction surface:

- `docs/plans/OPENFANG.md` exists as the operator-facing narrative and references the live state surfaces directly.
- `core/state/openfang_alignment.json` exists with schema `arda.openfang.alignment.v1`, source id `src_cebb6abe`, and source URL `https://github.com/RightNow-AI/openfang`.
- ATHENA ingest evidence exists at `data/athena/books/src_cebb6abe.jsonl`; the deep record reports `policy_readiness: policy_ready`, triad pass, hash-chain/citation checks, and confidence above the policy threshold.
- `core/state/crate_spawn_blueprint_contract.json` exists with authority `openfang_pattern_extraction + arda_spawn_law` and requires core/governance dependencies, ARDA visibility, state/metrics paths, and governance gates.
- `core/state/network_native_node_onboarding_contract.json` exists with local socket authority retained, Tailscale as internal mesh, identity binding before role promotion, and operator confirmation required for stale identity cleanup.
- Search evidence found no existing `docs/plans/OPENFANG.md` prior to this review, so this narrative fills the missing human/operator artifact.

## Accepted Patterns

### Crate-spawn / capability packaging

OpenFang's autonomous Hands map cleanly to Arda crate-spawn templates. Arda adaptation keeps new crates wired to sovereign task execution, metrics, ARDA visibility, governance checks, and control-plane boundaries from first boot.

### Network-native node onboarding

OpenFang's network-native onboarding and mutual-auth posture maps to Arda fleet onboarding above the local runtime layer. Unix sockets remain local; Tailscale remains an internal mesh transport; role promotion requires identity binding plus live informant/runtime evidence.

### Security stack

OpenFang's signing, taint tracking, sandboxing, and audit-trail emphasis maps to WARDEN, HADES, `.aipkg` receipts/signatures, and lockdown projections.

### Desktop embodiment

OpenFang's Tauri desktop emphasis remains useful for ARDA embodied interface and Pepper's Ghost controller surfaces, but only as an operator/device experience pattern, not as core authority.

## Rejected / Bounded Patterns

- **WhatsApp-first control path** is not adopted as Arda core doctrine.
- **Marketplace in the hot path** is rejected; package law, receipts, and attestations remain separate from economics.
- **Single distribution surface bias** is rejected; Arda keeps sovereign local, internal mesh, and package/runtime boundaries explicit.

## Follow-up Work

1. **Crate-spawn blueprint contract hardening**
   - Keep `core/state/crate_spawn_blueprint_contract.json` aligned with workspace boundary gates, governance requirements, and ARDA/metrics/state requirements.

2. **Network-native onboarding reconciliation**
   - Resolve operator-confirmation-required identity binding items in `core/state/network_native_node_onboarding_contract.json` before role promotion.

3. **AIPKG marketplace separation**
   - Preserve `.aipkg` as open package law and keep marketplace/economic surfaces outside core truth.

4. **ARDA embodiment slice**
   - Reuse only bounded desktop embodiment patterns that support operator experience without expanding core authority.

## Verification Commands

Useful focused checks for this plan surface:

```bash
python -m json.tool core/state/openfang_alignment.json >/dev/null
python -m json.tool core/state/crate_spawn_blueprint_contract.json >/dev/null
python -m json.tool core/state/network_native_node_onboarding_contract.json >/dev/null
test -f docs/plans/OPENFANG.md
test -f data/athena/books/src_cebb6abe.jsonl
scripts/check_task_queue_append_only.sh
```

Refresh projection evidence before closeout or next active queue selection:

```bash
cargo run -p arda-cli -- export queue-hygiene
```

## Alignment with Arda Principles

- **Evidence-first extraction:** accepted patterns are grounded in ATHENA source evidence and runtime contract files.
- **Sovereign local authority:** network-native patterns do not replace local Unix socket authority or operator identity gates.
- **Append-only truth:** queue closeout is recorded through same-id terminal records rather than raw ledger rewrites.
- **Marketplace separation:** reusable package law remains separate from marketplace economics.
- **Governance gates:** Triad, Bacon-lite, JouleWork, and Love gates remain required for production adoption.

## Open Questions

1. Should the crate-spawn blueprint contract be promoted into a dedicated versioned schema under `spec/`?
2. Should network-native onboarding produce a separate operator approval packet for stale identity cleanup?
3. Should ARDA embodiment reuse be deferred until server/GPU-gated visual spikes provide runtime evidence?

## References

- Crate/surface: `docs/plans/OPENFANG.md`
- Original narrative: `docs/plans/OPENFANG.md`
- `docs/plans/OPENFANG.md` — operator-facing narrative and quick reference
- OpenFang alignment contract: `core/state/openfang_alignment.json`
- Crate-spawn blueprint: `core/state/crate_spawn_blueprint_contract.json`
- Network-native onboarding contract: `core/state/network_native_node_onboarding_contract.json`
- ATHENA source evidence: `data/athena/books/src_cebb6abe.jsonl`
