---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "current_state_audit"
  owner: "HADES"
  status: "active"
  reviewed: "2026-08-20"
---

> 🜏 Soterion: 📜 current_state_audit | owner: HADES | status: active | reviewed: 2026-08-20

# Core Arda Usefulness Audit — Review Before Expansion

## Operator objective

Review Arda comprehensively and introspectively against the actual operator vision, record the evidence, plan the smallest authoritative repairs, execute bounded core fixes, and assess whether the human workflow becomes useful.

The intended loop is:

```text
review → record → plan → execute → assess
```

This audit does not authorize Mirromere, RELIC expansion, sensors, external accounts, public agent posting, wallet creation, trading, payments, or commercialization. Economic intelligence and an Arandur agent-community presence are operator-authored future objectives, but funds, accounts, posting, and deployment remain explicit gates.

## Live evidence reviewed

Evidence was collected from the running workstation and current source on 2026-08-20 PDT:

- `arda.service`, `arda-hud.service`, `hermes-gateway.service`, `arda-varda.service`, and `arda-metrics-exporter.service` were active/running.
- `arda-mirromere.service` was inactive/dead.
- The harness answered on `127.0.0.1:7878`.
- Personal Operations returned an honest empty state.
- Research returned zero current questions and watchlists plus one old cancellation-safety brief.
- The live operator projection and `/v1/runs` still exposed historical acceptance runs because the installed root binary predates the source repair.
- The canonical project queue now contains three operator-authored objectives; generated queue projections have not yet been refreshed from that authority.
- The metrics exporter was repaired and verified on loopback `127.0.0.1:9101` with stable `NRestarts=0`.
- Cargo metadata shows 18 workspace packages. The root default dependency closure includes the engine, core, governance, Aulë, Oromë, Vairë, Varda, Rúmil, Mandos, Manwë, and outpost protocol.

## Introspective verdict

Arda has a large, technically serious kernel, but the kernel is not yet composed into the operator relationship described by the product doctrine. The dominant failure is not missing schemas or crates. It is that implemented capabilities frequently stop at one of four boundaries:

1. compiled but never invoked by the root path;
2. exposed by an endpoint but not reached naturally from Hermes/HUD;
3. durable but not converted into one trustworthy cross-domain next action;
4. tested with fixtures but not installed, used, and accepted by the operator.

The running system therefore feels like infrastructure because it mostly reports services, stores, fixtures, and projections. It does not yet carry a normal operator statement through recognition, durable intent, context, one useful next action, review, execution, receipt, and restart recovery.

## Capability maturity matrix

| Capability | Implemented/tested | Root composed | Operator reachable | Workflow/usefulness verdict |
|---|---:|---:|---:|---|
| Hermes conversation and gateway | yes, external Hermes authority | service active | yes in Hermes | Arda operator-message bridge exists, but live gateway-to-Arda delivery is not yet proved in this audit |
| Canonical project objectives/tasks | queue and append-only authority exist | projection producer exists | repaired source path from `arda objective` | three genuine objectives recorded; installed runtime/projection refresh still pending |
| Personal Operations capture | yes | harness route active | HUD and Hermes command source paths exist | usable controls repaired in source; no real content or operator acceptance yet |
| Classification/scheduling/reminders | yes | harness routes active | repaired HUD controls | real reminder delivery transport and operator dogfood remain open |
| Context resume | yes | harness endpoint active | Hermes `arda context` and HUD source paths | empty state is honest; no cross-domain synthesis of queue + Personal Ops + research yet |
| Research questions/watchlists | yes | harness routes active | HUD active | Warden outage formerly discarded intake; repaired source is not installed yet; Hermes natural-language research intake remains incomplete |
| Research briefs | yes | active | HUD active | old orphan brief appears current in installed runtime; repaired historical/stale labeling awaits install |
| Workbench runs | extensive | harness active | HUD active | installed runtime still shows old proofs; source now separates current-run registry from historical evidence |
| Capability composition | contracts and executor helper exist | no production call site found | no direct operator path | implemented library, not operational orchestration |
| Proactive cycle | durable store and tests exist | no production constructor/use found | no operator path | infrastructure only |
| Calendar adapters | ICS/CalDAV types and tests exist | no production constructor/use found | no configured HUD/runtime path | implemented adapter library, not active calendar automation |
| Governance enforcer | canonical engine enforcer exists | run-level integration exists in source | indirectly through run path | requires installed runtime and a genuine consequential flow to prove non-bypass |
| Vairë continuity | substantial crate and consumers | partial | indirect | restart stores exist, but the operator has not proved same-context return through Hermes/HUD |
| Manwë fleet truth | substantial | active | status projections | readiness can outlive endpoint truth; freshness repair remains open |
| Economic capability | partial commercial/payment contracts | not attached to base flow | none | future-gated; no funds authority |
| Agent-community/Moltbook presence | absent as a governed product path | no | no | research-only future objective; account/public/deployment gates remain closed |

## Root causes

### 1. Composition is treated as presence

A dependency edge or module export is repeatedly described as capability availability. Current source confirms several important helpers have no production call site, including dynamic capability composition, the proactive-cycle store, and calendar synchronization.

### 2. Installed runtime lags verified source

The running `arda.service` uses `~/.local/bin/arda`. The source release artifact predates the current repairs, and the running endpoints still show historical Workbench/research behavior fixed in source. Source verification is therefore not runtime repair until the root binary is rebuilt, installed, restarted, and re-probed.

### 3. No canonical cross-domain next-action selector

The queue, Personal Operations, Workbench, and research each have projections, but no current root path selects one trustworthy next action from operator-authored commitments while disclosing source, freshness, authority, and why it is next.

### 4. Intake remains command-shaped

The Hermes bridge recognizes explicit `arda ...` commands. That is useful plumbing, but the product vision requires Arda to recognize capture, question, context request, and consequential intent from ordinary operator interaction without making the operator remember internal verbs.

### 5. Fixture evidence remains too close to current-state surfaces

Historical proof is retained correctly, but installed projections still treat retained run/brief stores as current work. Source repairs establish explicit current-run membership and historical brief labeling; installed-runtime verification remains mandatory.

### 6. Recurring loops are mostly declarations

A proactive-cycle store, autonomy configuration, research paths, and systemd timers exist, but recurring execution must be tied to operator-authored objectives, source freshness, budgets, deduplication, review, and delivery receipts. A timer or process does not prove a useful loop.

## What is now genuinely recorded

The canonical queue contains these operator-authored objectives:

1. comprehensive core-Arda review, record, plan, execute, and assess;
2. governed economic-intelligence routines and a future explicitly budgeted agent account;
3. a governed Arandur Pi5 agent-community/Moltbook listening-post evaluation.

The latter two are `future-gated`, with no financial authority and no external-account authority.

## Immediate plan

The execution plan created from this audit is retained as the superseded
[`Core Arda Usefulness Repair`](../archive/2026-08-20-core-arda-usefulness-repair.md).
The operator later rejected its reduced interface-journey framing; active
authority is the
[`Arda Whole-System Completion Program`](../plans/ARDA_WHOLE_SYSTEM_COMPLETION_PROGRAM.md).
Its original order was:

1. install and verify the already-tested core source repairs;
2. make the three operator-authored objectives visible through generated projections;
3. add one source-truth next-action projection across current commitments;
4. prove natural Hermes/HUD capture and context recovery without fixture data;
5. finish fleet freshness truth;
6. assess with genuine operator use before enabling optional expansion.

## Assessment boundary

The engineering assessment is: substantial kernel, incomplete product composition.

The operator acceptance assessment is: open.

No test count, active service, queue row, endpoint, or audit document is evidence that Arda is useful to the operator. The next valid acceptance evidence is one genuine operator journey that survives restart and returns one useful next action with review and receipts where required.
