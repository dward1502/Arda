---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
annunimas_contract: human_knowledge.v1
title: "Technical Architecture Convention Candidates"
status: reviewed_candidate
authority: agent_generated
source_type: knowledge_candidate
owner: athena
created: 2026-05-20
updated: 2026-05-20
supersedes:
  - "human/03-Knowledge/Architecture/technical-architecture-notes-review-boundary.md"
superseded_by: []
source_paths:
  - "audit/HUMAN_INBOX_PHASE2F_2026-05-17/task-clusters/technical_architecture_notes.md"
  - "human/09-Archive/processed-inbox/2026-05-19/technical-notes/rust-hooks.md"
  - "human/09-Archive/processed-inbox/2026-05-19/technical-notes/scss-os-component-idea.md"
  - "human/09-Archive/processed-inbox/2026-05-19/technical-notes/setting-up-local-llm.md"
  - "human/09-Archive/processed-inbox/2026-05-19/technical-notes/ui-ux-terminology.md"
affected_agents:
  - athena
  - prometheus
  - forge-mind
  - oracle
affected_paths:
  - "human/03-Knowledge/Architecture/technical-architecture-convention-candidates.md"
  - "audit/ARANDUR_GATE_4_3_TECHNICAL_ARCHITECTURE_CONVENTIONS_2026-05-20/convention_packet.json"
privacy: private
review_required: true
confidence: medium
canonical_truth_claim: false
sigils: ["↝", "◈"]
promotion:
  phase: "Arandur Gate 4.3 dog-run"
  promoted_at_utc: "2026-05-20T04:22:15Z"
  decision: "extract-review-gated-convention-candidates"
  raw_inbox_preserved: true
  canonical_truth_claim: false
---

> 🜏 Soterion: 📜 documentation | owner: athena | status: reviewed_candidate | reviewed: 2026-05-21

# Technical Architecture Convention Candidates

This is a review-gated evidence surface, not a canonical engineering standard. It extracts durable candidate conventions from Phase 2F technical notes while preserving the previous boundary that no raw AI-conversation snippet becomes authoritative without codebase review and tests.

## Candidate conventions

1. Governance hook middleware remains a future spike candidate, not a current API. Current `annunimas-governance` exports `triad_validate`, `TriadConfig`, `TriadResult`, `GateOutcome`, `bacon_lite_validate`, and resonance functions; `HookablePhilosopher` and `PhilosopherChain` are note-level design sketches only.
2. Regex or structured validation can support audit gates for LLM outputs, but any adoption must be schema-backed, test-backed, and explicit about hard gate vs justification-producing gate behavior.
3. AI-generated UI work may benefit from semantic component manifests, tokens, mixins, and visual-library registries, but ARDA HUD changes must respect its existing decomposed Tauri/Three.js scene architecture.
4. Kinetic UI, holographic depth, data-ink HUD, and interaction-choreography terms are usable as private design vocabulary with provenance.
5. The local-LLM setup source contributes no conventions because the archived file is zero bytes.

## Implementation guardrails

- Read existing source structs, function signatures, and imports before proposing code against any Annunimas crate.
- Do not use production `unwrap()` in Rust.
- Keep ARDA HUD additions in active in-scene workstation/scene modules rather than monolithic `App.tsx` state.
- Verify procedural/material layers conform to target object surfaces when future 3D visual layers are introduced.
- Treat all items here as `review_required=true` until an operator approves a bounded implementation gate.

## Suggested follow-up gates

- `governance_hook_middleware_spike`: no production code until current triad flow is mapped, tests are written, and API compatibility is reviewed.
- `governance_regex_corpus_review`: inventory existing Bacon/Aurelius/Sun Tzu heuristics and evaluate schema-backed validation opportunities.
- `arda_visual_component_inventory`: inspect active ARDA HUD modules and decide whether a semantic visual component manifest is useful.
