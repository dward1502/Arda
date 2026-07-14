---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "plan"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-06-19"
---

# Annunimas Platform OS Plan
Goal: shape Annunimas into an auditable local agent control plane that can later become a bootable appliance, without blocking current operator work.
Scope: crate boundary, queue contract freeze, privatization of tenant crates/apps, staging of unimplemented features.

## North star
Bluefin Linux appliance: USB boots into an Annunimas-first environment with local-first runtime, deterministic state, and composable app modules.

## Current reality
- Core plating is stronger than app maturity.
- Several crates are aspirational means to influence the OS, not required for first-runtime.
- Apps currently share the OS workspace and build surface.
- ANNUNIMAS-HUMAN, council, forge-mind, signal-grid, and service-registry are each implemented or scaffolded enough to plan a migration path, not a greenfield.

## Principles
1. OS core is small and stable.
2. Apps and tenants are separately deployable.
3. Queue/runtime JSON surfaces are the OS ABI.
4. Privatization and public extraction are separate from OS core.
5. Local overnight execution should be possible without remote providers.

## Stage S1. Core freeze
Decision gate: agree the 18-crate core set is stable.
Actions:
- Freeze core crate manifest.
- Freeze queue, federation, and runtime-state JSON schemas.
- Stop adding crates to workspace without explicit boundary review.
- Update architecture docs to reflect frozen surface.

## Stage S2. Tenant separation
Decision gate: choose private repo ownership for withdrawn crates.
Actions:
- annunimas-human -> second-brain tenant.
- annunimas-council -> staged feature project.
- annunimas-forge-mind -> staged feature project.
- annunimas-signal-grid -> staged feature project.
- arda-service-registry -> staged feature project.
- Keep backward-compatible remote references if integration is still desired.

## Stage S3. App extraction
Decision gate: apps compile against extracted core API, not workspace internals.
Actions:
- Extract ARDA HUD as a private tenant of the OS.
- Extract CITADEL Avatar as a private tenant.
- Extract RELIC/Kiosk surfaces as a private tenant.
- Prove tenant build against published core surface.

## Stage S4. Bootable proof
Decision gate: bootable image boots and health checks pass.
Actions:
- Produce an image from Bluefin that boots Annunimas core services.
- Prove first-boot health check via CLI status.
- Prove queue and runtime state are readable at first boot.
- Use `docs/operations/operator-start-of-day-boot-tasks.md` as the bounded operator start-of-day boot checklist for build, user-systemd, CLI status, queue/runtime projection, packaging, and evidence capture.

## Crate disposition

Keep in OS core:
- annunimas-core
- annunimas-cli
- annunimas-charon
- annunimas-prometheus
- annunimas-apollo
- annunimas-hermes
- annunimas-comm
- annunimas-oracle
- annunimas-governance
- annunimas-plutus
- annunimas-athena
- annunimas-mnemosyne
- annunimas-hades
- annunimas-warden
- annunimas-chronos
- annunimas-systemd
- annunimas-tool-harness
- annunimas-mcp

Move out of OS core:
- annunimas-human -> second-brain tenant
- council -> staged feature project
- forge-mind -> staged feature project
- signal-grid -> staged feature project
- service-registry -> staged feature project

## Hermes / comms combination
Assessment: likely combinable and probably worth evaluating.
Rationale:
- COMM is described as shared comms primitives used by Hermes/edge bridges.
- HERMES is the communications/Discord/A2A delivery surface.
- Combined crate could reduce transmission-layer fragmentation.
- Must verify no trait-bound or transport assumption is violated.

## Council redesign
Stage once review-objection loop is ready.
Model: a read responder that ingests audit/output artifacts, returns a verdict/recommendation, and yields control back to the runtime.

## Local overnight viability
- Yes for non-network-backed local models.
- Long-running overnight task execution does not require live remote providers.
- Network features can be gated to confirm-only, never fully autonomous.
- Use local GGUF runners where CUDA/MPS is available; fall back to CPU if not.

## Next action items
- P0: confirm stage gate S1 and finalize the 18-crate core set.
- P1: extract annunimas-human into a private tenant repo.
- P1: evaluate comm+hermes combine without breaking Hermes CLI command surface.
- P2: extract council, forge-mind, signal-grid, service-registry into staged feature projects.
- P3: extract ARDA HUD, CITADEL, RELIC as private consumers of core API.
- P4: produce Bluefin-based bootable proof with health check.

## Risk notes
- Breaking public-extraction plans that already rename these crates must not be disrupted.
- Queue contract freeze requires evidence checks before tagging stable.
