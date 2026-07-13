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

# Crates Directory Structure

This directory contains the 23 Rust crates that make up the Annunimas agent system.

## Crate Overview

| Crate | Purpose | Status |
|-------|---------|--------|
| annunimas-core | Shared primitives and core definitions | ✅ Active |
| annunimas-charon | Inference routing and LLM provider mesh | ✅ Active |
| annunimas-hermes | Communications, Discord, and A2A primitives | ✅ Active |
| annunimas-athena | Ingest, research, and knowledge management | ✅ Active |
| annunimas-oracle | Reasoning and triad validation | ✅ Active |
| annunimas-plutus | JouleWork tracking and economics | ✅ Active |
| annunimas-apollo | Workflow execution and task orchestration | ✅ Active |
| annunimas-warden | Monitoring, security, and alerting | ✅ Active |
| annunimas-hades | Lifecycle management and cleanup | ✅ Active |
| annunimas-mnemosyne | Memory continuity and recall | ✅ Active |
| annunimas-prometheus | CEO orchestration pipeline | ✅ Active |
| annunimas-systemd | Typed `systemctl --user` client (supervision *policy* in `annunimas-prometheus`, *executor* remains systemd) | ✅ Active |
| annunimas-fleet | Fleet and topology coordination | ✅ Active |
| annunimas-signal-grid | Cross-agent signal mesh | ✅ Active |
| annunimas-service-registry | Service endpoint registry | ✅ Active |
| annunimas-tool-harness | Shared tool execution surface | ✅ Active |
| annunimas-mcp | MCP server bridge | ✅ Active |
| annunimas-comm | Communications primitives | ✅ Active |
| annunimas-council | Multi-agent boardroom deliberation | ✅ Active |
| annunimas-forge-mind | Code/build agent integration | ✅ Active |
| annunimas-ceo | Pipeline orchestration scaffolding | ⚠️ Incomplete |
| annunimas-governance | Triad, Resonance, Game Theory | ✅ Active |
| annunimas-cli | Operator entrypoint and commands | ✅ Active |

## Directory Structure

```
crates/
├── annunimas-core/
├── annunimas-charon/
├── annunimas-hermes/
├── annunimas-athena/
├── annunimas-oracle/
├── annunimas-plutus/
├── annunimas-apollo/
├── annunimas-warden/
├── annunimas-hades/
├── annunimas-mnemosyne/
├── annunimas-prometheus/
├── annunimas-systemd/
├── annunimas-fleet/
├── annunimas-signal-grid/
├── annunimas-service-registry/
├── annunimas-tool-harness/
├── annunimas-mcp/
├── annunimas-comm/
├── annunimas-council/
├── annunimas-forge-mind/
├── annunimas-ceo/
├── annunimas-governance/
└── annunimas-cli/
```

## Build Information

All crates are built using Cargo with the release profile. Build output goes to:
`~/.cache/annunimas-build/target/`

To build all crates:
```bash
source scripts/runtime_build_env.sh && cargo build --release
```

To build a specific crate:
```bash
cargo build -p <crate_name> --release
```
