# HUD Frontend–Backend Integration Audit

**Audited:** 2026-08-06
**Scope:** read-only audit of the current HUD source and its live Rust/Tauri/HTTP/SSE owners
**Conclusion:** the Workbench canonical loop is connected. Contract preparation is explicit and production-path C0 implementation is now active; Stage 5 release qualification proceeds independently.

## Authority rule

Rust remains authoritative for identity, project attachment, approvals, run topology, node state, verification, receipts, health, freshness, and recovery. React may collect operator intent and project those records. It must not mint an approval decision, manufacture a run graph, declare completion, synthesize a receipt, or convert transport success into service health.

## Integration matrix

| Surface | Backend owner | Transport | Live state now | Missing or unstable contract | Acceptance test |
|---|---|---|---|---|---|
| System/runtime health — Hermes Dashboard | Hermes runtime plus the versioned Rust projection in `apps/arda-hud/src-tauri/src/lib.rs` | React → Tauri `read_hermes_runtime_health`, `ensure_hermes_runtime_surface`, `open_hermes_runtime_window` | **Converged contract path:** `arda.system-health.hermes.v1` carries identity, revision/time, explicit state, probes, failure, and recovery action | The registered health command now matches its React caller. Launch and health share one identity predicate; window results return that identity; out-of-order projections are rejected; one module mount issues one start/attach mutation. Native installed-window continuity remains acceptance evidence, not a second implementation authority. | Three Rust tests cover all five states, verified-only identity, and exact frontend serialization; React tests cover healthy/error rendering and stale healthy rejection. `cargo check`, 510 HUD tests, and production build pass. Live `:9119` was unavailable during the evidence run and was not mislabeled healthy. |
| System/runtime health — Manwë provider/routing view | `arda-manwe` HTTP service through Rust aggregation in `apps/arda-hud/src-tauri/src/commands/system_health.rs` | Installed React → Tauri `read_manwe_runtime_projection`; browser development fallback only → `/healthz`, `/providers/capabilities`, `/provider_candidates`; polling | **Converged production projection:** versioned `arda.system-health.manwe.v1` envelope with deterministic revision/time, per-source state/error, aggregate state, and recovery action | Installed production has one Rust-owned producer and preserves bounded partial data. Browser direct HTTP is explicitly a development fallback, not installed authority. Stale-window policy remains a later hardening item rather than a parallel producer. | Four Rust aggregation tests cover complete, partial, degraded, and unavailable states; React tests cover partial-source preservation and recovery rendering. Live `:7171` returned `ok: true` from all three sources on 2026-08-11. |
| Workbench project validation and attachment | `arda-engine` harness and HUD Tauri bridge in `commands/workbench.rs` | React → Tauri → `POST /v1/projects/validate` and `/v1/projects/attach` | Live canonical backend | React currently collects proposal/approval IDs and `envelope()` creates `decision: policy_safe`, timestamps, and idempotency keys. Backend must issue or resolve operator identity and approval envelopes; UI should submit intent/reference only. | Attempt attachment with valid, absent, expired, mismatched, reused, and denied approval references; only the backend-issued valid reference mutates state and returns durable lineage. |
| Workbench objective and run planning | `arda-engine` run graph | React → Tauri → `POST /v1/runs/plan` | Live canonical backend, but React builds the initial `RunGraph` and IDs in `workbench.ts` | Move canonical run ID, objective ID, node/edge topology, authority, retry, budget, checkpoint, and provenance creation to Rust. Freeze versioned request/response and explicit partial/unavailable/failed states. | Submit the same objective twice, restart, and replay. The backend returns stable/idempotent records; malformed client topology is rejected or ignored; React only renders the returned graph. |
| Workbench approval, cancellation, provider execution, operator completion, and review | `arda-engine` harness run endpoints | React → Tauri → typed HTTP mutations | Live canonical mutations and durable reads | React currently mints approval envelopes, accepts an arbitrary receipt digest and arbitrary evidence JSON, and phrases success locally. Freeze cancellation/retry/recovery semantics and require backend-issued receipt/evidence validation. | Exercise approve, reject/cancel, provider execute, failed verification, retry, completion, and recovery. Every visible terminal state must match a durable backend record and receipt after restart; fabricated IDs/digests/evidence fail closed. |
| Workbench run events and resume | `arda-engine` run event store and SSE route | Harness SSE → Tauri event emitter → React; durable `GET /v1/runs/{id}` fallback on explicit resume/event | Live, ordered by optional `sequence` in React | No declared `Last-Event-ID`/cursor contract, gap detection, automatic reconnect/backoff, terminal close semantics, or multi-run stream ownership. Tauri stores one stream task and replaces the previous stream. React stores objective and approval IDs in browser local storage. | Drop and restore the stream, duplicate/reorder events, open two runs, and restart HUD. Detect gaps, reconnect from a backend cursor, deduplicate deterministically, recover from durable backend state, and never rely on local storage for authority. |
| Research questions, watchlists, briefs, pause/resume/retire | `arda-engine::harness::research_operator` | Browser HTTP directly to `127.0.0.1:7878` | Live API; local React state holds selected question/brief and status text | Client defaults `operator-0` and constructs proposal/approval IDs, `policy_safe`, timestamps, and idempotency keys. Freeze operator authentication, server-issued mutation authority, error/degraded/stale shapes, response versioning, and receipt lineage. | For create and lifecycle mutations, prove backend-authenticated operator and server-validated approval authority, idempotent retries, durable receipt display, restart recovery, and distinct offline/denied/stale/failed states. |
| Personal Operations snapshot and capture/classification/reminder/data controls | `arda-engine::harness::personal_ops` and append-only personal store | Browser HTTP directly to `127.0.0.1:7878`; parallel snapshot reads | Live API and durable personal event log; operator-scoped export and deletion receipts are implemented | Client defaults `operator-0`; mutation keys remain client timestamp/random values; snapshot is assembled from separately fetched projections without one revision/freshness boundary. Freeze authenticated operator sessions, aggregate snapshot version, stable retry keys, stale/partial semantics, and common mutation receipt shape. | Capture, classify, acknowledge, export, and delete; retry each mutation; restart between mutation and refresh; prove one operator cannot read/mutate another, snapshots are revision-consistent or visibly partial, and system receipts survive personal-data deletion. |
| Research and Personal Operations rendering | React modules | Local component state over the live clients above | Live when the loopback harness is reachable; error strings otherwise | Introduce a common typed load state: `loading`, `healthy`, `stale`, `partial`, `degraded`, `unavailable`, `failed`, with source time/revision and operator-readable recovery. Avoid treating an empty list as healthy evidence. | Component tests feed every state, including mixed partial responses and stale timestamps; native HUD inspection proves states are distinguishable without changing authored geometry. |
| Five upper monitor surfaces and workstation continuity | HUD React/Three.js plus Tauri workstation windows; corrective plan remains authoritative | Typed monitor-session events and native windows | Not accepted; current settings expose only `monitor_left_1` through `monitor_left_4`, while the authored center aperture has no independent contract slot | Freeze the prepared five-slot v2 contract (`monitor_left_1`, `monitor_left_2`, `monitor_center`, `monitor_right_1`, `monitor_right_2`) with session identity, owner, revision, lease/expiry, typed content payload, remote-preview fallback, reload recovery, and same-session workstation handoff. Keep World View display-only. | Native acceptance proves five concurrent independently owned sessions, full authored-aperture rendering, web/media/docs/terminal/custom content, same live session in workstation windows, isolated reassignment, and restart recovery. |

## C0 contract-preparation result

The draft authority and transport boundary is now versioned at [`spec/hud-convergence/v1`](../../spec/hud-convergence/v1/README.md). Its shared fixture records:

- backend-authenticated operator sessions and backend-issued mutation receipts;
- objective-only planning intent and Rust-owned run topology;
- approval/rejection intent without browser-issued `policy_safe` decisions or timestamps;
- completion intent without browser-authored receipt/evidence truth;
- the seven-state projection vocabulary with revision, source time, and recovery action;
- per-run opaque SSE cursors, gap reload, deterministic deduplication, terminal closure, and durable recovery;
- five canonical independently owned monitor sessions with same-session workstation continuity.

`tests/test_workbench_contract_fixtures.py` is the preparation gate. Rust, Tauri, and React consume the same passing fixture while C0 production handlers and projections are converged.

## Finite 1.0 convergence backlog

### C0 — Freeze shared contracts before changing screens

1. Version endpoint and Tauri command request/response schemas.
2. Establish backend-issued operator identity and mutation authority.
3. Move Workbench graph/ID/approval/receipt authority out of React.
4. Define one state vocabulary for unavailable, partial, stale, failed, degraded, and healthy projections.
5. Freeze SSE sequence, cursor, gap, reconnect, deduplication, terminal, and multi-run behavior.
6. Require stable run, node, event, receipt, project, operator, question, watchlist, and personal-event identifiers.

### C1 — Vertical workflow order

1. System/runtime health.
2. Workbench objective → approval → execution → durable receipt → restart recovery.
3. Recovery and diagnostics.
4. Research/evidence projection.
5. Personal Operations.
6. Five-monitor projection/workstation continuity.

RELIC/CITADEL and Mirromere are outside the 1.0 convergence scope.

## Performance baseline before optimization

Measure without redesigning:

- cold launch and first useful render;
- HUD idle CPU/RSS;
- Three.js frame time and GPU load;
- React commit frequency;
- SSE event-to-visible-update latency;
- workstation opening latency;
- monitor/media memory growth;
- background polling and duplicate requests;
- long-session resource growth.

Optimize only measured bottlenecks after the contracts and interaction structure stabilize.

## Evidence inspected

- `apps/arda-hud/src/lib/workbench.ts`
- `apps/arda-hud/src/lib/manweLive.ts`
- `apps/arda-hud/src/lib/research.ts`
- `apps/arda-hud/src/lib/personalOps.ts`
- `apps/arda-hud/src/lib/hermesDashboardLauncher.ts`
- `apps/arda-hud/src/components/arda/modules/WorkbenchModule.tsx`
- `apps/arda-hud/src/components/arda/modules/HermesDashboardModule.tsx`
- `apps/arda-hud/src/components/arda/modules/ResearchModule.tsx`
- `apps/arda-hud/src/components/arda/modules/PersonalOperationsModule.tsx`
- `apps/arda-hud/src-tauri/src/commands/workbench.rs`
- `crates/engine/src/harness.rs`
- `crates/engine/src/harness/personal_ops.rs`
- `crates/spine/runtime/manwe/src/adaptive/transport/http.rs`
