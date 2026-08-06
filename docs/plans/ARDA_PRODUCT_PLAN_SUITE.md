# Arda Product Plan Suite

**Status:** Active portfolio index  
**Updated:** 2026-08-05
**Strategic source:** [Arda 1.0 Product Readiness Assessment](ARDA_1_0_PRODUCT_READINESS.md)
**Completion authority:** [Arda System Unification and Usability Plan](2026-08-02-arda-system-unification-and-usability-plan.md)

This index preserves the complete first-party application vision while keeping Arda 1.0 focused on one supportable product promise.

## Product decision

**Arda Workbench is the Stage 4–6 release-critical product.**

The other first-party applications reuse the same kernel contracts and progress independently. They do not delay Workbench 1.0 and cannot silently create parallel task, memory, policy, identity, communications, or receipt systems.

The product and subsystem scope is frozen to the existing documented plan estate. Remaining development completes, composes, simplifies, hardens, and makes that system usable; it does not add new agents, applications, or parallel authorities.

## Finite active-plan ownership ledger

This table is the portfolio classification and ownership authority required by
U0. An unchecked checklist item is owned only by the plan named in its row and
is accepted only at that plan's stated gate. Cross-links express dependencies;
they do not transfer or duplicate implementation ownership.

| Active authority | Classification | Open ownership and acceptance surface | Release relationship |
|---|---|---|---|
| [Stage 5 Release Candidate](2026-07-29-stage-5-release-candidate-plan.md) | **release-critical** | Owns its 3 unchecked RC gates and its release evidence packet. | The finite blocker queue is final signed-artifact reconciliation, one valid uninterrupted 24-hour reliability receipt from the frozen final source, and one qualifying independent non-author evaluator receipt. The security gate closed through the bounded, checksum-pinned upstream `glib 0.18.5` safety backport on 2026-08-05. |
| [System Unification and Usability](2026-08-02-arda-system-unification-and-usability-plan.md) | **release-critical** | Owns U1–U6 composition, workflow, recovery, install, usability, and closeout requirements. U0 is closed by this ledger. | Coordinates Stage 5/6 without replacing either release authority. |
| [Stage 6 Legitimate 1.0](2026-07-29-stage-6-legitimate-1.0-plan.md) | **dependency-blocked** | Owns its 11 release-decision/evidence checks. | Cannot begin as a release decision until Stage 5 closes. |
| [Personal Operations](2026-07-29-personal-operations-plan.md) | **optional but bounded** | Owns its 5 remaining private-alpha checks and plan-local acceptance evidence. | Never a Workbench 1.0 blocker. |
| [RELIC/CITADEL](2026-07-29-relic-citadel-plan.md) | **optional but bounded** | Owns its 5 final soak/operator-acceptance checks. | Feature-flagged projection; expansion remains blocked by its own base acceptance, not Workbench. |
| [HUD Universal Agent Monitor Surfaces](2026-08-03-arda-hud-universal-agent-monitor-surfaces-plan.md) | **optional but bounded** | Owns its 19 corrective monitor-session and native visual acceptance checks. | Operator acceptance in the running HUD is required for this surface only. |
| [Mirromere](2026-07-29-mirromere-plan.md) | **optional but bounded** | Owns all enumerated monitor-alpha tasks and gates; physical-mirror work remains deferred until after Workbench 1.0. | Opt-in application; no Workbench release dependency. |
| [Company Operations](2026-07-29-company-operations-plan.md) | **dependency-blocked** | Owns the four real Stage 6 pilot observations. | Blocked on operator-selected real engagement, approval, delivery, and reviewed-outcome evidence; none may be fabricated. |
| [External Product Integration](2026-07-29-external-product-integration-plan.md) | **optional but bounded** | Portfolio research record only. It authorizes no implementation by itself; a measured need must create a candidate-specific bounded plan with its own acceptance gate. | No candidate or spike is release-critical. |
| [Federated Communications](FEDERATED_COMMS.md) | **optional but bounded** | Doctrine/state authority only; it owns no open Workbench checklist. | Future transports remain adapters above local sovereign control. |
| [Rúmil project audit](../../crates/spine/runtime/arda-rumil/README.md) | **optional but bounded** | Read-only project audit coordination, normalized findings, baselines, comparisons, and review-only organization plans. | No Rúmil finding authorizes mutation or blocks Workbench unless a release gate explicitly consumes its evidence. |

Completed Warden Research work is archived and remains the historical authority
for its 29 accepted checks. [Manwe Single-Runtime Convergence](../archive/MANWE_SINGLE_RUNTIME_CONVERGENCE.md)
is also archived after U1 accepted its nine convergence checks and preserved the
coordinated `:7171` contract. The stale OpenFang and Platform OS planning
surfaces are archived: their unique still-relevant boundaries are now owned by
the U1 topology result, Rúmil's review-only audit surface, Federated Communications, External Product
Integration, and the corrective HUD plan rather than parallel architecture
queues.

## Stage plans

| Stage | Outcome | Plan |
|---|---|---|
| 4 — Private Beta | complete and operator-accepted; independent evaluator evidence is optional supplementary validation | [Archived Stage 4 Private Beta](../archive/2026-07-29-stage-4-private-beta-plan.md) |
| 5 — Release Candidate | active: independently installable, secure, recoverable, supportable candidate | [Stage 5 Release Candidate](2026-07-29-stage-5-release-candidate-plan.md) |
| 6 — Legitimate 1.0 | signed supported Workbench release with independent user evidence | [Stage 6 Legitimate 1.0](2026-07-29-stage-6-legitimate-1.0-plan.md) |

## First-party application plans

| Application | Purpose | Target maturity | Plan |
|---|---|---|---|
| Arda Workbench | sovereign governed development environment | Stage 4 implementation tranche complete; Stage 6 supported | [Completed Workbench Private Beta implementation plan](../archive/2026-07-29-arda-workbench-private-beta-plan.md) |
| Personal Operations | capture, reminders, schedule, recovery, contacts | Stage 5 private alpha; optional beta | [Personal Operations](2026-07-29-personal-operations-plan.md) |
| Mirromere | consent-governed ambient voice/avatar projection | Stage 5/6 preview | [Mirromere](2026-07-29-mirromere-plan.md) |
| Warden Research | bounded cited research and watchlists | implemented and accepted; historical optional-beta record | [Archived Warden Research](../archive/2026-07-29-warden-research-application-plan.md) |
| RELIC/CITADEL | read-only geometric runtime presence | optional Stage 5 projection beta; expansion blocked pending base acceptance | [RELIC/CITADEL](2026-07-29-relic-citadel-plan.md) |
| Company Operations | opportunity, client, experiment, value, and commitment cockpit | Stage 5 internal alpha; Stage 6 beta | [Company Operations](2026-07-29-company-operations-plan.md) |

## External product integration

[External Product Integration Research and Adoption Plan](2026-07-29-external-product-integration-plan.md) records the live research, candidate matrix, boundaries, spikes, and stage sequence.

Priority:
1. complete existing MCP and OpenTelemetry boundaries;
2. choose a Workbench graph renderer;
3. add standards-first iCalendar/CalDAV and optional local STT;
4. benchmark databases, voice frameworks, communications, CRM, and external agents before adopting them;
5. keep all optional products removable and subordinate to Arda contracts.

## Implementation authorities and completed supporting records

Active plans remain authoritative for their implementation domains; completed
supporting records preserve verified dependencies and operator procedures:

- [Warden → Varda → Aulë governed learning loop](../archive/2026-07-27-warden-varda-ceo-learning-loop.md) — completed Warden/Varda backend receipts and authority
- [Pi5 deployment, fleet, and recovery](../archive/2026-07-23-pi5-outpost-integration-plan.md) — completed AArch64 delivery, fleet/SSH truth, and shared node recovery record
- [Manwe single-runtime convergence](../archive/MANWE_SINGLE_RUNTIME_CONVERGENCE.md) — completed U1 governed-runtime, `:7171`, admission-fairness, and process-smoke authority
- [RELIC/CITADEL](2026-07-29-relic-citadel-plan.md) — canonical presence and presentation authority
- Embodied interface planning is consolidated into [RELIC/CITADEL](2026-07-29-relic-citadel-plan.md) and [Mirromere/RELIC outpost vision](MIRROMERE_RELIC_OUTPOST_VISION.md). The earlier standalone `plans/EMBODIED_INTERFACE.md` was stale and has been removed; do not reference it.
- [Federated communications](FEDERATED_COMMS.md)
- [Rúmil project-audit authority](../../crates/spine/runtime/arda-rumil/README.md)

Where an older plan contains stale Annunimas names, paths, ports, or machine claims, current Arda source and runtime evidence win. The new plans define product scope; existing domain plans retain implementation detail until audited, merged, or closed.

## Dependency graph

```text
Arda core contracts + existing service receipts
        |
        +--> Workbench run graph + project contract
        |       +--> Stage 4 private beta
        |       +--> Stage 5 adapter/release hardening
        |       +--> Stage 6 Workbench 1.0
        |
        +--> Warden/Varda research --> Workbench + Company Operations
        |
        +--> Personal Operations --> Mirromere projection
        |                               +--> optional presence adapters
        |
        +--> runtime presence projection --> RELIC/CITADEL
        |
        +--> Workbench + Research + Economics + Oromë
                                --> Company Operations
```

## Execution order

### Completed — Stage 4 critical path

The Workbench contracts, durable engine/recovery path, HUD objective-to-review flow, Rust/Python adapters, explicit-question Warden/Varda evidence path, and isolated clean-profile install/recovery evidence are complete and operator-accepted. External-person evaluation remains optional supplementary validation because no separate evaluator or clean machine is available.

### Parallel bounded design work

- [x] Preserve Personal Operations capture and reminder contracts in `arda.personal-ops.v1`; service and UI activation remain deferred.
- [x] Audit Mirromere/RELIC prototype provenance; keep unlicensed/non-Git external source out of Arda.
- [x] Review external-product spikes against open Stage 4 gates; no spike is authorized because current blockers are internal contracts, native commands, adapters, and UI flows.
- [x] Repair touched legacy operational truth without launching broad migration work.

### Now — Stage 5 release candidate

1. Reconcile the final signed artifact set against local packaging and lifecycle evidence.
2. Close the remaining Workbench security gate and obtain one valid uninterrupted 24-hour reliability receipt.
3. Execute system unification through the [unification and usability plan](2026-08-02-arda-system-unification-and-usability-plan.md); do not expand product scope.
4. Require one qualifying independent non-author evaluator receipt for Stage 5; broader cohort repetition remains optional, and proxy sign-off must never be fabricated.
5. Start secondary betas only after their independent dependencies pass.
6. Keep Mirromere, RELIC/CITADEL, and Company Operations feature-flagged and independently recoverable; do not begin optional RELIC/CITADEL expansion before its base projection beta closes.

### Stage 6

- Freeze Workbench 1.0 scope.
- Ship only extensions that independently meet safety, recovery, packaging, licensing, and support gates.
- Classify all remaining applications honestly as beta, preview, or research.

## Portfolio rules

- A plan is not evidence that a feature exists.
- A completion claim includes focused tests and the plan's release gate.
- A first-party application reuses kernel authorities rather than cloning them.
- A new task must close an existing plan item, defect, usability failure, operational gate, or measurable improvement; otherwise it is out of scope.
- New external dependencies require a measured spike, license/SBOM review, failure tests, and removal instructions.
- Hardware and sensor paths are opt-in and not Workbench dependencies.
- Commercial progress is measured by operator time recovered, useful repeat usage, paid delivery evidence, and realized—not forecast—value.
- Completed active plans are removed or archived according to repository policy; this index must be updated at the same time.
