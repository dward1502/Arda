# arda-rumil index

- [`src/contracts.rs`](src/contracts.rs) — versioned `arda.rumil.*` packet contracts
- [`src/policy.rs`](src/policy.rs) — bounded root, exclusion, budget, and provider policy
- [`src/inventory.rs`](src/inventory.rs) — generic project inventory
- [`src/adapters/`](src/adapters/) — generic, Cargo, and Git adapters
- [`src/providers/`](src/providers/) — allowlisted read-only providers and source selection
- [`src/findings.rs`](src/findings.rs) — provider-neutral finding normalization and feedback
- [`src/baseline.rs`](src/baseline.rs) — explicit baseline and bounded memory projection
- [`src/comparison.rs`](src/comparison.rs) — deterministic finding lifecycle comparison
- [`src/organization.rs`](src/organization.rs) — project-neutral review-only organization planning
- [`tests/`](tests/) — contract, inventory, adapter, provider, comparison, and organization fixtures
- [`OWNERSHIP.md`](OWNERSHIP.md) — authority and neighboring-system boundaries
- [`STATUS.md`](STATUS.md) — live implementation status
