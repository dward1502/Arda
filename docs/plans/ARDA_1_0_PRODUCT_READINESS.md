# Arda 1.0 Product Readiness and Product Direction

**Assessment date:** 2026-07-29  
**Current classification:** Integrated Platform Alpha  
**Readiness score:** 51.6/100  
**Recommended first product:** Sovereign Agentic Development Workbench

## Executive verdict

Arda is beyond the prototype or crate-collection stage. It has a coherent
local-first control-plane architecture, tested Rust packages, governance and
receipt contracts, model routing, process supervision, evidence, memory,
economics, observability, and bounded outpost behavior. It is correctly shaped
as a platform rather than a single chatbot.

Arda is not yet a legitimate 1.0 product. Its strongest capabilities are not
composed into one default operator workflow, its UI does not yet expose the
system's real value, several service-capable crates are library-only at the root
boundary, and there is no installer-to-success path for a user who is not the
creator. The current stage is **Integrated Platform Alpha**: a credible system
foundation and developer preview, approximately halfway from architecture to a
repeatable product.

The correct 1.0 wedge is not "all of JARVIS." It is a focused product that Arda
can already support: a sovereign agentic development environment that attaches
to an existing project, accepts a typed or spoken objective, plans and executes
bounded work, presents approvals, records receipts, verifies results, and can
resume later. Mirromere, personal operations, embodied presence, and proactive
learning should be first-party applications on the same kernel, not requirements
that delay the first sellable release.

## Six-stage product ladder

| Stage | Definition | Arda status |
|---|---|---|
| 1. Concept | Vision, architecture sketches, isolated experiments | Complete |
| 2. Subsystem foundation | Real packages, contracts, tests, state, and domain ownership | Complete |
| 3. Integrated Platform Alpha | Composition root, runnable core, partial UI, connected but incomplete workflows | **Current** |
| 4. Private Beta | One complete use case, installer, recovery, dogfood users, stable project contract | Next |
| 5. Release Candidate | Upgrade path, security review, soak evidence, support docs, no critical workflow gaps | Future |
| 6. Legitimate 1.0 | Repeatable user outcome, measurable reliability, distributable product, supportable boundaries | Target |

## Weighted 1.0 readiness rubric

The score is intentionally product-weighted. Package quality alone cannot make a
1.0 if the user cannot complete a valuable workflow.

| Dimension | Weight | Current | Evidence and principal gap |
|---|---:|---:|---|
| Architecture and contract model | 12% | 8.5/10 | Fifteen first-class workspace packages, clear domain ownership, contract registry, typed state and receipts. Some legacy names and duplicated narrative remain. |
| Governance, authority, and auditability | 12% | 8.0/10 | Strong policy, readiness, evidence, outpost authority, and append-only behavior. Must be proven on the default product workflow rather than only component surfaces. |
| Runtime composition and service operations | 14% | 5.0/10 | Root daemon, registry, supervisor, health checks, harness, and Manwe are runnable. Most advanced runtimes are not root-composed or default-enabled. |
| End-to-end agent workflows and loops | 14% | 4.5/10 | Execution, research, queue, learning, and receipt machinery exists. No single correlated request traverses the full default graph. |
| Operator UI/UX | 12% | 3.5/10 | Launcher and HUD establish a desktop entry point. The HUD mostly exposes models rather than tasks, evidence, approvals, memory, costs, and outcomes. |
| External project and tool integration | 10% | 4.0/10 | Tool contracts, manifests, MCP/harness direction, service registry, and polyglot-friendly boundaries exist. A stable project-adapter SDK and compatibility suite do not. |
| Personal-assistant modalities | 8% | 2.0/10 | Embodied, voice, camera, and Mirromere requirements are documented and safety-gated, but not active default runtime capabilities. |
| Reliability, packaging, and deployment | 8% | 5.5/10 | Rust workspace gates and local daemon smoke are strong. One-command install, upgrades, rollback, backup/restore, and independent-machine soak are missing. |
| Evaluation and governed learning | 6% | 5.5/10 | Evaluation and learning receipts plus an active governed-learning plan exist. The full learning chain is not default-composed and policy promotion remains gated. |
| Commercial product readiness | 4% | 2.5/10 | Product identity and a compelling owned-platform story exist. Target user, onboarding funnel, pricing, licensing, support boundary, and user outcome evidence are not established. |
| **Total** | **100%** | **51.6/100** | **Integrated Platform Alpha** |

A 1.0 candidate should exceed roughly 80/100 **and** pass the non-negotiable
release gates below. The numeric score is a prioritization aid, not permission to
ship around failed safety or reliability gates.

## What Arda 1.0 should be

### Product promise

> Connect an existing software project to an AI development environment you
> own. Give it an objective by text or voice. Arda plans, researches, executes,
> verifies, explains, records, and resumes the work under operator-controlled
> policies.

### Target user for 1.0

The initial target is a technically capable solo developer or small product team
who wants local-first agentic development without surrendering project state,
provider choice, operational history, or approval authority to a SaaS agent.

The creator is the first power user, but 1.0 must be usable on a clean machine by
a second developer without repository-specific oral history.

### Required golden path

1. Install and launch Arda.
2. Complete a readiness check and choose local/cloud provider policy.
3. Attach an existing repository through a versioned project contract.
4. Type or speak an objective.
5. See the proposed task graph, evidence needs, cost/risk estimate, and gates.
6. Approve the bounded run or adjust its limits.
7. Watch agents research, edit, test, and produce correlated receipts.
8. Review the diff, evidence, tests, cost, and governance verdict in one UI.
9. Accept, reject, or request revision.
10. Close Arda and later resume from durable state without losing context.

If this works reliably, Arda is a credible private beta and a commercially useful
product even before the full embodied assistant exists.

## Is Arda graph engineering?

Yes, architecturally Arda is much closer to **graph engineering** than a linear
prompt chain.

### Existing graph elements

- Services, agents, providers, projects, tasks, evidence, policies, tools, and
  memories are nodes.
- Typed contracts, HTTP/IPC calls, queue records, and receipt parent IDs are
  edges.
- Governance constrains which edges may activate.
- Aulë projects topology and execution state.
- Vairë preserves state across graph traversals.
- Manwe selects among provider/model branches.
- Warden and Varda create evidence branches.
- Mandos and governance create review and decision branches.
- Economics weights execution choices with resource consequences.

A prompt is therefore only one possible **event that enters the graph**. Other
entry events can be a timer, stale evidence, a calendar event, repository change,
failed health check, sensor observation, budget threshold, incoming message, or
an unresolved task.

### What is still missing

Arda does not yet have one canonical runtime representation of an active run
graph. The repository contains graph-shaped components and several loops, but the
default root runtime still behaves mostly as a supervisor plus model gateway.

A complete run graph needs:

- one `run_id` across every node and receipt;
- explicit node state: pending, ready, blocked, running, succeeded, failed,
  cancelled, superseded;
- typed edge conditions;
- bounded retries and timeouts;
- budgets and authority on every executable edge;
- checkpoint and resume behavior;
- compensation or rollback edges;
- graph visualization in the HUD;
- deterministic replay from receipts.

Do not reduce this to a generic visual DAG builder. Arda's differentiator is a
**governed, stateful, receipt-producing execution graph**.

## The loops Arda needs

Loops do not replace the graph. A loop is a controlled repeated traversal of part
of the graph.

### 1. Interaction loop — immediate

```text
capture intent -> clarify/plan -> act -> verify -> present -> remember
```

This powers ordinary text and voice interaction.

### 2. Development execution loop — 1.0 product loop

```text
objective -> inspect project -> plan -> approve -> edit -> test -> review -> close
                         ^                    |             |
                         +-- revise on evidence/failure ----+
```

This should be the first complete product loop.

### 3. Governed learning loop — existing active direction

```text
coverage gap -> Warden research -> Varda evaluation -> approved delta
             -> Vairë memory -> proposal -> governance -> operator
```

The existing Warden-to-Varda-to-Aulë plan correctly keeps observation,
knowledge promotion, queue mutation, and execution as separate authorities.

### 4. Proactive improvement loop — post-1.0 or tightly bounded beta

```text
observe system/project -> detect opportunity -> gather evidence
                       -> propose advancement -> operator decision
                       -> bounded implementation -> evaluate
```

This is how Arda can learn about new technology and suggest integrations without
silently rewriting itself.

### 5. Personal operations loop — first-party application

```text
voice/text capture -> inbox -> classify -> schedule/remember
                   -> daily brief -> reminder -> completion/replan
```

For an AuDHD-friendly experience, the primary principle should be **capture now,
organize later**. The operator should not have to choose a project, category,
priority, date, and format before recording a thought.

### 6. Embodied presence loop — opt-in application

```text
presence/wake event -> local context -> decide whether to engage
                    -> speak/show -> listen -> action/record -> return ambient
```

Camera and microphone processing should be local-first, visible, and easy to
mute. Presence detection should be context, not clinical measurement or sole
identity proof. The avatar must gracefully avoid interruption and offer replay,
text, and quiet-mode equivalents.

## Product architecture: kernel plus first-party applications

Arda should remain a stable kernel with multiple applications rather than one
monolithic interface.

### Arda kernel

- `arda` and `arda-engine`: composition, lifecycle, run graph, checkpointing.
- `arda-core`: shared run, task, project, tool, event, and receipt contracts.
- `manwe`: inference routing.
- `arda-governance`: authority and policy.
- `arda-orome`: communication and delivery truth.
- `arda-vaire`: durable memory.
- `arda-aule`: telemetry and operator projections.
- Economics, Mandos, Varda, and Warden: optional domain services attached by
  contract.

### First-party applications

1. **Arda Workbench** — agentic software and product development; first 1.0.
2. **Mirromere** — JARVIS-like avatar and ambient interface.
3. **Personal Operations** — capture, schedule, reminders, daily replay, and
   low-friction organization.
4. **Warden Research** — bounded recurring research and ecosystem monitoring.
5. **RELIC/CITADEL** — embodied geometric visualization and edge presence.
6. **Company Operations** — proposals, product work, revenue experiments,
   communications, and governance.

Each application consumes the kernel through contracts. It must not duplicate
routing, memory, governance, or task state.

## Polyglot project integration contract

Arda should integrate useful external software rather than rebuild it merely to
make it Rust.

The Rust boundary should govern execution; it should not dictate the language of
the attached project.

### Proposed `arda.project-contract.v1`

A repository-owned manifest should declare:

- stable project ID and display name;
- repository root and allowed worktree boundaries;
- languages and runtime adapters;
- install, build, test, lint, format, and package commands;
- health/readiness checks;
- environment-variable names without secret values;
- network, filesystem, device, and process permissions;
- tool manifests and MCP/HTTP/stdio adapters;
- artifact and receipt locations;
- rollback strategy;
- license and provenance metadata;
- operator approval classes;
- project-specific memory scope;
- supported contract version.

### Adapter types

- Native Rust trait for in-process trusted integrations.
- MCP for tool-oriented integrations.
- HTTP/gRPC for persistent services.
- JSON Lines over stdio for lightweight Python, Node, Go, or shell adapters.
- Supervised sidecar process for existing applications.
- Container/sandbox adapter for untrusted or dependency-heavy projects.

### Integration rule

A Python project can remain Python. Arda reads its contract, launches its
approved environment, invokes declared commands/tools, captures structured
results, applies policy, and emits Rust-owned receipts. A later Rust rewrite is a
product decision, not an integration prerequisite.

### Compatibility requirement

The SDK needs a conformance suite that proves an adapter can:

1. initialize;
2. advertise capabilities;
3. pass health checks;
4. receive a bounded request;
5. stream progress;
6. return structured success/failure;
7. cancel cleanly;
8. emit provenance;
9. recover after restart;
10. respect denied capabilities.

## Legitimate 1.0 release gates

Arda should not be called 1.0 until all of these are true.

### Product outcome

- At least one non-author user completes the golden path on a clean machine.
- The development loop successfully closes real tasks in at least three attached
  repositories, including one non-Rust project.
- Users can understand what Arda changed, why, what it cost, and how it was
  verified without reading raw ledgers.

### Runtime and reliability

- One supported installation path.
- One supported upgrade and rollback path.
- Backup and restore for operator state and memory.
- Crash/restart resumes an interrupted run without duplicate mutation.
- Defined offline and provider-unavailable behavior.
- A sustained soak test with bounded state growth.
- Versioned migration for every persisted schema used by the golden path.

### Safety and sovereignty

- Secrets never enter project contracts or model-visible logs by default.
- Network, filesystem, process, camera, microphone, and external-send authority
  are explicit and revocable.
- Destructive operations require an approval receipt or a narrowly scoped policy.
- Every code mutation is attributable to a run, tool, model/provider, and
  operator policy.
- Local-only mode is usable for the supported golden path where local models are
  capable.

### UI/UX

- One coherent home surface.
- Task graph, current state, blockers, approvals, evidence, diff, test results,
  cost, and receipts are understandable without CLI archaeology.
- Voice capture has a text equivalent and replay/transcript.
- Quiet mode, interruption controls, flexible reminders, and low-friction inbox
  capture are first-class accessibility behavior.
- Errors state what failed and what the operator can do next.

### Integration and support

- Versioned project contract and adapter SDK.
- Rust plus at least one polyglot reference adapter, preferably Python.
- Contract compatibility tests.
- Clear supported/experimental distinction.
- Licensing, third-party attribution, telemetry policy, and support boundary.

## Product and revenue path

### First revenue hypothesis

Arda Workbench can be sold as a local-first agentic development environment for
technical individuals and small teams who want:

- provider independence;
- local-model support;
- durable project memory;
- approval-controlled agent execution;
- auditable changes and receipts;
- existing-repository integration;
- an owned operational environment rather than a hosted black box.

### What can be open versus paid

One plausible model:

- **Open kernel:** contracts, local runtime, base gateway, core project adapter.
- **Paid desktop product:** polished Workbench/HUD, onboarding, project packs,
  backups, visualization, support, and curated integrations.
- **Optional private/fleet tier:** multi-machine routing, policy packs, team
  approvals, remote outposts, and managed update channels.

The final licensing model requires a separate decision, but the product must be
useful without forcing all core sovereignty through a hosted dependency.

### Evidence before pricing optimization

Before optimizing pricing, prove:

- time from install to first verified task;
- task completion rate;
- recovery rate after failure;
- operator interventions per task;
- local versus cloud execution mix;
- cost per successfully closed task;
- weekly return usage;
- whether users trust and understand the approval model.

## Three priorities

### Immediate — complete one development vertical slice

Use a real external repository, ideally one Rust and one Python project. Carry one
objective through project inspection, plan, approval, bounded execution, tests,
review, receipt bundle, and resume. Use existing crates and contracts; do not add
another domain crate unless ownership cannot fit an existing one.

### Immediate — make the HUD show work, not only infrastructure

The first product UI should prioritize:

- inbox/objective capture;
- active run graph;
- next required operator action;
- proposed/approved mutations;
- evidence and reasoning summary;
- diff and verification results;
- cost/provider route;
- timeline and resume.

Provider catalogs and service health remain supporting views.

### Mid-term — ship the polyglot project SDK

Stabilize `arda.project-contract.v1`, a JSON Lines or MCP adapter protocol, and a
Python reference adapter. This unlocks the wider ecosystem without importing
entire projects into the Arda workspace or rewriting them in Rust.

## Principal product risks

1. **Platform-before-product trap:** continuing to strengthen subsystems while no
   outside user completes one valuable workflow.
2. **UI as decoration:** building a visually unique avatar before the underlying
   run, approval, resume, and recovery experience is coherent.
3. **Autonomy theater:** enabling timers and proactive triggers before they have
   deduplication, budgets, evidence gates, and useful operator outcomes.
4. **Integration sprawl:** copying external repositories into Arda instead of
   defining adapters and preserving provenance.
5. **Creator-only operability:** relying on knowledge that exists only in the
   creator's head, shell history, or local machine layout.
6. **Too many simultaneous products:** treating Workbench, JARVIS, personal
   operations, embodied hardware, company automation, and a general OS as one
   release milestone.

## Recommended next milestone

**Milestone: Arda Workbench Private Beta Vertical Slice**

**Implementation update (2026-07-30):** the versioned project-contract model and
fixtures, canonical run-graph validation, durable journal/recovery foundation, and
an explicitly draft-only Workbench objective/graph HUD surface now exist with
focused tests. Native project attachment, harness APIs, adapter execution, real
edit/test/review receipts, golden paths, packaging, and evaluator evidence remain
open; this milestone and Stage 4 are not complete.

The milestone is complete when:

- a repository is attached through a versioned project contract;
- a text objective creates a visible run graph;
- the operator approves bounded authority;
- an agent executes one real code change;
- project-native tests verify it;
- all evidence, routes, actions, costs, and outcomes share one `run_id`;
- the HUD shows the complete timeline and diff;
- the run survives an Arda restart and resumes without duplicate mutation;
- the same adapter model works for one Rust and one Python repository.

This milestone converts Arda from an integrated platform alpha into a credible
private-beta product while directly helping its creator return to external
product and client work.
