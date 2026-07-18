---
sigil: SCROLL
soterion:
  id: bluefin-integration
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
    lineage: bluefin-integration-doc
    memory_type: general-knowledge
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

---
sigil: SCROLL
soterion:
  id: annunimas-bluefin-integration
  version: 1.0.0
  classification: architecture-integration
  author: Aulendil
  created: 2026-03-22
  last_edited: 2026-05-03
  status: active
  domain: architecture
  tags:
    - annunimas
    - bluefin
    - integration
    - host-platform
  mnemosyne:
    lineage: annunimas-bluefin-integration
    memory_type: architecture-plan
---

# Annunimas On Bluefin Plan

## Purpose

Define a realistic path for Annunimas to become an agentic control layer running on top of a Bluefin-based Linux host.

This document does not treat Annunimas as a kernel replacement or a new Linux distribution.

The intended model is:
- Bluefin provides the host OS substrate
- systemd, containers, sockets, filesystems, and the immutable base provide operational stability
- Annunimas provides the agentic control plane above that substrate

## Core Framing

## Core Framing

The right question is not:
- can Annunimas become an operating system

The right question is:
- can Annunimas become the sovereign agentic control layer for a host operating system

For this repo, the answer is yes, with more work.

That means Annunimas should eventually own:
- task routing
- local and edge execution selection
- operator interaction
- governance and destructive-action controls
- memory and continuity
- observability and maintenance
- bounded product surfaces built on shared system state

Bluefin should continue to own:
- kernel and drivers
- immutable base system
- host package and image lifecycle
- user session and desktop
- base networking primitives
- container and toolbox substrate
- systemd service supervision foundation

## Why Bluefin Is A Good Fit

Bluefin is a strong substrate for this direction because it is:
- immutable by default
- stable under upgrades
- container-forward
- systemd-native
- good for reproducible host baselines
- well-suited to “thin host, rich user-space platform” designs

Annunimas benefits from exactly that kind of host because it reduces the number of uncontrolled moving parts below the autonomy layer.

In practical terms:
- Bluefin reduces host drift
- Annunimas can focus on runtime cognition, control, and orchestration
- local model services can live in containers, user services, or dedicated host processes without redefining the base OS

## Target Layering

### Layer 0: Host OS

Owned by Bluefin:
- kernel
- GPU and hardware drivers
- immutable base image
- rpm-ostree or image update lifecycle
- base networking and firewall
- user accounts and login session
- storage mounts and permissions

Annunimas should not try to replace this layer.

### Layer 1: Host Runtime Substrate

Owned mostly by systemd plus container/tooling ecosystem:
- user services
- system services
- podman or distrobox/toolbox surfaces
- local model servers
- Tailscale
- local IPC socket directories
- service logs and restart policy

Annunimas should integrate with this layer, not bypass it.

### Layer 2: Annunimas Core Platform

This is the layer the repo is already approaching.

Owned by Annunimas:
- CLI control plane
- service daemons
- policy enforcement
- task orchestration
- provider routing
- memory and continuity
- runtime maintenance
- exports and state contracts

Current components that fit here:
- `annunimas-cli`
- `annunimas-athena`
- `annunimas-charon`
- `annunimas-hermes`
- `annunimas-hades`
- `annunimas-mnemosyne`
- `annunimas-prometheus`
- `annunimas-apollo`
- `annunimas-plutus`
- `annunimas-oracle`
- governance and fleet support crates

### Layer 3: Shared Contract Surfaces

This is where Annunimas becomes inspectable and UI-friendly.

Owned by Annunimas:
- `core/state`
- selected `core/metrics/by_crate/*`
- key machine-readable receipts and ledgers in `data/`

This layer is extremely important. It is what allows:
- HUDs
- dashboards
- external adapters
- operator tools
- product satellites

to consume the same system reality.

### Layer 4: Product and Interface Surfaces

Examples:
- ARDA HUD
- CITADEL avatar
- future AIPKG product surface
- edge and fleet control tools

These should increasingly behave like clients or satellites of the core platform, not alternate backends.

## Current Readiness

### What Already Supports The Vision

Annunimas already has the following ingredients:

1. Service decomposition
- real subsystem identities exist

2. CLI-centered control
- `annunimas-cli` is now the practical operator surface

3. State-contract discipline
- `core/state` is already functioning as a shared machine-readable contract layer

4. Governance surfaces
- the system already thinks in terms of permission, quorum, rulesets, and runtime constraints

5. Local plus edge concepts
- routing, fleet state, remote workers, and edge orchestration already exist conceptually and operationally

6. Observability and receipts
- the platform already records and exports machine-readable traces of what it is doing

These are exactly the ingredients you would want in an agentic control layer.

### What Is Not Ready Yet

The following areas still need hardening before this feels like a true platform:

1. Oversized modules
- there are still major concentration points in large Rust files

2. Service lifecycle formalization
- startup, readiness, degraded mode, and recovery flows still need a cleaner platform contract

3. Packaging and distribution
- the repo is closer to a sovereign workspace than a polished installable platform

4. Host integration story
- Bluefin-specific service layout, data paths, secrets handling, and update model are not fully formalized yet

5. Product boundary cleanup
- some surfaces are visible seams but are not fully extracted or owned as distinct products

## Recommended Host Contract

If Annunimas runs on Bluefin, the host contract should look roughly like this.

### Annunimas Should Assume

- user-writable home directory
- systemd user service support
- Podman or equivalent available
- Tailscale optional but supported
- local GPU runtime available when model services need it
- predictable runtime directories under:
  - `~/.config/annunimas`
  - `~/.local/state/annunimas`
  - `~/.cache/annunimas`
  - or repo-local equivalents during development

### Annunimas Should Not Assume

- mutable root filesystem
- direct package installation into the host base
- hand-edited long-lived host state everywhere
- uncontrolled ad hoc background processes

### Bluefin Integration Model

The preferred production posture is:
- Annunimas binaries in user space
- model runtimes either local user services or containers
- systemd user units for Annunimas daemons and supervisors
- explicit config and state roots
- explicit upgrade story for binaries and rulesets

## Recommended Filesystem Model

The repo already hints at a useful split, but it should become more formal.

### Source-Owned

- `crates/`
- `apps/`
- `docs/`
- `config/*.example`
- `spec/`

### Machine-Generated Shared Contracts

- `core/state/`
- selected `core/metrics/`

### Runtime / Evidence / Mutable State

- `data/`

### Operator / Human Knowledge

- `human/`

For Bluefin, this should eventually be mapped into install/runtime directories explicitly rather than relying on “current repo checkout as universe.”

## Service Model On Bluefin

The target operational pattern should be:

1. Host boots into Bluefin
2. systemd user services start Annunimas core services
3. model providers and helpers start or are probed
4. Annunimas runs a preflight and reconciliation pass
5. state contracts are refreshed
6. UI/product satellites attach to the resulting system state

This suggests a future service set such as:
- `annunimas-charon.service`
- `annunimas-prometheus.service`
- `annunimas-hermes.service`
- `annunimas-mnemosyne.service`
- optional `annunimas-athena.service`
- `annunimas-supervisor.service`
- optional local model service units

## Dataflow Model

The intended platform dataflow should be:

1. Operator or automation submits intent
2. CLI or service validates policy
3. CHARON selects route and provider posture
4. Domain service executes work
5. PROMETHEUS supervises and records operational state
6. HADES / MNEMOSYNE / ATHENA persist relevant evidence or memory
7. `core/state` updates projections
8. HUDs and product surfaces read the same projections

This is already partially true in the current system. The next step is to make it more uniform and less file-specific.

## Architectural Risks

### 1. CLI Gravity

The CLI is becoming the control plane, which is good.

But if too much logic remains CLI-local, Annunimas risks becoming:
- one big command router
- plus many semi-thin services

That is not ideal for a long-term platform.

### 2. Projection Bloat

Exports are powerful, but too many projections can become a parallel architecture.

The goal should be:
- projections as bounded system contracts
- not projections as the only place where business logic becomes visible

### 3. Too Much Repo-Local Assumption

Right now many flows still assume:
- full repo checkout
- direct shell access
- development-style execution

That is fine during active construction, but not ideal for a future Bluefin-hosted platform product.

### 4. Mixed Runtime Ownership

Some runtime concerns are still split between:
- service crates
- CLI helpers
- shell wrappers
- app-side wrappers

That needs to become cleaner over time.

## Suggested Roadmap

### Phase 1: Platform Hardening

Goal:
- reduce large-file concentration
- clarify service ownership
- reduce CLI-only logic

Concrete targets:
- split `annunimas-prometheus/src/core_link.rs`
- split `annunimas-athena/src/ingest.rs`
- split `annunimas-cli` control/export concentration further

### Phase 2: Bluefin Runtime Contract

Goal:
- define how Annunimas lives on an immutable Linux host

Concrete targets:
- define runtime directories
- define secrets loading contract
- define systemd service topology
- define local model hosting pattern
- define upgrade and rollback story

### Phase 3: Installable Platform Posture

Goal:
- move from sovereign dev workspace to reproducible platform deployment

Concrete targets:
- packaging or release artifacts
- service manifests
- bootstrap and doctor flows that do not depend on ad hoc local context
- clean host integration docs

### Phase 4: Product Surface Extraction

Goal:
- let selected subsystems graduate into clearer products

Most obvious candidates:
- AIPKG
- edge/fleet control surface
- CITADEL-style embodied interface surface

## What “Success” Looks Like

Annunimas on Bluefin is successful when:
- the host image is boring and stable
- Annunimas services start predictably
- local and edge execution routes are machine-controlled
- governance is enforced by system logic, not memory
- state contracts are refreshed consistently
- HUDs and operator clients do not need hidden script glue
- upgrades do not require manual repo archaeology

## Bottom Line

The idea is not too far-fetched.

It is actually a good fit, provided the framing stays disciplined:
- Bluefin is the host substrate
- Annunimas is the agentic control layer above it

The current repo is not fully there yet, but it now has enough of the right architectural pieces that the direction is credible.

The next work is not fantasy work. It is platform engineering:
- decomposition
- lifecycle definition
- packaging
- host contracts
- operational hardening
