---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-21"
crate: platform-os
owner: prometheus
status: in_progress
reviewed: "2026-06-21"
---

> Arda Platform OS: 📜 local agent control plane / appliance roadmap | owner: prometheus | status: in_progress | reviewed: 2026-06-21

# Platform OS Plan Narrative

`PLATFORM_OS` is the current Arda local agent control-plane planning surface.
Historic narration is preserved at
`docs/plans/PLATFORM_OS.md`. This document merges
the prior operator narrative with the current Arda surface names so old detail is
retained without stale crate assumptions.

Status: in_progress; core freeze identified, not fully enforced
Owner: prometheus
Operator plan: `docs/plans/PLATFORM_OS.md`
Primary queue surface: `core/state/queue_active.json`
Queue summary surface: `core/state/queue_summary.json`
Runtime settings projection: `core/state/runtime_settings.json`
Task ledger: `core/state/queue.jsonl`

## Purpose

The Platform OS plan shapes Annunimas into an auditable local agent control plane that can later become a bootable appliance. The current north star is a Bluefin-based Annunimas-first environment with local-first runtime, deterministic state, and composable app/private-consumer modules.

## Current Review Summary

The core Platform OS plan is present and active at `docs/plans/PLATFORM_OS.md`. It defines a staged migration path: freeze the OS core surface, separate tenant/staged crates, extract private consumer applications, and later prove a bootable Bluefin appliance with first-boot health checks.

Current evidence shows the 18-crate core surface has been identified, but enforcement remains in progress. `docs/plans/platform-os-core-manifest-audit.md` reports the workspace still has 26 members, including 8 non-core/staged/private members. `docs/plans/platform-os-schema-freeze-audit.md` reports queue/federation/runtime projection contracts are frozen at evidence level, with open follow-up gaps for standalone JSON Schema files and top-level mutation policy consistency on some projections.

The active queue confirms this plan review remains the next high-priority packet and that follow-on Platform OS implementation tasks already exist for S1/G3, S2 tenant migration, and S3 app/private-consumer extraction.

## Platform OS Contract

Platform OS should preserve these boundaries:

- OS core is small, stable, and auditable.
- Apps and tenants are separately deployable.
- Queue/runtime JSON projections are the OS ABI.
- Public extraction, private tenant separation, and OS-core freezing are related but distinct workstreams.
- Local overnight execution should remain viable without remote providers.
- Network features should stay gated to confirm-only unless approved by policy and operator receipts.

## Frozen 18-Crate Core Surface

The current planned OS core surface is:

- `arda-core`
- `arda-cli`
- `arda-charon`
- `arda-prometheus`
- `arda-apollo`
- `arda-hermes`
- `arda-comm`
- `arda-oracle`
- `arda-governance`
- `arda-plutus`
- `arda-athena`
- `arda-vaire`

## Current Workspace Drift

`Cargo.toml` currently lists 26 workspace members. The 8 non-core members still present are:

- `arda-ceo`
- `arda-onboarding`
- `arda-service-registry`
- `arda-council`
- `arda-forge-mind`
- `arda-signal-grid`
- `arda-fleet`
- `arda-human`

This is acceptable for the current plan-review closeout because the review packet is documentation/evidence closure, not the S1/G3 enforcement task. Enforcement belongs to the queued S1/G3 boundary review gate.

## Runtime and ABI Evidence

| Surface | Review result |
| --- | --- |
| `docs/plans/PLATFORM_OS.md` | Exists; active plan; defines stages S1-S4 and the 18-crate OS core set. |
| `Cargo.toml` | Exists; workspace still has 26 members, including all 18 planned core crates plus 8 non-core/staged/private members. |
| `docs/plans/platform-os-core-manifest-audit.md` | Exists; identifies exact core surface and records workspace mismatch as follow-on enforcement work. |
| `docs/plans/platform-os-schema-freeze-audit.md` | Exists; records queue/federation/runtime projection ABI freeze evidence and open schema hardening gaps. |
| `core/state/queue_active.json` | Exists; active queue count 19 after PROMETHEUS closeout; next active packet is this Platform OS plan review. |
| `core/state/queue_summary.json` | Exists; counts show 97 completed and 19 queued project tasks after PROMETHEUS closeout. |
| `core/state/runtime_settings.json` | Exists; runtime settings projection and environment templates are available. |

## Priority Follow-up Work

1. Execute S1/G3: freeze workspace `Cargo.toml` and add a boundary review gate for new crates.
2. Preserve the 18-crate OS core as the stable surface while tenant/staged crates are migrated or isolated.
3. Keep queue, federation, and runtime-state projection contracts frozen while adding standalone JSON Schema hardening.
4. Execute tenant separation for `arda-human`(arda-human deprecated), council, forge-mind, signal-grid, and service-registry.
5. Extract ARDA HUD, CITADEL Avatar, and RELIC/Kiosk as private consumers under the external/private-consumer pattern.
6. Defer Bluefin bootable proof until first-runtime health checks, queue readability, and runtime state readability are stable.

## Gates

Platform OS work remains subject to:

- Soterion traceability for queue, plan, and runtime evidence;
- Bacon-lite validation for workspace/runtime truth claims;
- boundary review before adding or removing workspace crates;
- Triad review for structural OS policy changes;
- append-only queue integrity before and after same-id task closeout;
- operator approval for destructive migration, external publication, or production-impacting changes.

## References

- Crate/surface: `docs/plans/PLATFORM_OS.md`
- Operator plan: `docs/plans/PLATFORM_OS.md`
- Runtime projections: `core/state/queue_active.json`, `core/state/queue_summary.json`, `core/state/runtime_settings.json`
- Audits: `docs/plans/platform-os-core-manifest-audit.md`, `docs/plans/platform-os-schema-freeze-audit.md`
