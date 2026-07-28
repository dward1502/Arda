# arda-mandos status

Crate: `crates/spine/runtime/arda-mandos`
Current state: first-class active; Packet 4 closed
Branch: `manwe`
Documentation: `README.md`, `INDEX.md`, `BREAKDOWN.md`, `STATUS.md`, `OWNERSHIP.md`

Current signature: Mandos evaluates validated queries once per restart-safe request identity, persists the verdict before exposure in a versioned digest-linked JSONL ledger, reports typed advisory outcomes and conditions, and exposes shared bounded IPC/HTTP operations for status, evaluation, history, verification, and atomic verified export.

Closeout evidence:

- `cargo fmt -p arda-mandos -- --check`: passed
- `cargo test -p arda-mandos --all-features`: 75 unit + 2 integration passed
- `cargo test -p arda-mandos --no-default-features`: 68 unit + 2 integration passed
- strict all-feature and no-default-feature Clippy: passed with all targets, no dependencies, and warnings denied
- all-feature Rustdoc with warnings denied: passed
- `cargo test` and `cargo check` for all-feature direct consumers `arda-aule` and `arda-orome`: passed

Operational notes:

- Mandos is advisory decision support; execution and approval authority remain external.
- Ledger verification and export are operator workflows, not repair workflows. Corrupt or legacy unchained ledgers are reported and refused for export.
- IPC/HTTP export destinations are relative to the Mandos `exports` directory; absolute and traversal paths are rejected.
- Prometheus exposition remains caller/`arda-aule` owned; Mandos status provides bounded low-cardinality counters.
