---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "architecture_audit"
  owner: "HERMES"
  status: "draft"
  reviewed: "2026-10-01"
---

> 🜏 Soterion: 📜 architecture_audit | owner: HERMES | status: draft | reviewed: 2026-10-01

# Package and dependency map

This is a structural and sampled-source map, not a certification of every function. Source line totals include tests and comments. Dependencies below are declared internal normal dependencies (including optional ones), not a resolved active-feature or runtime-call graph.

## arda

- Location: `.`
- Observed role / assessment: Process composition and daemon entry point; delegates to Engine, not a second domain engine.
- Entry-point inspection: `src/main.rs` (sampled; not a full file-by-file semantic review).
- Selected-extension source lines: 3,446.
- Internal normal dependencies: `arda-aule`, `arda-engine`.
- Next verification: targeted checks recorded in 05; full behavioral coverage remains unmeasured.

## arda-aule

- Location: `crates/spine/observability/arda-aule`
- Observed role / assessment: Telemetry, projections, CLI and substantial Prometheus/autopilot execution machinery. Most evident mismatch between directory role and actual ownership.
- Entry-point inspection: `crates/spine/observability/arda-aule/src/lib.rs` (sampled; not a full file-by-file semantic review).
- Selected-extension source lines: 50,280.
- Internal normal dependencies: `arda-core` (optional), `arda-economics` (optional), `arda-governance`, `arda-mandos` (optional), `arda-orome` (optional), `arda-outpost-protocol`, `arda-vaire` (optional), `arda-varda` (optional).
- Next verification: trace one real caller through persistence and failure handling; run package-specific tests before assigning maturity.

## arda-core

- Location: `crates/spine/governance/arda-core`
- Observed role / assessment: Shared domain contracts, run graphs, governance interfaces and historical loop substrate. Keep contracts independent of implementation.
- Entry-point inspection: `crates/spine/governance/arda-core/src/lib.rs` (sampled; not a full file-by-file semantic review).
- Selected-extension source lines: 18,814.
- Internal normal dependencies: none.
- Next verification: trace one real caller through persistence and failure handling; run package-specific tests before assigning maturity.

## arda-economics

- Location: `crates/spine/runtime/arda-economics`
- Observed role / assessment: Resource/energy accounting and meter abstraction. Distinguishes measured from estimated samples; do not equate either with proven savings.
- Entry-point inspection: `crates/spine/runtime/arda-economics/src/lib.rs` (sampled; not a full file-by-file semantic review).
- Selected-extension source lines: 3,647.
- Internal normal dependencies: `arda-core`, `arda-governance`.
- Next verification: trace one real caller through persistence and failure handling; run package-specific tests before assigning maturity.

## arda-governance

- Location: `crates/spine/governance/arda-governance`
- Observed role / assessment: Governance/evidence evaluation implementations layered on core. Review consumers for consistent enforcement, not merely recorded verdicts.
- Entry-point inspection: `crates/spine/governance/arda-governance/src/lib.rs` (sampled; not a full file-by-file semantic review).
- Selected-extension source lines: 10,718.
- Internal normal dependencies: `arda-core`.
- Next verification: trace one real caller through persistence and failure handling; run package-specific tests before assigning maturity.

## arda-mandos

- Location: `crates/spine/runtime/arda-mandos`
- Observed role / assessment: Reasoning/verdict substrate, consuming governance, economics and audit contracts. Effectiveness of model-based judgment not evaluated here.
- Entry-point inspection: `crates/spine/runtime/arda-mandos/src/lib.rs` (sampled; not a full file-by-file semantic review).
- Selected-extension source lines: 7,276.
- Internal normal dependencies: `arda-core`, `arda-economics`, `arda-governance`, `arda-rumil`.
- Next verification: trace one real caller through persistence and failure handling; run package-specific tests before assigning maturity.

## arda-rumil

- Location: `crates/spine/runtime/arda-rumil`
- Observed role / assessment: Project-audit coordination substrate. Audit outputs should remain advisory, not become a parallel execution owner.
- Entry-point inspection: `crates/spine/runtime/arda-rumil/src/lib.rs` (sampled; not a full file-by-file semantic review).
- Selected-extension source lines: 5,511.
- Internal normal dependencies: `arda-outpost-protocol`.
- Next verification: trace one real caller through persistence and failure handling; run package-specific tests before assigning maturity.

## arda-outpost-protocol

- Location: `outposts/arda-outpost-protocol`
- Observed role / assessment: Bounded wire/projection contracts shared across outposts and runtime; low-dependency seam worth preserving.
- Entry-point inspection: `outposts/arda-outpost-protocol/src/lib.rs` (sampled; not a full file-by-file semantic review).
- Selected-extension source lines: 3,192.
- Internal normal dependencies: none.
- Next verification: trace one real caller through persistence and failure handling; run package-specific tests before assigning maturity.

## arda-orome

- Location: `crates/spine/interface/arda-orome`
- Observed role / assessment: Communications and council-facing semantics/transports. Dependencies on reasoning, memory and governance make it more than a transport library.
- Entry-point inspection: `crates/spine/interface/arda-orome/src/lib.rs` (sampled; not a full file-by-file semantic review).
- Selected-extension source lines: 18,567.
- Internal normal dependencies: `arda-core`, `arda-economics` (optional), `arda-governance`, `arda-mandos` (optional), `arda-vaire` (optional).
- Next verification: trace one real caller through persistence and failure handling; run package-specific tests before assigning maturity.

## arda-vaire

- Location: `crates/spine/memory/arda-vaire`
- Observed role / assessment: Memory, context assembly, retention and continuity. Explicit store/retrieval/promotion separation is a useful existing pattern; test compilation currently fails.
- Entry-point inspection: `crates/spine/memory/arda-vaire/src/lib.rs` (sampled; not a full file-by-file semantic review).
- Selected-extension source lines: 9,519.
- Internal normal dependencies: `arda-core`, `arda-economics`, `arda-governance`.
- Next verification: trace one real caller through persistence and failure handling; run package-specific tests before assigning maturity.

## arda-varda

- Location: `crates/spine/executors/arda-varda`
- Observed role / assessment: Research/evidence ingestion and related execution support. Ingest implementation is a high-priority large-file review target.
- Entry-point inspection: `crates/spine/executors/arda-varda/src/lib.rs` (sampled; not a full file-by-file semantic review).
- Selected-extension source lines: 15,979.
- Internal normal dependencies: `arda-core`, `arda-economics`, `arda-governance`, `arda-outpost-protocol`, `arda-rumil`, `arda-vaire`.
- Next verification: trace one real caller through persistence and failure handling; run package-specific tests before assigning maturity.

## arda-engine

- Location: `crates/engine`
- Observed role / assessment: Objective scheduling, project/run authority, durable stores, harness HTTP API, adapters and process supervision. Integration center; broad responsibility requires internal boundaries.
- Entry-point inspection: `crates/engine/src/lib.rs` (sampled; not a full file-by-file semantic review).
- Selected-extension source lines: 88,483.
- Internal normal dependencies: `arda-aule`, `arda-core`, `arda-governance`, `arda-mandos`, `arda-orome`, `arda-outpost-protocol`, `arda-rumil`, `arda-vaire`, `arda-varda`, `manwe`.
- Next verification: trace one real caller through persistence and failure handling; run package-specific tests before assigning maturity.

## manwe

- Location: `crates/spine/runtime/manwe`
- Observed role / assessment: Inference gateway and adaptive provider routing. Default adaptive feature is enabled; disabled-feature stub is not evidence that default routing is missing.
- Entry-point inspection: `crates/spine/runtime/manwe/src/lib.rs` (sampled; not a full file-by-file semantic review).
- Selected-extension source lines: 31,877.
- Internal normal dependencies: `arda-aule` (optional), `arda-core` (optional), `arda-economics` (optional), `arda-governance` (optional), `arda-vaire` (optional).
- Next verification: trace one real caller through persistence and failure handling; run package-specific tests before assigning maturity.

## arda-contract-registry

- Location: `crates/spine/contract/arda-contract-registry`
- Observed role / assessment: Explicit contract registry loading and version lookup. Small boundary with passing targeted tests; registry presence is not workflow acceptance.
- Entry-point inspection: `crates/spine/contract/arda-contract-registry/src/lib.rs` (sampled; not a full file-by-file semantic review).
- Selected-extension source lines: 264.
- Internal normal dependencies: none.
- Next verification: targeted checks recorded in 05; full behavioral coverage remains unmeasured.

## arda-mirromere

- Location: `crates/spine/interface/arda-mirromere`
- Observed role / assessment: Shared Rust display/lifecycle contract substrate, distinct from the native application with an underscore in its package name.
- Entry-point inspection: `crates/spine/interface/arda-mirromere/src/lib.rs` (sampled; not a full file-by-file semantic review).
- Selected-extension source lines: 1,240.
- Internal normal dependencies: `arda-outpost-protocol`.
- Next verification: trace one real caller through persistence and failure handling; run package-specific tests before assigning maturity.

## arda-launcher

- Location: `apps/arda-launcher/src-tauri`
- Observed role / assessment: Native launcher and setup/runtime lifecycle integration. A main-workspace member also consumed by the other native apps.
- Entry-point inspection: `apps/arda-launcher/src-tauri/src/main.rs` (sampled; not a full file-by-file semantic review).
- Selected-extension source lines: 6,332.
- Internal normal dependencies: `arda-contract-registry`, `arda-core`.
- Next verification: trace one real caller through persistence and failure handling; run package-specific tests before assigning maturity.

## arda-outpost-scout

- Location: `outposts/arda-outpost-scout`
- Observed role / assessment: Bounded research/survey outpost consuming Rumil and Vaire. Not an independent authority for consequential execution.
- Entry-point inspection: `outposts/arda-outpost-scout/src/lib.rs` (sampled; not a full file-by-file semantic review).
- Selected-extension source lines: 3,664.
- Internal normal dependencies: `arda-outpost-protocol`, `arda-rumil`, `arda-vaire`.
- Next verification: trace one real caller through persistence and failure handling; run package-specific tests before assigning maturity.

## arda-relic-bridge

- Location: `outposts/arda-relic-bridge`
- Observed role / assessment: Read-only runtime-presence projection bridge; freshness/replay rejection covered by passing targeted tests.
- Entry-point inspection: `outposts/arda-relic-bridge/src/lib.rs` (sampled; not a full file-by-file semantic review).
- Selected-extension source lines: 608.
- Internal normal dependencies: `arda-outpost-protocol`.
- Next verification: targeted checks recorded in 05; full behavioral coverage remains unmeasured.

## arda-project-adapter-sdk

- Location: `sdk/rust`
- Observed role / assessment: External-project framing and capability negotiation SDK; bounded parser tests pass. External adoption not established.
- Entry-point inspection: `sdk/rust/src/lib.rs` (sampled; not a full file-by-file semantic review).
- Selected-extension source lines: 149.
- Internal normal dependencies: none.
- Next verification: targeted checks recorded in 05; full behavioral coverage remains unmeasured.
