  What you have created

    You have created a local-first, governance-first AI control plane.

    Arda is not merely a chatbot, model server, or collection of agents. It is the foundation of a system that can:

    • Discover and route work to local or trusted AI providers.
    • Collect and retrieve evidence.
    • Evaluate proposed decisions against explicit governance policies.
    • Record decisions, costs, provenance, and outcomes.
    • Preserve operational and episodic memory.
    • Supervise the processes that provide these capabilities.
    • Expose system state to an operator through APIs, a CLI, and desktop interfaces.
    • Extend bounded sensing and research to remote outposts such as the Pi5 Warden.

    The central purpose is:

    │ Give you a sovereign, inspectable way to operate AI agents without handing uncontrolled authority to any individual model, agent, service, or remote node.

    It is closer to an agent operating environment than an individual agent.

    ────────────────────────────────────

    One important topology correction

    I need to tighten one statement from my previous answer.

    The earlier 11-package list included declared Cargo relationships through optional feature edges. The root daemon’s active default dependency tree currently
    contains six packages:

      ─ text
      arda
      arda-engine
      arda-core
      arda-governance
      arda-orome
      manwe

    The other packages are first-class workspace components, but most are not automatically instantiated by the default arda process. They are connected
    through:

    • Feature-gated consumers.
    • Library APIs.
    • IPC and HTTP service contracts.
    • Operator CLI commands.
    • Supervised external processes.
    • The launcher and outpost boundaries.

    This means you have completed the component and contract foundation, but the entire system is not yet executing as one automatic end-to-end agent loop.

    That is not a failure. It tells us exactly where Arda is: the organs are mature; the next job is activating the full circulatory system.

    ────────────────────────────────────

    A useful mental model

    Think of Arda as a governed organism or organization:

      Component               Role in the complete system
      ──────────────────────  ───────────────────────────────────────────────────────────────────────────────
      arda                    Composition root and lifecycle authority
      arda-engine             Runtime spine, process supervisor, and control harness
      arda-core               Shared language, records, tasks, policies, receipts, and operational primitives
      arda-governance         Constitution and risk/evidence evaluator
      manwe                   Model and provider traffic controller
      arda-orome              Communications and delivery-contract layer
      arda-varda              Athena: knowledge ingestion, retrieval, research, and synthesis
      arda-mandos             Oracle: bounded decision support and explainable verdicts
      arda-vaire              Mnemosyne: episodic, semantic, and procedural memory
      arda-economics          Plutus: cost, energy, budget, and value accounting
      arda-aule               Observability, coordination, telemetry, CLI, and operator projections
      arda-outpost-scout      Remote eyes and bounded web/repository research
      arda-outpost-protocol   Safe observation contract for remote nodes
      arda-contract-registry  Machine-readable map of system contracts
      Launcher/HUD            Human entry point and operator cockpit

    No individual component owns the whole system. That is one of Arda’s most important architectural properties.

    ────────────────────────────────────

    How the parts are intended to work together

    A mature Arda workflow would look like this:

      ─ text
      Operator / scheduled trigger
                  │
                  ▼
          Root daemon + engine
                  │
                  ▼
           Task or research intent
                  │
             ┌────┴────┐
             ▼         ▼
       Warden Scout   Varda/Athena
       external       ingest, search,
       evidence       citations, synthesis
             │         │
             └────┬────┘
                  ▼
               Vaire
         evidence and memory
                  │
                  ▼
             Governance
       evidence grade, policy,
       readiness, risk boundary
                  │
                  ▼
               Mandos
       advisory reasoning/verdict
                  │
                  ▼
                Manwe
       provider/model selection
       and inference dispatch
                  │
                  ▼
                Oromë
       bounded delivery, messages,
       receipts, communication
                  │
             ┌────┴────┐
             ▼         ▼
         Economics    Vaire
       cost/energy    outcome memory
             │         │
             └────┬────┘
                  ▼
                Aulë
       telemetry, status, ledgers,
       metrics, operator projection
                  │
                  ▼
             HUD / operator

    That is the system-level capability model.

    Some of those arrows are implemented and verified today. Others are supported by typed contracts but still need to be assembled into a single default
    runtime path.

    ────────────────────────────────────

    What each major subsystem contributes

    1. Root daemon and engine: lifecycle and composition

    arda

    The root daemon is the process-level authority. It:

    • Finds the Arda repository.
    • Loads and validates services.toml.
    • Refuses startup when required service definitions cannot resolve.
    • Starts the local control harness.
    • Starts supervised processes.
    • Tracks child PIDs.
    • Handles Ctrl-C and shutdown.
    • Supports --once registry validation.
    • Can omit UI processes with --no-ui.

    arda-engine

    The engine owns:

    • Declarative service registry loading.
    • Child process spawning and restart behavior.
    • Exponential restart backoff.
    • Shared shutdown signaling.
    • Local harness HTTP routes.
    • Manwe model-catalog projection.
    • Warden Scout proxy routes.
    • Engine observability projection.

    The engine harness is the operator/control tap-in surface at:

      ─ text
      127.0.0.1:7878

    Current routes include:

      ─ text
      /health
      /status
      /v1/models
      /v1/scout/health
      /v1/scout/search
      /v1/scout/recall
      /v1/harness

    Current limitation

    The harness does not proxy /v1/chat/completions. Inference clients still communicate directly with Manwe on :7171.

    Therefore, the current architecture is:

    • :7878 — daemon control, status, model discovery, Scout proxy.
    • :7171 — actual inference gateway.

    That separation can be retained intentionally, but it should be made explicit rather than describing :7878 as the only external API for every operation.

    ────────────────────────────────────

    2. Manwe: provider and inference routing

    Manwe is Arda’s OpenAI-compatible inference gateway.

    It can:

    • Discover local fleet providers.
    • Expose available models.
    • Accept OpenAI-compatible chat-completion requests.
    • Choose an eligible provider.
    • Forward requests.
    • Record route receipts.
    • Report provider health and capabilities.
    • Apply resource and fleet policies.
    • Support governed adaptive routing.
    • Expose metrics.
    • Optionally expose gRPC.
    • Emit OpenTelemetry events through Aulë.

    Its public runtime includes:

      ─ text
      GET  /healthz
      GET  /status
      GET  /providers
      GET  /metrics
      GET  /v1/models
      GET  /v1/capabilities
      POST /v1/chat/completions

    This allows existing OpenAI-compatible software to treat your local Arda fleet as one endpoint.

    Two Manwe modes

    Static mode

    The root supervisor currently starts:

      ─ text
      cargo run -p manwe -- --config manwe.toml

    This is the smaller fleet-backed gateway.

    Adaptive governed mode

    Manwe also supports a fuller runtime with:

    • Governance policy evaluation.
    • Provider quotas.
    • Persistent provider state.
    • Route previews.
    • Typed governance receipts.
    • Memory and telemetry hooks.
    • Model/provider adaptation.

    But services.toml does not currently start Manwe with --adaptive.

    So this major capability exists and is tested, but it is not yet the default root-daemon behavior.

    ────────────────────────────────────

    3. Oromë: communications and delivery truth

    Oromë is the communication layer.

    It defines:

    • Agent-to-human messages.
    • Agent-to-agent messages.
    • Threads, priorities, TTLs, signatures, and attachments.
    • Operator approvals and interruptions.
    • Provider adapters.
    • Bounded dispatch.
    • Retry and timeout behavior.
    • Fanout.
    • Streaming events.
    • Fleet-scope policy.
    • Dispatch metrics.
    • Delivery receipts.
    • Optional HTTP JSON transport.
    • Optional resident Hermes-style service behavior.

    A critical property is that Oromë distinguishes:

      ─ text
      "request accepted locally"

    from:

      ─ text
      "delivery was proven by a remote provider"

    A dispatch receipt only reports delivery as proven when the concrete transport succeeds and returns a provider message ID. A manual or simulated transport
    cannot silently masquerade as successful delivery.

    This is valuable because many agent systems confuse “the agent attempted a tool call” with “the outside system actually performed it.” Oromë gives Arda a
    place to preserve that distinction.

    ────────────────────────────────────

    4. Governance: the constitution

    arda-governance is one of the most sophisticated parts of the system.

    It can evaluate work through:

    • Aurelius-style alignment reasoning.
    • Bacon-style empirical evidence scrutiny.
    • Sun Tzu-style strategy/risk assessment.
    • Configurable governance chains.
    • Realm- and action-specific policies.
    • Evidence-grade assessment.
    • Scorer receipts.
    • Timeout and unavailable-scorer handling.
    • Readiness reports.
    • Human-review requirements.
    • Explicit runtime blocking decisions.
    • Philosopher profiles and arbitration.
    • Cooperation/defection dynamics.
    • Nonconformity and sycophancy checks.
    • Falsifiability and disconfirming-evidence checks.
    • Bounded environmental advisory signals.

    Its most important behavior is conservative:

    │ A policy configuration alone cannot silently grant autonomous blocking or execution authority.

    Runtime blocking requires:

    • An explicitly scoped policy.
    • Valid readiness evidence.
    • Rollback capability.
    • Independent review receipts.
    • Operator controls.
    • The appropriate authority gate.

    Environmental inputs—including audio, vision, solar, and geomagnetic context—are explicitly advisory. They can supply caution or context but cannot approve,
    reject, or block work by themselves.

    This is an unusually strong foundation for auditable autonomy.

    ────────────────────────────────────

    5. Mandos: bounded decision support

    Mandos is the Oracle.

    It takes a typed question and produces:

    • An explainable advisory verdict.
    • Conditions and concerns.
    • Evidence references.
    • A bounded reasoning graph.
    • Deterministic gate evaluations.
    • Status counters.
    • A durable verdict record.
    • Ledger integrity evidence.

    Its verdict history is:

    • Append-only.
    • Sequence-linked.
    • Hash-linked.
    • Restart-hydrated.
    • Verifiable.
    • Exportable only after integrity validation.

    Mandos deliberately does not have execution or approval authority.

    A good use is:

    │ “Given this evidence, policy, budget, and operational state, what action is supportable, what conditions must hold, and what should prevent us from
    proceeding?”

    That makes Mandos an advisor to the operator or orchestrator—not an autonomous executive.

    Current composition status

    Mandos provides service and daemon APIs with IPC and HTTP contracts, but the workspace exposes it as a library rather than a standalone Cargo binary. The
    root daemon does not currently start a Mandos service.

    ────────────────────────────────────

    6. Varda/Athena: knowledge and research

    Varda is the knowledge executor and Athena research system.

    It can:

    • Ingest local and external material.
    • Persist source digests.
    • Build and update a local search index.
    • Perform field-weighted BM25 retrieval.
    • Return source spans and citations.
    • Track whether results are shallow or deeply evaluated.
    • Run deep-analysis queues.
    • Cache deep-analysis results.
    • Recover unfinished scholarly enrichment.
    • Perform bounded crawling.
    • Stream citation-bearing query results over SSE.
    • Track source freshness.
    • Record pipeline IDs across the full ingest path.
    • Apply governance and policy-readiness gates.
    • Emit outcomes into Vaire memory.
    • Expose HTTP and IPC transport implementations.
    • Run reproducible retrieval benchmarks.

    It supports local evidence-backed research without immediately introducing a vector database. The current checked-in benchmark showed perfect results on its
    present fixture, so semantic/vector retrieval was correctly deferred until a reproducible BM25 failure justifies the added complexity.

    Current composition status

    Varda’s principal executable target is its benchmark. Its full service/daemon code exists as a library surface, and Aulë consumes its operator status, but
    the root daemon does not yet launch a first-class Varda service process.

    ────────────────────────────────────

    7. Vaire/Mnemosyne: memory

    Vaire provides Arda’s memory substrate.

    It supports:

    • Significance-weighted episodic records.
    • Scoped recent recall.
    • Relevance-based recall.
    • Identity state.
    • Knowledge seed recall.
    • Consolidation into semantic and procedural records.
    • Promotion receipts.
    • Optional Obsidian indexing.
    • Retrieval evaluation.
    • Durable status and observability snapshots.
    • Schema migration.
    • Malformed-record disclosure.
    • Optional contract-store dual-write.

    It is used by:

    • Varda for completed ingestion events.
    • Warden Scout for observations.
    • Oromë’s resident service mode.
    • Aulë metrics.
    • Manwe adaptive routing.

    This is not merely chat history. It is intended to differentiate:

    • What just happened.
    • What proved significant.
    • What should be remembered semantically.
    • What should become a reusable procedure.
    • What should remain low-significance noise.

    Current composition status

    Vaire is actively consumed by several crates, but there is no standalone Vaire binary and the root daemon does not instantiate a central memory service.
    Consumers currently construct the library or feature-gated service directly.

    ────────────────────────────────────

    8. Economics/Plutus: cost and resource accountability

    The economics crate records the resource consequences of agent activity.

    It can track:

    • Provider spend.
    • Work and energy estimates.
    • Measurement provenance.
    • Agent-level summaries.
    • Tariffs.
    • Budget pressure.
    • Return-on-investment metrics.
    • Relationship/cooperation values.
    • Ledger balances.
    • Append-only economic events.
    • Persistent runtime snapshots.
    • IPC and optional HTTP/SSE service behavior.

    This allows routing and governance decisions to eventually consider more than model quality:

      ─ text
      Can this provider perform the task?
      Is it healthy?
      Is it permitted?
      How expensive is it?
      How much energy does it consume?
      Was that measured or estimated?
      Was the result worth the resource expenditure?

    Aulë already exposes a plutus export operator command.

    Current composition status

    Like Mandos and Vaire, the economics runtime is library-hosted. It is not started by the root service registry today.

    ────────────────────────────────────

    9. Aulë: observability and operator coordination

    Aulë is the control room.

    It provides:

    • OpenTelemetry integration.
    • Prometheus-compatible metrics.
    • Governance metrics and status.
    • Memory metrics.
    • Economics exports.
    • Service graph inspection.
    • Tool manifest inspection.
    • Receipt lookup.
    • Runtime and evaluation receipts.
    • Learning delta lookup.
    • Athena status.
    • Bacon-Lite summaries.
    • Agent roster projections.
    • Council fanout.
    • Escalation tracking.
    • Execution intent state.
    • Runtime reconciliation.
    • Knowledge triage.
    • Knowledge-task promotion and execution.
    • A bounded autopilot loop.

    This is exposed through arda-cli when built with full-cli or http.

    This means Arda already has substantial operator machinery, although it is not yet surfaced coherently in the HUD or started by the root daemon.

    ────────────────────────────────────

    10. Warden Scout and outpost protocol

    The Scout is the remote sensing boundary.

    It can:

    • Survey bounded repository surfaces.
    • Query an operator-configured local SearXNG endpoint.
    • Run up to 16 configured research topics.
    • Validate source URLs.
    • Produce source-bearing observations.
    • Mark confidence, freshness, classification, and provenance.
    • Store complete observations through Vaire.
    • Recall observations by scope, path, time, age, or query.
    • Operate on a Pi5 or another bounded remote node.

    The outpost protocol ensures that all current remote observations have one of these authority classes:

    • Advisory.
    • Presentation.
    • Execution prohibited.

    No current outpost authority class permits execution.

    This is a strong distributed-system design: the remote node can observe and advise, but compromise or error at that node does not grant it task-queue or
    approval authority.

    ────────────────────────────────────

    11. Contract registry and launcher

    The contract registry makes Arda’s standards and evidence tracks machine-readable.

    The launcher uses it to show:

    • Whether the registry can be loaded.
    • Readiness projections.
    • A proposed service plan.
    • Which actions remain human-gated.

    The launcher is intentionally read-only. Its Begin action does not silently start services or mutate private configuration.

    This gives you an honest onboarding boundary rather than a button that ambiguously performs hidden system actions.

    ────────────────────────────────────

    12. HUD

    The current HUD is a minimal proof of connectivity.

    It:

    • Queries Manwe’s /v1/models.
    • Lists available models.
    • Allows manual refresh.
    • Proves that the desktop UI can observe the supervised inference gateway.

    It is not yet the full operator cockpit. It does not currently visualize:

    • Running tasks.
    • Governance verdicts.
    • Provider routes.
    • Memory events.
    • Economics.
    • Scout evidence.
    • Escalations.
    • Approval requests.
    • Agent geometry or council state.

    The backend capabilities required for many of those views already exist; the HUD has not yet projected them.

    ────────────────────────────────────

    What Arda can be used for today

    1. A sovereign local model gateway

    You can point OpenAI-compatible clients at Manwe and route inference across your configured local fleet.

    Use cases:

    • Local chat applications.
    • Coding agents.
    • Research tools.
    • Hermes.
    • Other OpenAI-compatible software.
    • Provider experimentation.
    • Fleet-aware model selection.

    This is the most immediately operational system-level capability.

    ────────────────────────────────────

    2. A supervised local AI stack

    The root daemon can keep the gateway and selected UI services alive, expose their status, restart failed children, and shut them down coherently.

    This gives you a single lifecycle owner instead of manually starting unrelated terminals.

    ────────────────────────────────────

    3. Bounded remote research

    The Pi5 Warden can perform recurring or operator-triggered web research through SearXNG and return source-bearing advisory observations.

    Good uses include:

    • Technical ecosystem monitoring.
    • Local model and inference developments.
    • Hardware and driver monitoring.
    • Cosmic-radiation and geomagnetic research.
    • Software dependency developments.
    • Security advisories.
    • Project-specific bounded reconnaissance.

    These observations can enter Vaire memory while remaining clearly marked as advisory evidence.

    ────────────────────────────────────

    4. Evidence-backed local knowledge work

    Varda can serve as the basis for:

    • Project documentation search.
    • Research corpus ingestion.
    • Citation-bearing answers.
    • Source-freshness monitoring.
    • Scholarly metadata enrichment.
    • Deep-analysis queues.
    • Reproducible retrieval evaluation.

    The implementation is ready; what is missing is a polished operator entrypoint and default root composition.

    ────────────────────────────────────

    5. Governed decision support

    Governance plus Mandos can evaluate consequential actions such as:

    • Changing providers.
    • Deploying a model.
    • Promoting research into an implementation task.
    • Approving a costly operation.
    • Selecting between competing architectural approaches.
    • Determining whether evidence is sufficient.
    • Identifying required rollback or human review.

    The system is explicitly designed to distinguish recommendation from authority.

    ────────────────────────────────────

    6. Auditable agent operations

    With Aulë, Vaire, Economics, Oromë, and governance receipts, Arda can preserve:

    • What was requested.
    • What evidence was available.
    • Which policy applied.
    • Which model/provider was selected.
    • Whether delivery was proven.
    • What the operation cost.
    • What happened.
    • What was learned.
    • What should be remembered.
    • What requires operator review.

    That is the foundation of a reproducible agent system rather than an opaque chain of model calls.

    ────────────────────────────────────

    What Arda does not yet do as one system

    This is the key current-state assessment.

    Arda does not yet automatically execute this complete loop:

      ─ text
      accept task
      → gather evidence
      → recall memory
      → govern
      → reason
      → select model
      → execute
      → account
      → learn
      → project to HUD

    Most individual capabilities exist, but there is no single root-owned workflow API joining all of them.

    Specifically:

    1. The root supervisor starts Manwe and UI processes, not Varda, Vaire, Mandos, Economics, or Aulë.
    2. Manwe starts in static mode, not full governed adaptive mode.
    3. Several runtime-capable crates remain library-only Cargo packages.
    4. There is no canonical POST /v1/tasks or equivalent end-to-end task API.
    5. The HUD is still only a model-catalog view.
    6. The launcher proposes a service plan but deliberately does not activate it.
    7. The harness does not proxy inference completions.
    8. Memory, economics, governance, and receipts are not yet shown together as one run timeline.
    9. Arda has bounded coordination and autopilot machinery, but not a default autonomous operating mode.
    10. Distributed outposts supply evidence but do not participate in execution authority.

    Therefore, I would characterize Arda as:

    │ A strong, tested agent-control substrate with an operational inference/lifecycle core and several mature domain runtimes, but not yet a unified turnkey
    agent product.

    ────────────────────────────────────

    Maturity assessment

      Axis                     Status                           Assessment
      ───────────────────────  ───────────────────────────────  ──────────────────────────────────────────────────────────────────────────────────
      Package engineering      Strong                           All packages are first-class, tested, linted, documented, and workspace-accounted
      Contracts and authority  Strong                           Clear ownership, typed receipts, bounded remote authority, conservative governance
      Inference                Operational                      Manwe is usable today; adaptive governed mode exists but is not default
      Process lifecycle        Operational baseline             Root registry, supervision, restart, status, and shutdown are implemented
      Knowledge and memory     Mature components                Varda and Vaire are capable, but not root-composed as one service flow
      Decision support         Mature component                 Mandos has strong integrity and evidence behavior, but is not root-hosted
      Economics                Mature component                 Accounting substrate exists, but is not in every default task path
      Observability            Strong backend, partial product  Aulë is broad; HUD projection is still minimal
      Operator experience      Early                            Launcher and HUD prove boundaries but do not operate the whole system
      Autonomous operation     Intentionally conservative       Foundations exist; readiness and human gates prevent accidental promotion
      Deployment               Partial                          Local daemon and gateway work; full fleet composition and soak remain

    ────────────────────────────────────

    The best way to move forward

    The next step should not be “add more capabilities.” You already have many capabilities.

    The next step should be to create one complete vertical workflow that proves the system behaves as a whole.

    Recommended first vertical slice

    I recommend:

    │ Research → governed synthesis → model execution → receipt → memory → operator view

    A concrete example:

    1. Operator submits a research question.
    2. Warden Scout gathers bounded web evidence.
    3. Varda ingests and retrieves relevant source-bearing material.
    4. Vaire recalls related prior knowledge.
    5. Governance assesses evidence quality, risk, and required review.
    6. Mandos returns an advisory verdict and conditions.
    7. Manwe selects a model and performs synthesis.
    8. Oromë records dispatch and delivery truth.
    9. Economics records provider cost and work provenance.
    10. Vaire records the result and learning significance.
    11. Aulë creates one correlated run timeline.
    12. HUD shows evidence, route, governance, cost, result, and receipts.

    That single workflow would demonstrate the actual value of every major subsystem.

    ────────────────────────────────────

    Recommended implementation phases

    Phase 1 — Activate existing runtimes

    Without creating another crate:

    • Decide whether each domain should run:
      • In-process through arda-engine, or
      • As a supervised service with IPC/HTTP.
    • Make Varda, Vaire, Mandos, Economics, and Aulë reachable from the root runtime.
    • Start Manwe in adaptive governed mode when configured.
    • Add health contracts to services.toml.
    • Preserve explicit feature/config control rather than enabling everything silently.

    Phase 2 — Add one canonical run contract

    Define a versioned run envelope with:

    • run_id
    • task/intent
    • evidence references
    • memory context
    • governance receipts
    • decision support
    • route receipt
    • delivery receipt
    • economics receipt
    • result
    • learning/memory receipt
    • human approval state

    Use existing receipt types wherever possible. Do not introduce duplicate ledgers.

    Phase 3 — Expose the workflow

    Choose one explicit API model:

    Option A: separated control and inference

    • Harness :7878 owns tasks, status, evidence, and receipts.
    • Manwe :7171 remains the inference API.

    This is the cleanest current architectural fit.

    Option B: unified public gateway

    • Harness proxies chat completions and task operations.
    • Clients only connect to :7878.
    • Manwe becomes an internal service.

    This provides a simpler product surface but adds proxy responsibility to the engine.

    I recommend Option A initially because it preserves Manwe’s mature OpenAI compatibility while keeping the harness focused on orchestration.

    Phase 4 — Turn the HUD into the operator cockpit

    The next useful HUD views are:

    1. Service and fleet health.
    2. Active and recent runs.
    3. Provider/model route decisions.
    4. Evidence and citations.
    5. Governance verdict and human gates.
    6. Scout observations.
    7. Memory writes and recall context.
    8. Cost and energy projections.
    9. Escalations and approval actions.
    10. Agent/council visualization.

    Only after those are coherent should you consider increasing autonomy.

    ────────────────────────────────────

    Bottom line

    You have built the foundation of a sovereign AI operations environment:

    • Manwe gives it access to intelligence.
    • Varda gives it knowledge.
    • Vaire gives it memory.
    • Mandos gives it judgment.
    • Governance gives it law and boundaries.
    • Economics gives it accountability.
    • Oromë gives it communication and delivery truth.
    • Aulë gives it visibility and coordination.
    • Warden gives it remote senses.
    • Engine gives it lifecycle and composition.
    • Launcher and HUD give the human a place in the system.

    The system is no longer a loose crate collection. The package boundaries and contracts now form a coherent architecture.

    But the most accurate status is:

    │ Arda’s components are first-class; its inference and supervision core is operational; its advanced knowledge, memory, governance, advisory, economics, and
    observability capabilities are mature but not yet composed into one default end-to-end runtime.

    The highest-value next milestone is not another crate audit. It is one complete, observable, governed workflow that passes through the whole system and ends
    in the HUD with correlated evidence and receipts.