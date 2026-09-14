# HUD frontend/backend convergence contract v1

**Status:** Adopted C0 contract; production-path implementation in progress.

This directory defines the shared C0 fixture boundary for Rust, the Tauri bridge, and React. The operator ended the replacement Stage 5 soak on 2026-08-07 and authorized production-path convergence; release qualification now proceeds independently at the next actual candidate freeze.

## Authority decisions

- `arda-engine` authenticates operator sessions and owns authoritative identifiers, timestamps, policy decisions, topology, revisions, and durable mutation receipts.
- React submits typed intent. It never submits a `RunGraph`, `policy_safe`, an approval timestamp, completion evidence, or a receipt digest as authority.
- Tauri transports typed intents and projections without creating authority.
- An idempotency key is stable for one operator intent and retry; the backend binds it to operator, action, and target and rejects conflicting reuse.
- Accepted mutations return a backend-issued receipt with a durable reference. Transport success alone is not acceptance.

## Workbench request boundary

| Operation | React supplies | Rust supplies |
|---|---|---|
| Plan | project ID, objective text/input mode | run/objective/node/edge IDs, graph topology, authority, budgets, retry/checkpoint/provenance, idempotency, revision |
| Approve/reject | run ID, node ID, requested operator action, approval reference | resolved policy decision and lineage, approval time, idempotency, node transition, receipt |
| Complete | run ID, node ID, approval reference, provider receipt or operator review evidence | resolved authority, idempotency, evidence validation, completion truth, durable transition |

Cancellation, retry, project attachment, Research, Personal Operations, and monitor claims use the same intent/receipt split.

## Endpoint and command inventory

Names remain unchanged during preparation. C0 implementation versions the payloads behind these boundaries before changing consumers.

| Surface | Current transport names | Rust owner | Prepared change |
|---|---|---|---|
| Hermes system health | Tauri `read_hermes_runtime_health`, `ensure_hermes_runtime_surface`, `open_hermes_runtime_window` | HUD Tauri Hermes runtime state projecting `arda.system-health.hermes.v1` | **Implemented contract path:** five explicit states, verified-only stable identity, source revision/time, recovery action, shared launch/health identity predicate, and stale-response rejection. Native workstation continuity remains an acceptance gate. |
| Manwë system health | Tauri `read_manwe_runtime_projection`; Rust reads `/healthz`, `/providers/capabilities`, `/provider_candidates` | HUD Tauri `commands::system_health` projecting `arda.system-health.manwe.v1` | **Implemented:** one installed producer, deterministic revision/time, per-source diagnostics, bounded partial state, and recovery action. Direct browser HTTP is development fallback only. |
| Project | Tauri `validate_project_contract`, `attach_project_contract`; HTTP `POST /v1/projects/validate`, `POST /v1/projects/attach`, `GET /v1/projects` | `arda-engine::harness::projects` with HUD Rust approval resolution | **Implemented contract path:** browser submits an approval reference only; Rust requires configured operator identity plus an externally issued, exact-match, policy-safe, non-expired Oromë envelope and derives resource-bound idempotency. |
| Workbench planning and mutation | Tauri `plan_workbench_run`, `approve_workbench_run`, `complete_workbench_run_node`, `execute_workbench_provider_node`, `cancel_workbench_run`; HTTP `/v1/runs/plan` and `/v1/runs/:id/...` | HUD Rust canonical intent projection plus `arda-engine::harness::runs` durable journal/checkpoint authority | **Implemented contract path:** React sends objective/approval intent only; Rust creates graph/IDs/authority/budgets/retry/checkpoint/provenance/idempotency and forwards resolved approval. React renders returned records. |
| Workbench reads and stream | Tauri `get_workbench_run`, `get_workbench_run_events`, `start_workbench_run_event_stream`; HTTP `GET /v1/runs/:id`, `/events`, `/events/stream` | run store plus Tauri bridge | Backend cursor, per-run stream ownership, gap reload, deduplication, reconnect, and terminal closure. |
| Research | Direct HTTP `/v1/research/questions`, `/watchlists`, `/watchlists/:id/{pause,resume,retire}`, `/briefs` | `arda-engine::harness::research_operator` | Authenticated session plus intent/receipt; remove browser-created approval authority. |
| Personal Operations | Direct HTTP `/v1/personal/...` capture/item/reminder/data/projection endpoints | `arda-engine::harness::personal_ops` | Authenticated session, stable retry key, aggregate revision, common load state and receipt. |
| Monitor sessions | Tauri typed claim/release/refresh/playback/restore commands | Durable Tauri Rust registry under app data | **Implemented contract path:** five canonical slots, typed session/content identity, revision/lease, atomic durable reload, rollback on write failure, and same-session handoff. Native operator acceptance remains separate. |

## Projection vocabulary

Every projection uses exactly one of:

- `loading`: no authoritative response has completed;
- `healthy`: complete and within its freshness window;
- `stale`: last valid complete value exceeded its freshness window;
- `partial`: some named sources are missing, but a bounded projection remains usable;
- `degraded`: complete enough to operate with an explicit reduced-capability reason;
- `unavailable`: the authority cannot currently be reached and no usable value exists;
- `failed`: the authority responded or recovery ran, but the operation/projection failed.

All non-loading projections name source revision/time. Non-healthy states provide an operator-readable recovery action. Empty data is not evidence of `healthy`.

## Event stream contract

- The cursor is backend-issued and opaque to clients.
- Sequence is monotonic per run; `(run_id, sequence)` is the deduplication key.
- Reconnect sends the last accepted cursor.
- A gap triggers durable run reload before accepting later events.
- Each run owns an independent stream; opening another run never replaces it.
- Terminal events close only that run's stream.
- Browser storage may retain navigation hints, never recovery authority.

## Monitor-session v2 boundary

The canonical upper slots are `monitor_left_1`, `monitor_left_2`, `monitor_center`, `monitor_right_1`, and `monitor_right_2`. Each has independent owner/session/revision/lease/content identity. Opening a workstation uses the same `session_id`; it does not create a copy. The shared fixture deliberately covers web, media, document, terminal, and custom content.

The runtime maps five canonical physical slots to `monitor_1` through `monitor_5`; the contract fixture's semantic left/center/right names remain a portable layout vocabulary rather than a second registry authority.

## Shared fixtures

- Schema: `hud-frontend-backend-contract.schema.json`
- Passing fixture: `fixtures/valid-shared-contract.json`
- Fail-closed fixture: `fixtures/invalid-client-authority.json`

The fixed-fixture gate in `tests/test_workbench_contract_fixtures.py` validates the schema, both fixtures, the seven-state vocabulary, five canonical monitor slots, independent owners, and same-session workstation continuity.

Freeze-safe preparation consumers load this same fixture directly:

- `crates/engine/tests/hud_convergence_contract.rs`
- `apps/arda-hud/src-tauri/tests/hud_convergence_contract.rs`
- `apps/arda-hud/src/lib/hudConvergenceContract.test.ts`

They prove that all three layers resolve one fixture and agree on authority, state, stream, and monitor invariants. They do not claim that production handlers or React projections implement the contract yet.
