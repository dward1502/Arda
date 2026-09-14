---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "foundation_audit"
  owner: "HADES"
  status: "active"
  reviewed: "2026-08-21"
---

> 🜏 Soterion: 📜 foundation_audit | owner: HADES | status: active | reviewed: 2026-08-21

# Digital Organism Foundation Maturity Matrix

Task: `digital-organism-s0-foundation-matrix`

Program (archived): [`../archive/digital-organism/README.md`](../archive/digital-organism/README.md)

Machine sidecar: `../../.hermes/evidence/digital-organism/foundation-matrix.json`

Repository baseline: `be0fdaf20816fdc700ec9301e2400132cb9f5e0b`

## Verdict

Arda has substantial organism anatomy, and several of its most important foundations are real: the root daemon is installed and supervising Manwë; the harness composes Core, Governance, Oromë operator intake, durable runs, Vairë continuity, Varda evidence, Rúmil review, Personal Operations, and Aulë presence; the engine has a tested and historically workflow-proven Hermes worker adapter with restart-safe receipts; Manwë performs live governed inference routing; Warden and two local/edge model nodes answer; Hermes carries real operator conversation and routes delegation through Manwë.

The system is not yet a cohesive digital organism because the nervous-system and executive identity layers have multiple competing or latent authorities. The root does not run an A2A mesh. Oromë’s message envelope is production-compiled but not transported; its message router is test-only. Three agent/topology registry families coexist. Arandur/CEO runs as a separate read-only timer process and still references stale/missing realm/world authority. Fleet configuration is richer than current live node truth. Queue projections call dependency-gated plan tasks active while next-action and Arandur correctly exclude them. The operator projection is fresh but does not converge current services, capabilities, workers, and completed research evidence.

This is an integration and authority-convergence problem, not evidence that the prior work is useless. Task 1 establishes what can be reused. It does not authorize Stage 1 implementation, archive any component, or claim the organism loop is proven.

## Audit method and evidence boundary

The audit used:

- root `Cargo.toml`, `cargo metadata`, default dependency closure, manifests, and crate roots;
- root `src/main.rs`, `services.toml`, engine harness, run scheduler, Hermes adapter, and direct consumers;
- installed user services/timers, process tree, listeners, binary hashes, and exact endpoints;
- live fleet health/model probes and one real Manwë inference request;
- current Hermes status, toolsets, profiles, delegation/MoA configuration, enabled plugin, and plugin tests;
- canonical queue, generated projections, next-action, operator projection, historical run receipts, and current Arandur/autopilot output;
- focused Rust checks/tests listed below.

Maturity terms remain separate:

`documented → implemented → focused-tested → default compile closure → root-composed → installed → running → workflow-proven → operator-accepted`

A library dependency is not root construction. A listener is not workflow proof. A timer is not executive coherence. Historical proof is retained but is not current running state.

## Current runtime snapshot

### Root and installed artifacts

- `arda.service` is active under PID `1823` and had run for approximately eleven hours at audit time.
- The installed `/var/home/mythos/.local/bin/arda` and source-built `target/release/arda` matched byte-for-byte:
  `sha256:6fdbab06403a2648ec4fc97fcb7b51bdfaea5d129ed266118238fed9db9c290d`.
- Root runs `arda --no-ui` and supervises only Manwë PID `1916`.
- Harness `127.0.0.1:7878` and Manwë `127.0.0.1:7171` listen on loopback.
- Varda, metrics, Crawl4AI, and RELIC run as separate user services rather than root-supervised children.
- No user-systemd units were failed.

### Live route and fleet evidence

| Node | Configured | Current direct probe | Evidence class |
|---|---|---|---|
| Core hub / `edge_core` | active | HTTP 200, one model; real Manwë completion selected it | inference-proven for bounded probe |
| Pi5 Warden / `edge_guardhouse` | active | health HTTP 200; scout proxy HTTP 200 | service reachable; advisory node |
| Beelink / `edge_beelink_light` | active | HTTP 200, one model | listener/model catalog reachable |
| Beelink Carnice | offline | connection refused | matches intentional offline state |
| Laptop | offline | timed out | matches optional/offline posture |
| Backbone review | active | timed out | configuration/runtime disagreement |
| Backbone coder | active | timed out | configuration/runtime disagreement |
| CITADEL avatar | active | timed out | physical delivery unconfirmed |

A real `model=auto` completion returned Manwë route evidence for `edge_core/Qwen3.5-9B-Q4_K_M`. The request proved routing and inference. It did not obey the exact-output instruction before a 32-token cap because it emitted visible thinking text and ended with `finish_reason=length`; exact instruction compliance is not claimed.

### Hermes evidence

- Hermes v0.20.5 gateway is running with Discord configured.
- Only the `default` profile exists; no Bot roster is active.
- Delegation is enabled and points to `custom:arda-manwe` at `127.0.0.1:7171/v1`.
- MoA is configured but not active.
- Hermes A2A toolset is disabled.
- `arda-operator-bridge` v0.5.0 is the only enabled user plugin.
- The plugin registers only `pre_gateway_dispatch`; it does not currently emit subagent lifecycle, placement, or A2A receipts.
- Plugin verification passed 16/16 tests.
- Hermes reported 42 upstream commits available; updating was outside Task 1.

### Current work and projection evidence

- Canonical queue has 15 records: three operator objectives and twelve digital-organism stage tasks.
- `queue_active.json` calls all 15 active.
- Authenticated `arda.next-action.v1` selects only the operator’s critical current objective and excludes the twelve program tasks as `inferred_without_review`.
- Arandur’s read-only objective selector similarly considers only the three operator objectives and selects none because all require review.
- The operator projection is fresh but shows one pending economic-intelligence run with no workers, no capabilities, and no service rows. Source-backed research artifacts already exist for that run, so terminal/outcome reconciliation remains incomplete.
- Historical `queue-runtime-proof-20260813-v2` contains a valid succeeded Hermes execution receipt with tool/test evidence and usage. This proves the bounded historical Workbench path, not a current multi-node organism.

## Foundation maturity matrix

Legend: **Y** = directly verified; **N** = absent/not active; **H** = historical or narrower proof only; **—** = not applicable.

| Component | Owner | Implemented/tested | Default closure | Root-composed | Installed/running | Workflow proof | Primary gap | Disposition |
|---|---|---:|---:|---:|---:|---:|---|---|
| Root daemon and harness | `arda-engine` | Y | Y | Y | Y | Y | Root supervises Manwë only; other organs have separate lifecycle owners | **ADAPT** |
| Core task/run/capability contracts | `arda-core` | Y | Y | Y | Y | Y | organism manifest/context absent; duplicate Oromë state family exists | **REUSE** |
| Governance primitives | `arda-governance` | Y | Y | Y | Y | Y | execution paths must prove enforcement calls; crate does not own enforcement | **REUSE** |
| Worker scheduling/recovery | `arda-engine` | Y | Y | Y | Y | Y | route classes are not enrolled node identities; delegate_task is outside receipts | **ADAPT** |
| Hermes execution adapter | `arda-engine` | Y | Y | Y | Y | H/Y | durable historical proof exists; no cross-node/A2A lifetime yet | **ADAPT** |
| Oromë operator bridge/provider dispatch | `arda-orome` | Y | Y | Y | Y | Y | ManualTransport does not prove delivery; historical delivery rows remain pending | **REUSE** |
| Oromë A2A envelope/thread types | `arda-orome` | Y | Y | N | installed library, not active | N | no cross-process consumer or standard A2A mapping | **ADAPT** |
| Oromë in-memory registry | `arda-orome` | Y | feature/test only | N | N | N | duplicates Core and Prometheus registries | **UNRESOLVED / INSPECT** |
| Oromë `MessageRouter` | `arda-orome` | Y | test-only | N | N | N | queue/retry/dead-letter behavior never enters production closure | **UNRESOLVED / INSPECT** |
| Core persistent Oromë registry/router | `arda-core` | Y | Y | not constructed | installed library | N | second registry; shared router implementation is a placeholder | **UNRESOLVED / INSPECT** |
| Hermes conversation/worker runtime | Hermes Agent | Y | external | external bridge | Y | Y | A2A disabled; one profile; no subagent lifecycle receipts | **ADAPT** |
| Manwë routing | `manwe` | Y | Y | Y | Y | Y | catalog contains unreachable active routes; no canonical node-resource input | **ADAPT** |
| Vairë memory/continuity | `arda-vaire` | Y | Y | Y | Y | Y | no general organism context capsule or memory-use receipt | **REUSE** |
| Varda evidence/research | `arda-varda` | Y | Y | Y through harness | separate service Y | Y | service `/health` 404; pending run not reconciled with evidence | **ADAPT** |
| Rúmil audit/review | `arda-rumil` | Y | Y | Y/on demand | library | Y | no generic node placement/role contract | **REUSE** |
| Mandos reasoning/evidence | `arda-mandos` | Y | Y | Y/on demand | library | Y | legacy Oracle vocabulary; not a live node | **REUSE** |
| Economics/JouleWork | `arda-economics` | Y | Y | Y | library | Y | placement does not join measured node energy/pressure | **REUSE** |
| Aulë metrics/presence | `arda-aule` | Y | Y/basic only | Y/basic only | metrics Y | H/Y | basic projection composed; full CEO stack excluded | **ADAPT** |
| Prometheus/Arandur CEO | Aulë full CLI | Y | full-cli only | N | timer Y | N | held read-only; no selected objective/outcome; stale realm/world identity | **ADAPT** |
| Fleet and outpost contracts | engine/outpost | Y | Y | Y/partial | configuration/services | H/Y | config is not expiring topology; three active nodes unreachable | **ADAPT** |
| Warden scout | outpost scout | Y | external node | proxy composed | Y | Y | advisory, not generic A2A worker | **REUSE** |
| RELIC presence bridge | RELIC | Y | external service | N | Y | N | CITADEL unreachable; no delivery acknowledgement | **ADAPT / HOLD** |
| Contract registry | contract registry | Y | launcher/CLI only | N | launcher consumer | H | July metadata/claims; root does not consume it | **ADAPT** |
| Queue ledger | canonical task authority | Y | full-cli consumer | timer path | Y | Y | none requiring replacement | **REUSE** |
| Queue active/summary projections | generated consumers | Y | external generator | N | current | H | dependency-gated tasks mislabeled active; metadata lost | **ADAPT** |
| Operator projection | engine/Aulë | Y | Y | Y | Y | H | services/capabilities/workers/evidence do not converge | **ADAPT** |
| Legacy `world.json`/realm projections | historical Prometheus/Core | Y | full-cli compatibility | N | stale file | N | Arandur ONLINE/READY from March heartbeat; expected realm paths absent | **ARCHIVE CANDIDATE** |
| HUD/Mirromere surfaces | applications | Y | external | N under `--no-ui` | stopped/dirty | H | must consume future organism truth, not mint it | **ADAPT / HOLD** |

The machine sidecar separates some combined table rows and records 24 disposition groups: 8 reuse, 12 adapt, 2 adapt/hold, 1 unresolved/inspect, and 1 archive candidate.

## Source-backed architectural findings

### 1. The root is already a substantial integration spine

`arda-engine` is not a stub. Its default closure includes Core, Governance, Oromë, Manwë, Vairë, Varda, Rúmil, Mandos, Aulë, and outpost protocol. The harness exposes concrete paths for operator intake, continuity, Personal Operations, projects, research, runs, model catalog, next action, operator projection, presence, and remote Warden proxying.

The limitation is runtime composition: root constructs these mostly as request-scoped library paths and supervises only Manwë. Independent services and Hermes are not represented as enrolled organs under one live topology contract.

### 2. The A2A framework exists but is anatomy, not circulation

Oromë’s A2A message contract carries useful semantics: request/response/notification/handshake/heartbeat, priority, TTL, reply/thread lineage, signatures, and hops. Those types are exported in the default library.

The agent registry is feature/test gated; the message router is test-only. No root path serializes an Oromë A2A envelope to a standard A2A peer or receives a result. Stage 2 should map semantic fields onto standard Hermes/Linux Foundation A2A instead of turning the test router into a proprietary network protocol by default.

### 3. Registry ownership is the largest Stage 0 ambiguity

At least three registry/state models overlap:

1. Oromë `AgentRegistry`: in-memory, capability/realm indexes, heartbeat/pruning, feature/test gated.
2. Core `orome_runtime::AgentRegistry`: file-persisted records and a separate `RouterState`; shared-router storage is a placeholder.
3. Prometheus roster/world: parsed from `core/state/world.json` and realm files.

Fleet TOML adds a fourth node inventory but represents hosts/model lanes rather than agent sessions. Hermes profiles/Bots provide a fifth agent identity domain externally.

Task 1 does not select a winner. Task 3 must decide:

- node identity versus conversational agent identity;
- semantic registry persistence owner;
- which state is observed versus configured;
- how Hermes A2A Agent Cards project into it;
- how stale identity expires.

### 4. Arandur exists as policy/projection code but not as a coherent live executive

Arandur objective packets and selection are current and correctly fail closed. The timer writes a fresh `data/ceo/autopilot.state.json`, reports `hold`, and does not promote tasks.

But:

- CEO/Prometheus modules require Aulë `full-cli` and are absent from root composition;
- `CoreAutonomyProfile` expects `core/realm/boot.toml`, which does not exist;
- realm files currently live under `core/knowledge/realm/`;
- `core/state/world.json` is stale and labels Arandur ONLINE/READY with a March heartbeat;
- no `data/arandur/` recommendation ledger exists;
- no objective, plan, or outcome was processed in the current cycle.

Stage 6 should not create another CEO. It should reconcile this existing full-CLI executive path with current root topology, Vairë context, engine receipts, and canonical queue decisions.

### 5. Fleet configuration is not organism topology

`config/fleet.toml` already expresses valuable identity and hardware/model attributes. It distinguishes active/offline and includes restart routes. Direct probes prove three reachable compute/evidence nodes and three configured-active failures.

What is missing is an expiring observation contract joining:

- stable node identity/enrollment;
- general capabilities/tools/data locality;
- current CPU/GPU/RAM/storage/network/thermal/power pressure;
- supported agent transports;
- privacy/trust class;
- heartbeat and validity interval;
- current minimal work/inference proof.

The existing outpost protocol should be extended before creating a parallel node store.

### 6. Queue authority is good; projection semantics need repair

The append-only JSONL is canonical and guarded. Latest-by-ID behavior is implemented. Next-action and Arandur preserve operator/review gates.

The generated active/summary views, however, drop `origin`, `scope`, and dependency metadata and count every pending dependency-gated stage as active. This directly conflicts with the program’s “later stages are not active backlog” rule. Repair the projections; do not replace the queue.

### 7. Receipts are real but fragmented

The historical Workbench receipt proves bounded Hermes execution, tool/test evidence, digest lineage, usage, and restart-safe terminal state. Varda research artifacts, Vairë continuity, Oromë operator sessions, Manwë routes, and RELIC presence each have separate evidence families.

The missing organism behavior is correlation across them. The operator projection currently cannot answer, in one view:

- which operator objective is active;
- which roles/nodes are assigned;
- what A2A handoffs occurred;
- what evidence was used;
- which worker failed or recovered;
- what memory affected the result;
- whether acceptance conditions were met.

## Disposition register

### Reuse now

- `arda-core` task/run/capability authorities;
- `arda-governance` semantic policy;
- engine durable run, scheduler, recovery, and Hermes adapter;
- Oromë operator bridge/provider receipt semantics;
- Vairë canonical memory/continuity;
- Rúmil review-only audit;
- Mandos evidence/reasoning;
- economics/JouleWork;
- Warden scout as an advisory remote evidence node;
- canonical append-only queue.

### Adapt through later stages

- root topology and external-service representation;
- Oromë A2A envelope mapped to standard A2A;
- Hermes bridge lifecycle/subagent receipts;
- Manwë node/resource-aware placement inputs;
- Varda service readiness and run reconciliation;
- Aulë/operator projection convergence;
- Arandur/Prometheus executive composition;
- fleet/outpost node observations;
- contract registry freshness/root admission;
- queue projection semantics;
- RELIC/HUD/Mirromere as read-only consumers.

### Unresolved; inspect before choosing

- Oromë in-memory registry;
- Core persistent Oromë registry/router state;
- Prometheus world roster;
- Fleet node inventory versus Hermes profile/Agent Card identity.

No registry or router should be promoted, deleted, or renamed until Task 3 assigns exact ownership and migration behavior.

### Archive candidate; no deletion authorized

- stale `core/state/world.json` and legacy realm-projection placement, after a live topology/identity successor is accepted and all consumers are migrated.

## Stage gate result

Task 1 acceptance is technically satisfied:

- every relevant component has implementation, test, compile, root, install, runtime, workflow, duplication, and disposition evidence;
- no historical proof was promoted to current running state;
- the report names a successor/alignment direction for every disposition;
- no source, service, registry, or projection was deleted or rewritten.

Stage 0 is not complete. Stage 1 is not ready. Open blocking decisions:

1. trace one current objective end to end (`digital-organism-s0-current-flow-trace`);
2. choose semantic node/agent registry ownership;
3. define the Oromë semantic-envelope to standard A2A boundary;
4. define live Arandur identity/topology sources and retire stale world truth only afterward;
5. reconcile dependency-gated queue projection semantics.

## Verification results

- `cargo check -p arda` — passed.
- `cargo test -p arda-orome --no-default-features` — 50 passed.
- engine `worker_orchestrator`, `orome_smoke`, `run_recovery`, and `hermes_adapter_contract` — 26 passed.
- Aulë `ceo_surface` and `autopilot_surface` with `full-cli` — 2 passed.
- contract registry tests — 8 passed.
- outpost protocol/scout/RELIC focused check — passed.
- Hermes operator bridge — 16 passed.
- direct root/harness/Manwë/fleet/service probes — completed with failures classified above.

## Next task

`digital-organism-s0-current-flow-trace` — trace the actual current operator objective and pending research run through Hermes ingress, canonical queue/next-action, Arandur selection, run graph, Manwë/worker placement, Varda/Vairë evidence, receipts, and HUD/operator projection. The trace should decide which of the matrix ambiguities is an immediate execution blocker and which is merely dormant compatibility debt.
