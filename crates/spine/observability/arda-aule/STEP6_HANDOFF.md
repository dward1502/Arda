# arda-aule Consolidation Handoff

Status: consolidation complete.

## Resolution

- Kept one `arda-cli` binary and attached CEO plus coherent Prometheus library surfaces through
  `full-cli`.
- Restored Prometheus service, IPC, optional HTTP daemon, orders, thoughts, escalation, planning,
  roster, drift, council evidence, and projection capabilities.
- Attached CEO autopilot through the canonical task queue without fabricating execution completion.
- Assigned provider/fleet routing to Manwe through explicit execution-intent ownership.
- Migrated Aule-owned Prometheus and autopilot commands into the supported `arda-cli` binary.
- Removed duplicate roots and detached source after its replacement or canonical owner was verified.

## Verification

- `cargo check -p arda-aule --features full-cli --all-targets`: passing
- `cargo test -p arda-aule --features full-cli --lib --tests`: passing
- `cargo test -p arda-aule --all-features --lib --tests`: passing serially
- `cargo clippy -p arda-aule --all-targets --all-features -- -D warnings`: passing
- `cargo fmt -p arda-aule -- --check`: passing
- Process smoke checks for `prometheus autopilot status`, `prometheus autopilot once --read-only`,
  and `prometheus execution-intents`: passing
- Workspace-wide `cargo fmt --all -- --check` is currently blocked by unrelated launcher
  Tauri formatting drift outside this closeout scope.

No migration handoff remains inside `arda-aule`.
