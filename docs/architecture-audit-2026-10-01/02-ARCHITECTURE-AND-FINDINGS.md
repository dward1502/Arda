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

> 🜏 Soterion: 📜 architecture_audit | owner: HERMES | status: draft | reviewed: 2026-10-01

# Architecture assessment and findings

## Judgment

Arda has a coherent intended architecture and real shared implementation. It is not simply a collection of disconnected demonstrations. The stronger criticism is that implementation ownership, compatibility history, deployment entry points and evidence are harder to understand than the intended design. This makes adding features easier than proving a small set of reliable user outcomes.

The existing doctrine describes a personal agent ecosystem, not just a coding executor. Preserve that scope: information intake, assessment, planning, authorized action, verification and continuity across domains. Hermes is the conversation/worker harness; it is not a subsystem to replace. Workbench is one capability. A growing number of contracts, receipts and workstations cannot substitute for observed useful outcomes.

This is a landscape-level architectural judgment from sampled source. It is not a claim that every path works or every subsystem was deeply reviewed.

## Existing structure worth keeping

- Thin root composition: `src/main.rs`, root `Cargo.toml:62-64`, and `crates/engine/src/lib.rs` put the daemon over a reusable Engine rather than another independent orchestration implementation.
- Shared contracts and bounded edges: arda-core, arda-outpost-protocol and the SDK offer seams between services. The targeted protocol/registry/bridge tests exercised real code and passed; see 05.
- Distinct control and inference endpoints: `crates/engine/src/harness.rs:1-6,45-49` separates the Engine harness from Manwe. This is an intelligible division of responsibilities.
- Memory internals already express separation: `crates/spine/memory/arda-vaire/src/service.rs:10-26` names persistence, retrieval, promotion and policy modules instead of putting every concern behind one file.
- Durable run events: `crates/engine/src/runs/store.rs:14-60` models transitions, evidence, governance, recovery and idempotency explicitly. Whether every restart path honors the contract remains a deeper verification question.
- Retired writer paths refuse rather than silently revive: `scripts/task-pivot.sh:4-6` is an example worth preserving until consumers migrate.
- Explicit energy provenance: `crates/spine/runtime/arda-economics/src/meter.rs:37-66` distinguishes hardware measurements from estimates.

## Observed connection map

    Operator / Hermes / native UI
        -> adapters and Engine harness
        -> objectives, projects, run state and governance
        -> capability-specific execution
           - Workbench: Engine adapter -> Aule explicit workbench executor
           - Research: Engine harness -> Varda ingest/store
           - Inference: Manwe adaptive routing
           - Communications: Orome
        -> receipts / retained state / Vaire continuity
        -> projections / HUD / Mirromere / read-only outposts

This combines observed imports and composition seams, not a recorded end-to-end execution. Research calls into Varda are visible in `crates/engine/src/harness.rs:63-97`. Workbench's dependency is visible in `crates/engine/src/objectives/workbench.rs:5,25-27,84-126`. The package map records declared dependency edges separately from runtime evidence.

## Findings register

### F01 - Confirmed build blocker: Vaire test fixture drift

Evidence: `cargo check --workspace --all-targets --locked --offline -j 2` emitted E0063 at `crates/spine/memory/arda-vaire/tests/context_capsule.rs:14`: missing `excluded_refs` in `OrganismContext` initializer. Root `cargo check -p arda` passes, so this is specifically a broader target validation gap, not evidence that the root daemon cannot compile.

Impact: a clean workspace-wide test compilation baseline is unavailable. Later errors may be masked by this first failure. Priority: high. Recommendation: repair the fixture to reflect intended excluded-reference semantics, then rerun all targets and focused Vaire tests. Do not treat this report as authorization to make that repair.

### F02 - Confirmed layering issue: live execution under observability

Evidence: Engine imports `arda_aule::prometheus::autopilot` explicit execution types and adapter in `objectives/workbench.rs`. Aule's `src/prometheus/autopilot/mod.rs:1-33,99-102` gates a broad planning/execution surface under `full-cli`; `Cargo.toml:9-17` couples this feature to multiple subsystems and telemetry.

Impact: observational infrastructure also supplies live domain execution. Directory ownership and feature names conceal the real dependency. AGENTS principles: separation of concerns, cohesion, encapsulation and dependence on abstractions. Priority: high architectural risk, not a demonstrated runtime incident.

Recommendation: first identify the minimal explicit execution contract and its authority/persistence dependencies. Establish a narrow boundary using existing owners, then separate the implementation from legacy CLI/autopilot machinery. Do not simply rename or delete Aule; it has live consumers.

### F03 - Confirmed validation boundary: root workspace does not cover native apps

Evidence: HUD and Mirromere native manifests each declare `[workspace]` at their beginning; root Cargo members include launcher but not these apps. Their JavaScript manifests define separate tests/builds. Both consume shared Rust or UI components.

Impact: a green root Cargo check cannot establish either native app's build health. Independent workspaces may be intentional; the defect risk is claiming aggregate validation from only one. Priority: high verification gap.

Recommendation: one documented build/test matrix covering root default/explicit targets, HUD native/frontend, Mirromere native/frontend and shared UI. Do not force a workspace merger without weighing desktop dependency costs.

### F04 - Confirmed repository CI gap; remote policy unknown

Evidence: `.github/workflows/` contains documentation-health and release-sign workflows. The former checks plans/parser tests; the latter verifies/signs published release assets. Neither is a general source compilation/test workflow.

Impact: repository-local CI definitions do not provide continuous broad source validation. External CI or branch protection may exist; not queried. Priority: high. Recommendation: add bounded required source checks after establishing a reproducible baseline; keep release provenance and document checks as separate guarantees.

### F05 - Confirmed inventory concern: product source and runtime/private state coexist

Evidence: path inventory contains large tracked `core/`, `data/`, `audit/` and `.hermes/evidence/` trees. A contract registry is loaded from `core/state/contract_registry.json` (`registry.rs:4,65-66`), while Engine run persistence writes under `data/runs` (`runs/store.rs:92-96`). These directories cannot be classified uniformly as disposable outputs.

Impact: installation defaults, versioned contracts, operator state, evidence and generated observations need different lifecycle/security rules. This creates portability and accidental-publication risk; no secret exposure was established and no personal records were inspected. Priority: high follow-up.

Recommendation: a path-level ownership/lifecycle table: shipped immutable input, private mutable store, derived cache, retained evidence, test fixture, archived history. Decide backup/export/redaction and retention before moving or deleting anything.

### F06 - Confirmed documentation drift obscures the real map

Evidence: `docs/CODEMAP.md:70,85` says 18 workspace packages; metadata enumerates 19. Its device section does not describe the present in-repo Mirromere app/library arrangement. Active-plan checking found four broken links to the retired/moved objective-runtime-cutover document; see retained check output in 05.

Impact: an operator or agent can start from an obsolete topology or unusable evidence pointer. Priority: medium. Recommendation: reconcile existing codemap and plan links after reviewing this assessment; do not create a competing architecture canon.

### F07 - Confirmed concentration; responsibility risk is partly inferred

Evidence: task_queue.rs 8,155 lines; workbench_executor.rs 7,082; runner.rs 4,546; Manwe adaptive HTTP transport 4,172; HUD App.tsx 2,788. These totals include tests and comments. HUD imports cover numerous independent domains; Aule files contain legacy and current concerns.

Impact: high navigation/review cost and likely change coupling. Size itself is not a defect, and dedicated large test files should not be treated like production god objects. Priority: medium, with F02 first.

Recommendation: extract along existing domain/persistence/transport boundaries only after caller mapping and characterization tests. Moving arbitrary blocks to meet a line budget does not satisfy AGENTS.

### F08 - Confirmed development-oriented registry; deployment interpretation unresolved

Evidence: `services.toml:22-28` starts launcher via cargo tauri dev and probes a dev-server port; lines 50-56 start HUD via pnpm dev. Adapter entries also declare installed/health/eligible states. Manwe is separately started from target/release.

Impact: this file mixes development process composition with capability declarations. It cannot by itself prove a home-installable packaged runtime, native readiness or live adapter eligibility. Not a claim that installed deployment is broken. Priority: medium.

Recommendation: trace which installed launcher/daemon consumes each field, distinguish declared capabilities from observations and document development versus installed composition.

### F09 - Source-backed product coherence question: Mirromere boot surface

Evidence: `apps/arda-mirromere/src-tauri/src/lib.rs:90-148` ensures a Hermes dashboard process and navigates the main webview to it. `apps/arda-mirromere/src/App.tsx` is largely a display-selection/connection shell. This is more specific than a standalone embodied agent UI.

Impact: the present bootstrap needs reconciliation with the intended second-display agent experience. It may be a deliberate stage in integration; native behavior, avatar/voice continuity and display ownership were not observed. Priority: product decision, not a verified runtime defect.

Recommendation: judge the actual native session against the intended experience. Preserve Hermes integration, but do not declare an embodied experience proven merely because a conversation dashboard opens. Also, the frontend's connecting state alone is not a confirmed bug: native navigation replaces the page on success.

## AGENTS-based synthesis

| Principle | Current evidence | Assessment |
|---|---|---|
| Separation of concerns / SRP | Root versus Engine; Vaire submodules; execution inside Aule | Real progress, inconsistent major boundary |
| Encapsulation | Shared protocols versus imports from deep autopilot modules | Some stable seams, some exposed implementation structure |
| Cohesion / loose coupling | Shared Engine, broad Aule feature bundle, broad HUD composition | Connected, but costly to reason about |
| DRY | Canonical contracts plus retained legacy machinery and many projections | Multiple representations exist; semantic duplication must be traced, not assumed |
| KISS / YAGNI | Many surfaces and contracts, unproven aggregate user outcomes | Pause breadth until representative outcomes establish value |
| Depend on abstractions | Protocol/SDK seams; concrete Aule adapter as Engine default | Narrow contracts are available patterns to reuse |
| Composition over inheritance | Service/library composition dominates inspected Rust | No inheritance problem established |
| Open/closed with discipline | Capability/adapter declarations | Verify real second consumers before adding extension frameworks |

The highest-value change is not a rewrite. It is making the existing connected paths easier to prove, govern, install and explain, then removing genuinely superseded paths with evidence.
