# Arda + Annunimas Environment Assessment

Purpose: simplify, reduce repetition, professionalize, and document the
current state so structural cleanup can proceed without guessing.

## Current reality

### Repo roots
- `/var/home/mythos/Eregion/Arda`
  - Rust workspace with 29 flat `annunimas-*` vendored crates.
  - Central entrypoint: `src/main.rs` → `arda_engine`.
  - Frontend: `apps/arda-launcher`.
- `/var/home/mythos/Annunimas`
  - Older Annunimas workspace with same crate names under `crates/`.
  - Extra app surfaces: `apps/citadel-companion`, `apps/first-light`, `apps/onboarding-console`.
  - Heavy non-code directories: `audit/`, `data/`, `docs/`, `human/`, `scripts/`, `spec/`, `tests/`, `tmp/`.

### Crate duplication
Arda and Annunimas both mirror the same vendored crate tree. This is the main
redundancy to resolve. Example overlap:
- Same crate name, same README size, same `Cargo.toml` shape under both roots.
- Arda's `Cargo.toml` explicitly says ` Vendored Annunimas crates (flat siblings)`.

### Top-level clutter in Annunimas
- Hundreds of dated audit folders, many with README stubs.
- Many tiny README files that add navigation noise without useful content.
- Mixed operational directories that should not live in the repo root.

### Documentation redundancy
- README count:
  - Arda: ~50 README files including many trivial stubs.
  - Annunimas: ~85 README files including `data/*`, `human/*`, `audit/*`, and `scripts/*`.
- Docs overlap: Annunimas `docs/arda/` duplicates Arda top-level docs.
- Multiple onboarding surfaces across apps and crates.

## Recommended direction

1. Keep `arda` as the single public repo for the platform.
2. Stop carrying full duplicate copies of vendored crates; instead use one of:
   - subtree/patch dependency from a single Annunimas repo
   - or namespace them under a shared `vendor/` area
3. Slim non-code directories in Annunimas to operational data paths only.
4. Collapse redundant README trees into one docs structure with links, not copies.
5. Use domain groups from `docs/DOMAIN_STRUCTURE.md` to organize crates.

## Priority actions

1. Declare Arda as canonical.
2. Remove `crates/annunimas-*` flat duplication in Annunimas when trust in Arda's vendored copies matures.
3. Remove dated README stubs in `audit/`, `data/`, `scripts/*`.
4. Replace duplicated `docs/` trees with one structured docs folder per repo.

## Files created during this assessment
- `docs/DOMAIN_STRUCTURE.md`
- `docs/ENVIRONMENT_ASSESSMENT.md`
