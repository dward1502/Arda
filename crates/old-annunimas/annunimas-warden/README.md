---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
crate: annunimas-warden
kind: monitoring
agent: warden
realm: monitoring
sigil: "𓃭"
capabilities:
  - runtime-monitoring
  - container-audit
  - foreign-agent-protocol
  - heartbeat-alerting
status: active-prototype
search_tags: [warden, monitoring, podman, alerts, tailscale]
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active-prototype | reviewed: 2026-05-21

# annunimas-warden

Runtime guardian for system and fleet health.

## Purpose
Observe runtime/container state, handle foreign-agent trust transitions, and emit health/alert signals.

## What's in this crate
- `monitor.rs`, `podman.rs`: runtime/container observation.
- `foreign.rs`: quarantine/probation/trust state machine for external agents.
- `crypto.rs`: report crypto helpers.
- `alerts.rs`: webhook heartbeat posting (Discord-compatible).
- `main.rs`: warden binary startup and optional heartbeat emission.
- `src/informant/main.rs`: sidecar binary entrypoint.
