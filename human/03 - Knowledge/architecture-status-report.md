---
sigil: SCROLL
soterion:
  id: architecture-status-report
  version: 1.0.0
  classification: general-document
  author: Aulendil
  created: 2026-03-20
  last_edited: 2026-05-03
  status: active
  domain: general
  tags:
    - documentation
    - general
  mnemosyne:
    lineage: architecture-status-report-doc
    memory_type: general-knowledge
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

---
sigil: SCROLL
soterion:
  id: knowledge-architecture-status-report
  version: 1.0.0
  classification: architecture-status
  author: Aulendil
  created: 2026-03-22
  last_edited: 2026-05-03
  status: active
  domain: architecture
  tags:
    - architecture
    - status-report
    - rust-migration
  references:
    - id: annunimas-architecture-core
      path: /var/home/mythos/Annunimas/crates/annunimas-cli/src/main.rs
    - id: state-contracts
      path: /var/home/mythos/Annunimas/core/state
  mnemosyne:
    lineage: architecture-status-report-2026-03-22
    memory_type: status-report
---

# Current Architecture Status

Date: 2026-03-22

## Executive Summary

Annunimas is now a Rust-first multi-crate autonomy platform with a much clearer control plane than it had before the recent integration pass.

The core improvement is structural:
- the hidden Python and shell sprawl has been pulled inward
- `annunimas-cli` is now the main operator and orchestration surface
- service crates own more of the real runtime behavior
- `core/state` remains the shared machine-readable contract layer
- `data/` remains the runtime evidence and ledger layer

This is materially better than the earlier shape where significant behavior lived in scattered script entrypoints.

That said, the architecture is not yet fully decomposed. Some of the old script complexity has been absorbed into large Rust modules rather than fully split into smaller domain libraries.

## Current Topology

At the highest level, the system now looks like this:

1. Rust workspace in `crates/`
2. CLI control plane in `crates/annunimas-cli`
3. Domain services as separate crates
4. Shared state contracts in `core/state`
5. Runtime and evidence surfaces in `data/`
6. Product and interface satellites in `apps/`

The workspace is defined in [Cargo.toml](/var/home/mythos/Annunimas/Cargo.toml) and includes these major services:
- `annunimas-athena`
- `annunimas-charon`
- `annunimas-hermes`
- `annunimas-hades`
- `annunimas-mnemosyne`
- `annunimas-prometheus`
- `annunimas-apollo`
- `annunimas-plutus`
- `annunimas-oracle`
- supporting crates such as `annunimas-governance`, `annunimas-fleet`, `annunimas-core`, `annunimas-warden`

## What Improved In This Pass

### 1. CLI-Centered Authority

The main operational center is now `annunimas-cli`.

Instead of hiding logic behind dozens of Python entrypoints, the CLI now owns:
- export generation
- control and governance actions
- maintenance flows
- pipeline orchestration
- utility surfaces
- subsystem command families

Key files:
- [main.rs](/var/home/mythos/Annunimas/crates/annunimas-cli/src/main.rs)
- [commands/mod.rs](/var/home/mythos/Annunimas/crates/annunimas-cli/src/commands/mod.rs)
- [export_surface.rs](/var/home/mythos/Annunimas/crates/annunimas-cli/src/export_surface.rs)
- [policy_guard.rs](/var/home/mythos/Annunimas/crates/annunimas-cli/src/policy_guard.rs)
- [observability.rs](/var/home/mythos/Annunimas/crates/annunimas-cli/src/observability.rs)
- [support.rs](/var/home/mythos/Annunimas/crates/annunimas-cli/src/support.rs)
- [ipc_bridge.rs](/var/home/mythos/Annunimas/crates/annunimas-cli/src/ipc_bridge.rs)

### 2. Script Forest Collapsed

The repo now has zero `.py` files.

This is important not because Python is bad, but because too much living system behavior was hidden in one-off script entrypoints. That made autonomy fragile, hard to reason about, and hard to test.

Now:
- core exports live behind `annunimas-cli export ...`
- runtime governance lives behind `annunimas-cli control ...`
- task pipelines live behind `annunimas-cli pipeline ...`
- utility flows live behind `annunimas-cli utility ...`

### 3. ATHENA Front Door Is Cleaner

ATHENA ingest behavior is more visible now.

The human-ingest system that had been split across Python utilities now lives in the CLI and service layer:
- `athena human-corpus-wave`
- `athena human-document-wave`
- Scrapling fetch flow now routes through Rust instead of a Python shim script

This makes ATHENA more credible as a subsystem rather than a bundle of helper scripts.

### 4. Product/UI Surfaces Are More Satellite-Like

The app/demo layers in `apps/` are still meaningful, but they are less of a hidden backend now.

The remaining app helpers were converted to shell entrypoints, which keeps them separate from the core runtime without pretending they are part of the domain service mesh.

## Current Strengths

### State-Contract Model

One of the strongest parts of Annunimas is the use of `core/state` as a shared contract surface.

That gives the system:
- inspectable outputs
- stable handoff points between agents and UI
- easier HUD integration
- a machine-readable governance surface

This is one of the main reasons the system can plausibly evolve into something more OS-like later.

### Service Identity Is Real

The named subsystems now map to real areas of responsibility:
- ATHENA: ingest, research, knowledge absorption
- CHARON: routing, provider selection, local and edge execution choice
- HERMES: communications, boardroom, external/operator-facing delivery
- HADES: storage, destructive action boundaries, persistence cleanup
- MNEMOSYNE: continuity, memory, recall surfaces
- PROMETHEUS: orchestration, maintenance, operational supervision
- APOLLO: execution flow and task handling
- PLUTUS: spend and joulework accounting
- ORACLE: evaluation and verdicting

That is a strong conceptual base.

### Governance Is Not Bolted On

Governance is present as an architectural concern, not just documentation.

Files such as [policy_guard.rs](/var/home/mythos/Annunimas/crates/annunimas-cli/src/policy_guard.rs) and the ruleset/state surfaces in `core/state` show that destructive actions, permissions, and runtime policy are meant to be machine-enforced, not just manually respected.

That matters for any future agentic operating environment.

## Current Weaknesses

### 1. Large Rust Monoliths Still Exist

Several files are still too large and too responsibility-dense.

Notable examples:
- [core_link.rs](/var/home/mythos/Annunimas/crates/annunimas-prometheus/src/core_link.rs): about 7248 lines
- [ingest.rs](/var/home/mythos/Annunimas/crates/annunimas-athena/src/ingest.rs): about 4253 lines
- [athena_exports.rs](/var/home/mythos/Annunimas/crates/annunimas-cli/src/export_surface/athena_exports.rs): about 3684 lines
- [control.rs](/var/home/mythos/Annunimas/crates/annunimas-cli/src/commands/control.rs): about 2403 lines
- [main.rs](/var/home/mythos/Annunimas/crates/annunimas-cli/src/main.rs): about 1724 lines

This means the architecture is improved, but still not fully decomposed.

### 2. The CLI Is Becoming a Secondary Monolith

The CLI is now the right authority surface, but it is carrying a lot:
- dispatch
- policy
- exports
- utility tooling
- pipeline orchestration
- control operations

That is acceptable for now, but long term more logic should move out of CLI-only modules and into reusable domain crates or libraries.

### 3. Export Logic Is Still Too Concentrated

The export family is much cleaner than before, but it is still heavy.

There are now good domain submodules under `crates/annunimas-cli/src/export_surface/`, but this area still acts as a large projection factory. It would be healthier if more exports became:
- domain-owned library functions
- smaller projection crates
- or service-owned contract emitters

### 4. Some Product Boundaries Are Not Final Yet

AIPKG, CITADEL-adjacent tools, and some edge-worker surfaces are visible product seams, but they are not fully separated yet.

That is not a failure. It just means the architecture is in a transitional state between:
- one sovereign repo with many capabilities
- and a future with clearer productized boundaries

## Honest Assessment

Annunimas is no longer shaped like a research prototype held together by scripts.

It is now much closer to:
- a Rust service mesh
- a state-contract-driven autonomy layer
- a CLI-centered control plane
- a ledgered runtime with inspectable evidence

That is a serious step forward.

The honest caveat is that some of the old complexity did not disappear. It moved inward. The work ahead is to keep decomposing the large Rust files so that the new architecture does not become a cleaner-looking version of the old sprawl.

## Can This Become An Agentic Operating System Under Bluefin?

Yes, in a realistic sense.

No, if the phrase means “replace Linux or become a kernel-level operating system.”

Yes, if the phrase means:
- run on top of an immutable Linux base such as Bluefin
- own the agentic control plane above the host OS
- manage task routing, local/edge execution, governance, memory, observability, and operator-facing control
- expose applications and specialized services as bounded subsystems

That second version is plausible.

In that model:
- Bluefin provides the stable host substrate
- systemd, containers, sockets, and the immutable base provide reliability
- Annunimas becomes the sovereign cognitive and operational layer above that substrate

That is not far-fetched. It is probably the right framing.

The architecture already has several ingredients needed for that direction:
- service decomposition
- machine-readable state contracts
- governance surfaces
- local and edge routing concepts
- observability and maintenance flows
- product/UI satellites that can consume the same shared state

What is still needed before that vision feels fully credible:
- tighter service lifecycle management
- more explicit system bootstrap and reconciliation contracts
- better decomposition of oversized Rust modules
- stronger packaging and upgrade paths
- clearer separation between source code, generated state, and operator data
- a more formal host integration story for Bluefin or another immutable Linux base

## Recommended Next Moves

1. Split large service files by concern
   - especially PROMETHEUS `core_link.rs`
   - ATHENA `ingest.rs`
   - CLI `control.rs`

2. Reduce CLI-only logic
   - move reusable logic into service crates or small support libraries

3. Treat AIPKG as a candidate standalone product boundary
   - it already has the beginnings of a real seam

4. Formalize the “Annunimas on Bluefin” architecture
   - define which responsibilities belong to the host OS
   - define which belong to Annunimas
   - define what systemd, containers, sockets, and state directories should own

5. Continue converging on reproducible bootstrap flows
   - so the system can be installed, repaired, and promoted like a platform rather than managed like a handcrafted workspace

## Bottom Line

The recent integration pass made Annunimas much more architecturally real.

It now has:
- a clearer control plane
- stronger subsystem boundaries
- better state and evidence surfaces
- far less hidden behavior

It is not yet a finished agentic operating environment, but it is absolutely on a believable path toward becoming an agentic control layer running on top of a Linux base such as Bluefin.
