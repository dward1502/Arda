---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "architecture_audit"
  owner: "HERMES"
  status: "draft"
  reviewed: "2026-10-01"
---

# Installed experience versus intended use

## Assessment correction

The initial inventory did not establish what the application feels like or whether useful work flows through it. Advice to shelve the project was not supported by that investigation. This supplement records actual installed-runtime observations, not a replacement vision or an implementation plan.

The operator's stated goal is a central place to direct multiple model workers: independent research into subjects such as Maxwell physics, spacetime metrics and heart biology; cultivated contextual libraries containing different media; agents exchanging ideas and bringing back intelligible proposals and infographics; operator edits and approval before information enters a corpus or consequential action occurs. Business tasks, x402 services, video production and voice input are additional uses. The system should fit a minimal budget and an existing subscription. These examples are requirements, not authorization to post publicly, spend money, enroll services or conduct medical decision-making.

## What was actually operated

Started the existing installed arda-hud.service. It remained active at the follow-up check, with MemoryCurrent 316964864 bytes. The daemon and Manwe were already listening; neither was restarted. Native computer-use discovery found no windows on this Wayland desktop. Instead, native AT-SPI actions opened workstations, their accessibility content was read back, and desktop-portal screenshots were inspected. The retained screenshots are cropped to the HUD monitor rather than including unrelated desktop applications.

- [Actual boardroom screenshot](evidence/live/boardroom.png).
- [Actual personal workstation screenshot](evidence/live/personal-workstation.png).
- Native readbacks: [planning](evidence/live/planning-native-accessibility.json), [research](evidence/live/research-native-accessibility.json), [personal](evidence/live/personal-native-accessibility.json), [business](evidence/live/business-native-accessibility.json).

The boardroom is a real rendered spatial environment, not merely a diagram in a README. Its idle monitors show abstract patterns. They do not tell the operator which research worker is doing what, what it found, or what decision needs attention. This can be consistent with the documented idle display contract while still failing the desired initial experience. The personal panel visibly renders over that scene; opening a workstation was not just an accessibility-tree artifact.

## Concrete findings

### 1. Planning exposes implementation mechanics before useful work

The native Planning And Queue workstation displays stale source projections, large priority counts, stage-contract terminology and a task-pivot command targeting arda-cli. Its Workbench asks for an absolute project-contract path, approval reference, receipt digest, review-evidence JSON and run ID. It reports no attached project, no run, no tests and no waiting approvals. It does have a plain-language operator summary, but its next action is to validate a typed project contract.

This is not the user-facing action 'assign this worker a research topic and bring me a reviewable result.' The UI imposes knowledge of internal authority representations. Whether the underlying contract is necessary is separate from whether the human should author it manually. The large queue counts are not evidence of productive currently executing workers.

### 2. Research is present, but a functioning ongoing research loop is not established

The native Knowledge And Reasoning workstation successfully loaded the typed harness projection. It contains question composition, watchlist creation, pause/resume, six retained briefs, citation provenance and explicit preview/fetched/evaluated/approved/proposal distinctions. Existing questions include Rust guidance and lawful x402 earning opportunities. The displayed Tokio cancellation-safety brief cites two sources and is marked stale. A watchlist displays a 24-hour cadence. That does not prove it is successfully running every day.

Source anchors: ResearchModule.tsx lines 24-80 manage questions, watchlists and readback; lines 108-132 render briefs/citations and proposal/approval identifiers. crates/engine/src/harness/research.rs lines 1-5 describe the discovery -> canonical fetch -> evaluation -> advisory brief boundary. These are relevant existing parts, not an absent research design.

The configured remote Warden health endpoint timed out during a bounded direct check. No new research question or recurring watchlist was created during this assessment. Fresh retrieval, synthesis, human correction and approved corpus insertion were not demonstrated end to end. No dedicated infographic production or corpus-editing action was observed in this opened research panel; that is not proof none exists elsewhere.

### 3. Local inference works; distributed capacity is currently impaired

[Manwe status](evidence/live/7171_status.json) reported 22 configured providers, 8 enabled, 3 ready and 5 unhealthy among the enabled providers. The ready set was edge_core, openrouter and openai_sub. Readiness is not a successful workload benchmark or an assurance of free usage.

[Provider details](evidence/live/7171_providers.json) identify the Beelink Bonsai worker and two backbone lanes as unhealthy with transport failures. Guardhouse reports a prior HTTP 400. Direct checks of the configured Beelink inference endpoint, backbone review endpoint and Warden endpoint each timed out after a four-second connection bound. The local Tailscale status listed only this workstation. This establishes lack of reachability from this session, not that those machines are powered off or incapable.

I executed a small document-extraction task through Manwe with the explicit local model edge_core/Qwen3.5-9B-Q4_K_M. It returned HTTP 200 in 4.51 seconds, using 900 prompt tokens and 131 completion tokens. Its server-reported generation rate was approximately 81 tokens/second for this one short request. The answer correctly extracted README claims but does not independently verify those claims. No paid provider was selected.

[Actual response](evidence/live/local-inference-probe.json) and [independently read-back route receipt](evidence/live/verified-local-route.json) share route ID e516f49fdf3de41d. This proves usable local routed inference, not multi-worker autonomy, sustained throughput or scientific research quality. It does refute a claim that no usable local inference capacity exists.

### 4. Process health conceals a stalled execution subsystem

[Daemon status](evidence/live/7878_v1_status.json) shows Manwe healthy but fleet Prometheus unreachable and no Beelink targets. [Objective runtime](evidence/live/7878_v1_objective-runtime.json) reports ready=false, phase=degraded, last_error=objective_round_failed, no active leaves and no next wake time. Therefore a live daemon and green Manwe health do not establish continuous objective execution.

Unauthenticated probes of next-action and mesh returned 403. Those are access boundaries, not proof of broken endpoints; no auth bypass was attempted. The precise cause of the objective-round failure remains unresolved in this pass. Recent warning-level arda.service journal lookup had no entries; that does not establish absence of errors in other owners or logs.

### 5. Personal/business views accurately expose little current operational value

The native personal workstation reports no active Hermes session projection, stale human/business/personal/lineage sources, three missing referenced paths and no receipt-backed realized value. It withholds private context on the shared display; this privacy boundary was not bypassed.

The business workstation reports no active client engagements, qualified opportunities, approved commitments due, bounded paid experiments or external drafts awaiting approval. Its suggested action is 'Review evidence before creating commercial work,' not a specific evidence-backed opportunity. It also displays client-path and state-key inventory, which should not be confused with an operating business.

Rust x402 matches in this pass lead to an optional approval-bound payment capability and offline fixture verification (crates/spine/governance/arda-core/src/payment_capability.rs), not evidence of a live earning service. No claim is made that all integrations have been exhaustively excluded. Voice in the opened planning panel is explicitly described as a future producer of the same objective contract; voice input was not exercised.

## What this means for the stated goal

There are relevant foundations: a rendered central environment, real routed local inference, research questions/watchlists/retained citations, and authority/retention boundaries. The observed product nevertheless does not deliver the intended experience reliably. Too much of the first-contact UI explains Arda's internal machinery, while remote workers are unreachable, the objective runtime is degraded and retained research is stale.

The immediate demonstrated constraints are integration, reachability, runtime recovery and understandable operator actions. Budget is a separate constraint. A small local model successfully performed this short extraction; this offers evidence for bounded local assistance, not for running several expert autonomous research teams or revenue businesses around the clock. Neither commercial success nor impossibility follows from these checks. A subscription is not proof of unlimited automated capacity.

The appropriate acceptance target is an existing research workflow brought through one complete cycle: explicit question -> worker performs bounded retrieval and synthesis -> source-linked result shown to the operator -> operator edits/accepts/rejects -> approved material retained in the selected library -> worker resumes within the authorized budget. Distinct topics can then share a bounded queue rather than requiring one large simultaneously resident model per topic. This is an assessment of a credible verification target, not a claim that this loop already works or a request to narrow the whole product to research.

## Boundaries and next unresolved evidence

No application source, provider configuration, privacy policy or existing project contract was changed. No paid/cloud inference, public post, payment, new scheduled research task or business action was initiated. Started HUD remains running; the last inspected workstation is Business Operations. Audit navigation and one local inference call are the intentional runtime side effects.

Remaining assessment work includes: the exact objective-runtime failure, remote-node connectivity cause and capacity when reachable, a fresh research cycle and reviewed corpus insertion, media/infographic publication, actual voice input, and multi-worker pause/resume/recovery. This supplement is not full end-to-end acceptance or proof that all requested capabilities exist. It replaces speculation with a bounded installed-runtime assessment and names what is still unproven.
