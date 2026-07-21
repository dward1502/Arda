---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "memory_continuity"
  owner: "ARDA-VAIRE"
  status: "active"
  last_reviewed: "2026-07-21"
crate: arda-vaire
agent: mnemosyne
realm: memory
sigil: "𓁿"
status: operational
---

> Arda-VAIRE: 📜 memory continuity | owner: arda-vaire | status: operational | reviewed: 2026-07-21

# arda-vaire Plan Narrative

## Name / Identity

`MNEMOSYNE` is now implemented in `crates/spine/memory/arda-vaire`. This
document is the canonical operator plan for that memory surface. Historic
narration is preserved at `docs/plans/original-human-plan-narration/MNEMOSYNE.md`.

## Overview

`arda-vaire` owns significance-weighted memory, continuity, recall,
consolidation, and the bridge between machine continuity and human thought
surfaces. It is the widest-blast-radius substrate crate in the workspace and
is consumed by inference, knowledge, governance, observability, and
comms surfaces.

## Current Runtime State

- Crate root: `crates/spine/memory/arda-vaire`
- Core state: `core/state/mnemosyne_continuity.json`
- Data roots: `data/mnemosyne/episodic/`, `data/mnemosyne/noise.jsonl`,
  `data/mnemosyne/obsidian_index.jsonl`
- Tests/validation: `crates/spine/memory/arda-vaire/tests/*`

## Completed / Present Work

- Memory encoding, recall, and consolidation surfaces are live
- Checkpoint-policy guidance is exported for automation
- Chain integrity and continuity surfaces are projected into core state
- Service layer includes retrieval, store, promotion, significance weighting,
  and status shaping
- Transport layer exposes optional HTTP and IPC surfaces
- Public test coverage includes knowledge-delta flows and public API behavior
- Widely reused across Arda; many rev-deps depend on lib interfaces rather
  than daemon IPC

## Degraded / Blocked Work

- Memory substrate is now a library surface; former daemon/IPC assumptions
  are historical and should not be reintroduced at the crate boundary
- Legacy `human/` paths/sync flows are historical; current ownership is
  `arda-vaire`

## Current Frontier

- Reduce legacy path drift in operator docs and configs still mentioning
  `human/`, `arda-mnemosyne`, or old IPC assumptions
- Tighten schema/versioning for continuity state and episodic records
- Expand bounded coverage for retrieval, significance, and promotion paths
- Improve observability hooks without widening blast radius

## Hardening Contract

- Memory reads/writes must preserve provenance and significance lineage
- Continuity state should be append-safe and recoverable from malformed records
- Public surface stays small and stable; heavy internals remain unexported
- Human-external bridges remain optional and never force machine memory into
  non-canonical truth paths

## Primary Runtime Surfaces

- `crates/spine/memory/arda-vaire`
- `crates/spine/memory/arda-vaire/src/`
- `crates/spine/memory/arda-vaire/src/service/`
- `crates/spine/memory/arda-vaire/src/transport/`
- `core/state/mnemosyne_continuity.json`
- `data/mnemosyne/`

## Verification

- `cargo check -p arda-vaire`
- `cargo test -p arda-vaire`

## Alignment with Arda Principles

- Sovereign machine memory stays append-safe and provenance-bearing
- Significance-weighted recall keeps operator attention on durable structure
  instead of noise
- Human thought bridges are optional integrations, not authoritative truth

## Open Questions

- Which observability labels should mnemosyne/vaire emit for prometheus
  telemetry without widening blast radius?
- When should legacy doc/archive paths mentioning `human/` or IPC be fully retired?

## References

- Crate: `crates/spine/memory/arda-vaire`
- Crate docs: `crates/spine/memory/arda-vaire/README.md`,
  `crates/spine/memory/arda-vaire/BREAKDOWN.md`
- Original narration archive: `docs/plans/original-human-plan-narration/MNEMOSYNE.md`
- Archive docs: `docs/archive/MNEMOSYNE.md`,
  `docs/archive/VAIRE_MNEMOSYNE_RECONCEPTUALIZATION.md`,
  `docs/archive/MANWE_FLEET_LOOKUP_PLAN.md`
