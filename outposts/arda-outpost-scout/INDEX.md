# arda-outpost-scout index

## Canonical documents

- [`README.md`](README.md) — mission, runtime contract, consumers, and verification.
- [`STATUS.md`](STATUS.md) — current first-class status and recent evidence.
- [`BREAKDOWN.md`](BREAKDOWN.md) — exhaustive source/test graph and integration boundaries.
- [`OWNERSHIP.md`](OWNERSHIP.md) — collection, memory, projection, and authority ownership.

## Source entry points

- [`Cargo.toml`](Cargo.toml) — package targets and dependencies.
- [`Cargo.lock`](Cargo.lock) — standalone outpost lockfile.
- [`src/lib.rs`](src/lib.rs) — library root and public exports.
- [`src/main.rs`](src/main.rs) — Warden scout CLI/runtime binary.
- [`src/audit.rs`](src/audit.rs) — bounded Rúmil audit consumer and packet follow-up.
- [`src/runtime.rs`](src/runtime.rs) — HTTP API.
- [`src/research.rs`](src/research.rs) — governed SearXNG research.
- [`src/memory.rs`](src/memory.rs) — receipt ingestion and recall.
- [`src/survey.rs`](src/survey.rs) — bounded repository survey.
- [`src/observation.rs`](src/observation.rs) — survey observations.
- [`src/suggestion.rs`](src/suggestion.rs) — advisory summaries.
- [`src/error.rs`](src/error.rs) — typed survey failures.

## Test entry points

- [`tests/observation_fixtures.rs`](tests/observation_fixtures.rs)
- [`tests/survey_fixtures.rs`](tests/survey_fixtures.rs)
- [`tests/research_fixtures.rs`](tests/research_fixtures.rs)
- [`tests/memory_fixtures.rs`](tests/memory_fixtures.rs)
- [`tests/runtime_api.rs`](tests/runtime_api.rs)
- [`tests/runtime_cli.rs`](tests/runtime_cli.rs)
- [`tests/rumil_audit_contract.rs`](tests/rumil_audit_contract.rs)
- [`tests/rumil_runtime_api.rs`](tests/rumil_runtime_api.rs)