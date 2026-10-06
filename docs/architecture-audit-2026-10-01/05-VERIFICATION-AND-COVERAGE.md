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

# Verification and coverage

## Executed checks

All commands ran against the dirty working tree, not a committed release. No repair was applied.

| Check | Result | Retained evidence |
|---|---|---|
| cargo metadata --no-deps --format-version 1 --locked | 19 root-workspace packages enumerated | [Package data](workspace-map.json) |
| cargo check -p arda --locked --offline -j 2 | Exit 0; unused production-build helper warning | [Root check](evidence/root-check.log) |
| cargo check --workspace --all-targets --locked --offline -j 2 | Confirmatory run exit 101; E0063 missing excluded_refs in Vaire test initializer | [Workspace check](evidence/workspace-check.log) |
| cargo test -p arda-contract-registry -p arda-project-adapter-sdk -p arda-relic-bridge --locked --offline -j 2 | Exit 0; 17 tests passed, no failures | [Targeted tests](evidence/targeted-tests.log) |
| pnpm --dir apps/arda-mirromere test | Exit 0; one test file, six tests passed | [Frontend tests](evidence/mirromere-tests.log) |
| python scripts/rumil_markdown_link_check.py --root docs/plans --out <scratch-report> --check-completion-language | 80 local links checked; four broken; zero completion-language issues | [Plan health](evidence/active-plan-health.txt) |

The Cargo probes were bounded with timeout and two jobs. The confirmatory workspace check produced an actual compiler failure, not a timeout. Passing targeted tests do not establish full-system reliability, native acceptance or sufficient test coverage. The full test suite, all-feature matrix and native apps were not built/tested end to end.

## Coverage levels

| Area | What this pass establishes | What it does not |
|---|---|---|
| Files and directories | Full tracked/nonignored path inventory; top-level local entries; sizes and selected-extension line counts | Every ignored descendant, binary semantics, private-record review |
| Root Rust workspace | Membership, targets, declared internal dependencies, sampled entry-point source for each package | Complete call graph or behavior review of every module |
| Architecture seams | Selected Engine/Aule, research, memory, UI bootstrap and compatibility paths read | Every side-effect owner, concurrency path or authorization boundary verified |
| Large files | Ranked review list with test-boundary navigation hints | Line-by-line audit of each large file |
| Unused material | Evidence-based candidates and ruled-out false positives | A proof that any substantive source tree is safe to delete |
| Tests | Root compilation, reproduced all-target blocker, small targeted suites | Coverage percentage, fuzzing, load/soak, full regression pass |
| CI/docs | Local workflow definitions and scoped active-plan link check | Remote required checks or all historical document links |
| Runtime/product | Source composition evidence | Installed binary identity, live service health, devices, authenticated actions or operator acceptance |
| Security | Selected design/ownership concerns | Penetration test, exhaustive secret/license/dependency audit |

## Reproduction

Run the exact commands above from the repository root. inventory-generator.py regenerates the three JSON/CSV inventory snapshots using local Cargo metadata; it does not regenerate the authored assessments or directory/large-file reports. Re-running it updates the snapshot and can make these documents stale. Prefer a newly dated assessment when the working tree changes materially.

The source-files/source-lines columns count a fixed extension set documented in the generator. They include tests and comments and exclude vendor from the headline totals. Package counts are not a claim that every package is used at runtime. Declared optional dependencies are not active-feature edges.

No code, configuration or existing plan was intentionally edited by the audit. Commands can populate build caches; pre-existing dirty source and runtime files were preserved. The audit folder is untracked until deliberately added; git diff --check alone cannot validate untracked documents, so the audit files also receive direct whitespace/link/content checks.
