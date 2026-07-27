---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "governance_spine"
  owner: "arda"
  status: "active"
  last_reviewed: "2026-07-27"
---

# arda-core

Shared primitives and contracts for the Arda governance spine.
Owner: arda | Sigil: 📜 SCROLL | Status: active

## Summary
`arda-core` is the foundational governance/data-model crate of the Arda
agentic OS surface. It owns the canonical contract for tasks, agents,
routing, state/goals/plans/outcomes, an append-only Decision ledger,
LLM provider abstraction, force-capped background execution, learning
state, governance gates, tooling registry, systemd unit introspection,
and service-registry types moved into this crate from standalone
`arda-service-registry`.

`arda-engine` depends on it and re-exports `arda_core::service_registry`
so the supervision spine can access service/contract/registry types
without adding a second direct dependency.

## Where it lives
- Crate root: `/var/home/mythos/Eregion/Arda/crates/spine/governance/arda-core`
- Key file: `src/lib.rs`
- Module index: `src/INDEX.md`
- Dependency graph point: `crates/engine` -> `arda-core`

## Agentic-OS relevant abstractions
- Governance: `GovernanceGates`, `GovernancePolicyMode`, `GovernancePolicy`
  per `DecisionClass` and action class in `config/governance_gates.yaml`.
- Conscience loop: dispatcher, reflected pass, joule market (`JouleEstimator`,
  `BidBoard`, `TriadConsultant`), HALT file, daily goal joule budgets.
- Reflection/learning: `LearningState`, `LearningStore`, `routing_bias`,
  `best_agent`, loop economy snapshot.
- Execution safety: bounded background/sync gate with pressure admission
  and shed receipts (`ARDA_PRESSURE_ADMISSION_*` env knobs).
- LLM ops: `LlmProvider`, `OpenAiCompatibleProvider`, `LlmConfig`,
  `model_routes`, key resolution from direct/secret/env.
- Operational control: `SystemdClient`, service registry state validator,
  contract/service handles/records.
- Sigil indexing: `SoterionRegistry`, `SoterionWatcher`, sigil metadata.
- Messaging/auditability: `Ledger`, `Message`, JSONL append with Soterion
  envelope metadata.

## Crate layout
| Module | Role |
|--------|------|
| `agent.rs` | `Agent` trait, `AgentManifest`, sigil metadata. |
| `aipkg.rs` | AIPKG manifest/preflight law, explicit governance evidence, and fail-closed signed receipt-chain validation. |
| `config.rs` | Runtime config for governance/spine choices. |
| `contract/` | `Decision`, `DecisionClass`, `TriadOutcome`, `PhilosopherVerdict`, `Plan`, `Goal`, `Reflection`, `MemoryRecord`. |
| `daemon.rs` | IPC command/response envelopes. |
| `error.rs` | Canonical shared error type/result alias. |
| `governance/` / `governance_gates.rs` | Policy modes, corpus hints, per-class gates, YAML-loadable override map. |
| `learning.rs` | Outcome stats, learning state/store, gate lifecycle packets. |
| `ledger.rs` | Append-only Decision/message JSONL output with Soterion enrichment. |
| `layout.rs` | Layout/public-surface helper types. |
| `learning_adapter.rs` | GEN3 learning-to-domain adaptation + ledger receipt for interop. |
| `llm.rs` | Provider-agnostic LLM client + OpenAI-compatible HTTP backend. |
| `loop_alerts.rs` | Loop alert emitter tied to dispatcher outcomes. |
| `loop_economy.rs` | Leaderboard-style economy snapshot from ledger. |
| `loop_engine.rs` | Dispatcher, reflector, joule market, council billing, halting. |
| `loop_observability.rs` | GEN3 env-toggled loop observability config + bounded latency probes. |
| `orome_runtime.rs` | Shared registry/router runtime state types. |
| `message.rs` | Spine message type with Soterion envelope metadata. |
| `pipeline.rs` | Pipeline helpers for orchestrated execution stages. |
| `router.rs` | Capability-based agent router over `AgentManifest`/`Agent`. |
| `service_registry/` | Folded registry surface: registry, validator, contract, records, continuity, test support. |
| `soterion.rs` | Registry/index utilities for sigil → metadata mapping. |
| `soterion_watcher.rs` | Watcher around sigil registry changes. |
| `state.rs` | `StateRoot` + typed read/write helpers for goals/plans/outcomes/memory/queue. |
| `systemd.rs` | systemd list-units query surface for supervised services. |
| `task.rs` | `Task`, `TaskStatus`, joule/phased timing/resonance metrics. |
| `tool.rs` | `ToolRegistry`, `ToolEntry`, sigil/harness classification from `registry.toml`. |
| `tool_contract/types.rs` | Tool harness metadata, risk, side effects, envelopes, `InvocationPlan`. |
| `tool_contract/service.rs` | Governance-baseline helpers, idempotency enforcement, invocation plan wiring. |
| `background.rs` | Pressure-aware bounded work gate for sync/async/background work. |

## How arda-engine is connected
- `engine/src/manwe.rs:6` — re-exports `manwe` as the runtime supervision shim
  for Arda service operations.
- `engine/src/lib.rs:11` — re-exports `arda_core::service_registry` under
  `arda_engine::service_registry`.
- `engine/src/manwe.rs` proxies `/v1/models` and related manwe behavior.
- `engine/INDEX.md` documents this as duplicated exposure to keep consumers
  from needing two direct dependencies.

## Verification status
- Compile-time: `cargo check -p arda-core` -> OK
- Tests: `cargo test -p arda-core --all-features` -> 111/111 passing (110 unit, 1 smoke,
  0 doc-tests).
- Compile-time consumer check: `cargo check -p arda-engine` -> OK.
- Runtime wiring: `engine` imports `arda-core::service_registry`, observability,
  and `manwe` successfully; no linkage failure observed.
- Evidence: `engine/src/manwe.rs`, `engine/src/lib.rs`, and both
  `engine/INDEX.md` and `arda-core/INDEX.md`.

## Key source links
- `crates/spine/governance/arda-core/src/lib.rs`
- `crates/spine/governance/arda-core/src/loop_engine.rs`
- `crates/spine/governance/arda-core/src/governance_gates.rs`
- `crates/spine/governance/arda-core/src/learning.rs`
- `crates/spine/governance/arda-core/src/task.rs`
- `crates/spine/governance/arda-core/src/llm.rs`
- `crates/spine/governance/arda-core/src/background.rs`
- `crates/spine/governance/arda-core/src/service_registry/mod.rs`
- `crates/spine/governance/arda-core/src/systemd.rs`
- `crates/spine/governance/arda-core/src/message.rs`
- `crates/spine/governance/arda-core/src/loop_alerts.rs`
- `crates/spine/governance/arda-core/src/soterion_watcher.rs`
- `crates/spine/governance/arda-core/src/tool_contract/service.rs`
- `crates/engine/src/manwe.rs`, `engine/src/lib.rs`

## Warnings / follow-ups
- No `arda-core` compiler warnings in the 2026-07-27 strict Clippy check.
- Legacy `src/alerts.rs` was retired on 2026-07-25 after repository and history
  searches confirmed it had no export or consumer; `loop_alerts.rs` is canonical.
- `ServiceRegistry::from_snapshot` intentionally skips rejected records;
  duplicate-skip behavior is tested.
- Remaining GEN3 questions are maintained in `docs/interop/landscape.md`.
- Docs last refreshed against the `manwe` branch source of truth on
  2026-07-27.

## Foundation conclusion
The crate’s stabilization plan is complete: GEN1 documentation alignment and
GEN2 robustness are closed, implemented GEN3 surfaces are additive and tested,
and the current compiled surface passes all 111 tests. `arda-core` is therefore
the recorded stable foundation for subsequent crate-by-crate repair work.
