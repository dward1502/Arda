# AIPKG Developer Guide

Rust surface for AIPKG is `arda-core::aipkg`.

## Types

- `AipkgManifest`
- `AipkgPreflight`
- `AipkgGovernance`
- `AipkgReceiptPolicy`
- `AipkgPreflightReceipt`
- `AipkgExecutionReceipt`
- `AipkgValidationReceipt`
- `AipkgGovernanceEvidence`
- `AipkgReceiptChain`

## Validation

```rust
let manifest = AipkgManifest { ... };
manifest.validate()?;
```

## Preflight

```rust
let receipt = manifest.preflight_check_with_signature(operator_signature)?;
```

## Receipt rule

Manifest validation proves package metadata and policy shape. Receipt-chain
validation additionally enforces matching package identity/digest/profile,
preflight expiry, successful execution, all four explicit governance outcomes,
validator identity, and required signatures. Construct validation receipts via
`AipkgValidationReceipt::from_evidence`; do not infer unobserved gate outcomes.

The contract layer does not execute packages or produce cryptographic keys.
Executor profiles supply observed output, signatures, and governance evidence,
then call `AipkgReceiptChain::validate()` before accepting a result.
