---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "architecture_audit"
  owner: "HERMES"
  status: "draft"
  reviewed: "2026-10-01"
---

> 🜏 Soterion: 📜 architecture_audit | owner: HERMES | status: draft | reviewed: 2026-10-01

# Assessment decisions and deeper audit sequence

## What I would decide before building more

1. Keep the broad personal-system vision, but choose a small number of ordinary outcomes that justify it. The existing doctrine and master plan remain authoritative; this assessment is not a new backlog.
2. Make ownership explicit: who may accept work, authorize it, execute it, record it and project it. A projection, receipt or declaration must not become a second authority by accident.
3. Treat installation and user experience as part of architecture. A home-installable system needs predictable startup, upgrade, recovery, private-state handling and a clear interface, not only compile-active crates.
4. Prefer completing existing connections over introducing new subsystems. Refactor boundaries first, as AGENTS requires, then alter behavior in small verified slices.

## Suggested deeper audit batches

These are review scopes, not instructions to implement automatically. Each batch should extend this assessment with evidence rather than repeat the inventory.

| Order | Batch | Questions / concrete exit |
|---|---|---|
| 1 | Verification baseline | Understand excluded_refs fixture semantics, inventory all build roots/features, establish reproducible checks. Exit: exact passing/failing matrix, no inferred green state. |
| 2 | Authority and persistence | Trace Engine objective intake -> approval -> lease/snapshot -> execution -> verification -> close/recovery; include Aule explicit adapter and legacy refusals. Exit: one authoritative writer per transition, documented retries and crash windows. |
| 3 | Information-to-action loop | Trace Hermes/operator ingress -> research ingestion -> assessment -> authorized objective -> useful retained result -> Vaire recall. Include refusal, unavailable provider and stale evidence. Exit: caller/store graph and separately observed useful outcomes. |
| 4 | Personal and communication paths | Trace personal operations, proactive evaluation/delivery, Orome and operator review. Exit: no inference that a projection or scheduled record equals actual delivery. |
| 5 | Native experience and deployment | Trace launcher, HUD, Mirromere, shared UI, adapters, service registry and installed systemd composition. Exit: source/build/install/runtime roles reconciled; native behavior tested separately from fixtures. |
| 6 | State lifecycle and retirement | Classify core/data/config/audit/.hermes paths; inspect candidate call sites and external invocation. Exit: explicit retain/archive/delete proposals with privacy and recovery implications. |
| 7 | Large-file maintainability | Review Aule, Manwe, Varda and HUD concentrations after their authority boundaries are known. Exit: bounded refactors justified by responsibility, not arbitrary file size. |
| 8 | Release/security/operability | Dependencies, secret boundaries, endpoint authentication, upgrades, backup/restore, telemetry failure and outposts. Exit: supported deployment envelope and honest remaining risks. |

## Representative outcomes, not more demonstrations

- Discover/review a useful source, propose an action, obtain the required authority, execute it and later explain/reuse the retained result.
- Capture a personal commitment, present a timely suggestion, distinguish approval from delivery and survive restart without duplicates.
- Carry an authorized multi-project task through real verification and independent review, then recover safely from interruption.
- Converse through Hermes on the intended display while relevant context/media is shown and dismissed reliably; do not substitute a static dashboard for the embodied experience.

These examples cover different aspects of the same system. They are not all verified by this audit, and coding is not the product's definition.

## Questions for operator assessment

- Which existing outcomes are already useful enough to preserve unchanged?
- Which interface should feel primary day to day, and what is each other surface for?
- Which stored evidence is essential versus reproducible noise?
- Which experimental capabilities should remain visible, and which should be clearly unavailable until proven?

My recommendation: do not start a broad rewrite or delete legacy-looking directories. Start with the failing validation baseline and execution ownership map; then assess a real cross-domain outcome. That gives a defensible basis for consolidation instead of replacing one unproven structure with another.
