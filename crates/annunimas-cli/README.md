---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# sigil: REPAIR
---
crate: annunimas-cli
kind: binary
agent: ceo-interface
realm: command
capabilities:
  - run-task
  - list-tools
  - status-report
  - athena-ops
  - prometheus-ops
  - mnemosyne-ops
status: active
search_tags: [cli, entrypoint, commands, ceo]
---

# annunimas-cli

Primary command-line entrypoint for running Annunimas locally.

## Purpose
Boot config/provider/router and expose operational commands for task execution plus ATHENA/PROMETHEUS service operations.

## What's in this crate
- `main.rs`: argument parsing, config load, provider build, router build, pipeline invocation.

## Commands
- Core: `run`, `tools`, `status`
- ATHENA: `athena start|status|ingest|query|deep|digest`
- PROMETHEUS: `prometheus start|status|ops-dashboard|maintenance|roster|thoughts|escalate|resolve-escalation`
- CHARON: `charon start|status|state|providers|route|cooldown|provider-result|paths`
- MNEMOSYNE: `mnemosyne start|status|paths|stats|encode|recall-recent|consolidate|identity-state|obsidian-sync`
- HADES: `hades start|status|sweep|queue|log|remove|paths`
- HERMES: `hermes start|status|providers|subcomponents|boardroom|classify|send|retry-outbound|boardroom-post|calendar-sync|ingest-external|council-open|council-report|council-close|paths`

## Notes
`run` uses the PROMETHEUS pipeline (`annunimas-prometheus`) with `/core` linkage (`core/realm/boot.toml`, `core/state/world.json`) enabled by default.

`prometheus maintenance` supports `--async` to spawn a background maintenance cycle and avoid blocking the caller.

## Owns
- top-level command parsing and subcommand dispatch
- daemon startup for multiple services
- export surfaces and operator-facing convenience commands
- policy-guard and observability entry logic

## Main Areas
- [`src/main.rs`](/var/home/mythos/Annunimas/crates/annunimas-cli/src/main.rs): clap models and top-level dispatch
- [`src/commands/`](/var/home/mythos/Annunimas/crates/annunimas-cli/src/commands): service-specific operator commands
- [`src/export_surface/`](/var/home/mythos/Annunimas/crates/annunimas-cli/src/export_surface): exported operator surfaces and helper flows
- [`src/policy_guard.rs`](/var/home/mythos/Annunimas/crates/annunimas-cli/src/policy_guard.rs): autonomy/error-budget gating

## Common Commands
```bash
cargo run -p annunimas-cli -- status
cargo run -p annunimas-cli -- charon status
cargo run -p annunimas-cli -- hermes status
```

## Debug Path
- new or broken CLI flag:
  start with `src/main.rs`, then the matching file under `src/commands/`
- operator/export helper bug:
  start with the matching file under `src/export_surface/`
- autonomy block or gating issue:
  start with `src/policy_guard.rs`
