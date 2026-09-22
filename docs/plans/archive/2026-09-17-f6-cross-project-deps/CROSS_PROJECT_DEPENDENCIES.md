---
soterion:
  sigil: "SCROLL"
  glyph: "MARKER"
  code_point: "U+1F4DC"
  role: "dependencies"
  owner: "RUMIL"
  status: "completed"
  reviewed: "2026-09-17"
  tags: ["projects", "registry", "fabric", "f6", "dependencies"]
---

> Soterion: MARKER | dependencies | owner: RUMIL | status: completed | reviewed: 2026-09-17

# F6 -- Cross-Project Dependencies

## Context

This document records cross-project dependencies within the Arda ecosystem and
the compatibility checks used to validate them. Changes to one project may create
ordered work in another through canonical authority, not cloned code or a
competing queue.

## Arda Monorepo Internal Dependencies

### Core crate dependency graph

Arda-core is the foundation crate. All spine crates depend on it.

Arda-core
  +-- arda-contract-registry
  +-- arda-governance
  +-- arda-mirromere
  +-- arda-orome
  +-- arda-aule
  +-- arda-economics
  +-- arda-mandos
  +-- arda-rumil
  +-- arda-vaire
  +-- arda-varda

Arda-engine (daemon)
  +-- arda-core
  +-- arda-contract-registry
  +-- arda-governance
  +-- arda-mirromere
  +-- arda-orome
  +-- arda-aule
  +-- arda-economics
  +-- arda-mandos
  +-- arda-rumil
  +-- arda-vaire
  +-- arda-varda

### Individual spine crate dependencies

Each spine crate depends on arda-core and nothing else within the spine (as of 2026-09-17):

1. arda-contract-registry -- depends on arda-core
2. arda-governance -- depends on arda-core
3. arda-mirromere -- depends on arda-core
4. arda-orome -- depends on arda-core
5. arda-aule -- depends on arda-core
6. arda-economics -- depends on arda-core
7. arda-mandos -- depends on arda-core
8. arda-rumil -- depends on arda-core
9. arda-vaire -- depends on arda-core
10. arda-varda -- depends on arda-core

### Daemon crate dependencies

Arda-engine is the daemon crate. It depends on all spine crates plus the core
contract and governance crates. It is the runtime that hosts the workbench,
objective runtime, and harness.

## External Project Dependencies

### arda-tool-gate

Arda-tool-gate is an independent crate outside the Arda monorepo. It depends on
arda-core and uses the Manwe inference adapter. It interacts with the Arda
daemon through the Hermes operator bridge.

- Depends on: arda-core
- Uses: Manwe inference adapter
- Interacts with: Arda daemon (via Hermes adapter)

### Business applications

The business applications (covercoinc, wgtt, skylightpros) have no direct
code dependencies on the Arda ecosystem. They are external projects that Arda can
manage through the workbench, but they do not import Arda crates.

## Detached Worktree Project Relationships

The following projects are detached worktree copies of real crates inside the Arda
monorepo:

- arda-agent-loop-contract -- copy of Arda-Agent-Loop-Contract crate
- arda-service-registry -- copy of Arda-Service-Registry crate
- arda-human-reviewed -- copy of Arda-Human crate (documentation project)
- arda-tool-gate-reviewed -- copy of Arda-Tool-Gate crate (documentation project)

These copies have the same source as their canonical counterparts inside the Arda
monorepo. Changes to one should be reflected in the other through canonical authority.

## Compatibility Checks

### Monorepo crate changes

When a crate inside the Arda monorepo changes:

1. Run cargo check to verify the change compiles
2. Run cargo test for the changed crate
3. Run integration tests for the full system if needed

### Business application changes

When a business application changes:

- covercoinc: npm run build, npm run lint
- wgtt: pnpm run build, pnpm run lint
- skylightpros: pnpm run build, pnpm run test, pnpm run lint

### Cross-project validation

When a change in one project may affect another:

1. Identify the dependency relationship
2. Run compatibility checks on the dependent project
3. Record the compatibility result with the change receipt

## Dependency Change Flow

When a dependency changes, canonical authority creates ordered leaves for affected
projects. The flow is:

1. Detect the change
2. Identify affected projects through the dependency graph
3. Create ordered leaves through canonical authority
4. Execute leaves in dependency order
5. Record compatibility results with receipts

This is how a change in arda-core can create ordered work in multiple spine
crates, or how a change in a business application's dependencies can create work
in that application.
