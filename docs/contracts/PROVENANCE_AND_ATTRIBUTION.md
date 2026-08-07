---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "provenance_registry"
  owner: "arda-rumil"
  status: "active_review"
  reviewed: "2026-08-05"
---

> 🜏 Soterion: 📜 provenance_registry | owner: arda-rumil | status: active_review | reviewed: 2026-08-05

# Arda Provenance and Attribution

## Purpose

Arda combines original architecture, ideas inspired by external work, adapted design
patterns, incorporated code and assets, conventional engineering practice, and
AI-assisted implementation. This registry records those relationships without
inflating originality claims or erasing upstream contributors.

This document is an attribution process, not a software license. Attribution does not
replace compliance with an upstream license, copyright notice, trademark policy, data
license, model license, or API terms. Arda's repository-root [`LICENSE`](../../LICENSE)
declares MIT and the workspace manifests resolve first-party Rust packages to `MIT`.
The repository-root [`NOTICE`](../../NOTICE) preserves the currently verified
project-level attribution boundary. Dependency and incorporated-artifact license review
remains a release gate rather than being implied by the project license.

## Classification vocabulary

Use exactly one primary classification for each contribution and add secondary notes
when needed.

| Classification | Meaning | Minimum record |
|---|---|---|
| `original` | The project creator independently originated the identifiable concept or architecture. | Originator, first evidence/date if known, Arda implementation paths |
| `inspired` | External work influenced direction, vocabulary, or goals, but its implementation or detailed design was not copied. | Creator/title, stable URL or publication, influence statement |
| `adapted` | Arda deliberately transformed an external equation, algorithm, schema, workflow, or design pattern. | Exact source/version, what was retained, what changed, license review |
| `incorporated` | External code, text, assets, data, model weights, or generated artifact are included or distributed. | Exact source/version/hash, copied paths, license, required notices |
| `dependency` | Arda calls or links a third-party package/service without copying it into the repository. | Package/service and version, manifest path, license/terms reference |
| `conventional` | Common engineering practice with no specific known source. | Short rationale; do not manufacture a citation |
| `ai-assisted` | An AI development tool materially drafted, transformed, or reviewed an artifact under human direction. | Tool/model if recoverable, task/session/commit evidence, human review status |
| `unknown_pending_review` | Provenance is not yet known well enough to classify honestly. | Owner and concrete follow-up; never silently promote to `original` |

“Taken and used” must be replaced by either `adapted`, `incorporated`, or `dependency`.
Those categories have materially different notice and licensing obligations.

## Required attribution record

Each first-class subsystem should keep a local `*_PROVENANCE.md` or `PROVENANCE.md`
containing this minimum record per concept or artifact:

```text
Name:
Classification:
External creator(s) or originator:
Source title:
Canonical source URL / DOI / repository:
Source version, publication date, commit, or artifact hash:
Upstream license or terms:
Arda paths using it:
What Arda retained:
What Arda changed or added:
AI-assistance evidence, if known:
Human reviewer and review date:
Confidence: confirmed | probable | incomplete
Required follow-up:
```

If a source cannot yet be recalled, write `unknown_pending_review`; do not guess the
person, paper, URL, or license.

## System registry

| Subsystem or source | Current classification | Local evidence | Attribution state |
|---|---|---|---|
| `arda-governance` | Mixed: original, inspired, adapted, conventional, dependency, AI-assisted | [`GOVERNANCE_PROVENANCE.md`](../../crates/spine/governance/arda-governance/GOVERNANCE_PROVENANCE.md) | Exact JouleWork/Love Dynamics sources and boundaries recorded; Read Multiplex legal review remains required before public distribution |
| Arda Prometheus projection exporter | `adapted` | [`metrics_exporter.rs`](../../crates/spine/observability/arda-aule/src/cli/metrics_exporter.rs), adapted from `Annunimas/crates/annunimas-cli/src/commands/metrics.rs` at `c0b91edcdf777338b1eb42945b2adb4a3eaf6d7d` | Preserves the upstream MIT notice and metric compatibility names; Arda adds an explicit runtime root and `arda-cli` integration |
| OpenFang-derived patterns | `adapted` | [`../archive/OPENFANG.md`](../archive/OPENFANG.md), `core/state/openfang_alignment.json` | Source URL recorded; exact version and license review still required |
| Nous Hermes Agent | `inspired` / reference-only according to current absorption brief | `core/state/external_absorption_brief.json` | Source URL recorded; enumerate any implemented adaptations separately |
| Mission Control | `inspired` / reference-only according to current absorption brief | `core/state/external_absorption_brief.json` | Source URL recorded; enumerate any implemented adaptations separately |
| Terminal-Bench patterns | `inspired` or `adapted`, per component | `core/state/external_absorption_brief.json` | Publication URL recorded; implementation-level mapping required |
| Remaining Arda subsystems | `unknown_pending_review` until inventoried | Crate READMEs, plans, state records, git history | Create local records incrementally; do not infer originality from repository ownership |

## First-class review ownership

`arda-rumil` owns provenance review and registry closure for every first-class
crate/application below. This is review ownership, not a claim that Rumil authored each
component or can approve legal exceptions. A component steward may supply source evidence;
the release maintainer owns the final distribution decision.

| First-class crate or application | Live manifest/surface | Provenance review owner | Current state |
|---|---|---|---|
| Arda root CLI | `Cargo.toml` | `arda-rumil` | Inventory pending |
| Workbench / engine | `crates/engine/Cargo.toml` | `arda-rumil` | Inventory pending |
| Contract registry | `crates/spine/contract/arda-contract-registry/Cargo.toml` | `arda-rumil` | Inventory pending |
| Manwe | `crates/spine/runtime/manwe/Cargo.toml` | `arda-rumil` | Inventory pending |
| Arda Core | `crates/spine/governance/arda-core/Cargo.toml` | `arda-rumil` | Inventory pending |
| Arda Governance | `crates/spine/governance/arda-governance/Cargo.toml` | `arda-rumil` | Component registry active |
| Orome | `crates/spine/interface/arda-orome/Cargo.toml` | `arda-rumil` | Inventory pending |
| Aule | `crates/spine/observability/arda-aule/Cargo.toml` | `arda-rumil` | Prometheus adaptation recorded; full inventory pending |
| Economics | `crates/spine/runtime/arda-economics/Cargo.toml` | `arda-rumil` | Inventory pending |
| Mandos | `crates/spine/runtime/arda-mandos/Cargo.toml` | `arda-rumil` | Inventory pending |
| Rumil runtime | `crates/spine/runtime/arda-rumil/Cargo.toml` | `arda-rumil` | Inventory pending |
| Vaire | `crates/spine/memory/arda-vaire/Cargo.toml` | `arda-rumil` | Inventory pending |
| Varda | `crates/spine/executors/arda-varda/Cargo.toml` | `arda-rumil` | Inventory pending |
| Launcher | `apps/arda-launcher/package.json`, `apps/arda-launcher/src-tauri/Cargo.toml` | `arda-rumil` | Dependency inventory checked; component inventory pending |
| HUD | `apps/arda-hud/package.json`, `apps/arda-hud/src-tauri/Cargo.toml` | `arda-rumil` | Dependency inventory checked; component inventory pending |
| Outpost protocol | `outposts/arda-outpost-protocol/Cargo.toml` | `arda-rumil` | Inventory pending |
| Outpost scout | `outposts/arda-outpost-scout/Cargo.toml` | `arda-rumil` | Inventory pending |
| RELIC bridge | `outposts/arda-relic-bridge/Cargo.toml` | `arda-rumil` | Inventory pending |
| Rust adapter SDK | `sdk/rust/Cargo.toml` | `arda-rumil` | Inventory pending |

The Rust workspace metadata check on 2026-08-05 resolved all 18 workspace packages to
`MIT`; `cargo deny check licenses` passed. Production JavaScript dependency scans passed
for Launcher with no unknown classifications. HUD's package scan initially classified
`khroma 2.1.0` as unknown because the published package omits both its license field and
license file. The exact upstream `v2.1.0` tag resolves to commit
`4968165afb0d3d09be66497e7985a34f7bfe6d42`; its `license` blob
`433a5ecd4cb7c687f6a6ec968da740d490b1f776` declares MIT and requires preservation
of the copyright and permission notice. That text is preserved verbatim in the root
`NOTICE`. This is a reviewed metadata clarification, not permission to infer licenses for
other packages with missing metadata.

The registry records current evidence, not a permanent legal conclusion. A component can
move from `unknown_pending_review` to a more precise class only after its source and use
have been checked.

## Agentic-development attribution

AI-assisted development changes the evidence available, but it does not remove the need
to credit human and external sources.

For each material agent-authored change:

1. Preserve the human intent or task that directed the work.
2. Record the agent/tool and model when recoverable from session or commit metadata.
3. Record the files or symbols materially generated or transformed.
4. Require human review for provenance claims and external-source mappings.
5. Search generated text and code for recognizable copied passages before release.
6. Treat an agent's unsupported claim that something is “original” as unverified.
7. If an agent consulted a repository, paper, notebook, or web page, record that source
   even when the final code is rewritten.
8. Keep dependency license scanning separate from design/concept attribution; both are
   required.

A practical commit or review note is:

```text
AI-assisted: <tool/model or unknown>
Human direction: <task/plan>
Material paths: <paths>
External sources consulted: <URLs/DOIs/repos or none recorded>
Human-reviewed: <name/date/status>
```

## Repository accreditation workflow

- [x] Assign an owner for the provenance review of each first-class crate/application.
- [ ] Search plans, state records, source comments, dependency manifests, and git history
  for explicit external sources.
- [ ] Ask the project creator to resolve only the ambiguous conceptual origins that the
  repository cannot establish.
- [ ] Classify each material contribution with the vocabulary above.
- [ ] Add exact source identity and version; a project name without a URL/version is not
  sufficient for adapted or incorporated material.
- [ ] Record what Arda changed so attribution does not understate Arda's integration work.
- [ ] Verify licenses for adapted/incorporated artifacts and dependencies before public
  distribution.
- [x] Add a root `NOTICE` for the currently verified project-level attribution boundary;
  add upstream notice text only after its exact requirement is verified.
- [x] Confirm the repository-root MIT project license without implying that it relicenses
  third-party material.
- [x] Add the registry to the documentation index and enforce its root files and
  first-class ownership inventory in the tag-bound release workflow.
- [ ] Re-run the audit when an agent imports code, prompts, schemas, datasets, assets, or
  model artifacts from a new source.

## Release rule

A missing citation is not evidence of originality. A component with incomplete source or
license information remains distributable only according to the project's reviewed
release policy and must stay marked `unknown_pending_review` or `incomplete` here until
resolved.
