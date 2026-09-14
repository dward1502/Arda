---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "implementation_plan"
  owner: "HADES"
  status: "complete"
  reviewed: "2026-08-22"
---

> 🜏 Soterion: 📜 implementation_plan | owner: HADES | status: complete | reviewed: 2026-08-22

# Stage 5 — Homeostasis, Governance, and Recovery

## Objective

Make the organism detect degradation, conserve resources, reject unsafe expansion, recover work, and remain coherent when nodes, models, networks, or processes fail.

## Architecture

Engine/systemd own deterministic supervision. Aulë projects current state. Governance classifies authority. HADES/Rúmil audit lifecycle and drift. Arandur may propose recovery policy but does not serve as an LLM watchdog. Every recovery transition is receipt-backed.

## Work packets

### S5.1 — Define organism health from direct evidence

Health joins node heartbeat, service state, successful minimal work/inference, queue/attempt state, resource pressure, memory availability, and receipt freshness. Distinguish ready, degraded, intentional-offline, unobserved, unreachable, service-down, routing-drift, and unknown.

### S5.2 — Define homeostasis policies

Bound concurrency, retries, time, context, tokens, cost, CPU/GPU/RAM, thermal/power, network, storage, and operator attention. Policies select pause, degrade, reroute, shed optional work, request review, or stop.

### S5.3 — Reconcile work after failure

On node/process loss, inspect durable attempt and external side-effect evidence before retry. Terminal work wins. Running with no surviving authority becomes unknown until reconciled. Exactly-once actions require idempotency or explicit compensation.

### S5.4 — Preserve authority during adaptation

Rerouting, stronger hardware, council advice, or successful prior behavior cannot lower approval class, widen tools/data/egress, or convert advisory findings into commitments.

### S5.5 — Exercise failure injection

Inject worker crash, node heartbeat expiry, model timeout, network partition, stale route, malformed result, memory unavailability, and restart during handoff. Verify bounded recovery and visible degraded state.

## Verification

- source/current health agreement;
- process and endpoint probes;
- failure-injection fixtures plus at least one live process kill;
- no duplicate terminal receipt or side effect;
- compensation evidence where applicable;
- generated projections remain unstaged;
- operator-visible explanation of degradation and recovery.

## Exit gate

During a real multi-role run, one node/process is stopped. The organism preserves completed evidence, marks the interrupted attempt honestly, reassigns only eligible work, avoids duplicate mutation, completes or fails with a bounded explanation, and remains coherent after root/gateway restart.

## Implemented and verified — 2026-08-22

- `arda-engine::adapters::homeostasis` synthesizes explicit ready, degraded, intentional-offline, unobserved, unreachable, service-down, routing-drift, and unknown states from timestamped direct evidence. Missing or stale evidence never becomes optimistic readiness.
- Conservation policy explicitly bounds concurrency, retries, elapsed time, context/output tokens, cost, CPU, GPU, RAM, thermal, power, network, storage, and operator attention. Exceeded limits deterministically continue, degrade, shed optional work, pause, request review, or stop.
- Recovery reconciliation reads durable attempt state before action. Terminal evidence is preserved, uncertain non-idempotent external effects become `mark_unknown`, retry exhaustion stops, and reassignment requires a directly evidenced ready target whose tools, data, egress, and approval class do not widen authority.
- Recovery decisions are append-only receipts keyed by stable recovery identity and full input digest. A fresh store instance returns byte-stable replay without a second ledger row; changed evidence under the same key fails as a conflict.
- The Stage 5 fixture launches two real worker processes, kills the active worker, observes its terminal process state as `service_down`, reassigns eligible idempotent work to the still-running worker, reopens the durable store, and proves no duplicate mutation. It also covers terminal preservation, unknown side effects, stale heartbeat, intentional-offline state, resource conservation, and authority widening.
- Direct probes against every endpoint in `config/fleet.toml` are recorded separately from the process fixture. At proof time: three configured nodes exposed successful health/models GET surfaces, three active nodes were unreachable, and two nodes were intentionally offline. This evidence does not overclaim successful minimal inference.

Evidence:

- `.hermes/evidence/digital-organism/stage5-homeostasis-recovery-receipt.json`
- `.hermes/evidence/digital-organism/stage5-configured-fleet-probe.json`
