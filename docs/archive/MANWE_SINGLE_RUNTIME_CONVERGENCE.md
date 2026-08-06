---
title: Manwe Single-Runtime Convergence Plan
status: complete
date: 2026-08-02
owner: Manwe / Arda runtime
related:
  - STATUS.md
  - BREAKDOWN.md
  - PROVIDERS.md
  - README.md
  - docs/archive/MANWE_FOUNDATION_CHECKLIST.md
---

# Manwe Single-Runtime Convergence Plan

**Status:** Complete — 2026-08-04
**Date:** 2026-08-02
**Decision owner:** Runtime / Manwe maintainers

## 1. Decision

**Manwe has converged to a single production runtime.**

The adaptive / governed path becomes the sole production implementation.
Governance, tool-fit recording, learning signals, and task-class routing remain inside the crate — they are core to Manwe’s identity.

The former static path was migration scaffolding and is retired. The governed runtime now owns the executable, provider catalog, state, transport, routing, learning, and observability surfaces.

“Lite” or deterministic behavior (if ever required for debugging or constrained environments) will be expressed as configuration of the single runtime, not as a parallel code path or second binary.

This decision corrects the translation loss that turned a piece-by-piece migration scaffold into a permanent dual-mode architecture.

---

## 2. Context and Problem Statement

### Origin of dual mode

Dual mode (static fleet gateway vs full adaptive/governed service) was introduced during the transfer from the earlier CHARON-era implementation. The original intent was incremental and correct:

> Validate local connections piece-by-piece so that fleet probing, resource groups, eligibility, fallbacks, and basic forwarding could be proven before layering richer governance and selection logic.

Somewhere in the process that safety scaffold calcified into an intentional dual-runtime design. The convergence step was never completed. The result is two parallel implementations that share only public types and the external contract while maintaining separate catalogs, transports, config universes, selection logic, and state models.

### Why this is now a problem

- The static path cannot deliver the product vision (tool-fit learning, Hermes-aware task classification, multi-provider registrar that covers free and paid cloud, concurrent mixed workloads).
- Dual maintenance tax is permanent and compounding.
- Learning signals already exist in the adaptive surface (tool_fit_ledger, bandit, lane fitness, capability receipts, route receipts) but are currently unmonitored and not clearly driving selection.
- Operator and maintainer cognitive load remains high (“which mode am I running?”, two config models, scoped capability claims).
- The original piece-by-piece goal has been inverted: instead of one path that grows capability, two paths must both stay correct forever.

The 2026-07-27 foundation work (source-graph reconciliation, verification matrix, process smoke, documentation cleanup) makes convergence safer than it would have been earlier. The residue is gone and the boundaries are explicit.

---

## 3. Clarified Product Vision

Synthesized directly from the current requirements:

Manwe should be **the best local inference router / boundary** that can:

1. **Discover, register, and use** heterogeneous providers with a uniform model:
   - Local inference (llama.cpp and similar)
   - Free cloud providers (subscription or account based)
   - Paid cloud providers

2. **Understand request semantics**, especially:
   - Hermes-agent style OpenAI-protocol tool calls
   - Pure chat / avatar conversation
   - Other task shapes that appear in multi-agent swarms

3. **Record and learn**:
   - Which tools work best with which models
   - Model quirks (what succeeds, what fails, under what conditions)
   - Outcomes by task class
   - Continuously improve routing decisions from that data

4. **Support concurrent mixed workloads** in one runtime:
   - Orchestrator / swarm agents performing tool calls and building
   - Simultaneously a user chatting with an avatar
   - Correct model selection for each concurrent task class so tool work and chat do not interfere or receive inappropriate models

5. **Keep governance inside the crate**. Governance is part of Manwe’s identity. Extracting it would force a separate product and is explicitly out of scope.

Everything already funnels through the engine → Manwe path, so a single runtime is both desirable and operationally clean.

---

## 4. Pre-Convergence Snapshot (2026-08-02)

| Area | Status |
|------|--------|
| Foundation baseline | Complete (2026-07-27). Checks, clippy, tests, process smoke, docs validator all green. |
| Local connections | Stable. llama.cpp has had no issues. |
| Primary traffic today | Local use; mostly routes to gpt-5.6 because local context is too small for tool work. |
| Adaptive path | In local use. Governance and learning surfaces exist. |
| Learning signals | Written (tool_fit_ledger, bandit, lane fitness, receipts, capability receipts) but **not monitored** and not clearly driving selection. |
| Dual mode | Still present. Static remains the supervised default in services.toml. |
| Downstream | All traffic already goes through engine → router service. Cut-over surface is small. |
| Resource groups | Designed for physical ownership + concurrency; suitable foundation for mixed workloads. |

---

## 5. Architecture Decision Record (Summary)

**Decision:** Converge on the adaptive/governed runtime as the single production path.

**Rationale:**
- Only the adaptive surface can deliver tool-fit learning, rich task-class routing, Hermes awareness, and the multi-provider registrar required by the vision.
- Governance is core identity and must stay inside the crate.
- Dual mode was transitional scaffolding; its isolation benefit has already been harvested.
- Continuing dualism permanently prevents the coherent learning router the project needs.

**Consequences:**
- Static path is frozen and scheduled for retirement.
- New capabilities land only in the adaptive tree (or a thin shared core extracted opportunistically).
- “Lite” behavior becomes configuration of the same runtime.
- Documentation language shifts from “two selectable runtimes” to “one runtime with progressive capabilities”.
- Process smoke, local llama.cpp parity, and engine integration remain the hard safety gates.

---

## 6. Implemented Architecture Shape

```
                    ┌─────────────────────────────────────┐
                    │         Manwe (single runtime)      │
                    │                                     │
  Providers ───────►│  Unified enrollment + health +      │
  (local / free /   │  capability + fitness overlays        │
   paid)            │                                     │
                    │  Request classification             │
                    │  (Hermes tool-call vs chat vs ...)  │
                    │                                     │
                    │  Resource-group lease + concurrency │
                    │                                     │
                    │  Scoring / bandit / lane fitness    │
                    │  + governance / policy              │
                    │                                     │
                    │  Forward + headers + receipts       │
                    │                                     │
                    │  Continuous recording of            │
                    │  tool↔model fitness & quirks        │
                    └─────────────────────────────────────┘
                                      │
                                      ▼
                             Engine / Hermes-agent /
                             Swarm orchestrators /
                             Avatar chat clients
```

Key properties:
- One HTTP entry point and one config/state universe.
- Governance stays inside.
- Learning signals are both written **and consumed**.
- Task class is first-class in selection.
- Resource groups remain the physical concurrency primitive (shared or isolated according to workload needs).
- Port 7171, OpenAI-compatible surface, and receipt format remain the stable external contract.

---

## 7. Closed Gaps (historical checklist)

1. **Open learning loop** (highest product leverage)
   Signals exist but are unmonitored and not demonstrably driving better routing. “Hope it keeps agents aligned” is not yet “the system learns and we can see it”.

2. **Task-class / Hermes classification**
   Needs hardening so tool-call traffic is reliably distinguished from pure chat and other swarm shapes.

3. **Dual maintenance and dual documentation**
   Still present and actively costly.

4. **Provider model uniformity**
   Local works; free-cloud and paid-cloud need the same first-class enrollment, health, capability, and fitness treatment.

5. **Observability of selection reasoning**
   Operators and agents currently lack a clear “why this model/provider was chosen” signal.

---

## 8. Completed Piece-by-Piece Sequencing

The original incremental spirit is preserved — applied now to convergence and to closing the learning loop.

### Phase 0 — Immediate (no behavior change)
- Freeze all new feature work on the pure static path.
- Document this plan and update STATUS.md language to reflect the decision.

### Phase 1 — Safety & Cut-over Preparation
- Validate that the adaptive path already handles local llama.cpp, resource groups, basic task routing, and receipts at least as well as static.
- Use existing process_smoke plus a short real-traffic validation window.
- Prepare services.toml / launcher / engine discovery changes (but do not flip yet).

### Phase 2 — Promote Adaptive
- Switch the supervised production path to the adaptive runtime.
- Keep a short-lived static binary only if any hard dependency still exists; remove it as soon as possible.
- Confirm engine → router funnel continues to work unchanged.

### Phase 3 — Close the Learning Loop (highest leverage)
- Surface existing signals: tool_fit_ledger, bandit state, lane fitness, capability receipts, recent route outcomes by task class.
- Tag every receipt with clear `task_class` (tool_call / chat / other) and presence of Hermes-style tool schema.
- Ensure selection actually consumes these signals.
- Add a lightweight “why this route was chosen” diagnostic (header, status field, or operator projection).
- Make the data queryable for operators and for further tuning.

### Phase 4 — Hermes & Task-Class Hardening
- Confirm exact request shape of Hermes-agent tool calls as they arrive at Manwe.
- Harden classification and routing so tool work and chat are treated as distinct first-class concerns.

### Phase 5 — Unified Multi-Provider Registration
- One enrollment + health + capability + fitness model that cleanly covers:
  - Local (llama.cpp and peers)
  - Free cloud (subscription / account)
  - Paid cloud
- Expand discovery and registration surfaces as needed.

### Phase 6 — Concurrent Mixed Workload Validation
- Confirm resource-group + concurrency settings correctly support simultaneous swarm/tool work and avatar chat.
- Tune isolation vs sharing based on real behavior.

### Phase 7 — Retirement
- Delete remaining static-only modules.
- Remove dual-mode language from README, BREAKDOWN, PROVIDERS, STATUS, and operator docs.
- Update any remaining tests and smoke coverage to the single runtime.

---

## 9. Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Promoting complexity whose learning value is still unmeasured | Make observability of tool-fit / task-class / selection reasons the first concrete work after (or slightly before) cut-over. |
| Breaking the stable llama.cpp path | Explicit parity validation + process_smoke + short real-traffic window before flipping supervised process. |
| Concurrent tool + chat workloads interfere | Early validation of resource-group behavior; support both shared and deliberately isolated groups. |
| Hermes request shape unclear | Capture exact headers / tool-schema presence / model patterns as an early input to Phase 4. |
| Binary / dependency weight of full adaptive | Acceptable because governance is core identity. Use runtime config / feature flags for lighter operational modes if ever required; never a second code path. |
| Drift during the short dual window | Freeze static; all new work lands only in adaptive (or a thin shared core). |

---

## 10. Success Criteria

The plan is complete when all of the following are true:

- [x] Governed runtime is the supervised production path (`services.toml`, root daemon, engine).
- [x] Static modules and active dual-mode documentation language have been removed.
- [x] Tool↔model fitness, task-class outcomes, and model-quirk observations are queryable and visible.
- [x] Selection demonstrably consumes those signals, including Hermes tool-call outcomes.
- [x] Hermes-style tool-call traffic is classified and routed separately from pure chat.
- [x] Local (llama.cpp), free-cloud, and paid providers share a uniform enrollment, health, and fitness model.
- [x] Explicit bulk execution/background admission cannot consume the capacity reserved for avatar/interactive traffic.
- [x] Port `7171`, OpenAI surface, receipt format, and bounded admission primitives remain stable.
- [x] Process smoke and engine integration tests are green.

---

## 11. Explicit Non-Goals

- Extracting governance into a separate product or crate.
- Keeping a permanent parallel static implementation.
- Claiming “learning” or “alignment” until the signals are both visible and consumed by selection.
- Changing the coordinated 7171 contract or breaking the engine → router funnel.
- Big-bang rewrite that risks the already-stable local connection path.

---

## 12. Completion Evidence

1. `services.toml` and the root registry declare one required governed Manwë process on `:7171`.
2. `src/main.rs` always starts `adaptive::service::run`; `--adaptive` is a hidden compatibility no-op and `--grpc` fails with a retirement message.
3. `/status`, `/providers/capabilities`, `/observability`, `/route_history`, route headers, state events, lane fitness, bandit state, and tool-fit receipts expose the inputs and outcomes used by selection.
4. HTTP admission has one global limit plus a bounded bulk lane. Unmarked/interactive requests retain reserved capacity when explicit execution, orchestrator, batch, probe, or background traffic saturates its lane.
5. `cargo test -p manwe --all-features`, the single-process smoke, documentation validator, root integration, and `arda-engine` suites pass.

---

## 13. Resolved Questions

1. Hermes request classification uses typed body metadata and `x-manwe-*` route headers; tool schemas alone do not force execution unless the request requires tool use.
2. Fitness and quirk evidence is exposed through the existing status, observability, capability, route-history, metrics, state, and receipt surfaces.
3. Task classes share the global HTTP gate while explicit bulk lanes have a lower cap, reserving capacity for interaction without a second runtime.
4. Consumer searches and root-composed smoke found no remaining dependency on static-only modules or a selectable runtime layout.

---

## 14. Relationship to Existing Documentation

This plan is the evolutionary next step after the 2026-07-27 foundation closure documented in:

- `STATUS.md` — verification matrix, current capabilities, open risks
- `BREAKDOWN.md` — module graph, source-graph reconciliation evidence, consumers
- `PROVIDERS.md` — static vs adaptive config ownership (to be simplified)
- `README.md` — operator-facing surface (to be updated after cut-over)

The foundation work made convergence safe. The parallel runtime paths are retired; references above describe only the historical migration problem.

---

## 15. Summary

Manwe’s dual mode was useful scaffolding that outlived its purpose and is now retired.

The product vision requires one coherent, learning, multi-provider, task-aware router with governance inside the crate. The adaptive surface is the only path that can become that system.

The governed implementation is now the single runtime. Tool-fit and model-quirk outcomes feed bounded routing adaptation, and the parallel implementation has been removed.

The original piece-by-piece discipline is preserved — applied now to convergence and to making the system demonstrably intelligent rather than merely hoping that it is.

---

*Completed from source-backed convergence evidence, 2026-08-04.*
