# Arda Architecture Review — External Review Packet

**Date:** 2026-08-13  
**Repository snapshot:** `f03a6837` (`fix(autopilot): recover pre-dispatch claims immediately`)  
**Audience:** independent architecture reviewers, including Grok and Claude  
**Scope:** canonical Arda repository; source, composition, interfaces, persistence, governance, product surfaces, tests, and CI  
**Excluded:** secret values, private conversational content, speculative components not represented in source

> Reviewer instruction: Treat this document as a map and a set of falsifiable claims, not as proof by itself. Challenge the priorities, inspect the cited source, and distinguish library existence from root composition, tests from deployed behavior, and plans from completed runtime proof.

## 1. Executive verdict

Arda is a serious local-first personal-agent platform, not a thin chat wrapper. Its strongest architectural property is that it models autonomy as governed execution: identity, policy, approvals, budgets, durable events, receipts, cancellation, replay, memory, observability, and human intervention are explicit domain concepts.

The main risk is **convergence**, not lack of primitives. Arda has broad, well-tested subsystem implementations, but the integration surface is large enough that different endpoints and stores can enforce different trust, durability, and configuration rules. The next architectural phase should therefore prioritize:

1. one authenticated and digest-bound mutation envelope;
2. one authoritative durable-write discipline;
3. executable root-composition and cross-process acceptance gates;
4. full pull-request CI across Rust, HUD, schemas, security, and architecture constraints;
5. smaller bounded modules around the root harness and HUD shell.

The architecture is credible for an internal `0.9` baseline. It should not claim a public `1.0` until whole-product gates demonstrate that the same governed path is used from operator ingress through action, receipt, projection, restart, and deletion.

## 2. Method and evidence standard

This review used:

- root composition in `src/main.rs` and `services.toml`;
- the engine and harness under `crates/engine/`;
- governance, runtime, memory, interface, observability, contract, and executor crates under `crates/spine/`;
- the HUD and launcher under `apps/`;
- repository doctrine, active baseline, prior audits, and live proof artifacts;
- workspace metadata and source-size inventory;
- direct verification on 2026-08-13.

Evidence labels:

- **Implemented:** source exists.
- **Tested:** an automated test covers the claim.
- **Root-composed:** the canonical binary constructs or exposes it.
- **Live-proven:** a dated artifact records real runtime behavior.
- **Planned:** a plan or doctrine describes it, but runtime proof is not implied.

## 3. System snapshot

### 3.1 Scale

At the audited snapshot:

- 18 Rust workspace packages;
- approximately 182,153 nonblank Rust source lines in 586 `.rs` files;
- approximately 49,681 nonblank TypeScript/TSX lines in 409 files;
- approximately 11,732 nonblank Python lines in 62 files;
- 131 HUD test files and 518 passing HUD tests in the direct verification run.

The source inventory excludes generated/build trees and dependency copies (`target`, `node_modules`, `vendor`, `dist`, `.git`, and `Arda-worktrees`). This is a platform-sized codebase. Architectural controls need to be executable because manual consistency review no longer scales.

### 3.2 Logical layers

| Layer | Primary components | Responsibility |
|---|---|---|
| Operator ingress | Hermes bridge, `arda-orome`, engine harness | Capture authenticated operator intent and expose local APIs |
| Product surfaces | `apps/arda-hud`, `apps/arda-launcher` | Human observation, intervention, launch, and workstation control |
| Composition/control plane | root `arda`, `arda-engine`, `services.toml`, supervisor | Construct canonical runtime, own local harness, supervise subprocesses |
| Governance/identity | `arda-core`, `arda-governance`, `arda-contract-registry` | Identity, policy, approval, contract, receipt, and authority concepts |
| Execution/routing | `manwe`, `arda-varda`, `arda-mandos`, outposts | Provider routing, worker execution, councils, delegation, and adapters |
| Memory/learning | `arda-vaire`, learning-loop code, `arda-rumil` | Governed memory, learning evidence, project/audit coordination |
| Observability | `arda-aule`, harness projections, telemetry stores | Events, metrics, traces, receipts, operator-readable projections |
| Durable state | JSONL journals, checkpoints, receipts, local state bundles | Replay, resume, audit, and local ownership |

### 3.3 Root-composed topology

The canonical `arda` binary:

1. loads `services.toml` through `arda_engine::registry::Registry`;
2. resolves enabled services and UI policy;
3. creates a `Supervisor` and shared shutdown signal;
4. builds `HarnessState`, including service statuses, Manwe URL, operator identity, presence, and workbench root;
5. binds the local engine harness;
6. supervises configured child processes and coordinates shutdown.

Evidence: `src/main.rs`, especially `Registry::load`, `Supervisor::new`, `HarnessState`, and `harness::serve`.

`services.toml` is the declarative process registry. At this snapshot the launcher and Manwe are required supervised services; the HUD is an optional supervised UI service. Many workspace crates are in-process libraries, not independently supervised services. Reviewers should not infer process isolation from crate boundaries.

## 4. Principal runtime flows

### 4.1 Operator message capture

```text
phone / Hermes message
  -> authorized Hermes lifecycle hook
  -> loopback Orome bridge
  -> append-only operator-message journal
  -> engine projection / resume / HUD consumers
  -> acknowledgement and evidence
```

The 2026-08-09 P2.4 evidence records a real phone-originated message crossing the production Hermes hook into Arda's loopback bridge with operator identity preserved and one durable journal append. This is **live-proven**, not merely fixture-tested.

Evidence:

- `crates/spine/interface/arda-orome/src/operator_bridge.rs`
- `crates/engine/src/harness/operator_messages.rs`
- `docs/evidence/2026-08-09-p2.4-live-phone-acceptance.md`
- `docs/plans/2026-08-08-arda-1.0-personal-agent-ecosystem-plan.md` (P2.4)

### 4.2 Governed run lifecycle

```text
operator/project intent
  -> immutable run graph
  -> governance evaluation
  -> approval or policy-safe transition
  -> worker scheduling
  -> append-only run events + checkpoints
  -> receipts/results
  -> projection, resume, cancel, or retry
```

`arda-engine` implements graph validation, transition enforcement, idempotency keys, run journals, worker leases/heartbeats, retries, cancellation, checkpoints, receipts, and result artifacts. The run store rejects sequence gaps and unsupported event versions, syncs journal appends, and uses atomic writes for snapshots/results/receipts.

Evidence:

- `crates/engine/src/runs/{graph,governance,orchestrator,store}.rs`
- `crates/engine/src/harness/runs.rs`
- `crates/engine/tests/`

### 4.3 Personal operations

```text
capture
  -> durable inbox item
  -> classify/schedule/complete
  -> reminder or brief
  -> explicit operator acknowledgement
  -> export/delete with receipts
```

The engine exposes personal capture, inbox, classification, scheduling, completion, reminders, briefs, resume, export, and deletion endpoints. Proactive cycles are bounded and store cycle state. The architecture intentionally keeps optional revenue, Web3, councils, and local inference from blocking personal/home utility.

Evidence:

- `crates/engine/src/harness/personal_ops.rs`
- `crates/engine/src/personal_ops/proactive_cycle.rs`
- `docs/releases/0.9/BASELINE.md`

### 4.4 Provider routing and execution

Manwe is the model/provider routing authority and is root-supervised. The engine proxies model discovery through the harness and uses provider/executor abstractions for work. Varda provides execution surfaces; Mandos provides governed council/delegation concepts; Aule records observability; outposts extend execution through explicit protocol types.

Evidence:

- `crates/spine/runtime/manwe/`
- `crates/spine/executors/arda-varda/`
- `crates/spine/runtime/arda-mandos/`
- `crates/spine/observability/arda-aule/`
- `outposts/arda-outpost-protocol/`

### 4.5 HUD and launcher

The HUD is an agent-native Tauri/React/Three.js workstation: a display world plus controlled workstation surfaces, not a conventional web dashboard. The Tauri shell owns native integration and command surfaces; React owns the experience and live projections. The launcher is a separate Tauri application.

Evidence:

- `apps/arda-hud/src/App.tsx`
- `apps/arda-hud/src-tauri/src/lib.rs`
- `apps/arda-hud/BREAKDOWN.md`
- `apps/arda-launcher/`

## 5. Trust boundaries and authority model

### 5.1 Existing strengths

1. **Loopback by default.** Engine and bridge surfaces reject non-loopback peers in their intended configurations.
2. **Stable operator identity.** The root validates a nonblank operator identity and passes it into the harness.
3. **Fail-closed governance concepts.** Policy decisions, approvals, contracts, budgets, and receipts are represented as typed structures rather than informal prompts.
4. **Append-only evidence.** Multiple critical flows use JSONL journals with sequence/idempotency data and durable sync.
5. **Explicit deletion/export.** Personal data ownership includes export and deletion semantics rather than treating storage as irreversible.
6. **Human intervention is architectural.** Approval, pause, cancel, retry, acknowledgement, and operator projection are first-class.

### 5.2 Critical boundary inconsistency

Mutation authorization is not yet uniform.

Some harness domains validate the configured operator identity from headers. Other mutation paths rely on loopback plus a caller-supplied approval structure whose validation checks semantic fields such as `PolicySafe`, IDs, scope, and references. The inspected approval envelope is not cryptographically signed, expiry-bound, or visibly consumed as a single-use capability.

Loopback is a useful network boundary, but it does not authenticate one local process to another. A compromised browser extension, desktop app, or local process can be inside that boundary.

**Recommendation:** Create one daemon-issued mutation capability used by every write endpoint:

```text
capability_id
operator_id
subject/run/task id
action + canonical payload digest
policy/approval reference
issued_at + expires_at
nonce / single-use state
issuer signature or keyed MAC
```

Require validation and append a consumption receipt before mutation. Keep low-risk reads loopback-only if desired; do not let domain-specific handlers invent different mutation trust rules.

## 6. Persistence and restart semantics

### 6.1 Strong properties

- run events carry schema versions, monotonic sequences, and idempotency keys;
- journal recovery validates corruption and sequence gaps;
- appends call `sync_all`;
- derived snapshots, results, receipts, council artifacts, and composition artifacts use atomic replacement;
- Orome's bridge uses file locking around append;
- orchestrator state includes leases, heartbeats, retries, and recovery behavior;
- governed memory ties writes to policy/budget/audit structures.

### 6.2 Remaining consistency risk

The repository contains several persistence implementations with different locking, transaction, and recovery behavior. For example, Orome visibly applies an inter-process file lock, while the run store's append path computes its next sequence from recovered length before append. The latter may be safe under its current single-writer/in-process orchestration assumptions, but that assumption is not encoded as a cross-process invariant at the storage boundary.

**Recommendation:** Establish a common durable-store substrate or an explicit single-writer daemon contract with:

- inter-process locking or transactional storage;
- append sequence allocation under the lock;
- checksummed frames and schema migration;
- temp-write, fsync, rename, and parent-directory fsync rules;
- crash/fault injection tests;
- compaction with preserved receipt roots;
- documented ownership for every state file.

Do not migrate to a database merely for fashion. SQLite in WAL mode is a reasonable option for concurrent indexes and transactions; hardened JSONL remains reasonable for audit logs if the single-writer and locking contracts are explicit.

## 7. Architecture strengths worth preserving

### 7.1 Governed autonomy rather than prompt-level safety

Arda puts policy and receipt boundaries in code. This is materially stronger than relying on system prompts for authorization.

### 7.2 Local-first, owner-controlled data

The architecture favors loopback services, filesystem evidence, export, deletion, and inspectable artifacts. This aligns with a personal/home-agent system where privacy and recoverability matter more than cloud-native scale.

### 7.3 Durable orchestration

Run graphs, event journals, checkpoints, leases, idempotency, cancellation, retries, and receipts form a credible durable execution substrate.

### 7.4 Operator-native product model

The HUD is designed around observing and intervening in agent behavior rather than mirroring backend tables. This is differentiated and consistent with Arda's purpose.

### 7.5 Evidence-aware engineering culture

The repository contains baseline documents, dated proof artifacts, breakdowns, plan reconciliation, and extensive tests. P2.4 is particularly valuable because it records a real cross-system path instead of calling fixture behavior live proof.

### 7.6 Useful crate boundaries

Governance, routing, memory, execution, observability, contracts, and interface concerns have named packages. The boundaries provide a foundation for dependency rules even though root integration still needs stronger executable checks.

## 8. Principal risks and recommended improvements

### P0 — before broader autonomy or public 1.0

#### P0.1 Unify mutation authentication and approval consumption

**Risk:** Local endpoints apply different identity/approval rules; loopback is mistaken for authentication.

**Action:** Implement the digest-bound, expiring, single-use capability described in §5.2. Apply it through shared Axum middleware and domain adapters. Test replay, payload substitution, wrong operator, expiry, and concurrent consumption.

**Done when:** every mutating route is enumerated and rejects all five negative cases; successful mutation and capability consumption share one durable receipt chain.

#### P0.2 Make root composition executable

**Risk:** A crate can be implemented and tested without being reachable from the canonical runtime.

**Action:** Introduce a machine-readable architecture manifest mapping each capability to:

```text
authority owner -> package -> root constructor -> process/thread
-> endpoint/event -> durable store -> projection/consumer -> acceptance test
```

Validate it in CI against workspace metadata, `services.toml`, route registration, and targeted acceptance tests.

**Done when:** release claims can be generated from passing composition checks rather than prose inspection.

#### P0.3 Standardize durable-write authority

**Risk:** Multiple JSONL/state implementations have inconsistent inter-process and crash semantics.

**Action:** Adopt one common journal/snapshot primitive or enforce one daemon as the only writer. Add process-concurrency and kill-at-every-write-boundary tests.

**Done when:** restart/replay tests prove no duplicate action, lost receipt, sequence collision, or silent truncation under concurrent and abrupt-exit scenarios.

#### P0.4 Add full pull-request CI

**Risk:** The repository's visible GitHub workflows cover documentation health and signed releases, but not the complete Rust/HUD quality suite on pull requests.

**Action:** Add required jobs for:

- `cargo fmt --check`;
- `cargo check --workspace --all-targets`;
- `cargo clippy --workspace --all-targets` with an agreed warning policy;
- workspace tests plus selected fault/restart tests;
- HUD typecheck, Vitest, build, and lint;
- schema/link/architecture-manifest validation;
- dependency/license/vulnerability checks;
- a root-composition smoke test.

**Done when:** protected-branch status proves these gates for every change.

### P1 — next maintainability and reliability cycle

#### P1.1 Split oversized composition modules by bounded context

Several files are large enough to become merge and comprehension bottlenecks, including the root harness router and major HUD/Tauri shells. Large files are not intrinsically wrong, but 100–200 KB modules carrying many domains weaken ownership boundaries.

**Action:** Keep the root router declarative. Move each domain to a typed route module with shared auth, error, and persistence middleware. Continue splitting HUD orchestration into scene, runtime authority, projection, and native-command boundaries without turning the UI into generic cards.

#### P1.2 Define and enforce dependency direction

**Action:** Encode allowed package dependencies. Suggested direction:

```text
protocol/domain types
  <- governance/contracts/memory abstractions
  <- routing/execution/observability implementations
  <- engine composition
  <- root binary and product shells
```

Reject cycles, UI-to-storage shortcuts, and provider-specific types crossing domain boundaries.

#### P1.3 Consolidate configuration authority

**Risk:** URLs, ports, state roots, compatibility aliases, and feature switches can be sourced from CLI, environment, TOML, or defaults.

**Action:** Define a typed effective configuration with source provenance. Expose a redacted `/v1/config/effective` projection and validate that consumers use it. Preserve existing bind assumptions until consumer evidence supports changes.

#### P1.4 Add end-to-end latency and reliability SLOs

Measure:

- operator message -> durable append -> acknowledgement;
- approval -> dispatch;
- cancellation request -> worker stop -> receipt;
- restart -> resumed projection;
- memory write/delete -> proof receipt;
- HUD projection freshness and frame-time budgets.

Use Aule to make these first-class and alert on missing receipts, not just process failure.

#### P1.5 Strengthen privacy and at-rest protection

Local-first reduces exposure but does not protect stolen disks, backups, or other local users.

**Action:** Classify state by sensitivity, support encrypted-at-rest stores for personal data and memories, separate secrets from evidence, define retention, and test export/deletion across derived projections and backups. Preserve receipt proofs without retaining deleted personal payloads.

#### P1.6 Add architectural decision records and ownership maps

Use short ADRs for irreversible choices: mutation capability format, durable store substrate, process boundaries, outpost sandbox, projection/event schemas, and encryption. Keep `BREAKDOWN.md` files generated or verifiably synchronized with source.

### P2 — scale and ecosystem hardening

#### P2.1 Sandbox outposts and adapters

Move from protocol conformance alone to declared capabilities, resource limits, network/filesystem policy, signatures, provenance, and revocation. Treat project adapters as untrusted until admitted.

#### P2.2 Formalize event/schema evolution

Give every durable event and public projection:

- a versioning rule;
- backward/forward compatibility tests;
- migration tooling;
- golden fixtures;
- retention and compaction semantics.

#### P2.3 Separate audit evidence from mutable operational state

Keep append-only proofs in a stable evidence root, runtime queues/state in mutable stores, and generated summaries reproducible. Avoid allowing active runtime churn to obscure review diffs.

#### P2.4 Performance budgets for the HUD

The verified production build succeeds but emits large Three.js/core and main bundles plus substantial scene assets. Define budgets for startup, interaction latency, GPU/CPU use, memory, bundle chunks, and sustained FPS. Optimize through measured scene/resource lifecycles rather than flattening the agent-native design.

## 9. Test and verification posture

### 9.1 Direct verification on 2026-08-13

| Command | Result |
|---|---|
| `cargo check -p arda-engine --lib -vv` | pass |
| `cargo check --workspace --all-targets` | pass; warnings originate in vendored `glib-0.18.5` |
| `pnpm test` in `apps/arda-hud` | 131 files, 518 tests passed |
| `pnpm build` in `apps/arda-hud` | pass; Vite reported one mixed static/dynamic import warning and large scene/runtime chunks |

A background-run wrapper initially failed before Cargo execution because the user systemd scope bus was unavailable. The same Cargo checks passed in foreground. This is an execution-environment issue, not a source failure.

### 9.2 Current CI gap

The repository has visible workflows for documentation health and release signing. Those are useful but do not constitute continuous integration for the main code and product surfaces. The direct local gates above should become required CI, supplemented by security and whole-product acceptance tests.

### 9.3 Test improvements with the highest architectural value

1. property tests for governance transitions and approval capability replay;
2. multi-process writers against every durable store;
3. fault injection at append/fsync/rename/receipt boundaries;
4. root-process restart during running, cancelling, retrying, and approval states;
5. contract tests for Hermes bridge, engine, HUD, Manwe, and outposts;
6. golden schema migration tests;
7. one phone-to-action-to-receipt-to-HUD acceptance path;
8. one deletion path proving payload removal and retained non-sensitive receipt integrity.

## 10. Recommended 90-day architecture sequence

### Phase A — authority convergence

- inventory every mutating route;
- implement shared mutation capabilities and middleware;
- bind approval to action digest and consume once;
- produce authorization coverage report.

### Phase B — durability convergence

- inventory stores and writer ownership;
- harden common append/snapshot primitives;
- add cross-process and fault-injection suites;
- document recovery and migration contracts.

### Phase C — executable architecture and CI

- add architecture manifest and dependency rules;
- add required Rust/HUD/security CI;
- convert release claims to generated evidence;
- add root-composition smoke and restart tests.

### Phase D — product proof

- run one canonical personal-agent journey from phone capture through governed action, receipt, resume, and HUD intervention;
- run export and deletion proof;
- capture performance and reliability SLOs;
- reconcile the `0.9` baseline and keep public `1.0` gates intact.

## 11. Questions for Grok and Claude

Please answer independently and cite repository paths or this document's evidence sections.

1. Is the proposed priority—authority convergence before more capabilities—correct? What would you reorder?
2. Should Arda retain hardened JSONL plus a single-writer daemon, adopt SQLite/WAL for operational state, or use a hybrid? Give migration boundaries, not a generic database preference.
3. What is the smallest robust capability format for authenticating local mutations and binding approvals to exact actions?
4. Which crate/process boundaries are wrong or overly coupled? Propose a dependency graph and identify costly migrations.
5. Which components are implemented but not convincingly root-composed or live-proven?
6. Where can duplicate execution, stale approval, lost cancellation, or split-brain state occur after crash/restart?
7. What five fault-injection or property tests would expose the most serious failures?
8. Does the HUD/Tauri boundary preserve a clean authority model, or can the UI bypass governed runtime paths?
9. Which configuration values need one authoritative source, and how should effective config be projected safely?
10. How should owner identity and local trust work across Hermes, engine, HUD, launcher, and outposts without creating a cloud dependency?
11. What privacy risks remain in journals, derived projections, backups, logs, and deletion receipts?
12. What evidence is still required for a truthful whole-product `0.9` and eventual public `1.0`?
13. Which recommendations in this review are over-engineered for a single-owner local-first system?
14. What architectural advantage should Arda protect even if it slows feature delivery?

## 12. Requested reviewer response format

```markdown
# Independent Arda Architecture Review

## Bottom-line verdict
## Claims you agree with
## Claims you dispute
## Top 5 risks (severity, evidence, failure scenario)
## Recommended target architecture
## 30/60/90-day sequence
## Tests that would change your confidence
## Questions or missing evidence
```

## 13. Evidence index

### Canonical composition

- `Cargo.toml`
- `src/main.rs`
- `services.toml`
- `crates/engine/src/registry.rs`
- `crates/engine/src/supervisor.rs`
- `crates/engine/src/harness.rs`

### Governance and durable execution

- `crates/spine/governance/arda-core/`
- `crates/spine/governance/arda-governance/`
- `crates/spine/contract/arda-contract-registry/`
- `crates/engine/src/runs/`
- `crates/engine/src/harness/runs.rs`
- `crates/engine/tests/`

### Interface, identity, and bridge

- `crates/spine/interface/arda-orome/src/types.rs`
- `crates/spine/interface/arda-orome/src/governance.rs`
- `crates/spine/interface/arda-orome/src/operator_bridge.rs`
- `crates/engine/src/harness/operator_messages.rs`
- `docs/evidence/2026-08-09-p2.4-live-phone-acceptance.md`

### Memory, learning, observability, and routing

- `crates/spine/memory/arda-vaire/`
- `crates/spine/runtime/arda-rumil/`
- `crates/spine/observability/arda-aule/`
- `crates/spine/runtime/manwe/`
- `crates/spine/executors/arda-varda/`
- `crates/spine/runtime/arda-mandos/`

### Product surfaces

- `apps/arda-hud/`
- `apps/arda-launcher/`
- `apps/arda-hud/BREAKDOWN.md`

### Doctrine, baseline, and status

- `README.md`
- `docs/architecture/ARDA_1_0_PERSONAL_AGENT_ECOSYSTEM.md`
- `docs/releases/0.9/BASELINE.md`
- `docs/archive/2026-08-12-arda-0.9-baseline-and-improvement-plan.md`
- `docs/audits/2026-08-08-arda-1.0-vision-to-live-gap-report.md`
- `ARDA_SYSTEM_STATUS_REPORT.md`

## 14. Final assessment

Arda's architectural thesis is sound: personal agents should be durable, governed, observable, interruptible, and owner-controlled. The codebase already contains unusually strong primitives for that thesis. The highest-value improvement is not another subsystem. It is forcing every existing subsystem through the same authenticated authority, durable evidence, root composition, and whole-product proof path.
