# Arda Rúmil Project Audit and Organization Plan

**Status:** Complete; operator-accepted and archived
**Created:** 2026-08-03  
**Owner:** `arda-rumil`  
**Target:** Project-independent, evidence-first repository audit and organization coordination for Arda, Annunimas reference workspaces, and unrelated supported projects

## Purpose

Create `arda-rumil` as the reusable audit coordinator that can inspect an arbitrary project through bounded adapters, normalize tool output into versioned evidence packets, compare current state with prior observations, and produce review-only organization plans.

Rúmil owns coordination, evidence normalization, history comparison, and review boundaries. It must not reimplement Cargo, rust-analyzer, Tree-sitter, dependency-audit, Git, or language-specific analysis tools that already provide those capabilities.

## Naming decision

New implementation and contract names use the Silmarillion name `Rúmil` / `arda-rumil`.

- New Rust package name: `arda-rumil`
- Rust library identifier: `arda_rumil`
- New contracts: `arda.rumil.*`
- New state paths: `data/rumil/` and `core/state/rumil_*` where a repository-owned projection is required

Historical `HADES` names, records, and paths remain readable migration inputs. This plan does not perform a broad rename of existing HADES-generated documentation or runtime state. Legacy conversion is a later, receipt-backed migration task.

## Verified current boundaries

The live Arda workspace already contains:

- `arda-vaire` at `crates/spine/memory/arda-vaire`: durable episodic/semantic/procedural memory, scoped recall, consolidation, and receipt-backed observation storage.
- `arda-outpost-scout` at `outposts/arda-outpost-scout`: bounded repository survey and Warden research; it currently discovers Cargo/app structure but does not own a complete audit.
- `arda-mandos` at `crates/spine/runtime/arda-mandos`: advisory reasoning, evidence-linked verdicts, scoring, and transport; it is not a filesystem walker.
- `arda-varda` at `crates/spine/executors/arda-varda`: evaluation/executor-side governance boundary; it must not receive arbitrary filesystem authority.
- `arda-engine` at `crates/engine`: harness/proxy surface for Warden and Workbench paths.
- Existing HADES-derived state and scripts under `data/hades/`, `core/state/hades_*`, `scripts/hades_*`, and `audit/`.

The root `Cargo.toml` now registers `crates/spine/runtime/arda-rumil` in both `members` and `default-members`.

## Product promise

Given a project root and an explicit bounded audit profile, Rúmil can produce:

1. a project identity and revision record;
2. a complete or explicitly partial file/tree inventory;
3. project/package/module/dependency structure from appropriate adapters;
4. source, test, build, lint, security, and organization findings where configured;
5. evidence hashes and command/tool provenance;
6. comparison against prior Rúmil packets;
7. a review-only organization plan with no implicit mutation authority;
8. a packet consumable by Warden, Mandos, Vairë, and Workbench.

Rúmil must state what it did not inspect. A successful process with partial coverage is not a complete audit.

## Authority invariants

- Rúmil observes and coordinates; it does not approve or execute destructive changes.
- Every scan is bounded by root policy, path exclusions, file/byte/token limits, timeout, and tool allowlist.
- Every command is represented by a receipt with argv digest, working directory, exit status, output digests, truncation, and authority class.
- Every finding links to evidence or explicitly states that evidence is unavailable.
- Organization output is a dry-run plan by default.
- Moves, archives, rewrites, or deletes require a separate operator/governance path and are not enabled by the Rúmil audit crate.
- Audit packets are evidence/state, not model-training data and not execution authorization.
- Public research remains Warden's responsibility; Rúmil may provide project evidence but does not silently cross the network boundary.
- Mandos may evaluate Rúmil evidence; it does not gain arbitrary filesystem access through that evaluation.
- Vairë stores approved/eligible continuity and receipts; it is not replaced by Rúmil's packet store.

## Reuse-first tool policy

Rúmil must call or consume established tools before adding custom analyzers:

| Concern | Preferred existing source | Rúmil responsibility |
|---|---|---|
| Workspace/package identity | `cargo metadata` / `cargo_metadata` | Normalize package records and provenance |
| Resolved dependency graph | `cargo tree` / metadata dependency edges | Normalize graph and detect drift |
| Rust module structure/orphans | `cargo-modules` or equivalent adapter | Store bounded structural evidence |
| Rust syntax | `syn`, Tree-sitter, or rust-analyzer adapter | Normalize selected symbols/modules only |
| Multi-language syntax | Tree-sitter grammars / `rust-code-analysis` where appropriate | Adapter selection and bounded output |
| Security/license policy | `cargo-audit`, `cargo-deny`, `cargo-vet`, project tools | Preserve tool version/config/findings |
| Git state/history | `git`/`gix`/`git2` adapter | Record revision, dirty state, and bounded provenance |
| Generic traversal | `ignore`/`walkdir` | Apply root/path/file/budget policy |
| Memory/continuity | `arda-vaire` | Store eligible receipts/observations through existing contracts |

A provider may be unavailable, fail, or be skipped by policy. The packet must disclose that state rather than silently replacing it with a heuristic.

## Scope phases

### RUMIL-0 — Contract and ownership baseline

**Files likely to create:**

- `crates/spine/runtime/arda-rumil/Cargo.toml`
- `crates/spine/runtime/arda-rumil/src/lib.rs`
- `crates/spine/runtime/arda-rumil/src/contracts.rs`
- `crates/spine/runtime/arda-rumil/README.md`
- `crates/spine/runtime/arda-rumil/INDEX.md`
- `crates/spine/runtime/arda-rumil/BREAKDOWN.md`
- `crates/spine/runtime/arda-rumil/STATUS.md`
- `crates/spine/runtime/arda-rumil/OWNERSHIP.md`

**Work:**

- [x] Add the crate only after confirming no existing Arda crate already owns the coordinator role.
- [x] Define `arda.rumil.audit-request.v1`, `arda.rumil.audit-report.v1`, `arda.rumil.command-receipt.v1`, `arda.rumil.finding.v1`, `arda.rumil.organization-plan.v1`, and `arda.rumil.comparison.v1`.
- [x] Define explicit completeness states: `complete`, `partial`, `structure_only`, `failed`, and `not_requested`.
- [x] Define authority, provenance, redaction, truncation, budget, and failure fields.
- [x] Define project identity independently from local absolute paths so the same project can be audited on host and Pi without false identity matches.
- [x] Define compatibility rules for importing historical HADES reports without presenting them as native Rúmil evidence.
- [x] Add malformed, unknown-field, round-trip, and future-schema fixtures.

**Gate:** Contract tests pass and ownership docs clearly distinguish Rúmil, Vairë, Warden, Mandos, Varda, and the existing scout.

**Implemented evidence (2026-08-02):** strict `arda.rumil.v1` envelopes and all six phase contracts are implemented in `src/contracts.rs`; `tests/contract_compliance.rs` covers 15 contract cases, including malformed JSON, unknown envelope/payload fields, round trips, future-schema rejection, completeness states, canonical kinds, and review-only defaults. `README.md`, `INDEX.md`, `BREAKDOWN.md`, `STATUS.md`, and `OWNERSHIP.md` now define the crate and neighboring authority boundaries.

### RUMIL-1 — Generic bounded inventory

**Files likely to create:**

- `src/policy.rs`
- `src/inventory.rs`
- `src/tree.rs`
- `src/hash.rs`
- `tests/inventory_contract.rs`
- `tests/fixtures/`

**Work:**

- [x] Implement root validation and canonical project identity.
- [x] Walk only approved roots with explicit maximum depth, file count, byte count, and timeout.
- [x] Exclude `.git`, `target`, `node_modules`, secrets, credential files, and policy-defined private paths by default.
- [x] Emit directories, files, symlinks, unreadable paths, excluded paths, and truncation reasons.
- [x] Compute stable file hashes only for files within the policy budget.
- [x] Preserve relative paths and avoid leaking absolute host paths into portable packets.
- [x] Test empty roots, unreadable paths, symlink loops, binary files, oversized files, exclusion rules, and deterministic ordering.

**Gate:** A generic project with no Cargo files still receives a truthful inventory packet; no inventory claims complete coverage when limits were hit.

**Implemented evidence (2026-08-02):** `tests/inventory_contract.rs` covers 15 generic-inventory cases, including non-Cargo roots, empty roots, bounded file/byte/timeout truncation, default and policy exclusions, unreadable paths, binary hashing, symlink loops, selected subtrees, portable relative paths, and deterministic ordering. Verified with `cargo test -p arda-rumil --all-features`, `cargo test -p arda-rumil --no-default-features`, and warning-denying Clippy.

### RUMIL-2 — Project adapters

**Files likely to create:**

- `src/adapters/mod.rs`
- `src/adapters/cargo.rs`
- `src/adapters/git.rs`
- `src/adapters/generic.rs`
- later: `src/adapters/node.rs`, `src/adapters/python.rs`
- adapter fixtures and tests

**Work:**

- [x] Start with Cargo and Git adapters because Arda and Annunimas are Rust workspaces.
- [x] Use `cargo metadata --no-deps` for membership and `cargo metadata`/resolved edges where dependency resolution is required.
- [x] Capture `cargo tree` or equivalent only under explicit command policy.
- [x] Detect package manifests, source roots, binaries, libraries, examples, benches, and tests.
- [x] Record Git revision, branch, dirty-state summary, and bounded status output without treating Git state as source truth for code behavior.
- [x] Define adapter capability discovery so unsupported project types become `not_requested` or `unsupported`, not false negatives.
- [x] Add a generic adapter for non-Cargo projects before adding language-specific analysis.

**Gate:** The Arda workspace and a minimal non-Cargo fixture produce structurally different but valid packets with honest capability disclosures.

**Implemented evidence (2026-08-02):** opt-in `cargo` and `git` features expose normalized, relative-path-only Cargo workspace and read-only Git snapshots; the generic inventory remains in the default feature set. `tests/adapter_contract.rs` covers Cargo members/targets/resolved edges, non-Cargo fallback, provider allowlisting, Git revision/branch/dirty entries, unborn repositories, and bounded truncation. `cargo test -p arda-rumil --all-features` passes 35 tests; no-default tests and warning-denying Clippy also pass.

### RUMIL-3 — Analysis provider integration

**Files likely to create:**

- `src/providers/mod.rs`
- `src/providers/cargo_commands.rs`
- `src/providers/module_structure.rs`
- `src/providers/security.rs`
- `src/providers/source.rs`
- provider fixtures/tests

**Work:**

- [x] Add an allowlisted command-provider interface with bounded stdout/stderr and timeout.
- [x] Integrate existing Cargo and security tools through receipts instead of duplicating their logic.
- [x] Add optional `cargo-modules` integration for module tree, dependency, and orphan views where installed.
- [x] Add targeted, profile-selected Rust source excerpts without adding a syntax analyzer before measured need exists.
- [x] Keep source excerpts bounded, redacted, and selected by request/profile rather than dumping repositories.
- [x] Preserve tool version, configuration digest, exit status, and output digest for each provider.
- [x] Separate heuristic findings from tool-backed findings.

**Gate:** Provider absence, timeout, malformed output, and nonzero exit are represented as evidence states and do not become fabricated success.

**Implemented evidence (2026-08-02):** the opt-in `provider` feature supplies code-registered provider specifications for Cargo check, `cargo-audit`, `cargo-deny`, and `cargo-modules`; `ProviderRunner` enforces provider-ID policy, project-relative working directories, command timeouts, bounded stdout/stderr, full-stream digests, tool version, configuration digest, and review-only receipts. `tests/provider_contract.rs` passes 11 provider/source cases covering allowlist denial, success, nonzero exit, timeout, unavailable tools, truncation, malformed JSON, path escape, version capture, and selected redacted excerpts.

### RUMIL-4 — Findings, baselines, and historical comparison

**Files likely to create:**

- `src/findings.rs`
- `src/comparison.rs`
- `src/baseline.rs`
- tests/fixtures for prior/current packets

**Work:**

- [x] Normalize provider results into `arda.rumil.finding.v1`.
- [x] Assign stable finding IDs from project identity, category, relative path, and evidence identity.
- [x] Compare current packets to a selected prior packet by project identity and source revision.
- [x] Classify new, resolved, persistent, changed, stale, and unverifiable findings.
- [x] Track false-positive/operator disposition as explicit feedback, not silent classifier mutation.
- [x] Define how eligible summaries may be written to Vairë without copying raw source content into memory.
- [x] Do not call this machine learning until a separate, tested learning policy exists.

**Gate:** Replaying the same audit is idempotent; reordered provider output does not create false changes; changed files produce a bounded comparison.

**Implemented evidence (2026-08-02):** `src/findings.rs`, `src/baseline.rs`, and `src/comparison.rs` provide UUIDv5 stable finding IDs, explicit baseline selection, order-independent lifecycle comparison, non-learning operator feedback, and a counts/receipt-only Vairë observation projection. `tests/findings_comparison_contract.rs` passes 7 cases covering replay/order stability, changed/resolved/new/persistent/stale/unverifiable classifications, cross-project and duplicate-ID rejection, feedback invariants, and raw-summary exclusion from memory projections.

### RUMIL-5 — Organization planning and safe lifecycle boundary

**Files likely to create:**

- `src/organization.rs`
- `src/approval.rs` only if a non-mutating packet projection needs it
- plan/receipt fixtures and tests

**Work:**

- [x] Generalize the old HADES organization concepts into project-neutral organization profiles.
- [x] Support missing README/INDEX and case-colliding path checks from inventory, plus tool-backed stale-generated, misplaced-output, and documentation-drift observations, only when enabled by profile.
- [x] Produce review-only organization plans with candidate, evidence, recommended action, risk, and affected paths.
- [x] Preserve no-delete/no-move/no-rewrite defaults.
- [x] Emit a dry-run receipt even when there are zero candidates.
- [x] Define a future mutation handoff to the existing governance boundary without implementing mutation in the first crate.
- [x] Keep legacy HADES reports importable as historical evidence but do not re-emit HADES contracts from new code.

**Gate:** Organization planning can inspect Arda, an unrelated Rust fixture, and a non-Rust fixture without applying changes or assuming Soterion metadata.

**Implemented evidence (2026-08-02):** `src/organization.rs` implements declarative project-neutral rule profiles, deterministic candidate and plan identities, inventory-backed missing-document/case-collision checks, provider-observed stale-generated/misplaced-output/documentation-drift checks, zero-candidate dry-run receipts, hard review-only/no-mutation invariants, an unimplemented external-governance handoff contract, and SHA-256-preserving historical-only HADES import. `tests/organization_contract.rs` passes 7 all-feature cases and 6 no-default-feature cases covering rule gating, deterministic review-only candidates, Arda/unrelated-Rust/non-Rust parity, zero-candidate receipts, mutation-boundary disclosure, and legacy provenance.

### RUMIL-6 — Warden and memory integration

**Files likely to modify:**

- `outposts/arda-outpost-protocol/src/` for the audit request/observation contract if a shared wire contract is required
- `outposts/arda-outpost-scout/src/survey.rs` and `src/observation.rs`
- `outposts/arda-outpost-scout/src/memory.rs` only through the existing Vairë bridge
- `crates/engine/src/harness/` only after the library contract is stable
- direct consumer tests

**Work:**

- [x] Replace or extend the scout's shallow survey with a Rúmil-backed audit request path without breaking the existing survey contract.
- [x] Keep the scout as Warden's bounded research/outpost surface; do not move all audit code into the scout.
- [x] Return a canonical Rúmil audit receipt ID and completeness state to Warden.
- [x] Store only eligible bounded observations/receipt metadata through `arda-vaire`; retain large packets at their audit-owned path.
- [x] Permit Warden to ask targeted follow-up questions against an audit packet without granting arbitrary filesystem authority.
- [x] Preserve advisory authority and explicit degraded state in scout observations.
- [x] Add offline replay, duplicate request, expired request, and partial-audit fixtures.

**Implemented evidence:** `outposts/arda-outpost-scout/src/audit.rs` adds a relative-root-only direct consumer over Rúmil's generic bounded inventory. `POST /audit` validates expiry, read-only authority, capability, root policy, and non-zero budgets; emits deterministic audit IDs; classifies complete/partial coverage; persists the full packet under `data/warden/rumil_audits/`; and appends a digest-bound idempotency receipt. Replays return the same packet without a second receipt or Vairë write, while request-ID/content conflicts are rejected. Vairë receives only a compact advisory `rumil_audit_receipt` observation. `POST /audit/followup` accepts an audit ID, fixed section enum, bounded record limit, and packet-relative prefix; it has no filesystem-root field. Direct and Axum fixtures cover first execution, packet persistence, compact Vairë projection/recall, replay, duplicate conflict, expired/elevated rejection, partial completeness, and packet-only follow-up. The legacy `/survey` contract remains unchanged.

**Gate:** Warden can request an audit, receive a source-bearing Rúmil packet receipt, and use it to scope research without creating a second queue or execution path.

### RUMIL-7 — Mandos/Varda evaluation and Workbench projection

**Files likely to modify:**

- `crates/spine/runtime/arda-mandos/` only for additive evidence-consumption surfaces
- `crates/spine/executors/arda-varda/` only for existing evaluation contracts
- `crates/engine/src/harness/research.rs` and related projection code
- `docs/plans/2026-07-29-warden-research-application-plan.md` reconciliation section

**Work:**

- [x] Add Rúmil evidence references to research briefs without embedding unbounded source content.
- [x] Let Mandos distinguish tool-backed, heuristic, historical, partial, and unavailable evidence.
- [x] Let Varda evaluate claims against Rúmil receipts only through existing governed evidence paths.
- [x] Show audit completeness, stale baseline, rejected provider, and missing evidence states in Workbench/HUD projections.
- [x] Keep every resulting brief advisory and prevent audit findings from becoming executable work.

**Gate:** A research brief can cite a Rúmil audit packet, disclose partial coverage, and preserve Warden/Mandos/Varda authority boundaries.

**Evidence:** `arda-rumil::RumilEvidenceReference` carries only bounded packet identity, digest, classifications, completeness, and degraded-state metadata. `arda-mandos::classify_rumil_evidence` preserves the five evidence classes, while `arda-varda::evaluate_rumil_evidence` produces advisory-only acceptance/review/rejection receipts with `execution_authorized: false`. The Workbench research brief and HUD research module project the packet reference, completeness, stale baseline, rejected providers, missing evidence, and evaluation status without embedding packet contents. Focused Rúmil, Mandos, Varda, engine, and HUD fixtures cover the path.

### RUMIL-8 — Project profiles and generalized deployment

**Work:**

- [x] Define profile files for Arda/Annunimas, generic Rust, Node, Python, and mixed projects.
- [x] Keep profiles declarative: roots, exclusions, providers, budgets, organization rules, retention, and redaction.
- [x] Add profile validation and unknown-provider/config failure states.
- [x] Define host-side audit execution versus Pi-side research consumption.
- [x] Add migration tooling to import historical HADES reports into Rúmil comparison baselines with `legacy_source` provenance.
- [x] Document how a project owner supplies a root, profile, revision, and retention policy.

**Gate:** The same Rúmil coordinator can audit Arda and an unrelated project without source-specific code paths in the core coordinator.

**Evidence:** Five checked-in TOML profiles under `crates/spine/runtime/arda-rumil/profiles/` feed one validated `ProjectProfile` and `audit_with_profile` path. Profiles reject traversal roots, empty/invalid budgets, mutation-capable organization settings, and unknown providers. `ExecutionTarget` keeps full audit execution on the host and rejects host profiles on Pi research consumers. `import_legacy_hades_findings` converts retained HADES JSON into deterministic comparison baselines whose evidence remains `historical` and whose provenance records `legacy_source = "hades"`.

## Explicit non-goals

- No immediate full port of `annunimas-hades` into Arda.
- No broad rename of existing HADES docs, scripts, state, or historical audit artifacts.
- No custom replacement for Cargo, rust-analyzer, Tree-sitter, `cargo-modules`, `cargo-audit`, `cargo-deny`, Git, or language tooling.
- No autonomous file moves, archive, rewrites, or deletion.
- No arbitrary shell execution API.
- No automatic model training or unreviewed policy learning from packets.
- No second durable memory system alongside `arda-vaire`.
- No replacement of Warden research, Mandos reasoning, or Varda evaluation roles.
- No requirement that every project support every provider.

## Verification matrix

Every implementation tranche must run the narrowest relevant checks first, then the crate/consumer matrix:

```bash
cargo metadata --no-deps --format-version 1
cargo test -p arda-rumil --all-features -- --test-threads=1
cargo test -p arda-rumil --no-default-features -- --test-threads=1
cargo clippy -p arda-rumil --all-targets --all-features -- -D warnings
cargo clippy -p arda-rumil --all-targets --no-default-features -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc -p arda-rumil --no-deps
cargo test --manifest-path outposts/arda-outpost-scout/Cargo.toml --all-features -- --test-threads=1
cargo test -p arda-vaire --all-features -- --test-threads=1
cargo test -p arda-mandos --all-features -- --test-threads=1
cargo test -p arda-varda --all-features -- --test-threads=1
cargo test -p arda-engine --all-features -- --test-threads=1
```

Run commands only after the corresponding crate/feature exists. Before broad formatting, preserve the existing dirty set and use scoped formatting/checks.

## Completion criteria

Rúmil is complete for the first reusable release only when:

- [x] Cargo and generic project profiles work through the same coordinator.
- [x] The audit packet explicitly reports complete versus partial coverage.
- [x] Existing analysis tools are used through receipts rather than reimplemented.
- [x] Findings, baselines, and comparisons are deterministic and replay-safe.
- [x] Organization plans are review-only and no-delete by default.
- [x] Historical HADES organization evidence and retained findings can be imported as historical-only metadata/baselines with explicit `legacy_source` provenance.
- [x] Warden can consume a Rúmil packet without gaining execution authority.
- [x] Vairë receives only bounded eligible continuity/receipt data.
- [x] Mandos/Varda can evaluate Rúmil evidence without filesystem access.
- [x] Direct crate tests and crate documentation state are updated from live evidence, including downstream evaluation and HUD projection integration.
