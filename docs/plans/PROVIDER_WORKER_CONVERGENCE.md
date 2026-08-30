---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "implementation_plan"
  owner: "MANWE"
  status: "active"
  reviewed: "2026-08-25"
  tags: ["providers", "routing", "local-inference", "hermes", "workers"]
---

> 🜏 Soterion: 📜 implementation_plan | owner: MANWE | status: active | reviewed: 2026-08-25

# Provider and Worker Convergence

## Outcome

Canonical Arda work uses all suitable available capability—deterministic tools, local inference, subscription providers, free cloud, and paid cloud—through one observable placement decision. Local capacity is preferred when it can satisfy the task; stronger or paid capability is selected when evidence, risk, context, tools, or time justify it.

## Verified starting point

- Manwë is live on `127.0.0.1:7171` with 22 configured providers.
- Three providers were ready at the audit snapshot: two local and one OpenAI subscription route.
- Other enabled routes were unhealthy or lacked configured credentials; disabled providers are not available capacity.
- The harness adaptive-placement endpoint selects role-specific node/provider/model profiles and executes through Manwë with actual-route receipt checks.
- The queue executor launches Hermes without provider/model selection; the active Hermes default is `openai-codex`.
- Hermes delegation is routed through Manwë, but Workbench queue execution is not.
- Some Hermes auxiliary custom-provider endpoints target inactive port `5110` while the live Manwë endpoint is `7171`.

## Implementation sequence

### P1 — Configuration reconciliation

Produce one redacted reconciliation command that compares repository Manwë config, live provider projection, Hermes provider/auxiliary/delegation routes, fleet capability records, and installed service environment. Fail visibly on stale endpoints, absent credentials by name, disabled expected capacity, or runtime/config disagreement.

### P2 — Queue placement contract

Add placement requirements to queue tasks: task kind, tools, structured output, context floor, privacy domain, maximum cost, latency class, review independence, and allowed access tiers. Send those requirements to Manwë before Workbench execution.

### P3 — Adapter routing

Either pass the selected provider/model explicitly into Hermes or execute through a Manwë-backed Hermes-compatible route. The adapter receipt must contain selected and actual provider/model IDs and reject silent route divergence.

### P4 — Role composition

Use deterministic code when sufficient. Otherwise select a worker. Add a critic for named material risk and an adjudicator only for unresolved material disagreement. Review should normally use a distinct failure profile, not duplicate the worker blindly.

### P5 — Health and fallback

Respect live health, cooldown, context, capability, request limits, and cost. Fallback must be explicit in the receipt. Local failure may route to subscription/paid cloud within budget; missing authority or budget produces a decision request rather than silent degradation.

### P6 — Learning

Feed verified task outcomes, latency, failure class, correction rate, and cost into placement learning. Never promote a route from self-reported quality alone. Revocation and operator correction override learned preference.

## Acceptance proofs

1. A deterministic task uses no model.
2. A suitable bounded task executes on healthy local inference.
3. A tool/context-demanding task selects a capable route rather than the cheapest incompatible model.
4. A forced local outage falls back to an authorized hosted route and records the reason.
5. A worker result receives independent review on another eligible profile.
6. A private task remains local even if hosted capacity is stronger.
7. Actual route, budget, and verification evidence are visible in the canonical run.

## Done

Provider breadth is not complete because configuration lists many names. It is complete when canonical tasks demonstrably consume the right live capacity under policy, and failures produce truthful fallback or blocked decisions.
