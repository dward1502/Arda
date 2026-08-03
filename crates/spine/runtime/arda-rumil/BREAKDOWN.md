# arda-rumil breakdown

## Implemented

- RUMIL-0: strict v1 packet envelopes, completeness/authority/provenance contracts, portable identity, and legacy HADES import metadata.
- RUMIL-1: deterministic bounded inventory with exclusions, hashing, truncation, unreadable-path, and symlink disclosure.
- RUMIL-2: generic, Cargo metadata, Cargo tree, and read-only Git adapters.
- RUMIL-3: allowlisted command runner, bounded stdout/stderr, timeout/unavailable/nonzero/malformed states, Cargo/security/module provider specifications, tool/config/output provenance, and selected redacted source excerpts.
- RUMIL-4: stable finding IDs, deterministic selected-baseline comparison, explicit operator dispositions, and bounded Vairë-eligible observation projection.
- RUMIL-5: project-neutral organization profiles, inventory/tool-backed checks, deterministic review-only plans and dry-run receipts, explicit non-mutation handoff, and historical-only HADES import.
- RUMIL-6: Warden/scout bounded audit consumer, audit-owned packet persistence, compact Vairë receipt projection, digest-bound replay, and packet-only follow-up.
- RUMIL-7: bounded packet evidence references, Mandos five-class reasoning projection, Varda advisory evaluation receipts, and Workbench/HUD degraded-state disclosure.
- RUMIL-8: validated declarative Arda/Rust/Node/Python/mixed profiles, one generalized coordinator, host/Pi target separation, and deterministic historical HADES baseline import.

## Primary implementation map

- `src/evaluation.rs`: bounded consumer references and evidence classification.
- `src/profile.rs` and `profiles/*.toml`: project-neutral profile schema, validation, generalized inventory, and execution-target boundary.
- `src/baseline.rs`: deterministic comparisons and historical HADES finding migration.
- `tests/evidence_consumer_contract.rs`: bounded/degraded evidence projection.
- `tests/profile_generalization_contract.rs`: cross-project profiles, target separation, and migration provenance.
