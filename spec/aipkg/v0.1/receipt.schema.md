---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-21"
---
> Arda-AIPKG: 📜 sovereign package/runtime contract | owner: prometheus | status: active | reviewed: 2026-07-21

# AIPKG Receipt Schema v0.1

This document defines the receipt payloads emitted during AIPKG lifecycle
operations.

The machine-readable authority is `receipt.schema.json`. The compiled Rust
authority is `arda_core::aipkg::AipkgReceiptChain::validate`.

## Preflight receipt: `AipkgPreflightReceipt`

| Field | Type | Notes |
|---|---|---|
| `package_id` | string | dotted identity |
| `version` | string | package version |
| `digest` | string | original manifest digest |
| `runtime_profile` | enum | `wasm-wasi`, `oci-sandboxed`, `local-sovereign` |
| `approved` | boolean | `true` only if `validate()` succeeded |
| `joule_budget` | integer or null | optional budget hint |
| `expires_at_utc` | string | RFC3339 timestamp |
| `signature` | non-empty string | operator or profile-supplied signature |

## Execution receipt: `AipkgExecutionReceipt`

| Field | Type | Notes |
|---|---|---|
| `package_id` | string | |
| `version` | string | |
| `started_at_utc` | string | RFC3339 |
| `completed_at_utc` | string | RFC3339 |
| `joule_cost_actual` | integer | observed cost |
| `exit_code` | integer | process result |
| `output_digest` | string | result handle |
| `signature` | string | signed attestation |

## Validation receipt: `AipkgValidationReceipt`

| Field | Type | Notes |
|---|---|---|
| `package_id` | string | |
| `version` | string | |
| `validated_at_utc` | string | RFC3339 |
| `triad_passed` | boolean | |
| `bacon_lite_passed` | boolean | |
| `joule_within_budget` | boolean | |
| `love_acceptable` | boolean | |
| `overall_passed` | boolean | |
| `validator_id` | string | signing service |
| `signature` | string | signed attestation |

## Operators

This schema does not create runtime effects or fabricate governance evidence.
Actual attestation and logging depend on surrounding execution policy. A valid
chain requires matching package identity, version, digest, and runtime profile;
an unexpired approved preflight; successful execution; all required governance
outcomes; validator identity; and every manifest-required signature.
