---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "architecture_authority_map"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-08-21"
---

> 🜏 Soterion: 📜 architecture_authority_map | owner: PROMETHEUS | status: active | reviewed: 2026-08-21

# Digital Organism Authority and Transport Map

Program (archived): [`../../archive/digital-organism/README.md`](../../archive/digital-organism/README.md)

Foundation evidence: [`../audits/digital-organism-foundation-matrix.md`](../audits/digital-organism-foundation-matrix.md)

Current flow evidence: [`../audits/digital-organism-current-flow-trace.md`](../audits/digital-organism-current-flow-trace.md)

Machine map: `../../.hermes/evidence/digital-organism/authority-transport-map.json`

## Status and entry gate

This document is the active Stage 0 architecture decision for authority and transport ownership. It removes ambiguous duplicate canonical owners from the proposed Digital Organism architecture. It does not claim the successor contracts or migrations are implemented.

No registry, router, world file, service, or projection is deleted by this decision. The operator explicitly accepted this map and the documented migration-gated retirement successors on 2026-08-21. That decision opens Stage 1 contract implementation; it does not authorize any retirement, deletion, account, payment, external publication, or physical action.

## Core distinction

Every system boundary must identify three different roles:

1. **Authority** — the durable owner allowed to mint or change canonical state.
2. **Transport** — the mechanism that carries references, requests, data, or results between authorities.
3. **Projection** — a read-only representation of canonical state for operators, agents, metrics, or embodiment.

A transport response cannot silently become authority. A projection cannot promote maturity, health, approval, or completion. Configuration cannot claim current observation.

## Identity taxonomy

These identities are intentionally separate. They may reference one another but must not be collapsed.

| Identity | Canonical owner | Meaning | Must not become |
|---|---|---|---|
| `organism_id` | `arda-core` | stable Arda organism/install/federation identity | Hermes profile, hostname, or UI name |
| `operator_id` | configured Arda operator identity bound from Hermes authentication | sovereign human authority | platform display name or HUD input |
| `session_id` | Hermes Agent | conversational continuity and transcript | Arda objective, run, or memory ID |
| `objective_id` | `arda-core` canonical objective/task authority | durable requested outcome and acceptance lineage | phase label, question ID, Bot routine |
| `run_id` | `arda-engine` RunStore | one durable attempt graph for an objective | Hermes session or A2A context alone |
| `run_node_id` | `arda-core` run graph | typed step within one run | physical compute node ID |
| `worker_id` | engine work attempt | one assigned role and attempt | Hermes profile or machine identity by itself |
| `compute_node_id` | engine using outpost enrollment/observation contracts | physical/process host with capabilities and expiring state | model provider, agent profile, configured hostname alone |
| `agent_id` | Hermes profile or external A2A Agent Card | conversational/cognitive peer identity | compute node, worker attempt, Arda crate name |
| `route_id` | Manwë | provider/model inference route decision | node placement or approval |
| `evidence_id` | Varda or typed evidence producer | source/evaluation/audit evidence | execution approval or terminal outcome |
| `memory_id` | Vairë | organism memory, correction, use, and continuity lineage | Warden remote cache ID or Hermes memory item |
| `approval_id` | Arda governance and scoped operator decision ledger | semantic action authority | Hermes command approval, model vote, client label |
| `receipt_id` | typed minting subsystem; indexed into engine outcome when executable | immutable evidence of one bounded transition | universal untyped success token |

## Canonical authority map

| Concern | Canonical owner and durable authority | Allowed transports | Read-only projections | Explicitly prohibited competitors |
|---|---|---|---|---|
| Organism identity/contracts | `arda-core` versioned contracts | in-process Rust; harness reads | contract registry, operator projection, docs | Hermes profile metadata; HUD state; legacy world file |
| Platform authentication | Hermes gateway auth/allowlists; Arda root normalizes to configured operator | Hermes hook → loopback harness | operator session, continuity | display name; unbound transport JSON |
| Conversation/session | Hermes session store/transcript | CLI, TUI, gateway, API | Vairë continuity refs; surface handoff | second Arda transcript database |
| Objective/commitment | `arda-core`; append-only canonical task/objective identity | Hermes operator bridge; bounded HUD/harness/CLI mutation | next action, queue projections, Arandur packet, operator projection | Bot routine, research question, recommendation as commitment |
| Run/work attempt | engine RunStore + Core run graph | harness run API | operator projection, Workbench, queue terminal | delegate status or A2A task alone as durable run |
| Compute node | engine using outpost enrollment + expiring observation | local/Tailscale probe; bounded outpost; A2A card observation | Aulë topology, HUD fleet, RELIC | `fleet.toml` active flag, Manwë catalog, legacy world roster |
| Conversational agent/peer | Hermes profile or A2A Agent Card | Hermes Bot peer; Linux Foundation A2A | worker attempt reference; topology view | Oromë generic peer registry; mythic crate name as live agent |
| Semantic work/handoff envelope | Oromë, carrying canonical Core/engine IDs | in-process; harness; A2A adapter | transport/communication receipt | proprietary Oromë network or free-form prompt authority |
| Cross-process agent wire | Hermes/Linux Foundation A2A | authenticated HTTP/SSE/webhook A2A | Oromë handoff + engine attempt | test-only MessageRouter as wire; ad hoc SSH as protocol |
| Tool invocation | Hermes tools/MCP or tool-owning Rust service | MCP, plugin tool, in-process API | tool evidence in attempt receipt | MCP call as objective/approval/readiness |
| Node placement/lifetime | engine placement and work-attempt receipt | deterministic adapter, Hermes worker, A2A peer, systemd/outpost | worker/topology/presence | Arandur raw endpoint choice; Manwë physical placement alone |
| Provider/model route | Manwë current route and health receipt | OpenAI-compatible inference API | placement/provider/operator status | configured model; Bot selection; Arandur hard-code |
| Semantic approval | governance + scoped operator decision identity/scope/digest/expiry/single-use state | Oromë decision envelope; Hermes/HUD presentation | pending approval, Arandur packet | Hermes command approval; council/MoA vote; client `policy_safe` string |
| Host command/tool approval | Hermes Agent | CLI/TUI/gateway approval transport | Hermes tool/session history | Arda approval replacing host safety; plugin auto-allow |
| Research/evidence | Varda; Warden discovers; Rúmil audits | Warden API, Varda fetch/evaluate, RunStore `EvidenceLinked` | research brief, operator evidence, HUD Research | preview as evidence; brief as completion; evidence as approval |
| Collective memory/context | Vairë | in-process API; bounded harness context query | context capsule, continuity, human/persona notes | Hermes memory, Warden cache, or a second context DB as organism truth |
| Execution receipts/outcome | each typed producer; engine aggregates run outcome | append-only stores; adapter/A2A result | operator outcome, queue terminal, Vairë use, Aulë metrics | assistant summary, HTTP 200, UI success label |
| Health/homeostasis/topology | engine direct expiring observations; governance/economics policy; Aulë projects | local/remote probes, systemd, Manwë health, A2A observation | operator/HUD/RELIC | Aulë metric as authority; configured active; stale heartbeat |
| Executive composition | Arandur/Prometheus as consumer/planner | Aulë full-CLI cycle; Hermes/HUD review | recommendation and next action | parallel queue/registry; raw endpoint execution; self-approval |
| Platform delivery | Hermes platform adapter; Oromë normalized delivery receipt | Discord/other gateway adapter | communication/reminder state | bridge process or pending row as delivery proof |
| UI/embodiment | no mutation authority; consumers only | Hermes surfaces, HUD reads, RELIC/Mirromere projection | human-visible state | HUD queue, Mirromere memory, RELIC activity minting, World View workflow |
| Contract catalog | contract registry as declaration/index only | in-process loader; CLI/launcher read | onboarding/status | catalog maturity overriding direct runtime evidence |

## Transport stack

### 1. In-process Rust

Use when the caller and owning authority share the root process and a stable library API exists. This is the preferred path for deterministic policy, context, state transition, and projection assembly. A state-changing call must still emit the owning receipt.

### 2. Arda harness HTTP

Use for bounded local operator, run, research, context, and projection integration. It is loopback by default and routes requests into existing owners after authentication/idempotency checks. It is not the general inference port, peer-discovery service, or internet-facing API.

### 3. Hermes plugin hooks and tools

Use authenticated gateway hooks for conversation ingress and bounded continuity/lifecycle observations. Plugins may normalize and carry state but cannot create a second transcript, self-approve, widen routes, or mint canonical objectives without an Arda owner validating and appending them.

### 4. Linux Foundation A2A through Hermes

Use for cross-process, cross-machine, or cross-framework agent work. The A2A task/context ID is transport state. Arda authority comes from the enclosing Core objective, Oromë work envelope, engine work attempt, placement receipt, and terminal reconciliation.

Oromë must not implement a competing network protocol. It provides the semantic adapter:

```text
Arda objective/run/worker/capability/authority refs
  → arda.work-envelope.v1
  → A2A task/message/context
  → remote A2A result
  → arda.handoff-receipt.v1
  → engine work-attempt transition/outcome
```

### 5. MCP

Use for tool/resource invocation. MCP does not create organism objectives, approve work, prove node health, or identify a durable worker. Its result becomes tool evidence inside an enclosing attempt when governed.

### 6. Manwë inference API

Use only after engine placement requests a cognitive provider/model route. Manwë returns current route/model/provider evidence. It does not choose the physical node contract, semantic authority, task identity, or approval class.

### 7. Engine/systemd adapters

Use for deterministic service/process/device execution and recovery. These are preferred over LLM watchdogs. Execution requires engine/governance admission and a terminal or compensation receipt.

### 8. Outpost protocol

Use for typed remote observations, presence, and separately approved physical intents. Observation is advisory by default. A physical action requires an exact approved intent and terminal device receipt. The outpost protocol is not general A2A conversation or canonical memory.

## Execution lifetime selection

| Lifetime | Owner | Durable across Hermes/process loss | Use |
|---|---|---:|---|
| Deterministic in-process | engine/tool owner | Yes when owner appends | pure transforms, validation, local checks |
| Hermes conversation | Hermes | Session durable; active turn not necessarily | interactive reasoning; no organism mutation without Arda receipt |
| Hermes `delegate_task` | Hermes | No active-child resume | process-local parallel reasoning; loss becomes unknown |
| Engine-supervised Hermes worker | engine | Yes through RunStore/receipt | approved project work |
| A2A peer task | peer + engine attempt | Peer-dependent; Arda reconciles | cross-machine/framework work |
| systemd/outpost execution | engine/device adapter | Yes through explicit lifecycle receipt | deterministic services and approved physical actions |

Arandur requests roles and acceptance conditions. Engine selects lifetime and node. Manwë selects provider/model only when that lifetime needs inference.

## Approval model

Two independent gates may apply:

1. **Arda semantic approval** — whether the operator authorizes the consequential meaning of an action. It binds operator, objective/run/node, scope, action digest, expiry, and single-use state.
2. **Hermes host approval** — whether a concrete command/tool call may run on the current host/session.

Passing either gate never satisfies the other. Client-authored proposal/approval labels may request review but cannot mint canonical approval without a server-side pending action binding.

## Evidence, knowledge, and completion

- Warden preview/discovery is not evidence authority.
- Varda/Rúmil typed evaluation is evidence, not completion or approval.
- `EvidenceLinked` appends evidence but does not transition a node.
- A verifier/review rule decides whether acceptance conditions are met.
- Engine emits the explicit terminal transition and aggregate outcome.
- Vairë may record governed use/continuation only after evidence is consumed; success does not automatically promote content to knowledge.
- Queue terminal projection references the same canonical objective and outcome receipt.

## Operator projection convergence

One operator projection should join by canonical `objective_id`:

- queue objective and authority state;
- Hermes session/surface references;
- project and question references;
- run/node/worker/lifetime/placement;
- Manwë route where used;
- evidence and verification;
- semantic and host approvals as separate fields;
- Vairë memory-use/continuity references;
- communication/delivery state;
- terminal outcome and queue status;
- current node/service/route health.

The HUD, Hermes, RELIC, and Mirromere consume bounded views of that projection. They do not create alternate objective universes.

## Duplicate registry and router disposition

### Oromë `registry.rs`

Decision: **archive candidate after Stage 2 migration**.

Its useful capability/heartbeat ideas move to:

- Hermes/A2A Agent Cards for conversational peers;
- engine/outpost node enrollment and expiring observations for compute nodes;
- engine work attempts for assigned workers.

Do not delete until symbol consumers are inventoried and successor discovery/expiry tests pass.

### Oromë `router.rs`

Decision: **archive candidate after Stage 2 transport proof**.

Engine placement owns selection/attempts; Hermes A2A owns peer transport; RunStore owns retries/recovery. The test-only in-memory router must not become a second durable/network queue.

### Core `orome_runtime` registry/router state

Decision: **split and archive candidate**.

Core should retain canonical IDs/contracts, not a generic peer directory. The persistent registry and placeholder router state migrate only after all public consumers and persisted files are inventoried. Oromë keeps semantic envelopes/receipts; engine/outpost owns nodes; Hermes/A2A owns peers.

### Legacy `world.json` and realm projections

Decision: **archive candidate after live-topology cutover**.

The successor is fresh engine node/service/route observations plus Hermes/A2A agent projections and current Arandur cycle state. No file moves occur until operator projection and Arandur consume that successor and all required readers are migrated.

### Queue projections

Decision: **retain and repair**.

Do not add another task projection. Existing active/summary writers must retain eligibility, dependency, scope, origin, authority, and review metadata so dependency-gated plans are not labeled executable backlog.

### Contract registry

Decision: **retain and refresh**.

It remains a declaration/index, not runtime authority. Refresh source paths, versions, CLI verbs, receipt stores, and maturity from accepted decisions and direct validation.

## Stage 1 contract placement

| Proposed contract | Owner | Reuse before adding |
|---|---|---|
| `arda.organism-manifest.v1` | Core | service/task/contract identity patterns |
| `arda.organism-context.v1` | Vairë with Core refs | continuity and scoped recall |
| `arda.node-manifest.v1` | outpost protocol, consumed by engine | `OutpostEnrollment`, fleet config |
| `arda.node-observation.v1` | outpost protocol, consumed by engine | `OutpostObservation`, presence freshness |
| `arda.capability-request.v1` | Core | capability composition and worker roles |
| `arda.work-envelope.v1` | Oromë | A2A message/envelope plus Core IDs |
| `arda.work-attempt.v1` | engine | run node worker/checkpoint/receipt |
| `arda.handoff-receipt.v1` | Oromë + engine attempt ref | bridge/provider/handoff receipts |
| `arda.placement-receipt.v1` | engine with Manwë route ref | scheduler and route receipts |
| `arda.homeostasis-event.v1` | engine | service/fleet health and recovery |
| `arda.organism-outcome.v1` | engine aggregate | RunStore terminal + queue projection |

These names remain proposed until Stage 1 audits existing concrete structs field-by-field. New schemas are allowed only where extension would make an existing contract ambiguous or break its authority boundary.

## Stage 0 completion boundary

Technical decision work is complete when:

- every concern has one canonical owner;
- transport and projection roles are explicit;
- duplicate registries/routers have successors and migration gates;
- no deletion or runtime authority expansion is inferred;
- the machine map validates and referenced source paths exist.

Verified for this decision package:

- 14 identity namespaces, 22 authority concerns, 8 transports, 6 retirement decisions, and 11 Stage 1 contract placements are unique and structurally valid;
- HADES checked 69 active-plan links and 8 architecture links with zero broken;
- root Arda, Oromë all-features, Aulë full-CLI, and outpost-protocol compile checks passed;
- diff and append-only queue guards passed;
- no duplicate runtime authority was implemented, no source/runtime artifact was deleted, and Stage 1 remains disabled.

Entry to Stage 1 still requires explicit operator acceptance of this map. Acceptance authorizes bounded contract implementation only; it does not pre-approve later A2A deployment, worker execution, physical nodes, or archive deletion.
