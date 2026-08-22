---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "implementation_plan"
  owner: "HERMES"
  status: "active"
  reviewed: "2026-08-21"
---

> 🜏 Soterion: 📜 implementation_plan | owner: HERMES | status: active | reviewed: 2026-08-21

# Stage 2 — Node Mesh and A2A Interoperability

## Objective

Activate a real node/agent mesh in which enrolled nodes publish expiring capabilities, exchange typed work through standard A2A transport, and produce Arda lineage and delivery receipts.

## Reuse decision

- Hermes/Linux Foundation A2A is the preferred external/cross-process wire protocol.
- Oromë’s `A2AMessage`/`Envelope` remains a candidate internal semantic envelope for authority, expiry, thread, hop, and receipt lineage.
- Oromë’s existing registry/router must be audited before promotion; `router.rs` is currently test-only and must not be described as live.
- Outpost node observations remain hardware/device evidence, not agent-task transport.

## Work packets

### S2.1 — Converge node identity and enrollment

Define stable node IDs, key/attestation references, trust class, hardware/resource capabilities, supported transports, endpoint advertisement, privacy domains, and enrollment/revocation status. Extend an existing fleet/outpost/core contract where possible.

### S2.2 — Publish expiring capability and pressure observations

Nodes publish model/tool/data locality, CPU/GPU/RAM/storage/network availability, current load, thermal/power signals where available, and evidence freshness. Missing telemetry remains unknown, never zero.

### S2.3 — Map Arda work envelopes to standard A2A

Implement and test a codec/adapter between the selected Arda semantic envelope and A2A Agent Cards/tasks/messages. Preserve stable task/run IDs, capability request, expiry, redaction, parent receipt refs, and return contract. Do not fork the A2A wire specification.

### S2.4 — Activate bounded registry/routing

Promote or replace the existing test-only registry/router only after Stage 0 consumer evidence. Routing must be deterministic for exact targets, capability-aware for role requests, bounded, expiry-aware, and persistent/recoverable where required.

### S2.5 — Prove two-node exchange

Use two independently running nodes/processes. Node A discovers Node B from a real Agent Card/capability record, sends one read-only task, receives a correlated result, records both sides’ receipts, and expires B after heartbeat loss.

### S2.6 — Prove security and loop resistance

Cover unknown/revoked peer, token mismatch, untrusted prompt input, stale capability, oversized payload, replay, TTL expiry, message loop, cross-domain request, and forged completion.

## Likely owners and files

- `crates/spine/interface/arda-orome/`
- `outposts/arda-outpost-protocol/`
- `config/fleet.toml`
- `crates/engine/` topology consumers
- `~/.hermes/plugins/arda-operator-bridge/` only for Hermes adapter behavior; canonical source should remain in the repository adapter/plugin source when identified by Stage 0.

## Exit gate

Two nodes exchange one real typed task over A2A, with capability discovery, expiry, authentication, correlated delivery proof, and honest offline state visible from the root runtime. No manual file copying may stand in for transport.
