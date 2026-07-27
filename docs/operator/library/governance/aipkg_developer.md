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

## Validation

```rust
let manifest = AipkgManifest { ... };
manifest.validate()?;
```

## Preflight

```rust
let receipt = manifest.preflight_check()?;
```

## Receipt rule

All receipts are schema-shape data, not runtime side effects. The validator
only proves the manifest is well-formed; operator approvals and signatures are
still required for production use.
