---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "implementation_plan"
  owner: "HADES"
  status: "active"
  reviewed: "2026-08-21"
---

> 🜏 Soterion: 📜 implementation_plan | owner: HADES | status: active | reviewed: 2026-08-21

# Stage 0 — Foundation Salvage and Alignment

## Objective

Produce an authoritative, source-backed map of Arda’s existing organism anatomy and decide what to reuse, adapt, or archive before new runtime work begins.

## Current verified boundary

- Oromë exports A2A message/envelope/thread types in its default build.
- Oromë’s agent registry is available only under tests or `service-runtime`; its `MessageRouter` is test-only.
- Root `arda` composes Oromë’s default feature only.
- Arda engine already supervises Hermes workers and records durable run outcomes.
- Hermes delegation already routes through live Manwë `auto`, but ordinary subagent execution does not automatically obtain canonical Arda queue or receipt lineage.
- Vairë, Varda, governance, Aulë, outpost, operator-bridge, and queue authorities exist at different maturity levels and must not be inferred root-composed from workspace membership.

## Work packets

### S0.1 — Build the organism maturity matrix

**Inspect:** root `src/main.rs`, `services.toml`, workspace manifests, crate roots, systemd units, live endpoints, active plans, and current receipts.

**Create:** `docs/audits/digital-organism-foundation-matrix.md` and a machine-readable sidecar under `.hermes/evidence/digital-organism/`.

For every relevant component record: owner, declared contract, direct consumers, default compile closure, root construction, installed artifact, process/listener, current workflow proof, duplication, and proposed disposition.

**Acceptance:** every claim distinguishes implemented/tested/wired/running/proven; every disposition cites source and consumer evidence.

### S0.2 — Trace one objective through the current system

Trace the present operator objective from Hermes ingress through queue/next-action, Manwë, engine worker execution, Varda/Vairë/Aulë writes, and HUD projection. Record every dead end, duplicated identity, manual step, and lost correlation ID.

**Acceptance:** one sequence diagram shows the actual path and separately labels the intended organism path.

### S0.3 — Reconcile overlapping protocol and authority surfaces

Compare:

- Oromë A2A messages, registry, provider dispatch, and operator bridge;
- Hermes Bot Mode, delegation, A2A, plugins, API/TUI gateway, and session lifecycle;
- outpost node/observation/presence contracts;
- Manwë provider catalog and route receipts;
- Prometheus/Arandur registries, queues, planner, and council;
- Vairë context/memory contracts.

**Acceptance:** one decision table names which layer owns semantic envelope, wire transport, node identity, agent identity, session identity, task identity, memory, approval, placement, and delivery proof.

### S0.4 — Produce the bounded retirement list

Classify stale registries, test-only routers, duplicate task projections, compatibility shims, and decorative/runtime-fiction surfaces. Do not delete in Stage 0.

**Acceptance:** every archive/delete candidate names its successor and proves zero required default-runtime consumers; unresolved cases remain `adapt/inspect`, not guessed deletion.

## Verification

- `cargo metadata --no-deps --format-version=1`
- `cargo tree -p arda --edges normal`
- repeat with exact production feature flags identified from service/unit configuration;
- direct probes of configured endpoints;
- `systemctl --user status arda.service hermes-gateway.service --no-pager` or the actual owning manager;
- active-plan HADES link/completion-language gate;
- `git diff --check`.

## Exit gate

The operator accepts the maturity matrix and the reuse/adapt/archive map. No Stage 1 code begins while ownership of topology, A2A semantics, context, or task authority remains ambiguous.
