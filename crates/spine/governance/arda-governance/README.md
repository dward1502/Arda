---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-21"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-07-21

# arda-governance

Deterministic governance primitives, configuration contracts, scoring projections,
and evidence records for Arda applications.

## Crate boundary

This crate owns:

- Triad and configurable governance-chain evaluation;
- philosopher profile parsing, validation, and status projection;
- resonance, Love Equation/Love Dynamics, JouleWork, and philosopher alignment scoring;
- governance readiness projections;
- Bacon-Lite evidence record formatting and filesystem writes;
- game-theory-labelled local agent-selection heuristics;
- audio, vision, and solar governance signal types.

This crate does not own transports, provider dispatch, daemon/process lifecycle,
application state stores, policy enforcement outside returned verdicts, or claims of
autonomous consensus. Those responsibilities stay in the consuming application or
service crate.

## Public API map

| Surface | Primary input | Primary output | Failure behavior |
| --- | --- | --- | --- |
| `triad_validate` | `&arda_core::Task`, optional `TriadConfig` | `TriadResult` | deterministic; no I/O |
| `evaluate_governance_chain` | task and validated `GovernanceChainConfig` | `GovernanceChainResult` | deterministic; no I/O |
| `load_governance_chain[_from_str]` | explicit path or TOML | `GovernanceChainConfig` | `GovernanceChainError` preserves read/parse/validation class |
| `load_philosopher_profiles[_from_str]` | explicit path or TOML | `PhilosopherProfileSet` | `PhilosopherProfileError` preserves read/parse/validation class |
| `calculate_resonance*` | task plus optional live governance/environment signals | `ResonanceScore` | deterministic; missing optional signals are represented in metadata |
| `evaluate_love_dynamics` | `LoveDynamicsInput` | `LoveDynamicsScore` | non-finite/unit inputs are normalized conservatively |
| `profile_joulework` | `&Task` | `JouleWorkProfile` | reports measurement source; does not upgrade estimated data to observed truth |
| `interpret_alignment` | `AlignmentSignals` | `TriadPhilosopherVerdict` | deterministic advisory result |
| `default_governance_readiness_report` | none | `GovernanceReadinessReport` | conservative projection; defaults are not autonomy-ready |
| `record_bacon_lite_to` | task, context, explicit `BaconLiteLogPaths` | `BaconLiteEvent` | returns `std::io::Error`; creates parent directories and appends records |
| `GameTheory::select_agent_with_policy` | task/action class | `GameTheorySelectionResult` | explicit fallback policy and reason when no candidate qualifies |

The crate-root re-exports in `src/lib.rs` are the supported consumer surface. Public
modules remain available for specialised types, but consumers should prefer root
re-exports where provided.

## Filesystem and configuration

Library code does not infer the repository root from `CARGO_MANIFEST_DIR`.
Construct `GovernancePaths::new(base_dir)` and pass the resulting path to loaders,
or pass any explicit path directly. `record_bacon_lite_to` is the preferred writer
API because destinations are injected. The compatibility wrapper
`record_bacon_lite` resolves its base from `ARDA_ROOT`, then the process working
directory, with `ARDA_BACON_LITE_LOG_PATH` and `ARDA_BACON_LITE_HUMAN_PATH` as
individual overrides.

```rust
use arda_governance::{load_governance_chain, GovernancePaths};

let paths = GovernancePaths::new("/srv/arda");
let config = load_governance_chain(paths.chain_config())?;
# Ok::<(), arda_governance::GovernanceChainError>(())
```

## Compatibility contract

- Removing or renaming a crate-root re-export, public field, or serialized field is
  a breaking change.
- Existing enum wire names and schema-version strings are stable within the current
  major version.
- New serialized fields must be additive and deserializable from older records;
  use Serde defaults where omission is valid.
- New enum variants require consumer impact review because exhaustive matches may
  break even when serialization remains compatible.
- Error variants may be added, but existing failure classes and their source errors
  should not be collapsed into unstructured strings.
- `tests/fixtures/public_api_v1.json` and `tests/public_api_compat.rs` guard the
  externally consumed result shapes and wire encodings.

The synthetic `calculate_resonance` and `calculate_resonance_basic` paths are deprecated
and scheduled for removal in `arda-governance` 0.3.0. New production code must evaluate
the Triad or configured chain once and pass that result to resonance. A degraded caller
that genuinely has no governance result must call `calculate_resonance_without_governance`,
which serializes `triad_purity_source = "absent"` instead of inventing a score.

The crate currently has no optional capabilities. `default = []` is intentional.
Future feature flags must be additive, must not alter default wire formats, and must
be covered both with default features and `--all-features` before release.

## Verification

From the workspace root:

```text
cargo fmt -p arda-governance -- --check
cargo test -p arda-governance --all-features
cargo doc -p arda-governance --no-deps --all-features
```
