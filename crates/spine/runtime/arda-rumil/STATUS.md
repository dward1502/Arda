# arda-rumil status

**State:** first reusable release implemented; RUMIL-0 through RUMIL-8 complete locally

## Verified gates

- all-feature and no-default-feature tests
- crate-scoped warning-denying Clippy with `--no-deps`
- warning-denying rustdoc
- full workspace check and the RUMIL consumer test matrix
- queue append-only guard before plan reconciliation

The plan's exact Clippy command without `--no-deps` currently reaches the clean dependency `arda-outpost-protocol` and is blocked by its pre-existing `WatchlistEntry::new` `too_many_arguments` lint. Rúmil-owned Clippy is green; the dependency lint remains an external workspace gate and is not suppressed here.

## Warden/Vairë integration

Warden's scout now accepts expiring, bounded, advisory Rúmil audit requests,
persists large packets at audit-owned paths, emits digest-bound idempotency
receipts, and supports packet-only targeted follow-up. Vairë receives one compact
receipt observation on first execution and no duplicate record on replay.

## Governed consumers and profiles

Mandos preserves all five Rúmil evidence classes without opening project files.
Varda returns advisory-only receipt evaluations. Workbench and HUD show bounded
packet identity, completeness, baseline freshness, rejected providers, and
missing evidence while keeping execution disabled.

Five validated TOML profiles drive one generic inventory coordinator for Arda,
Rust, Node, Python, and mixed projects. Full audits remain host-side; Pi
consumers are receipt/inventory-only. Retained HADES findings import into
deterministic historical baselines with explicit `legacy_source` provenance.
