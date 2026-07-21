---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-21"
crate: aipkg
owner: prometheus
status: active
reviewed: "2026-07-21"
---

> Arda-AIPKG: 📜 sovereign package/runtime contract | owner: prometheus | status: active | reviewed: 2026-07-21

# AIPKG Plan Narrative

`AIPKG` is the current Arda package/preflight/receipt contract surface. Historic
narration is preserved at `docs/plans/original-human-plan-narration/AIPKG.md`.
This document merges the prior operator narrative with the current Arda surface
names so old detail is retained without stale crate assumptions.

Status: active
Owner: prometheus
- `docs/plans/AIPKG.md`
- `core/state/aipkg_contract.json`
Spec root: `spec/aipkg/v0.1/`
Task ledger: `core/state/queue.jsonl`

## Overview

AIPKG (Arda Intelligent Package) is a package management system designed for
autonomous agents, with a focus on safety, governance, and receipt-based
validation.

## Core Contract Surface

The core contract surface is defined in `core/state/aipkg_contract.json` and
includes:

- **Governance Requirements**: Bacon-lite, JouleWork budget, Love equation guard, Soterion trace, Triad gate
- **Manifest Structure**: Package digest, ID, version, governance flags, preflight requirements, receipt chain
- **Profiles**: wasm-wasi, oci-sandboxed, local-sovereign
- **Receipt Chain**: preflight, execution, validation, settlement, denial
- **Validator Harnesses**: bacon_lite, joulework, love_equation, triad

## Key Principles

1. **Open Standard Law**: Core package law is open and separate from marketplace economics
2. **Zero-Work Preflight**: Mandatory preflight validation before any execution
3. **Triad Validation**: Logic, strategy, and evidence gates must pass
4. **JouleWork Accounting**: Energy/budget validation before work begins
5. **Love Equation Guard**: Human/relational safety for user-facing packages
6. **Soterion Trace**: Documentation and safety evidence required

## Implementation Status

### Completed

- Core contract surface (`core/state/aipkg_contract.json`)
- Governance requirements definition
- Receipt chain specification
- Validator harnesses specification

### Pending

- Operator narrative consolidation in this document
- Specification files (`spec/aipkg/v0.1/`)
- Reference implementations
- Test harnesses

## Next Steps

1. **Create Specification Files** in `spec/aipkg/v0.1/`:
   - `AIPKG-CONTAINER-v0.1.md`
   - `execution-request.schema.json`
   - `manifest.example.json`
   - `receipt.schema.json`

2. **Develop Reference Implementation**:
   - Package validator
   - Receipt chain processor
   - Governance gate integrations

3. **Create Test Harnesses**:
   - Bacon-lite validator tests
   - JouleWork budget tests
   - Triad gate tests
   - End-to-end receipt chain tests

4. **Documentation**:
   - Operator guide
   - Developer guide
   - Safety and governance documentation

## Purpose

`.aipkg` defines the sovereign package, preflight, receipt, and runtime contract for portable agent tooling.

## Current Contract

- open standard law is separate from marketplace and financial tooling
- zero-work preflight is mandatory
- Triad, Bacon-lite, JouleWork, Love Equation, and Soterion trace are explicit governance gates
- receipts and signed attestations are first-class runtime truth
- profiles are split into `wasm-wasi`, `oci-sandboxed`, and `local-sovereign`

## Primary Runtime Surfaces
- `docs/plans/AIPKG.md`
- `core/state/aipkg_contract.json`
- `spec/aipkg/v0.1/manifest.example.json`
- `spec/aipkg/v0.1/execution-request.schema.json`
- `spec/aipkg/v0.1/receipt.schema.json`

## Alignment with Arda Principles

The AIPKG system aligns with core Arda principles:
- **Autonomy with Governance**: Packages must pass governance gates before execution
- **Receipt-Based Validation**: All operations generate verifiable receipts
- **Safety First**: Zero-work preflight and comprehensive validation
- **Open Standards**: Core law is open and separate from implementation

## Open Questions

1. Should marketplace economics be part of core specification or separate?
2. What additional validator harnesses are needed for production use?
3. How should package signing and identity be handled across different profiles?

## References

- Crate/surface: `docs/plans/AIPKG.md`
- `docs/plans/AIPKG.md`
- `core/state/aipkg_contract.json`
- Spec root: `spec/aipkg/v0.1/`
- Arda governance: `docs/SAFETY_MODEL.md`
- Triad validation: `docs/operations/TRIAD_GATE_OPERATIONS.md`
