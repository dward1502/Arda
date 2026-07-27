---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "archived"
  last_reviewed: "2026-07-27"
owner: prometheus
status: archived
reviewed: "2026-07-27"
---

> Arda-AIPKG: 📜 sovereign package/runtime contract | owner: prometheus | status: archived | reviewed: 2026-07-27

# AIPKG v0.1 Foundation Plan — Archived

AIPKG is Arda’s optional package/preflight/receipt contract surface. The
foundation plan was completed and archived on 2026-07-27. Current implementation
truth lives in `arda-core`, the v0.1 specification bundle, operator guidance,
and canonical state projections listed below.

Status at archive: complete
Task ledger: `core/projects/tasks/queue.jsonl`

## Delivered contract

- `Task::aipkg_manifest` remains optional and backward-compatible.
- Dispatch validates attached manifests before bids, JouleWork, Triad, or
  council allocation and records blocked evidence for malformed packages.
- Rust manifest validation matches the v0.1 schema for package identity,
  version, digest, runtime profile, preflight law, governance, and receipt law.
- Signed preflight construction uses a bounded 15-minute validity window.
- `AipkgReceiptChain::validate` fails closed on:
  - package identity, version, digest, or runtime-profile mismatch;
  - expired preflight or invalid RFC3339 timestamps;
  - failed execution or malformed output digest;
  - failed/incomplete Triad, Bacon-lite, JouleWork, or Love evidence;
  - missing validator identity;
  - missing manifest-required signatures.
- `AipkgValidationReceipt::from_evidence` derives `overall_passed` from explicit
  gate outcomes rather than inferring evidence from manifest declarations.
- The contract layer intentionally does not fabricate cryptographic signatures
  or executor observations; operator/profile executors supply both.

## Specification and drift protection

- `spec/aipkg/v0.1/manifest.schema.json`
- `spec/aipkg/v0.1/manifest.example.json`
- `spec/aipkg/v0.1/execution-request.schema.json`
- `spec/aipkg/v0.1/receipt.schema.json`
- `spec/aipkg/v0.1/receipt.schema.md`
- `scripts/aipkg/validate_spec.py`
- `scripts/aipkg/test_validate_spec.py`

The validator checks required bundle files, JSON syntax, the manifest example
against the supported schema constraints, the normative container document,
and compiled Rust receipt-chain markers. Regression tests prove both a valid
bundle and rejection of a schema-incompatible digest.

## Open-standard boundary

AIPKG core law remains open and local-first. Marketplace discovery, reputation,
payment rails, and settlement are separate optional extensions; they are not
required for local package validation or receipt-chain acceptance.

## Closure evidence

- Focused AIPKG Rust tests: 12 passed on 2026-07-27.
- Spec validator: passed on 2026-07-27.
- Validator regressions: 2 passed on 2026-07-27.
- JSON syntax checks passed for both state projections and the receipt schema.
- Full crate/workspace gates are recorded in the final task closeout rather than
  treated as permanent runtime health claims in this archive.

## Current authorities

- Rust contract: `crates/spine/governance/arda-core/src/aipkg.rs`
- Dispatch gate: `crates/spine/governance/arda-core/src/loop_engine.rs`
- Task attachment: `crates/spine/governance/arda-core/src/task.rs`
- Spec root: `spec/aipkg/v0.1/`
- Contract projection: `core/state/aipkg_contract.json`
- Marketplace boundary: `core/state/aipkg_marketplace_separation_contract.json`
- Operator guide: `docs/operator/library/governance/aipkg.md`
- Safety notes: `docs/operator/library/governance/aipkg_safety.md`
- Developer guide: `docs/operator/library/governance/aipkg_developer.md`
