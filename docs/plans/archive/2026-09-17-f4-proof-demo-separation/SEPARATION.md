---
soterion:
  sigil: "SCROLL"
  glyph: "[scroll]"
  code_point: "U+1F4DC"
  role: "separation"
  owner: "RUMIL"
  status: "completed"
  reviewed: "2026-09-17"
  tags: ["projects", "registry", "fabric", "f4", "separation"]
---

> Soterion: [scroll] | separation | owner: RUMIL | status: completed | reviewed: 2026-09-17

# F4 -- Proof/Demo vs Production Attachment Separation

## Context

The Arda workbench registry contains a mix of proof/demo attachments and actual
production attachments. These need to be separated so that real work is not
confused with test or verification artifacts.

## Current Registry State (2026-09-17, 25 entries)

### Production Attachments (10)

These are projects intended for real work.

#### Real Separate Root (1)
- arda-tool-gate-source-review -- /var/home/mythos/Eregion/Arda-Tool-Gate
  - Real external repository (github.com/dward1502/Arda-tool-gate.git)
  - kind: git-source, authority: read_only

#### Arda Monorepo Crates via Detached Worktree Copies (4)
- arda-agent-loop-contract -- target/arda-real-projects/agent-loop-contract (detached worktree)
- arda-service-registry -- target/arda-real-projects/service-registry (detached worktree)
- arda-human-reviewed -- target/arda-real-projects/human (detached worktree)
- arda-tool-gate-reviewed -- target/arda-real-projects/tool-gate (detached worktree)

These are real crates inside the Arda monorepo, but they are registered in the
workbench as detached worktree copies. Their physical root is the same as the
Arda monorepo (.).

#### Arda Monorepo Root Itself (3)
- arda-rust-example -- . (Arda monorepo root)
- arda-provider-route-audit -- . (Arda monorepo root)
- digital-organism-stage7-living-mesh -- . (Arda monorepo root)

These three entries all point to the same physical root (.) but were
registered for different purposes.

#### Business Applications (3) -- Added in F3
- covercoinc -- /var/home/mythos/Eregion/CoverCoINC
- skylightpros -- /var/home/mythos/Eregion/skylightpros
- wgtt -- /var/home/mythos/Eregion/wgtt

### Proof/Demo Attachments (15)

These are attachments for testing, acceptance testing, and verification. They are
not intended for production work.

#### M4 Acceptance Fixtures (10)
- m4-acceptance-a, m4-acceptance-b
- m4-acceptance-a-v2 through m4-acceptance-b-v5

Root: data/arda/acceptance/m4/project-*
Purpose: fixtures for M4 acceptance testing
kind: acceptance
authority: approval_required, filesystem.write: true (generated fixtures)

#### Autonomous Acceptance (2)
- arda-autonomous-acceptance
- arda-autonomous-acceptance-b

Root: target/arda-autonomous-acceptance-project[-b]
Purpose: fixtures for autonomous acceptance testing
kind: rust (acceptance use)
authority: approval_required

#### Other Fixtures (3)
- s4-c1-fixture -- . (Arda root), Stage 4 test fixture
- installed-retained-local-acceptance -- target/qualification/live-project, local acceptance test fixture
- digital-organism-stage7-living-mesh -- . (Arda root), Stage 7 homeostasis recovery test fixture

## Separation Criteria

### 1. kind field
- Production: kind "rust", "web-app", "git-source", "git-documentation"
- Proof/Demo: kind "acceptance", "fixture"

### 2. Root Path Pattern
- Production:
  - Real separate roots (/var/home/mythos/Eregion/...)
  - Arda monorepo root (.)
  - Detached worktree (target/arda-real-projects/...) -- copies of real crates
- Proof/Demo:
  - data/arda/acceptance/...
  - target/arda-autonomous-acceptance-project*
  - target/qualification/...

### 3. Command/Check Nature
- Production: real build/run/test commands (build, dev, start, lint, test)
- Proof/Demo: verification commands (verify, test) -- not intended for real work

### 4. Authority Pattern
- Production: appropriate authority for the work (approval_required, read_only, etc.)
- Proof/Demo: mostly approval_required, but since they are generated fixtures they must be distinguished from real work attachments

## Current Issues

1. Duplicate entries pointing to the Arda monorepo root: arda-rust-example,
   arda-provider-route-audit, and digital-organism-stage7-living-mesh all
   point to the same physical root (.) but are registered under different
   project_ids. They are different-purpose work items, but from a physical root
   perspective they overlap.

2. Detached worktree copies registered as separate entries: arda-agent-loop-
   contract, arda-service-registry, arda-human-reviewed, and arda-tool-gate-reviewed
   are real crates inside the Arda monorepo, but they are
   registered as detached worktree copies. From a physical root perspective they
   are the same as the Arda monorepo (.).

3. Proof/Demo and Production mixed: of 25 entries, 15 are proof/demo and 10
   are production. Without distinguishing them, real work can be confused with
   test or verification artifacts.

## Recommendations

1. Explicitly classify proof/demo entries: identify proof/demo entries in the
   registry and ensure they are not confused with production work attachments.

2. Clean up duplicate Arda monorepo root entries: arda-rust-example,
   arda-provider-route-audit, and digital-organism-stage7-living-mesh point to
   the same physical root, so the duplication should be cleaned up or the purposes
   should be clearly distinguished.

3. Review detached worktree entries: arda-agent-loop-contract,
   arda-service-registry, arda-human-reviewed, and arda-tool-gate-reviewed are
   detached worktree copies of real crates. From a physical root perspective they
   are the same as the Arda monorepo (.), so they should either be merged into
   entries that point to the real project roots, or clearly labeled as detached
   worktrees.

4. Classify business apps as production: covercoinc, skylightpros, and wgtt are
   production attachments for real business work.
