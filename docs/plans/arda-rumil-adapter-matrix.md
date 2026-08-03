# `arda-rumil` Adapter and Reuse Matrix

**Status:** Planning inventory; no dependencies selected for implementation yet.

The purpose of this file is to prevent Rúmil from reimplementing mature project-analysis tools. The first implementation should prove the adapter boundary with Cargo/Git and a generic filesystem profile before adding more providers.

## Adapter boundary

Each provider receives a validated Rúmil request context and returns provider output plus a command/provider receipt. Providers do not write memory, mutate files, approve work, or call Warden directly.

```text
AuditRequest + Policy
        ↓
ProviderAdapter::capabilities()
ProviderAdapter::run()
        ↓
ProviderResult + CommandReceipt
        ↓
Rúmil normalizer
        ↓
AuditReport / Finding / OrganizationPlan
```

## Initial adapters

| Adapter | Existing source/tool | Initial output | Status |
|---|---|---|---|
| `generic_inventory` | `ignore`/`walkdir`-style traversal | tree, file metadata, hashes, exclusions | Planned |
| `cargo_workspace` | `cargo metadata`, `cargo_metadata` | packages, targets, workspace members | Planned |
| `cargo_dependencies` | Cargo metadata / `cargo tree` | resolved dependency edges and duplicate versions | Planned |
| `git_state` | Git CLI, `gix`, or `git2` after comparison | revision, branch, dirty summary, bounded status | Planned |
| `rust_modules` | `cargo-modules` or selected Rust parser | module tree, internal edges, orphan candidates | Planned |
| `rust_source` | `syn`, Tree-sitter, or rust-analyzer adapter | selected symbols/modules and bounded excerpts | Deferred until need is measured |
| `dependency_security` | `cargo-audit`, `cargo-deny`, `cargo-vet` | tool-backed vulnerability/license/policy findings | Planned optional provider |
| `node_project` | package manifest/tool-specific commands | package scripts/dependencies/tests | Later |
| `python_project` | project metadata/tool-specific commands | packages/tests/tooling | Later |

## Cargo adapter requirements

Use the actual Arda workspace manifest as the first fixture:

```text
/var/home/mythos/Eregion/Arda/Cargo.toml
```

The adapter must derive package names from manifests or resolved Cargo metadata. It must not infer package identity from directory names.

The first Arda fixture should cover:

```text
crates/spine/memory/arda-vaire
crates/spine/runtime/arda-mandos
crates/spine/runtime/manwe
crates/spine/executors/arda-varda
outposts/arda-outpost-scout
```

The comparison fixture should include a minimal non-Cargo project to prove that the coordinator is not Cargo-only.

## Historical HADES mapping

The old HADES implementation supplied useful domain rules but is not itself the generic adapter:

| Old HADES behavior | Rúmil destination |
|---|---|
| `WalkDir` traversal | generic inventory provider |
| organization coverage | organization profile/rule provider |
| lifecycle findings | normalized finding provider |
| SHA-256 evidence | common evidence/hash layer |
| Warden/Athena handoff | consumer integration, not provider logic |
| Mnemosyne event emission | `arda-vaire` observation bridge |
| approval packets | review-only organization plan plus existing governance boundary |
| archive/remove execution | explicitly out of first Rúmil crate |
| Soterion/Markdown rules | optional legacy/Arda organization profile |

## Profile model

Profiles should declare:

```toml
id = "arda-rust-workspace-v1"
project_kinds = ["cargo", "git"]
required_capabilities = ["inventory", "cargo_workspace", "git_state"]
optional_capabilities = ["cargo_dependencies", "rust_modules", "dependency_security"]
root_excludes = [".git", "target", "node_modules"]
secret_patterns = [".env", "*.pem", "*.key", "credentials*"]
max_depth = 12
max_files = 100000
max_total_bytes = 268435456
max_excerpt_bytes = 65536
command_timeout_seconds = 60
organization_rules = ["documentation-drift", "generated-artifact-drift"]
```

The exact profile schema belongs in the future crate. This outline is not an instruction to add these values without verifying workspace conventions.

## Provider failure semantics

A provider may return:

```text
completed
failed
unavailable
skipped_by_policy
denied_by_budget
timed_out
malformed_output
```

These statuses must appear in the audit report and affect completeness. A failed optional provider may yield `partial`; a failed required provider cannot yield `complete`.

## Learning boundary

Adapters produce observations. They do not learn.

A separate later layer may calculate project baselines or provider-quality statistics from retained packets, but it must use:

- explicit schemas;
- retention policy;
- operator-visible provenance;
- bounded replay;
- false-positive disposition records;
- no automatic mutation of audit policy without review.

Until that layer exists, words such as `learning`, `training`, or `self-improving` must not appear in Rúmil completion claims.

## Implementation order

1. generic inventory and packet contract;
2. Cargo workspace adapter;
3. Git state adapter;
4. provider receipt normalization;
5. organization profiles;
6. baseline comparison;
7. Vairë observation bridge;
8. Warden integration;
9. optional source/security providers;
10. later multi-language profiles.
