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

# Repository landscape

## Scope and snapshot

Inventory captured 2026-10-01T20:14:15.538287-07:00 on `plan/ambient-agent-program`, HEAD `db43d6948c081affc8f4d9ced371f4a509d303b5`, with pre-existing tracked modifications and untracked files. This describes the working tree, not a clean release or the installed binaries.

- 6,375 tracked/nonignored paths, of which 6,341 are tracked.
- 1,429 selected-extension non-vendor source files; 367,385 lines including tests/comments.
- 19 root-workspace packages; separate HUD and Mirromere native workspace roots also exist.
- 1,156 directory rows in DIRECTORY-INDEX.md. Parent/child totals overlap.

The inventory uses git ls-files plus nonignored untracked paths. It excludes this report, .git internals and ignored descendants such as node_modules/target. It includes every listed path whether semantically reviewed or not. Top-level filesystem entries are also listed, including empty/ignored-only directories. Source extensions and classification are documented in the retained generator; .rs.inc and other unlisted extensions are not included in source-line totals. The runtime_or_personal_data category is a conservative path classification, not a claim that every core/ file is mutable. Bytes are inventoried file sizes, not filesystem disk usage. Sensitive-name contents and symlink targets were not read for line counts. No exhaustive credential scan was performed.

## Top-level directories

### `#!/`

Unexpected local directory with bin/ child.

- Inventory: 0 paths; 0 tracked; 0 source lines.
- Assessment: No inventoried files. Provenance unresolved; not automatically delete-safe.

### `.github/`

Documentation health and release-signing workflows, CODEOWNERS.

- Inventory: 3 paths; 3 tracked; 0 source lines.
- Assessment: No general source build/test workflow in this directory. Remote branch protection or external CI was not queried.

### `.hermes/`

Tracked agent plans and evidence.

- Inventory: 30 paths; 30 tracked; 75 source lines.
- Assessment: Project-local historical work artifacts; not the user profile. Avoid competing task authority.

### `.pytest_cache/`

Ignored test cache.

- Inventory: 0 paths; 0 tracked; 0 source lines.
- Assessment: Excluded from source inventory.

### `.tmp/`

Local scratch.

- Inventory: 0 paths; 0 tracked; 0 source lines.
- Assessment: Excluded from source inventory; not inspected for contents.

### `adapters/`

External boundary code: Hermes operator bridge and voice capture.

- Inventory: 11 paths; 11 tracked; 2,250 source lines.
- Assessment: Keep authentication, event translation and delivery receipts separate from objective authority. Bridge inspection is sampled; live delivery not tested.

### `apps/`

HUD, launcher and Mirromere desktop applications.

- Inventory: 929 paths; 929 tracked; 76,617 source lines.
- Assessment: Three application surfaces with different build inclusion. HUD is broad; native UX and packaged deployment need separate acceptance.

### `audit/`

Retained audit output trees, including workbench queue and system audit runs.

- Inventory: 239 paths; 239 tracked; 0 source lines.
- Assessment: Evidence is not executable source. Retention and source-of-truth indexing matter more than refactoring these files.

### `benchmarks/`

Hades and Prometheus benchmark assets.

- Inventory: 6 paths; 6 tracked; 693 source lines.
- Assessment: Inventoried only; freshness, representativeness and reproducibility not validated.

### `books/`

Empty local directory at inspection.

- Inventory: 0 paths; 0 tracked; 0 source lines.
- Assessment: No inventoried source; possible housekeeping.

### `config/`

Service, systemd, provider, fleet, governance, adapter and monitoring inputs.

- Inventory: 103 paths; 103 tracked; 61 source lines.
- Assessment: Do not conflate examples/declarations with installed configuration. Provider/secret content was not broadly read.

### `core/`

Knowledge, operator notes, project and state trees. Not the arda-core Rust crate.

- Inventory: 1,362 paths; 1,358 tracked; 0 source lines.
- Assessment: Contains both declared contracts and mutable state. Separate shipped defaults, private material, historical evidence and regenerated projections.

### `crates/`

Engine and spine Rust implementations.

- Inventory: 885 paths; 863 tracked; 262,175 source lines.
- Assessment: See package map for all workspace members. Folder taxonomy does not consistently match responsibility, especially Aule.

### `crawls/`

Empty local directory at inspection.

- Inventory: 0 paths; 0 tracked; 0 source lines.
- Assessment: May be created by tooling; check callers before removal.

### `data/`

Run receipts, memory and operational stores.

- Inventory: 1,996 paths; 1,988 tracked; 0 source lines.
- Assessment: Substantial tracked runtime material. Privacy, retention, replay and installation portability need a dedicated pass; no personal record contents reviewed.

### `docs/`

Doctrine, plans, architecture, operations and history.

- Inventory: 437 paths; 437 tracked; 39 source lines.
- Assessment: Codemap membership count is stale and active plans contain broken links. This audit is an assessment, not another authoritative product plan.

### `integrations/`

Hermes presentation extension.

- Inventory: 4 paths; 4 tracked; 172 source lines.
- Assessment: A separate integration home from adapters. Distinct roles can justify both, but installation and compatibility ownership should be explicit.

### `meta/`

Repository metadata.

- Inventory: 1 paths; 1 tracked; 0 source lines.
- Assessment: Inventoried only; no semantic validation.

### `metrics/`

Local metric output.

- Inventory: 1 paths; 1 tracked; 0 source lines.
- Assessment: Generated observation, not architectural authority.

### `outposts/`

Protocol, scout and relic bridge crates.

- Inventory: 58 paths; 58 tracked; 7,464 source lines.
- Assessment: Bounded edges are useful. Device installation and actual remote callers not checked.

### `packages/`

Shared Mirromere UI package.

- Inventory: 11 paths; 11 tracked; 869 source lines.
- Assessment: Both app manifests reference it; retain. Package tests/typecheck are separate from root Cargo.

### `project/`

Empty local directory at inspection.

- Inventory: 0 paths; 0 tracked; 0 source lines.
- Assessment: Not evidence of an active project implementation.

### `scripts/`

Build, release, maintenance, validation and runtime tooling.

- Inventory: 69 paths; 69 tracked; 12,610 source lines.
- Assessment: A script may be used manually or externally with no source importer. Retired task-pivot shim intentionally refuses work.

### `sdk/`

Project adapter SDK surfaces.

- Inventory: 11 paths; 11 tracked; 914 source lines.
- Assessment: Rust SDK tests passed; external client adoption and non-Rust surfaces remain unverified.

### `spec/`

Versioned protocol/schema contracts and fixtures.

- Inventory: 59 paths; 59 tracked; 0 source lines.
- Assessment: Schemas are design/compatibility assets, not proof every live writer and reader agree.

### `src/`

Root daemon composition entry point.

- Inventory: 1 paths; 1 tracked; 357 source lines.
- Assessment: Root cargo check passed; no daemon restart or live acceptance performed.

### `target/`

Cargo build output.

- Inventory: 0 paths; 0 tracked; 0 source lines.
- Assessment: Excluded from source inventory and byte totals; may contain running installed/referenced artifacts.

### `tests/`

Root integration and script tests.

- Inventory: 23 paths; 23 tracked; 3,089 source lines.
- Assessment: Test existence is mapped, not measured coverage. Workspace all-target compilation failed before a full test run.

### `vendor/`

Pinned glib security backport.

- Inventory: 122 paths; 122 tracked; 52,322 source lines.
- Assessment: Explicit build dependency, not obsolete duplication. Root manifest documents byte-level patch constraints.

## Root files

| File | Role |
|---|---|
| `.gitattributes` | Version-control file handling. |
| `.gitignore` | Version-control exclusions. |
| `AGENTS.md` | Design constraints used by this audit. |
| `ARDA_ROOT_PROTOCOL.md` | Root authority/navigation doctrine. |
| `ARDA_SYSTEM_STATUS_REPORT.md` | Status narrative; not revalidated as live truth in this pass. |
| `Cargo.lock` | Resolved root build dependency lock. |
| `Cargo.toml` | Build/workspace membership and dependency declarations. |
| `LICENSE` | Repository licensing. |
| `NOTICE` | Dependency/provenance notices. |
| `README.md` | Human onboarding. |
| `SECURITY.md` | Security policy, not a security audit result. |
| `deny.toml` | Dependency policy. |
| `manwe.toml` | Gateway configuration entry; not broadly inspected for sensitive contents. |
| `services.toml` | Declared service/capability composition; not live health. |

## Navigation data

- [Full file inventory](file-inventory.csv)
- [All represented directories](DIRECTORY-INDEX.md)
- [Inventory statistics](inventory-summary.json)
- [Cargo package/target/dependency data](workspace-map.json)

These are snapshots. Do not use this folder as runtime input or execution authority.
