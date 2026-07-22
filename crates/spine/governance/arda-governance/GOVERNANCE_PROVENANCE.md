---
soterion:
  sigil: "REPAIR"
  glyph: "🜏"
  code_point: "U+1F70F"
  role: "component_provenance"
  owner: "HADES"
  status: "draft"
  reviewed: "2026-07-21"
---

> 🜏 Soterion: 🜏 component_provenance | owner: HADES | status: draft | reviewed: 2026-07-21

# arda-governance Provenance

## Scope

This document records the conceptual and implementation provenance of
`arda-governance`. It applies the classifications defined in
[`docs/PROVENANCE_AND_ATTRIBUTION.md`](../../../../docs/PROVENANCE_AND_ATTRIBUTION.md).
It is intentionally conservative: unknown source identities and licenses are marked as
pending rather than guessed.

## Confirmed creator statement

The project creator has provided the following provenance statement:

- The philosopher-lens governance system is the creator's original contribution.
- JouleWork was borrowed and adapted from external work.
- Love Equation was borrowed and adapted from external work.
- The overall Arda governance architecture combines multiple design frameworks developed
  recently; some influenced the design while others were adapted or incorporated.

This statement establishes classification intent, but the external creator names,
canonical sources, exact versions, and licenses for JouleWork and Love Equation still
need to be supplied and verified.

## Contribution registry

| Contribution | Classification | Arda implementation evidence | Known Arda contribution | Attribution state |
|---|---|---|---|---|
| Philosopher-lens governance | `original` | `src/triad.rs`, `src/triad_philosopher.rs`, `src/philosopher_profiles.rs`, `config/governance/philosophers.toml` | Executable Aurelius, Bacon, and Sun Tzu lenses; arbitration; profile maturity; evidence and readiness boundaries | Creator statement recorded; first-design date and historical evidence pending |
| Integrated governance pipeline | `original` integration over mixed-source concepts | `src/lib.rs`, `src/resonance.rs`, `src/readiness.rs`, `config/governance/chains.toml` | Typed composition of philosophical evaluation, resonance, evidence maturity, receipts, and scoped autonomy readiness | Architecture mapping exists; external-framework-by-framework mapping incomplete |
| JouleWork | `adapted` | `src/joulework.rs`, resonance and consumer integrations | Arda task/resource profile, honesty semantics, serialized governance use, and runtime integrations | External creator, title, canonical URL/version, retained elements, and license pending |
| Love Equation compatibility proxy | `adapted` | `src/love_equation.rs` | Arda-compatible task scoring and result metadata | External creator, title, canonical URL/version, equation provenance, and license pending |
| Love Dynamics | `unknown_pending_review` | `src/love_dynamics.rs`, `src/triad_philosopher.rs` | Cooperation/defection projection and philosopher arbitration integration | Determine whether this is original, adapted from the same Love Equation source, or adapted from a separate framework |
| Resonance and phi-harmonic composition | `unknown_pending_review` | `src/resonance.rs` | Typed composition, compatibility/live Triad source disclosure, Arda task mapping | Identify any external equations or frameworks and distinguish them from conventional weighted scoring |
| Bacon-Lite validation and ledger | `inspired` plus implementation provenance pending | `src/bacon_lite.rs` | Empirical evidence gate and machine/human ledger outputs | Francis Bacon's historical method is an influence; audit whether any modern implementation or scoring scheme was adapted |
| Game-theory selection | `conventional` unless source audit finds a specific adaptation | `src/game_theory.rs` | Capability/action-class filtering and governance-aware selection | Algorithm/source audit pending; dependency-level techniques do not automatically imply a specific attribution |
| Readiness and independent-review receipts | `original` or `adapted`, pending historical audit | `src/readiness.rs` | Conservative maturity levels, independent receipt validation, scoped autonomy boundary | Compare against source plans/frameworks and record exact influences before claiming original status |
| Audio and vision coherence | `conventional` or `adapted`, pending audit | `src/audio.rs`, `src/vision.rs` | Typed optional advisory inputs to resonance | Identify any copied thresholds/equations or mark as conventional after review |
| Solar/geomagnetic signal | `dependency` plus possible adapted scoring | `src/solar.rs` | NOAA data retrieval and an Arda multiplier | Record NOAA endpoint/data terms and source of multiplier thresholds before release |
| Rust implementation produced in the agentic development environment | `ai-assisted` | Git history and development sessions; exact mapping not yet assembled | Human-directed integration, review, and acceptance through Arda tests | Tool/model/session attribution must be reconstructed where possible; human authorship/review record pending |

## Originality boundary

The originality claim for the philosopher system covers its use as an executable,
multi-lens Arda governance architecture. It does not claim ownership of the historical
works or identities of Marcus Aurelius, Francis Bacon, Sun Tzu, Stoicism, empirical
method, or classical strategy. Profile source editions and quotations, if added later,
must be cited separately and checked for copyright status.

The strongest integrative contribution currently visible in the crate is the typed
combination of:

- philosopher-specific policy lenses;
- evidence and maturity disclosure;
- live versus compatibility governance signals;
- JouleWork and Love-related scoring;
- versioned/configurable governance chains;
- independent-review receipts; and
- conservative scoped-autonomy readiness.

That combination can be described as an Arda contribution without claiming that every
underlying equation or design principle was independently invented.

## Required source records

### JouleWork

- [ ] External creator(s) or organization identified.
- [ ] Original title and canonical URL/DOI/repository recorded.
- [ ] Version, publication date, commit, or artifact hash recorded.
- [ ] Upstream license or usage terms reviewed.
- [ ] Original definition/equation copied into an audit note with citation.
- [ ] Differences between upstream JouleWork and `src/joulework.rs` documented.
- [ ] Required notice wording added to a future repository-root `NOTICE`.

### Love Equation

- [ ] External creator(s) or organization identified.
- [ ] Original title and canonical URL/DOI/repository recorded.
- [ ] Version, publication date, commit, or artifact hash recorded.
- [ ] Upstream license or usage terms reviewed.
- [ ] Original equation and variable definitions recorded accurately.
- [ ] Differences between the original work, `src/love_equation.rs`, and
  `src/love_dynamics.rs` documented.
- [ ] Required notice wording added to a future repository-root `NOTICE`.

### Other merged frameworks

- [ ] List each framework separately; do not use “several frameworks” as the final
  attribution record.
- [ ] Mark each one `inspired`, `adapted`, `incorporated`, or `dependency`.
- [ ] Map it to specific Arda files, schemas, equations, prompts, or workflows.
- [ ] Record what was retained and what Arda changed.
- [ ] Verify license/terms for anything adapted or incorporated.

## AI-assistance record

For existing implementation history:

- [ ] Identify commits and session records that materially created each module.
- [ ] Record the development tool/model when that evidence is available.
- [ ] Record which external sources were supplied to or consulted by the agent.
- [ ] Human-review the source mapping; an agent-generated attribution is not sufficient
  evidence by itself.
- [ ] Mark irrecoverable tool/model details as `unknown`, not as “human-only.”

For future changes, include this in the PR, commit body, or review receipt:

```text
AI-assisted: <tool/model or unknown>
Human direction: <issue, plan, or task>
Material paths: <files/modules>
External sources consulted: <URLs/DOIs/repositories or none recorded>
Human-reviewed: <reviewer/date/status>
Provenance changes: <entries added or none>
```

## Verification and release checklist

- [ ] Every non-original concept in the contribution registry has an exact source.
- [ ] Every adapted or incorporated item has a completed license review.
- [ ] Source comments link to this document where an equation or algorithm would
  otherwise appear internally invented.
- [ ] Public README language matches the originality boundary above.
- [ ] Serialized policy/version names do not reuse a third-party trademark deceptively.
- [ ] Required copyright and license notices are present in the repository-root notice
  surface once created.
- [ ] `FIRST_CLASS_CHECKLIST.md` includes provenance review in its release gate.
- [ ] This document is changed from `draft` to `active` only after the project creator
  fills and verifies the pending upstream records.
