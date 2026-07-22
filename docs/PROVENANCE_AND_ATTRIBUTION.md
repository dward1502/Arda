---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "provenance_registry"
  owner: "HADES"
  status: "draft"
  reviewed: "2026-07-21"
---

> 🜏 Soterion: 📜 provenance_registry | owner: HADES | status: draft | reviewed: 2026-07-21

# Arda Provenance and Attribution

## Purpose

Arda combines original architecture, ideas inspired by external work, adapted design
patterns, incorporated code and assets, conventional engineering practice, and
AI-assisted implementation. This registry records those relationships without
inflating originality claims or erasing upstream contributors.

This document is an attribution process, not a software license. Attribution does not
replace compliance with an upstream license, copyright notice, trademark policy, data
license, model license, or API terms. Arda currently has no repository-root `LICENSE` or
`NOTICE` file; project-wide release licensing therefore remains an explicit unresolved
release task.

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
| `arda-governance` | Mixed: original, adapted, conventional, AI-assisted | [`GOVERNANCE_PROVENANCE.md`](../crates/spine/governance/arda-governance/GOVERNANCE_PROVENANCE.md) | Partial; JouleWork and Love Equation upstream citations required |
| OpenFang-derived patterns | `adapted` | [`plans/OPENFANG.md`](plans/OPENFANG.md), `core/state/openfang_alignment.json` | Source URL recorded; exact version and license review still required |
| Nous Hermes Agent | `inspired` / reference-only according to current absorption brief | `core/state/external_absorption_brief.json` | Source URL recorded; enumerate any implemented adaptations separately |
| Mission Control | `inspired` / reference-only according to current absorption brief | `core/state/external_absorption_brief.json` | Source URL recorded; enumerate any implemented adaptations separately |
| Terminal-Bench patterns | `inspired` or `adapted`, per component | `core/state/external_absorption_brief.json` | Publication URL recorded; implementation-level mapping required |
| Remaining Arda subsystems | `unknown_pending_review` until inventoried | Crate READMEs, plans, state records, git history | Create local records incrementally; do not infer originality from repository ownership |

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

- [ ] Assign an owner for the provenance review of each first-class crate/application.
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
- [ ] Preserve required copyright and notice text verbatim in a future root `NOTICE`.
- [ ] Choose and add a repository-root project license without implying that it
  relicenses third-party material.
- [ ] Add provenance review to crate release gates and documentation indexes.
- [ ] Re-run the audit when an agent imports code, prompts, schemas, datasets, assets, or
  model artifacts from a new source.

## Release rule

A missing citation is not evidence of originality. A component with incomplete source or
license information remains distributable only according to the project's reviewed
release policy and must stay marked `unknown_pending_review` or `incomplete` here until
resolved.
