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

# Large files and retirement candidates

Line counts include comments, blank lines and embedded tests; size is a review priority, not a defect or deletion warrant. `first cfg(test)` is a navigation hint only: tests can be interleaved and the remainder is not necessarily all test code.

| File | Lines | First cfg(test) | Review emphasis |
|---|---:|---:|---|
| `crates/spine/observability/arda-aule/src/prometheus/autopilot/task_queue.rs` | 8155 | 1039 | responsibility, caller boundary, persistence and side effects |
| `crates/spine/observability/arda-aule/src/prometheus/autopilot/workbench_executor.rs` | 7082 | 1821 | responsibility, caller boundary, persistence and side effects |
| `crates/spine/observability/arda-aule/src/prometheus/autopilot/runner.rs` | 4546 | 3143 | responsibility, caller boundary, persistence and side effects |
| `crates/spine/runtime/manwe/src/adaptive/transport/http.rs` | 4172 | 2988 | responsibility, caller boundary, persistence and side effects |
| `crates/spine/executors/arda-varda/src/ingest.rs` | 3815 | 1691 | responsibility, caller boundary, persistence and side effects |
| `apps/arda-hud/src-tauri/src/lib.rs` | 3252 | 1544 | responsibility, caller boundary, persistence and side effects |
| `crates/spine/runtime/manwe/src/adaptive/service/full/route_policy_tests.rs` | 2948 | - | test organization / fixture cohesion |
| `apps/arda-hud/src/App.tsx` | 2788 | - | responsibility, caller boundary, persistence and side effects |
| `crates/spine/runtime/manwe/src/adaptive/service/full/tests.rs` | 2731 | - | test organization / fixture cohesion |
| `crates/engine/src/harness/runs.rs` | 2555 | 57 | responsibility, caller boundary, persistence and side effects |
| `crates/engine/tests/harness_runs.rs` | 2539 | - | test organization / fixture cohesion |
| `crates/engine/src/objectives/store.rs` | 2513 | 265 | responsibility, caller boundary, persistence and side effects |
| `crates/spine/runtime/arda-mandos/src/reasoning.rs` | 2454 | 1587 | responsibility, caller boundary, persistence and side effects |
| `crates/spine/interface/arda-orome/src/service.rs` | 2233 | 112 | responsibility, caller boundary, persistence and side effects |
| `crates/spine/observability/arda-aule/src/prometheus/core_link.rs` | 2182 | 736 | responsibility, caller boundary, persistence and side effects |
| `crates/spine/runtime/manwe/src/adaptive/service/full/route_selection.rs` | 2170 | 1109 | responsibility, caller boundary, persistence and side effects |
| `crates/engine/src/adapters/hermes.rs` | 2152 | 392 | responsibility, caller boundary, persistence and side effects |
| `crates/spine/observability/arda-aule/src/prometheus/autopilot/knowledge_triage.rs` | 2148 | 1621 | responsibility, caller boundary, persistence and side effects |
| `crates/engine/src/harness/operator_messages.rs` | 2090 | 27 | responsibility, caller boundary, persistence and side effects |
| `crates/engine/tests/harness_operator_messages.rs` | 2042 | - | test organization / fixture cohesion |
| `crates/spine/runtime/manwe/src/adaptive/service/full/route_policy.rs` | 1927 | 10 | responsibility, caller boundary, persistence and side effects |
| `crates/engine/src/harness/research.rs` | 1859 | 37 | responsibility, caller boundary, persistence and side effects |
| `crates/engine/tests/objective_store.rs` | 1800 | - | test organization / fixture cohesion |
| `crates/engine/tests/objective_runtime.rs` | 1703 | - | test organization / fixture cohesion |
| `crates/spine/runtime/manwe/src/adaptive/service/full/proxy.rs` | 1696 | 1679 | responsibility, caller boundary, persistence and side effects |
| `crates/spine/governance/arda-core/src/loop_engine.rs` | 1579 | 846 | responsibility, caller boundary, persistence and side effects |
| `crates/engine/src/harness/operator_messages/unfinished_tests.rs` | 1535 | - | test organization / fixture cohesion |
| `apps/arda-hud/src/scene/boardroom/BoardroomViewport.tsx` | 1489 | - | responsibility, caller boundary, persistence and side effects |
| `crates/spine/runtime/arda-mandos/src/service.rs` | 1436 | 832 | responsibility, caller boundary, persistence and side effects |
| `crates/spine/observability/arda-aule/src/cli/main.rs` | 1432 | 1290 | responsibility, caller boundary, persistence and side effects |
| `crates/spine/runtime/manwe/src/adaptive/service/full/bootstrap_overlay.rs` | 1386 | 935 | responsibility, caller boundary, persistence and side effects |
| `crates/spine/governance/arda-governance/src/bacon_lite.rs` | 1296 | 1006 | responsibility, caller boundary, persistence and side effects |
| `crates/engine/src/harness.rs` | 1287 | 940 | responsibility, caller boundary, persistence and side effects |
| `apps/arda-hud/src/lib/ardaSource.ts` | 1271 | - | responsibility, caller boundary, persistence and side effects |
| `crates/engine/src/objectives/workbench.rs` | 1261 | 845 | responsibility, caller boundary, persistence and side effects |
| `apps/arda-hud/src/lib/boardroomSlotSettings.ts` | 1256 | - | responsibility, caller boundary, persistence and side effects |
| `apps/arda-hud/src-tauri/src/commands/monitor_surface/browser_capture.rs` | 1254 | - | responsibility, caller boundary, persistence and side effects |
| `crates/spine/memory/arda-vaire/src/service.rs` | 1250 | 482 | responsibility, caller boundary, persistence and side effects |
| `crates/engine/src/harness/personal_ops.rs` | 1231 | - | responsibility, caller boundary, persistence and side effects |
| `crates/engine/tests/hermes_adapter_contract.rs` | 1217 | - | test organization / fixture cohesion |
| `scripts/arda_beta_ops.py` | 1200 | - | responsibility, caller boundary, persistence and side effects |
| `crates/spine/executors/arda-varda/src/transport/http.rs` | 1165 | 604 | responsibility, caller boundary, persistence and side effects |
| `apps/arda-hud/src-tauri/src/commands/workbench.rs` | 1148 | 880 | responsibility, caller boundary, persistence and side effects |
| `crates/spine/interface/arda-orome/src/service/council.rs` | 1146 | - | responsibility, caller boundary, persistence and side effects |
| `apps/arda-hud/src/lib/systemActionBus.ts` | 1123 | - | responsibility, caller boundary, persistence and side effects |
| `crates/spine/interface/arda-mirromere/src/lib.rs` | 1063 | 147 | responsibility, caller boundary, persistence and side effects |
| `crates/engine/src/runs/store.rs` | 1051 | - | responsibility, caller boundary, persistence and side effects |
| `crates/engine/tests/harness_personal_ops.rs` | 1039 | - | test organization / fixture cohesion |
| `crates/spine/runtime/manwe/src/adaptive/service/full_service.rs` | 1039 | 101 | responsibility, caller boundary, persistence and side effects |
| `crates/spine/runtime/manwe/src/adaptive/service/full/state_mutation.rs` | 1006 | 937 | responsibility, caller boundary, persistence and side effects |
| `crates/spine/runtime/manwe/src/adaptive/service/full/route_scoring.rs` | 999 | - | responsibility, caller boundary, persistence and side effects |
| `crates/spine/runtime/manwe/src/adaptive/service/full/routing.rs` | 968 | 49 | responsibility, caller boundary, persistence and side effects |
| `crates/spine/observability/arda-aule/src/prometheus/core_link/arda.rs` | 951 | 860 | responsibility, caller boundary, persistence and side effects |
| `crates/spine/governance/arda-governance/src/resonance.rs` | 946 | 537 | responsibility, caller boundary, persistence and side effects |
| `crates/engine/src/harness/research_operator.rs` | 933 | 584 | responsibility, caller boundary, persistence and side effects |
| `crates/spine/governance/arda-core/src/run_graph.rs` | 927 | - | responsibility, caller boundary, persistence and side effects |
| `crates/spine/observability/arda-aule/src/prometheus/autopilot/schedule.rs` | 919 | 593 | responsibility, caller boundary, persistence and side effects |
| `crates/spine/memory/arda-vaire/src/service/governance.rs` | 917 | 650 | responsibility, caller boundary, persistence and side effects |
| `crates/spine/governance/arda-governance/src/triad.rs` | 906 | 771 | responsibility, caller boundary, persistence and side effects |
| `crates/engine/tests/workbench_rust_golden.rs` | 894 | - | test organization / fixture cohesion |

## Content-supported priorities

1. Aule autopilot task_queue.rs: migration-only executor and queue analyzer coexist with extensive history/claim handling. Lines 2-5 explicitly retain legacy execution for migration tests. Separate read-only history, migration safeguards and live authority before considering deletion.
2. Aule workbench_executor.rs and Engine objectives/workbench.rs: trace the shared live explicit execution contract before moving any legacy code. Engine imports prove that deleting the autopilot directory would remove live dependencies.
3. HUD App.tsx: imports spanning business, personal operations, research, governance, workbench, world and multiwindow state (lines 15-135) show a broad composition surface. Inspect state ownership and failure isolation, not just JSX length.
4. Manwe adaptive HTTP transport and routing files: preserve streaming, cancellation and feature behavior while assessing transport/domain separation.
5. Varda ingest.rs: examine fetch, transformation, policy, persistence and indexing boundaries separately.

## Retirement triage (nothing removed)

| Candidate | Evidence | Disposition |
|---|---|---|
| scripts/task-pivot.sh | Entire script refuses legacy execution; scripts/test_retired_task_pivot.py references it | Keep the rejection shim until old callers and migration safeguards are accounted for |
| Aule ActiveQueueExecutor | task_queue.rs:2-5,118 marks legacy migration-only execution | Candidate for isolated migration/test support, not blanket deletion |
| core loop_engine compatibility dispatch | loop_engine.rs:9-11 says JSONL dispatch is retired and Engine owns live execution | Audit remaining public callers before shrinking historical API |
| config/charon.providers.toml.bak | Inventoried backup; bounded source/config reference search found no visible caller | Suspected leftover only; inspect deployment/scripts and operator use without exposing contents |
| books/, crawls/, project/ | Empty at inspection | Low-risk housekeeping candidates; absence of files is not proof no tool expects the directory |
| #!/ | Unexpected local directory, containing bin/; no inventoried files | Inspect provenance before cleanup; not identified as application code |
| target/, .pytest_cache/, .tmp/ | Local build/cache/scratch locations, not inventoried source | Regenerable in principle, but assess active users before cleaning |
| audit/, .hermes/evidence/, data/ | Evidence/runtime holdings | Not dead code; retention and privacy decisions required |

## False positives ruled out

- packages/arda-mirromere-ui is declared by both HUD and Mirromere package manifests. It is not an orphan merely because it is outside apps/.
- HUD and Mirromere native apps have independent Cargo workspace roots. Absence from root cargo metadata does not mean unused.
- Manwe routing_adapter.rs includes an error stub only without adaptive; Cargo.toml:18 enables adaptive by default. Do not report default routing as unimplemented from that stub.
- Engine independent_critic_review warning has a test caller at objectives/workbench.rs:938. This is production-build hygiene, not proof that all critic behavior is absent.
- vendor/glib-0.18.5 is deliberately referenced by the root and both native app manifests. Preserve the security patch and provenance.

## What would prove non-use

For each deletion candidate: check build targets/features, language import/module graphs, CLI and subprocess strings, config and service references, dynamic/plugin loading, installation scripts, tests, external operator invocation and retention obligations. A text-search miss alone does not prove non-use. This pass does not certify any substantive source module as safely deletable.
