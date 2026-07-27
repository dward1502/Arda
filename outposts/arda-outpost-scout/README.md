# arda-outpost-scout

Scout/survey crate for Arda outposts.

## What it does
Walks a repo surface, inspects crate metadata, and returns artifact-level survey data
suitable for Warden evidence review and council advisory review.

## Maturity
- Stable: bounded repo survey, Cargo.toml parsing, observation conversion
- Surface: single `survey_repo()` entrypoint plus local `CrateObservation` types
- Scope: one repo/outpost dir per call; depth is intentionally bounded

## Public API
- `survey::survey_repo(root)` -> `SurveyReport`
- `observation::{CrateObservation, CrateStatus, SurveyReport}`

## Build and test
```bash
cargo test
```

## Notes
- Intended to run standalone on Pi5/runtime toolchains
- Output is serializable with protocol contract types from `arda-outpost-protocol`
