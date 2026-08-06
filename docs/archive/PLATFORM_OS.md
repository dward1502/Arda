---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "archived"
  last_reviewed: "2026-08-04"
crate: platform-os
owner: prometheus
status: archived
reviewed: "2026-08-04"
---

> Arda Platform OS: 📜 historical local agent control-plane roadmap | owner: prometheus | status: archived | reviewed: 2026-08-04

# Platform OS Plan

`PLATFORM_OS` is the superseded Arda local agent control-plane planning surface.
Current runtime topology and release work are owned by
`docs/plans/2026-08-02-arda-system-unification-and-usability-plan.md` and the
Stage 5/6 plans.

## Purpose

The Platform OS plan shapes Arda into an auditable local agent control plane that can later become a bootable appliance. The north star remains a local-first runtime with deterministic state and composable application/private-consumer modules. The earlier Bluefin bootable-appliance north star is deferred until first-runtime health checks, queue readability, and runtime state readability are stable.

## Contract

Platform OS preserves these boundaries:

- OS core is small, stable, and auditable.
- Apps and tenants are separately deployable.
- Queue/runtime JSON projections are the OS ABI.
- Public extraction, private tenant separation, and OS-core freezing are related but distinct workstreams.
- Local overnight execution should remain viable without remote providers.
- Network features should stay gated to confirm-only unless approved by policy and operator receipts.

## Frozen 18-Crate Core Surface

The planned OS core surface is:

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

## Current Workspace State

`Cargo.toml` currently lists **17** workspace members (including the root `.` package added to enable `cargo build --bin arda`). The non-core/staged/private members still present are:

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
| `docs/plans/PLATFORM_OS.md` | Exists; active plan; defines the 18-crate OS core set and S1-S3 stages. |
| `Cargo.toml` | Workspace has 17 members; see list above. |
| `core/state/queue_active.json` | Exists; active task count 10; next active packet is this Platform OS plan review. |
| `core/state/queue_summary.json` | Exists; counts show work completed and queued. |
| `core/state/runtime_settings.json` | Exists; runtime settings projection and environment templates are available. |

The earlier `platform-os-core-manifest-audit.md` and `platform-os-schema-freeze-audit.md` references no longer exist in this checkout; their evidence is now reflected directly in this plan and the live state surfaces above.

## Priority Follow-up Work

1. Execute S1/G3: freeze workspace `Cargo.toml` and add a boundary review gate for new crates.
2. Preserve the 18-crate OS core as the stable surface while tenant/staged crates are migrated or isolated.
3. Keep queue, federation, and runtime-state projection contracts frozen while adding standalone JSON Schema hardening.
4. Execute tenant separation for `arda-human` (deprecated), council, forge-mind, signal-grid, and service-registry.
5. Extract ARDA HUD, CITADEL Avatar, and RELIC/Kiosk as private consumers under the external/private-consumer pattern.
6. Defer Bluefin bootable proof until first-runtime health checks, queue readability, and runtime state readability are stable.

## Gates

Platform OS work remains subject to:

- Soterion traceability for queue, plan, and runtime evidence
- Bacon-lite validation for workspace/runtime truth claims
- boundary review before adding or removing workspace crates
- Triad review for structural OS policy changes
- append-only queue integrity before and after same-id task closeout
- operator approval for destructive migration, external publication, or production-impacting changes

## References

- Plan: `docs/plans/PLATFORM_OS.md`
- Runtime projections: `core/state/queue_active.json`, `core/state/queue_summary.json`, `core/state/runtime_settings.json`
