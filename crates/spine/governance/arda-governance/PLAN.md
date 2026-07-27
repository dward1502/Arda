# arda-governance future work and discussion

Crate: `crates/spine/governance/arda-governance`
State: discussion queue; no current implementation commitments
Last reviewed: 2026-07-26

## Purpose

Use this file for proposed changes, design discussion, and future sequencing. It is not a
completion checklist and does not override `STATUS.md`. A proposal becomes committed work only
after it has an owner, acceptance criteria, consumer impact, and verification plan.

## Current commitments

None. The crate is stable for its current scope.

## Candidate discussions

### 0.3.0 deprecated resonance-path removal

Question: when should `calculate_resonance` and `calculate_resonance_basic` be removed?

Before approval:
- inventory all workspace and external consumers;
- migrate callers to a live Triad/chain result or explicit
  `calculate_resonance_without_governance`;
- document wire/source compatibility impact;
- update the public API fixture and add migration release notes.

### Shared ledger abstraction

Question: should Bacon-Lite persistence adopt a shared `arda-core` ledger trait?

Evaluate only if another durable evidence producer needs the same batching, rotation,
recovery, and failure semantics. Preserve bounded hot-path behavior and avoid moving
governance policy into the storage abstraction.

### Composite environmental signal type

Question: would a stable `GovernanceSignal` enum improve consumer integration?

Any proposal must preserve typed source health, freshness, quality, and confidence. Audio,
vision, and geomagnetic context must remain advisory-only unless a separate governance and
safety review explicitly changes that boundary.

### Additional live telemetry consumers

Question: which operator or HADES surface needs governance telemetry beyond the existing
`arda-aule`, Manwe, and Varda integrations?

Require a concrete operator use case, bounded labels, no new socket ownership in this crate,
and a consumer-level test before adding a new integration.

## Proposal template

Add proposals using this structure:

1. Problem and evidence.
2. Proposed contract and non-goals.
3. Public API, serialization, config, and consumer impact.
4. Safety, provenance, rollback, and observability impact.
5. Acceptance criteria and exact verification commands.
6. Owner and decision: proposed, accepted, deferred, or rejected.

## Required maintenance gates

For every accepted change:

- preserve conservative default readiness and advisory environmental semantics;
- review all seven direct workspace consumers when public contracts change;
- update `README.md`, `BREAKDOWN.md`, `STATUS.md`, and compatibility fixtures as applicable;
- run formatting, all-feature tests, strict Clippy, and rustdoc generation;
- record provenance changes in `GOVERNANCE_PROVENANCE.md`.