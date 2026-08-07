# External Product Integration Research and Adoption Plan

> **Lifecycle:** Deferred reference retained outside the Arda 1.0 release scope on 2026-08-06. Candidates require fresh need, license, security, maintenance, and egress review before adoption.

> **For Hermes:** Treat every external product as an adapter candidate, not an architecture mandate. Before dependency adoption, reverify the current upstream release, license, security posture, maintenance activity, transitive dependencies, and data-egress behavior.

**Goal:** Reuse mature external ecosystems where they accelerate Arda while retaining Arda's Rust-owned governance, identity, run graph, evidence, memory, authority, and receipt contracts.

**Research date:** 2026-07-29  
**Research method:** Live official documentation and upstream repository searches, cross-checked against the current Arda tree.  
**Decision vocabulary:** `adopt`, `adapter`, `optional sidecar`, `reference/benchmark`, `defer`, `reject`.

---

## Executive recommendation

Arda should not replace its kernel with an external agent framework. It should use open standards and supervised products at clear boundaries.

### Integrate or complete now

1. **Model Context Protocol** — complete Arda's existing governed MCP boundary; do not create a second client/server stack.
2. **OpenTelemetry** — expand the existing optional Aulë OTLP exporter for interoperability; receipts remain authoritative.
3. **React Flow / `@xyflow/react` and Cytoscape.js** — compare for Workbench and RELIC graph visualization, never graph execution.
4. **CalDAV/CardDAV/iCalendar** — protocol-first Personal Operations interoperability.
5. **whisper.cpp** — optional local STT sidecar for Mirromere and universal capture.
6. **Three.js / React Three Fiber** — continue the dependencies already used by HUD/Launcher; evaluate `three-vrm` only for avatar interchange.

### Spike behind adapters

7. **Qdrant** — optional vector backend for scale/performance experiments; Vairë/Varda semantics remain canonical.
8. **Pipecat versus LiveKit Agents** — time-boxed voice-pipeline comparison; select at most one and keep it outside authority decisions.
9. **Home Assistant** — device/presence adapter, never Arda's identity or policy authority.
10. **Matrix Rust SDK** — sovereign communications transport candidate behind Oromë.
11. **Twenty** — read-only CRM sidecar candidate for Company Operations.
12. **Meilisearch** — optional lexical/hybrid search sidecar if existing local search cannot meet measured needs.
13. **Grafana OSS stack** — optional dashboard projection over OpenTelemetry; keep receipts and run state authoritative.
14. **OpenProject or Mattermost** — human-facing company/project and communication adapters, not Arda task authority.

### Learn from or benchmark; do not embed in 1.0

15. **OpenHands, Aider, Continue** — external-agent/UX benchmarks and possible project adapters; Hermes/Arda remain the orchestration boundary.
16. **Temporal** — study durable workflow/retry/visibility concepts; do not add a second orchestration authority to 1.0.
17. **Langfuse** — optional OTLP/export compatibility target; do not replace Aulë receipts or require cloud telemetry.
18. **OpenCTI** — evidence-graph and connector reference for bounded threat-intelligence research, not a general Warden replacement.
19. **Weaviate** — secondary retrieval reference behind the Rust-friendlier Qdrant candidate.
20. **MediaPipe** — future opt-in local perception sidecar after Mirromere's consent and deletion controls exist.
21. **Piper** — local TTS candidate only after GPL/process-boundary and voice-model licensing review.
22. **Activepieces/n8n/ERPNext/ComfyUI** — optional workflow-specific sidecars after licensing and scope review; none belongs on the Workbench critical path.

## Verified Arda overlap

- Oromë already implements MCP server/tool surfaces and governed external-source connector contracts; its `mcp/protocol.rs` still identifies part of the protocol surface as a placeholder.
- Aulë already has optional OpenTelemetry 0.32/OTLP dependencies and an emission/shutdown test.
- HUD and Launcher already depend on Three.js and React Three Fiber.
- `docs/architecture/FEDERATED_COMMS.md` already selects Matrix as the leading federated-room candidate.
- Personal Operations is already planned around iCalendar first and CalDAV later.
- Arda already has task, governance, memory, routing, telemetry, evidence, communications, and receipt domains. External products must not create competing sources of truth.

---

## Candidate matrix

| Candidate | Arda use | Recommended boundary | Stage | Decision |
|---|---|---|---|---|
| MCP | polyglot tools/data/workflows | Oromë governed protocol | 4 | complete existing |
| OpenTelemetry | traces/metrics/log export | Aulë optional exporter | 4 | expand existing |
| React Flow | Workbench run-graph UI | HUD library | 4 | spike/adopt if accessible |
| Cytoscape.js | RELIC/evidence graph UI | HUD library | 4/5 | compare/adopt if scalable |
| Qdrant | vector retrieval scale | Vairë/Varda sidecar | 5 | benchmark first |
| Weaviate | vector/RAG feature reference | secondary benchmark | 5+ | reference behind Qdrant |
| Meilisearch | lexical/hybrid search | read/query sidecar | 5 | benchmark first |
| OpenHands | coding-agent runtime | project adapter/benchmark | 5+ | no kernel embedding |
| Aider | focused coding workflow | CLI adapter/benchmark | 4/5 | optional |
| Continue | IDE UX/integration | protocol/UX reference | 5+ | defer adapter |
| Temporal | durable workflow engine | design reference | post-1.0 | do not dual-orchestrate |
| Langfuse | LLM observability | OTLP/export target | 5 | optional only |
| Grafana OSS stack | operational dashboards | OTLP/metrics projection | 5 | optional, license-gated |
| OpenCTI | threat-intelligence graph/connectors | Warden source/sink/reference | 5+ | bounded adapt/reference |
| Pipecat | voice pipeline | supervised Python sidecar | 5 | compare |
| LiveKit Agents | realtime voice/session | supervised sidecar | 5 | compare |
| whisper.cpp | local STT | local process adapter | 5 | preferred spike |
| Piper | local TTS | local process adapter | 5 | license gate |
| Home Assistant | device/presence | authenticated API adapter | 5/6 | read-only first |
| MediaPipe | local perception | capability-scoped sidecar | 6+ | defer |
| three-vrm | avatar interchange | Mirromere renderer library | 5 | spike |
| Matrix Rust SDK | federated comms | Oromë transport | 5 | focused spike |
| Radicale | CalDAV/CardDAV reference server | external service/protocol fixture | 5 | fixture/optional sidecar |
| Twenty | CRM | read-only Company Ops adapter | 5/6 | spike |
| Mattermost | sovereign team communications | Oromë webhook/bot adapter | 5+ | adapt, license-gated |
| OpenProject | project/company operations | API projection | 5+ | adapt, not task authority |
| Activepieces | business automation | optional supervised sidecar | 6+ | reverify license |
| n8n | broad automation | API/sidecar only | 6+ | source-available caution |
| ERPNext | ERP/accounting/CRM | API/export only | post-1.0 | too broad now |
| ComfyUI | creative media graphs | job adapter | post-1.0 | optional |
| Obsidian | human-readable notes | Markdown vault interop | 5 | no proprietary embedding |

---

## Adoption Plan A — Open interoperability foundations

### A1. MCP contract alignment

**Official source:** <https://modelcontextprotocol.io/docs/getting-started/intro>  
MCP describes an open standard connecting AI applications to tools, data sources, and workflows.

**Arda work**
- Audit `crates/spine/interface/arda-orome/src/mcp/` against the current protocol version.
- Consolidate duplicated/placeholder request-response types.
- Add initialization, capability negotiation, version mismatch, cancellation, timeout, and structured content tests.
- Wrap every tool with Arda authority, provenance, input/output size, network, secret, and receipt controls.
- Register tested MCP server profiles through the existing connector catalog.

**Reject if** MCP transport can invoke tools before Arda policy checks or if protocol content is treated as trusted instruction.

### A2. OpenTelemetry exporter

**Official source:** <https://opentelemetry.io/docs/what-is-opentelemetry/>  
OpenTelemetry is a vendor-neutral observability framework, not an observability backend.

**Arda work**
- Keep Aulë receipts as authoritative state.
- Export correlated run/task/route spans with redacted attributes.
- Test exporter absence, endpoint failure, bounded queueing, shutdown, and no-secret guarantees.
- Make export opt-in and loopback/off by default.

**Reject if** observability failure changes run outcomes or requires cloud service enrollment.

**Optional dashboard projection:** evaluate the Grafana OSS stack only after OTLP export is stable. Operate it as a removable service over redacted telemetry, review the AGPL and component-specific licenses, and do not make Grafana availability part of run correctness.

### A3. Workbench and RELIC graph rendering

**Official source:** <https://github.com/xyflow/xyflow>  
Evaluate `@xyflow/react` for accessible run-graph rendering and interaction.

**Alternative official source:** <https://github.com/cytoscape/cytoscape.js>  
Evaluate Cytoscape.js for RELIC/evidence graphs where graph layouts, compound nodes, and graph-analysis extensions may be more useful than workflow editing.

**Spike**
- Render 500-node fixture plus event updates.
- Test keyboard navigation, screen-reader labels, reduced motion, critical-path focus, and collapsed subgraphs.
- Compare both libraries with a minimal in-house SVG renderer.

**Adopt only if** it improves accessibility/maintainability without making React UI state the runtime graph authority.

---

## Adoption Plan B — Development agent interoperability

### Candidates

- OpenHands: <https://github.com/All-Hands-AI/OpenHands>
- Aider: <https://github.com/Aider-AI/aider>
- Continue: <https://github.com/continuedev/continue>

### Intended use

- benchmark task completion, diff quality, context strategy, and recovery UX;
- permit a project contract to invoke an installed agent as a bounded adapter;
- import structured result/diff/test evidence;
- compare against Hermes/Codex lanes without forcing one agent implementation.

### Prohibited integration

- no agent owns Arda approvals;
- no agent gets unrestricted home-directory access;
- no agent result becomes `Complete` without project-native verification;
- no hidden cloud account or telemetry becomes mandatory;
- no vendoring before license/SBOM and subprocess-isolation review.

### Conditional Stage 4 spike — DEFERRED

Do not invoke an external product while Stage 4 is blocked by Arda-owned native
commands, adapter contracts, and execution flows. If a measured acceptance
failure later proves that an external-agent boundary is the blocker, first
implement one generic `ExternalAgentAdapter` fixture around a fake CLI. Only
after that fixture passes the isolation criteria below may Aider be evaluated.
OpenHands and Continue remain Stage 5 candidates after their current
architecture and distribution licenses are reverified.

### Success criteria

- cancellation terminates the process tree;
- stdout/stderr and protocol frames are bounded;
- diff stays inside project root;
- route/model/cost provenance is captured when exposed;
- agent crash or malformed output produces `Failed`/`NeedsReview`, never completion.

---

## Adoption Plan C — Retrieval and knowledge

### Qdrant

**Official source:** <https://github.com/qdrant/qdrant>  
Evaluate as an optional vector index, not the memory authority.

### Meilisearch

**Official source:** <https://github.com/meilisearch/meilisearch>  
Evaluate for local lexical/typo-tolerant search and hybrid candidate retrieval.

### Weaviate reference

**Official source:** <https://github.com/weaviate/weaviate>  
Use as a secondary feature/architecture benchmark for hybrid retrieval and cross-references. Prefer Qdrant for the first operational spike because its official Rust client better matches Arda's implementation boundary; adopt neither until the benchmark proves need.

### Benchmark before adoption

Use a frozen Arda corpus and measure:
- recall and precision for known-answer queries;
- source/citation preservation;
- update/delete latency;
- deterministic rebuild;
- disk/RAM/idle use;
- backup/restore;
- corrupted/unavailable sidecar behavior;
- Pi5 feasibility where relevant.

Vairë/Varda keep canonical records, evidence eligibility, retention, provenance, and deletion. The sidecar stores reconstructible indexes only.

**Decision gate:** adopt no database until existing search fails a measured Stage 5 requirement.

### OpenCTI evidence-graph reference

**Official source:** <https://github.com/OpenCTI-Platform/opencti>  
Use OpenCTI as a bounded reference for threat-intelligence ontologies, source connectors, and evidence relationships. A future Warden security-research adapter may exchange selected STIX/TAXII-shaped records, but OpenCTI must not become Arda's general evidence, governance, or memory authority. Review Community Edition versus enterprise-file licensing before reuse.

---

## Adoption Plan D — Voice, avatar, and presence

### Local STT: whisper.cpp

**Official source:** <https://github.com/ggml-org/whisper.cpp>  
Preferred first local STT spike because it offers a process boundary and local inference.

Measure first-token/final latency, word error on operator speech, CPU/GPU use, cancellation, offline behavior, model provenance, and temporary-audio deletion.

### Voice pipeline: Pipecat versus LiveKit Agents

- Pipecat: <https://github.com/pipecat-ai/pipecat>
- LiveKit Agents: <https://github.com/livekit/agents>

Run a two-week maximum spike using the same synthetic and consented local audio fixtures. Compare interruption/barge-in, endpointing, session recovery, local-provider support, resource cost, observability, packaging, and license/dependency surface.

**Choose at most one.** If neither clearly beats a small Arda-owned adapter, reject both.

### Local TTS: Piper

**Official source:** <https://github.com/OHF-Voice/piper1-gpl>  
The current upstream identifies GPL-3.0 licensing. Treat it as a separate optional process until legal/distribution and voice-model licenses are reviewed. Never assume model weights share engine licensing.

### Avatar: three-vrm

**Official source:** <https://github.com/pixiv/three-vrm>  
Spike as a VRM interchange/rendering extension over the Three.js stack already present. Mirromere's monitor-first geometric form does not depend on a humanoid model.

### Perception: MediaPipe

**Official source:** <https://github.com/google-ai-edge/mediapipe>  
The repository describes cross-platform live/streaming media ML and identifies Apache-2.0 licensing. Defer until camera capability indicators, consent, local-only processing, retention/deletion, and false-inference UX are already proven.

---

## Adoption Plan E — Personal operations and environment

### CalDAV/CardDAV and Radicale

**Official source:** <https://github.com/Kozea/Radicale>  
Radicale is a GPL-3.0 CalDAV/CardDAV server storing data in a simple filesystem layout. Use it as an optional sidecar and integration-test fixture, not embedded code.

Sequence:
1. `.ics` import/export;
2. protocol fixtures;
3. read-only CalDAV sync;
4. conflict and timezone tests;
5. governed writes;
6. optional Radicale deployment profile.

### Home Assistant

**Official source:** <https://github.com/home-assistant/core>  
Use its authenticated API/event stream as the device boundary instead of reimplementing every smart-home protocol.

Sequence:
1. read-only entity catalog and state;
2. allowlisted presence/environment events;
3. explicit service-call preview;
4. narrow reversible actions such as approved lighting scenes;
5. emergency stop/revocation.

Home Assistant observations are contextual and never medical or identity authority.

### Obsidian/local Markdown

**Official release source:** <https://github.com/obsidianmd/obsidian-releases>  
Obsidian is not an Arda runtime dependency. Support user-owned Markdown vault import/export with explicit roots, frontmatter mapping, conflict handling, and no assumption that proprietary app internals are stable APIs.

---

## Adoption Plan F — Communications and company operations

### Matrix Rust SDK

**Official source:** <https://github.com/matrix-org/matrix-rust-sdk>  
The SDK identifies Apache-2.0 licensing and Rust/client bindings. Implement behind Oromë so attempted, accepted, delivered, failed, encrypted, and read states remain transport-aware.

**Stage 5 spike**
- one allowlisted room;
- receive/read projection;
- approved outbound message;
- encrypted-session recovery;
- deduplication after restart;
- attachment limits and redaction.

### Twenty CRM

**Official source:** <https://github.com/twentyhq/twenty>  
Evaluate as a separately deployed CRM for Company Operations. Reverify its current license and API before adoption.

**Sequence**
1. API schema and auth review;
2. read-only organizations/contacts/opportunities;
3. external-ID mapping and deterministic sync;
4. privacy/redaction and backup tests;
5. only then consider governed outbound updates.

### OpenProject and Mattermost

- OpenProject: <https://github.com/opf/openproject>
- Mattermost: <https://github.com/mattermost/mattermost>

Evaluate OpenProject as a human-facing project/work-package projection and Mattermost as a sovereign notification, webhook, or bot transport. Arda remains authoritative for governed tasks, approvals, receipts, and execution. Both require explicit GPL/AGPL/open-core boundary review, API fixture tests, deterministic external-ID mapping, and a no-duplicate-delivery restart test.

### Workflow engines

- Activepieces: <https://github.com/activepieces/activepieces>
- n8n: <https://github.com/n8n-io/n8n>
- ERPNext: <https://github.com/frappe/erpnext>

Use only through APIs/webhooks or supervised sidecars. Reverify current licenses: n8n is commonly distributed under a source-available/fair-code posture rather than an OSI open-source license, and ERP/automation suites have far more scope than Arda 1.0 needs. Prefer a small project/company adapter until a measured workflow justifies deployment.

### Creative jobs

ComfyUI: <https://github.com/comfyanonymous/ComfyUI>  
Potential post-1.0 adapter for media-generation workflows. Treat workflow JSON, models, licenses, GPU contention, and output provenance as governed job artifacts. It is unrelated to Workbench's release-critical path.

---

## Products explicitly not selected as Arda's kernel

### Temporal

**Official source:** <https://github.com/temporalio/temporal>  
Study durable execution, event history, retry, cancellation, and visibility. Do not introduce Temporal as a second run graph/event authority during Stages 4–6. Revisit only if measured multi-node durability requirements exceed Arda's local-first engine and Rust SDK/support posture is acceptable.

### Langfuse

**Official source:** <https://github.com/langfuse/langfuse>  
Useful observability comparison or optional export target. Aulë/receipts remain canonical; no private prompt/evidence export by default.

### External agent frameworks

They may execute bounded tasks, but they do not own Arda identity, policy, memory promotion, run completion, or approval.

---

## Common integration contract

Every accepted product adapter must declare:
- product and adapter identity/version;
- upstream source, license, package/image digest, and SBOM;
- process/network/filesystem capabilities;
- secret references and data-egress policy;
- health/readiness/freshness;
- input/output schemas and limits;
- timeout, cancellation, retry, and idempotency;
- receipt correlation and provenance;
- backup/restore and uninstall behavior;
- offline/degraded behavior;
- support and compatibility policy.

## Spike template

Each spike produces:
1. a fixture-backed adapter;
2. a threat and data-flow diagram;
3. license/SBOM record;
4. measured latency/resource/reliability results;
5. failure and cancellation tests;
6. adopt/sidecar/reference/reject decision;
7. complete removal instructions.

A spike cannot silently become a production dependency.

## Stage adoption sequence

### Stage 4
- complete MCP boundary;
- expand existing OpenTelemetry correlation;
- choose Workbench graph renderer;
- compare Cytoscape.js for RELIC/evidence graph rendering;
- build generic external-agent test adapter and optional Aider spike;
- no new database or workflow engine.

**Gate review (2026-07-30): no external-product spike authorized in this
tranche.** The live Stage 4 blockers are native project attachment/run APIs,
bounded edit/test/review execution, receipt correlation, restart/resume, and
Rust/Python golden paths. The current draft graph is sufficient for contract
validation, so a React Flow/Cytoscape dependency would not unblock the next
gate. MCP, OTLP, Aider, database, voice, and presence spikes remain deferred
until a measured Stage 4 acceptance failure specifically requires one.

### Stage 5
- CalDAV/Radicale fixture and Personal Operations adapter;
- Qdrant/Meilisearch benchmark;
- whisper.cpp and Pipecat/LiveKit comparison;
- Matrix/Oromë transport spike;
- Home Assistant read-only adapter;
- Twenty read-only CRM spike;
- optional OpenProject/Mattermost API boundary assessment;
- optional Grafana dashboard over redacted OTLP telemetry;
- adapter SDK conformance and licensing automation.

### Stage 6
- ship only candidates that passed security, licensing, packaging, recovery, and support gates;
- classify all others beta/preview/unsupported;
- keep optional services removable and disabled by default;
- publish third-party notices and version compatibility.

## Research limitations

- Upstream capabilities and licenses can change; links and license statements are point-in-time evidence, not permanent legal advice.
- Repository popularity was not used as an adoption criterion.
- No external product was installed or benchmarked during this research pass.
- Product adoption remains blocked on the spike template and the stage-specific need.
