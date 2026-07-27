# arda-outpost-protocol

Shared outpost observation contract for Arda.

## What it provides
- `SCHEMA_VERSION`
- `OutpostObservation` with scope, classification, authority, confidence, provenance
- `ObservationScope`, `ObservationClassification`, `AuthorityClass`

## Why it exists
Outpost crates should emit structurally identical observations so Warden/HUD/council
tooling can validate, route, and visualize them consistently.

## Usage
```rust
use arda_outpost_protocol::{
    AuthorityClass, ObservationClassification, ObservationScope, OutpostObservation, SCHEMA_VERSION,
};

let observation = OutpostObservation::new(
    "source-id",
    ObservationScope::Crates,
    ObservationClassification::DerivedEstimate,
    AuthorityClass::Advisory,
    serde_json::json!({}),
)
.with_confidence(0.7)
.with_provenance("arda-outpost-protocol://example");
```

## Tests
```bash
cargo test
```
