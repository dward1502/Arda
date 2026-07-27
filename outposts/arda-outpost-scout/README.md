# arda-outpost-scout

Scout/survey crate for Arda outposts.

## What it does
Walks a repo surface, inspects crate metadata, and returns artifact-level survey data
suitable for Warden evidence review and council advisory review.

## Maturity
- Stable: bounded repo survey, Cargo.toml parsing, observation conversion, and
  advisory memory ingestion/recall
- Scope: one repo per survey call; scanning is restricted to `crates/`, `apps/`,
  and `outposts/` with a bounded depth

## Public API
- `survey::survey_repo(root)` -> `SurveyReport`
- `observation::{CrateObservation, CrateStatus, SurveyReport}`
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

## Build and test
```bash
cargo test
```

## Notes
- Intended to run standalone on Pi5/runtime toolchains
- Output is serializable with protocol contract types from `arda-outpost-protocol`
