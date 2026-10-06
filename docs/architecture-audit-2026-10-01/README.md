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

# Arda architecture audit - first-pass landscape

## Start here

For the installed application and the operator's intended multi-worker research/library experience, start with [Live experience and intended use](07-LIVE-EXPERIENCE-AND-INTENDED-USE.md). That supplement includes native screenshots, workstation readbacks, provider failures and an executed local Manwe inference task. It supersedes any suggestion that the static inventory established practical usefulness or justified shelving the project.

This is the requested lay-of-the-land assessment, saved separately from existing plans. It combines a comprehensive tracked/nonignored path inventory with sampled architectural source inspection and bounded executed checks. It is not a claim that every line, workflow or security boundary has been audited.

My assessment: Arda has real shared foundations and connections. The main concern is not a total absence of architecture; it is accumulated responsibility overlap, legacy machinery, mutable state and multiple user/build surfaces without equally clear proof of a coherent installed experience. A rewrite is not justified by this pass.

## Reading order

1. [Landscape and every top-level directory](01-LANDSCAPE.md)
2. [Architecture judgment and findings](02-ARCHITECTURE-AND-FINDINGS.md)
3. [All 19 root-workspace packages](03-PACKAGE-MAP.md)
4. [Large files and retirement triage](04-LARGE-FILES-AND-RETIREMENT.md)
5. [Executed checks and honest coverage limits](05-VERIFICATION-AND-COVERAGE.md)
6. [Assessment decisions and deeper review sequence](06-ASSESSMENT-AND-NEXT-PASS.md)
7. [Installed experience versus intended use](07-LIVE-EXPERIENCE-AND-INTENDED-USE.md)

For lookup rather than sequential reading: [directory index](DIRECTORY-INDEX.md), [file inventory](file-inventory.csv), [statistics](inventory-summary.json), [workspace map](workspace-map.json), [inventory generator](inventory-generator.py).

## Main results

- Inventory: 6,375 tracked/nonignored paths; 1,429 selected-extension non-vendor source files and 367,385 lines, including tests/comments. Build caches and ignored descendants are excluded.
- The root Rust workspace has 19 packages. HUD and Mirromere native apps have separate Cargo workspace roots.
- Root daemon compilation passed. Workspace all-target compilation failed on a Vaire test fixture missing excluded_refs.
- Seventeen targeted Rust tests and six Mirromere frontend tests passed. This is not full regression or live acceptance.
- Engine's live workbench path depends on execution code within Aule's observability/autopilot tree: a concrete ownership/layering concern.
- Four active-plan links are broken, and the codemap's workspace count is stale.
- Runtime/evidence/private-state paths are intermixed with versioned product material. Their lifecycle needs classification before cleanup.
- No substantive source file was declared safely unused. Retirement candidates and false positives are separated explicitly.

## Boundaries

Snapshot date: 2026-10-01. Source: dirty working tree on plan/ambient-agent-program; exact HEAD and initial capture time are in the statistics file. Existing modifications are not attributed to this audit. No application code, existing configuration or existing plan was intentionally changed. Nothing was deleted, committed, pushed or deployed. The subsequent live pass started the installed HUD and executed one bounded local inference task; its evidence and remaining acceptance boundaries are in the live supplement.

All authored conclusions are draft assessments for operator review. Observed defects, risks and unresolved questions are distinguished in the findings. Full semantic review of all source, unused-code proof, installed runtime acceptance and security assessment remain deeper audit work, not hidden claims of completion.
