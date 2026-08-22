---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "implementation_plan"
  owner: "MANWE"
  status: "active"
  reviewed: "2026-08-21"
---

> 🜏 Soterion: 📜 implementation_plan | owner: MANWE | status: active | reviewed: 2026-08-21

# Stage 3 — Adaptive Cognition and Work Placement

## Objective

Compose the smallest useful worker set and place each role on a suitable current node/model according to capability, evidence quality, privacy, resource pressure, expected cost, and recovery requirements.

## Boundary

Manwë owns provider/model route selection. The organism composer owns role and node requirements. Hermes executes flexible agent loops. Arda engine owns durable approved attempts and terminal reconciliation. MoA/reference models and councils provide advisory evidence only.

## Work packets

### S3.1 — Define role-based capability requests

Replace fixed agent/model names in objective execution with required roles, input/output contracts, tool/data needs, privacy/egress limits, reasoning depth, durability, and verification requirements.

### S3.2 — Implement bounded composition

Given an objective and context capsule, select no workers when deterministic code suffices, one worker for ordinary reasoning, independent critics only for named material risks, and an adjudicator only for unresolved disagreement. Enforce role and concurrency budgets.

### S3.3 — Join node and Manwë route evidence

Placement considers node capability/pressure plus Manwë model/provider health and route receipts. A model advertised in a catalog but failing current inference is not eligible. Estimated data is labeled and receives lower confidence.

### S3.4 — Execute through the correct lifetime

- conversational/ad hoc work: Hermes session or `delegate_task`;
- process-local specialist profile/Bot: persistent identity but non-durable attempt;
- durable approved work: canonical queue → engine-supervised Hermes worker;
- cross-machine/framework: A2A task;
- deterministic device/service action: engine/systemd/outpost adapter.

Every placement receipt records why this lifetime and node were chosen.

### S3.5 — Validate stronger-hardware transfer

Run the same capability request against a constrained fixture and a stronger node. The selected route may change, but objective, return contract, authority, and receipt semantics remain unchanged.

## Verification

- deterministic composition fixtures;
- no-worker/single-worker/critic/adjudication cases;
- privacy and unavailable-route denial;
- resource-pressure fallback;
- live minimal inference on every route claimed eligible;
- cost/token/latency/energy source metadata;
- cancellation and unknown-on-process-loss behavior.

## Exit gate

One objective is decomposed by role, placed across at least two distinct capability profiles, executed through the correct lifetime, and explained by source-backed placement receipts. Moving a role to a stronger node requires configuration/enrollment only, not workflow code changes.
