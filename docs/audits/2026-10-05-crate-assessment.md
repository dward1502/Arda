---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "architecture_assessment"
  owner: "ARDA"
  status: "recorded"
  reviewed: "2026-10-05"
---

# Arda crate assessment and improvement references

Arda has substantial governance, durable execution, recovery and scoped-memory
implementation. Its main deficiencies are repeatable connected acceptance,
continuous source validation, clear execution ownership and accurate operator
documentation. The assessment is an advanced development system with incomplete
operational acceptance, not a production-readiness certification or a measured
ranking against other projects.

Source baseline: main at `f5c8a361f87d95cf04a932a90a15fd52f15eb6d1`, before
the accompanying documentation corrections. Cargo metadata, manifests, public
exports and selected implementation paths were inspected. The
[October 1 audit](../architecture-audit-2026-10-01/README.md) supplied historical
context. No full Rust build, full test suite, native acceptance or performance
benchmark was run in this review. Existing uncommitted operator state was preserved.

The [ordered continuation list](../plans/CRATE_RELIABILITY_AND_ACCEPTANCE.md)
owns follow-up ordering; existing product/workstream plans retain their gates.

## Package responsibilities and improvement opportunities

Cargo metadata resolves 19 workspace packages and 17 default members.
[CODEMAP](../CODEMAP.md) owns package paths and current topology.

| Package | Implemented responsibility | Improvement focus |
|---|---|---|
| `arda` | Root configuration, composition, startup and shutdown | Readiness, failure reporting, development versus installed composition |
| `arda-engine` | Objective control/scheduling, project/run authority, adapters, recovery, HTTP control and supervision | Real interruption-to-completion acceptance; narrow internal ownership |
| `manwe` | Governed inference gateway, capability routing, health, retries/fallbacks and route evidence | Actual provider/tool/streaming/failure qualification and measured route quality |
| `arda-core` | Shared contracts plus ledgers, policies, provider abstractions and operational machinery | Separate stable contracts from executable implementation where changes justify it |
| `arda-governance` | Policy/evidence evaluation, readiness, scoring and advisory signals | Enforce verdicts at real effect boundaries; explain decisions and calibrate heuristics |
| `arda-economics` | Provider spend, measured/estimated energy, tariffs and economic events | Cost/energy per accepted result including retries; preserve measurement provenance |
| `arda-mandos` | Advisory reasoning/verdict service with evidence and integrity-checked history | Human-labeled decision-quality evaluation, false approvals/rejections and uncertainty |
| `arda-rumil` | Bounded inventory/audit coordination, normalized findings and baseline comparison | Revision/tool identity, actionable finding rates and verified resolution |
| `arda-vaire` | Persistent scoped memory, retrieval, context assembly, retention, correction/revocation | Repair fixture drift; measure relevant retrieval and actual cross-session use |
| `arda-varda` | Ingestion, BM25 search, citations/source spans, deep-analysis queues and caching | Retrieval/citation evaluation and truthful crawler/provider degradation |
| `arda-orome` | Hermes normalization, approval replay protection, communications and dispatch receipts | Real-channel reconnect/cancel/duplicate/delivery qualification; distinguish manual transport |
| `arda-aule` | Metrics, projections, CLI, coordination and concrete Workbench execution | Separate execution from observability behind a narrow contract |
| `arda-contract-registry` | Typed read-only registry loading/version lookup | Compatibility and consumer diagnostics; preserve small scope |
| `arda-mirromere` | Shared display/lifecycle/privacy/interaction contracts | Honest stale/unavailable projections and native acceptance |
| `arda-launcher` | Native setup, lifecycle control and readiness presentation | Clean install, upgrade, rollback and recovery outside a development checkout |
| `arda-outpost-protocol` | Versioned observation/presence/research/display contracts and process-local queues | Consumer conformance and explicit queue durability limits |
| `arda-outpost-scout` | Bounded surveys, SearXNG research, read-only audits and memory observations | Actual device disconnect/retry/staleness/degradation qualification |
| `arda-relic-bridge` | Read-only presence validation and last-valid caching | Truthful offline/stale display and reconnect behavior |
| `arda-project-adapter-sdk` | Bounded JSONL framing, envelope validation and capability negotiation | Complete adapter examples and reusable conformance checks |

HUD (`arda_hud`) and native Mirromere (`arda_mirromere`) are additional packages in
separate workspaces. Root Cargo success does not validate their Rust/frontend
builds. Native Mirromere currently bootstraps a Hermes dashboard; broader
embodiment remains distinct from that implemented behavior.

## Findings and evidence boundaries

| ID | Finding | Evidence and status |
|---|---|---|
| F01 | Vaire test fixture drift | [Initializer](../../crates/spine/memory/arda-vaire/tests/context_capsule.rs) omits `excluded_refs`, required by [OrganismContext](../../crates/spine/memory/arda-vaire/src/service/organism_context.rs). The October 1 audit recorded E0063; current source inspection confirms the mismatch remains. No fresh compilation in this review. |
| F02 | Broad source CI is absent from repository workflows | `.github/workflows` contains documentation health and release signing, not a general build/test workflow. External checks were not inventoried. |
| F03 | End-to-end acceptance remains open | [M1–M5 owner](../plans/autonomous-task-completion/README.md) has unchecked obligations. [M4 handoff](../plans/autonomous-task-completion/M4_REAL_MULTI_PROJECT.md) records isolated qualification, installed gaps and unresolved provider-readiness timeouts. |
| F04 | Execution is coupled to observability | [Engine Workbench](../../crates/engine/src/objectives/workbench.rs) imports concrete execution contracts/adapter from Aule's `prometheus::autopilot`. Architecture risk; not proof of a particular incident. |
| F05 | Source and mutable state share the repository | Tracked objective SQLite state, run evidence and operator configuration coexist with source. Define path-level ownership, backup and export rules before relocation. No secret exposure established. |
| F06 | Documentation overstates or misidentifies current behavior | Engine scope, Aule queue ownership, package count, native workspaces, milestone status and Manwe binding had specific mismatches. Corrected in the accompanying docs pass; runtime/code defects remain open. |
| F07 | Runtime health needs fresh diagnosis | Session observations below are time-bounded, not standing service facts or an established root cause. |
| F08 | Quality and burden reduction need comparable measurements | Retrieval evaluation and telemetry already exist. A common representative outcome dataset and repeated installed acceptance were not established by this review. |

## Runtime observations from the assessment session

The read-only service check on 2026-10-05 observed Crawl4AI in
`activating/auto-restart`, the Manwe inference probe failed, and the metrics
exporter inactive. Root Arda, Varda, snapshot keeper, RELIC bridge and Hermes
gateway initially appeared active. A request to `127.0.0.1:7878/v1/status` refused
connection. A subsequent root-unit inspection showed `MainPID=0` and exit status
1; its reported start/stop times were 22:23:08/22:23:13 PDT. Listener inspection
showed Varda on `127.0.0.1:5111` and no matching 7171/7878 listener at that instant.

Commands used: `systemctl --user list-units --all --type=service 'arda*' 'hermes*'`,
`systemctl --user show arda.service -p ExecStart -p MainPID -p ActiveEnterTimestamp`,
`curl -fsS --max-time 5 http://127.0.0.1:7878/v1/status`, and filtered `ss -ltnp`.
These observations were summarized from session tool output; a retained raw
journal bundle was not collected. Re-probe and inspect exact installed binaries,
configuration and recent logs before selecting a repair.

Mesh-llm had intentionally been stopped for gaming earlier in the session.
Do not infer it caused the failures, or restart GPU inference as a repair without
checking the operator's current intent. A failed probe and an intentionally
inactive optional service have different meanings.

## Comparable project patterns

These primary sources were consulted on 2026-10-05. The proposed applications
are design recommendations, not head-to-head benchmarks or adoption decisions.

| Reference | Relevant pattern | Application to existing Arda owners |
|---|---|---|
| [Temporal event history](https://docs.temporal.io/encyclopedia/event-history) and [LangGraph persistence](https://docs.langchain.com/oss/python/langgraph/persistence) | Durable workflow history; separate thread checkpoints and cross-thread memory | Engine interruption/replay tests; preserve separation from Vaire memory |
| [LiteLLM routing](https://docs.litellm.ai/docs/routing) | Cooldowns, fallbacks, timeouts, retries and deployment limits | Compare existing Manwe mechanisms on identical workloads and injected failures |
| [Letta memory blocks](https://www.letta.com/blog/memory-blocks/) | Persisted, inspectable, purpose-specific context with size bounds | Explain selected/excluded Vaire context, budget and consumption while preserving scope/revocation |
| [Haystack evaluation](https://docs.haystack.deepset.ai/docs/evaluation) | Component and end-to-end retrieval/answer evaluation | Extend Vaire/Varda datasets; compare retrieval ranking and citation faithfulness before storage changes |
| [Langfuse evaluation](https://langfuse.com/docs/evaluation/overview) | Traces, feedback, datasets and regression experiments | Turn disappointing runs into tests; connect objective, calls, artifacts, checks and acceptance |
| [OPA policy testing](https://www.openpolicyagent.org/docs/policy-testing) and [decision logs](https://www.openpolicyagent.org/docs/management-decision-logs) | Tested policy decisions with version/input/result evidence and redaction | Governance enforcement tests at effect boundaries; understandable policy decisions |
| [A2A specification](https://a2a-protocol.org/latest/specification/) | Task lifecycle, artifacts, cancellation, capabilities and subscriptions | Reference for Orome/adapter interoperability where needed; internal A2A naming does not establish compliance |

Existing Vaire retrieval evaluation already exposes Hit@1, Recall@K and MRR.
Manwe already contains retry/fallback/budget machinery. The opportunity is to
qualify and expose these capabilities, not assume they are missing or replace
the Rust system with the comparison projects.
