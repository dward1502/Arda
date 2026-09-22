---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "implementation_plan"
  owner: "MANWE"
  status: "active"
  reviewed: "2026-09-15"
  tags: ["providers", "routing", "local-inference", "hermes", "workers"]
---

> 🜏 Soterion: 📜 implementation_plan | owner: MANWE | status: active | reviewed: 2026-09-15

# Provider and Worker Convergence

## Outcome and current evidence

Canonical work uses suitable deterministic, local, subscription or hosted capacity
through one policy-bound placement decision with actual-route receipts. Prefer
local capacity when it meets requirements; stronger/paid capacity needs a justified
role and authorized budget, not merely a configured name.

Partial integration. [Historical provider repair](../audits/2026-09-07-objective-cutover-revalidation.md#provider-repair-and-independent-review-follow-up)
records local/subscription routing. The [cutover](2026-09-01-arda-objective-runtime-cutover.md#retained-evidence)
adds bounded installed Hermes execution; neither proves the full selection/fallback
matrix below. Old provider counts, the retired queue-executor route, port 5110
claims and billing/health snapshots are not current runtime truth. Inspect only
redacted configuration and observed route metadata before the next change.

## Work checklist

- [ ] P1 — Reconcile repository Manwë/fleet configuration, live catalog, installed
  service environment and Hermes primary/auxiliary/delegation routes using existing
  inspection surfaces. Produce one redacted comparison; identify stale endpoints,
  credential prerequisites by name, disabled capacity and config/runtime disagreement.
  Do not expose credentials or enable routes to force a passing result.
- [ ] P2 — Trace resident leaf requirements into the existing placement contract:
  task kind, tools, structured output, context floor, privacy, cost/latency limits,
  access tier and review independence. Repair missing bindings, not a new queue.
- [ ] P3 — Bind selected and actual provider/model/node to adapter receipts; reject
  silent divergence. Reuse explicit provider/model selection or the existing
  Manwë-backed Hermes route after inspecting current consumers.
- [ ] P4 — Compose deterministic execution when sufficient, otherwise the capable
  worker; use a materially independent critic for named risk and adjudication only
  for unresolved disagreement. Reuse the autonomous program's role receipts.
- [ ] P5 — Exercise health/cooldown/request/context/capability constraints, explicit
  fallback and cost ceilings. Missing authority produces a decision, never a silent
  cloud transfer or weaker check. Intentionally offline nodes remain unavailable.
- [ ] P6 — Feed verified outcomes, latency, failures, correction rate and cost into
  existing placement learning. Operator correction/revocation overrides learned
  preference; self-reported quality alone cannot promote a route.

## Gate 2 acceptance matrix

Use real canonical work and retain selection inputs, actual route, policy decision,
budget and verification evidence. Provider absence is a limitation, not success.

- [ ] G2.1 — A deterministic task uses no model.
- [ ] G2.2 — A suitable bounded task executes on healthy authorized local inference.
- [ ] G2.3 — A tool/context-demanding task chooses compatible capacity rather than the
  cheapest incompatible model, with actual-route evidence.
- [ ] G2.4 — An authorized local-outage test falls back to an eligible hosted route
  within budget and records why; also prove refusal when no authorized fallback exists.
- [ ] G2.5 — Independent review uses another eligible failure profile/identity.
- [ ] G2.6 — A private local-only task never leaves its permitted privacy boundary,
  including on local failure.
- [ ] G2.7 — Verified outcome feedback changes or justifiably retains subsequent
  placement; correction and revocation remain effective.

Exit gate: policy-constrained actual worker use and truthful fallback/refusal are
visible on canonical runs. Paid-provider billing availability is not mandatory
when an authorized non-paid hosted/subscription route satisfies the scenario.
No synthetic result or catalog health response substitutes for execution.

## Dependencies and verification

Provide the minimal admitted roles needed by [Gate 1](autonomous-task-completion/README.md)
without waiting for every provider to be integrated. Consume device identity from
[Personal System Experience P3](PERSONAL_SYSTEM_EXPERIENCE.md#p3--reconcile-and-use-real-devices-existing-placementconcurrency-owners);
that plan owns role reconciliation, not another placement implementation.

Before changing code, locate the current placement/adapter consumers and their
tests. Run affected policy, route-provenance and fallback/privacy regressions;
then the bounded installed matrix under approved authority. Record exact tested
source and installed configuration identities. Preserve offline optional devices,
credential boundaries and disabled providers. Retire this file after P1–P6 and
G2 evidence are linked from the [whole-system program](ARDA_WHOLE_SYSTEM_COMPLETION_PROGRAM.md).
