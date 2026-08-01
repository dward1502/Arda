# arda-outpost-scout breakdown

## Scope

`arda-outpost-scout` owns bounded repository survey, governed SearXNG research,
advisory observation conversion, durable Vairë memory ingestion/recall, and the
Warden HTTP/CLI runtime. It does not own task queues, approvals, dispatch,
execution, model inference, Athena ledger production, or council decisions.

## Supported source graph

All nine Rust files under `src/` are wired. The crate has no optional Cargo
features, build script, generated includes, or detached Rust source.

| Path | Classification | Role |
| --- | --- | --- |
| `src/lib.rs` | production/default | Module declarations and public exports |
| `src/main.rs` | production/default binary | `serve` and bounded `run-topics` CLI |
| `src/runtime.rs` | production/default | Axum health, search, survey, and recall routes |
| `src/research.rs` | production/default | Request policy/expiry validation, bounded SearXNG client, source validation, advisory conversion |
| `src/memory.rs` | production/default | Append-only Vairë ingestion receipts, filtered recall, degraded fallback state |
| `src/survey.rs` | production/default | Depth- and root-class-bounded Cargo/app discovery |
| `src/observation.rs` | production/default | Survey report and crate-observation contract/conversion |
| `src/suggestion.rs` | production/default | Non-authoritative advisory summaries |
| `src/error.rs` | production/default | Survey and manifest error contract |

Six integration-test targets are wired through Cargo:

| Path | Classification | Proof |
| --- | --- | --- |
| `tests/observation_fixtures.rs` | test-only | Root and nested workspace discovery |
| `tests/survey_fixtures.rs` | test-only | Advisory generation and protocol conversion |
| `tests/research_fixtures.rs` | test-only | Result bounds, policy/expiry, HTTP(S) provenance, advisory authority |
| `tests/memory_fixtures.rs` | test-only | Receipt preservation, append-only recall, fallback/stale behavior, no queue authority |
| `tests/runtime_api.rs` | test-only | Health, policy/expiry rejection, persistence, recall |
| `tests/runtime_cli.rs` | test-only | Supported CLI command surface |

`observation.rs` and `suggestion.rs` also contain adjacent unit tests for status
classification and advisory generation.

## Bounded research contract

- Exactly one source-policy identifier is accepted:
  `allowlisted_public_web`.
- The policy fixes tool access to the operator-configured SearXNG endpoint;
  callers cannot choose an engine, endpoint, model, shell, or arbitrary tool.
- Query size is at most 512 bytes and request expiry must be in the future but no
  more than 24 hours away.
- Requested result count is clamped to 1..=10.
- Search connect/request timeouts are 5 and 15 seconds.
- Every retained result URL must parse as HTTP(S) with a host. The report
  keeps provider, policy, expiry, result URL, engine, content, and score.
- Research conversion emits protocol schema v1, raw-measurement classification,
  advisory authority, and SearXNG query provenance.

The policy allowlists the web-search tool boundary rather than maintaining a
domain-level allowlist. Domain curation remains an upstream policy concern.

## Memory and authority boundaries

Successful `/search` writes the complete governed observation through
`arda-vaire` and returns its canonical memory ID in `SearchResponse.memory`.
Writes are append-only. Recall is bounded by hours, limit, optional filters, and
optional maximum age. Memory failure returns structured degraded state.

Scout has no dependency or API for project-task queue writes, approvals,
promotion, dispatch, or execution. Its protocol authority is always advisory.

## Dependencies and consumers

Normal dependencies are the shared workspace HTTP/async/serialization stack,
`arda-outpost-protocol`, and `arda-vaire`. There are no optional dependencies or
feature modes.

`cargo tree -i arda-outpost-scout --edges normal` identifies no Cargo consumer;
the package is a standalone library/binary leaf. Live integration consumers are:

- `crates/engine/src/harness.rs`: root daemon HTTP proxy for scout
  health/search/recall with an independently configured timeout.
- `apps/arda-hud/src/lib/ardaSource.ts` and `reviewGateDerivation.ts`: read-only
  projection of Athena scout request/finding ledgers and scout runtime state.

No live producer currently converts Warden receipts into
`data/athena/scout_requests.jsonl`, `data/athena/scout_findings.jsonl`, or
`core/state/scout_runtime.json`; that durable Warden→Athena/council handoff
remains active-plan work.

## Operational state

The Warden user service binds its Tailscale address on port 8092 and uses local
SearXNG on port 18080. Runtime port changes are coordinated integration changes;
this Packet does not alter existing bind assumptions.