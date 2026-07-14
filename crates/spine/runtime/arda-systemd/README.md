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

# arda-systemd

Thin typed client for `systemctl --user`. Consumed by
`arda-prometheus`'s autopilot service-health monitor so callers don't
have to parse `systemctl` output by hand.

## What this is
- Parsed `Unit` records (load/active/sub state, type-aware classification).
- A `SystemctlClient` that shells out to `systemctl --user` and returns typed
  results.
- A `SystemdClient` trait so consumers can mock the call surface in tests.

## What this is NOT
- Not an agent supervisor. systemd already owns single-host service
  start/stop/restart, and `arda-prometheus` owns supervision *policy*.
  This crate is a typed client, nothing more.
- Not a fleet orchestrator — that lives in `arda-fleet`.

## History
This crate is the repurposed remains of `arda-supervisor`. The old
crate tried to reimplement `scripts/agent_supervisor.sh` in Rust and was
scoped against systemd's existing role. It was retired in favor of this
focused, ~200-line typed wrapper.
