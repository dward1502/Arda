# arda-core ownership

Crate: `crates/spine/governance/arda-core`
Owner: ARDA / foundational runtime contracts
Status: active
Reviewed: 2026-07-28

## This crate owns

- Shared lifecycle data, task/goal/plan/reflection contracts, error and configuration primitives.
- The loop engine's dispatch, economy, alert, observability, and learning primitives.
- Service-registry, tool-contract, AIPKG, Soterion, ledger, systemd, and provider abstractions.
- Stable modules that direct consumers compile against.

## This crate does not own

- Governance scoring, policy decisions, evidence quality, or governance metrics; `arda-governance` owns those.
- Engine orchestration and aggregate runtime projections; `arda-engine` owns those.
- Observability transport/export policy; `arda-aule` owns that boundary.
- Memory authority; `arda-vaire` owns memory persistence and retrieval.
- User/operator interfaces; `arda-orome` and the launcher own those surfaces.
- Runtime routing/provider execution; Manwe owns that boundary.

## Change discipline

- Add foundational APIs only when more than one concrete consumer needs the contract.
- Keep generated/runtime state outside the crate source tree.
- Run strict crate gates plus affected direct-consumer checks for public contract changes.
- Treat `BREAKDOWN.md` as the compiled source map and `STATUS.md` as dated evidence.
