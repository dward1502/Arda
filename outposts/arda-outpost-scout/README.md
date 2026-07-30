# arda-outpost-scout

Bounded survey and source-bearing research runtime for Arda outposts.

## What it does
Walks a bounded repository surface, queries one operator-configured local
SearXNG endpoint, converts results into advisory outpost observations, and
stores complete observations through `arda-vaire` with durable receipt IDs.

## Maturity
- Stable: bounded repo survey, governed research requests, source validation,
  observation conversion, HTTP runtime, and advisory memory ingestion/recall
- Scope: one repo per survey call; scanning is restricted to `crates/`, `apps/`,
  and `outposts/` with a bounded depth

## Public API
- `survey::survey_repo(root)` -> `SurveyReport`
- `observation::{CrateObservation, CrateStatus, SurveyReport}`
- `SearxngClient::search(ResearchRequest)` -> `ResearchReport`
- `build_runtime_router(ScoutRuntimeState)` -> Axum router
- `ObservationMemoryBridge::encode_observation(observation)` -> `MemoryFallback`
- `ObservationMemoryBridge::recall_observations(request)` -> `ScoutRecallReport`
- `encode_observation_to_memory(root, observation)` and
  `recall_recent_observations(root, hours)` compatibility helpers

## Memory contract

- Stored content is the complete serialized `OutpostObservation`, preserving its
  id, observation timestamp, freshness, confidence, classification, authority,
  payload, and provenance.
- Successful ingestion returns the canonical `arda-vaire` memory id as an
  ingestion receipt. It does not promote or grant execution authority.
- Recall can be constrained by observation scope, crate/app name, path, free-text
  query, time window, result limit, and maximum observation age.
- Unavailable stores and stale records return structured degraded states instead
  of failing the scout observation path.
- Root precedence is an explicit bridge root, then `ARDA_ROOT`; an explicit
  fallback root, then `SCOUT_MEMORY_FALLBACK_ROOT`, is tried if the primary root
  cannot open.

## Research and authority contract

- Requests must use the fixed `allowlisted_public_web` policy, include an expiry,
  remain within a 24-hour validity window, and keep queries within 512 bytes.
- The request cannot choose a model, shell command, tool, search engine, or
  endpoint. The operator configures one local SearXNG endpoint; this crate has no
  model client or general tool-execution dependency.
- The accepted policy allowlists the SearXNG tool boundary, not individual result
  domains. Every retained result must still carry a valid HTTP(S) URL with a host.
- Result count is clamped to 10. SearXNG connect/request timeouts are 5/15
  seconds; topic-runner connect/request timeouts are 5/30 seconds and at most 16
  enabled topics run per invocation.
- Research observations are `raw_measurement` with `advisory` authority. They
  cannot approve, dispatch, promote, or append to the project task queue.

## HTTP and CLI runtime

- `serve`: `/health`, `/search`, `/survey`, and `/recall`.
- `run-topics`: submits at most 16 configured topics with the fixed source policy
  and a 15-minute expiry.
- The root daemon harness proxies health/search/recall when
  `ARDA_WARDEN_SCOUT_URL` is configured. The ARDA HUD consumes the separate
  Athena/runtime projection as a read-only evidence lane.

See [`BREAKDOWN.md`](BREAKDOWN.md), [`OWNERSHIP.md`](OWNERSHIP.md), and
[`STATUS.md`](STATUS.md) for the complete source graph and operational boundary.

## Build and test
```bash
cargo test -p arda-outpost-scout --all-features
cargo clippy -p arda-outpost-scout --all-targets --all-features -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc -p arda-outpost-scout --no-deps --all-features
```

## Notes
- Intended to run standalone on Pi5/runtime toolchains.
- Output is serializable with protocol contract types from `arda-outpost-protocol`
