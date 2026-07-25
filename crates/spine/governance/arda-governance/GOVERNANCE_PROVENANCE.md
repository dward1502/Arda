---
soterion:
  sigil: "REPAIR"
  glyph: "🜏"
  code_point: "U+1F70F"
  role: "component_provenance"
  owner: "HADES"
  status: "active"
  reviewed: "2026-07-25"
---

> 🜏 Soterion: 🜏 component_provenance | owner: HADES | status: active | reviewed: 2026-07-25

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

This statement establishes classification intent. The external identities, dated source
pages, adaptation boundaries, and observable terms for JouleWork and the Love Equation
were verified on 2026-07-25 and are recorded below. A dated web publication is the
consulted upstream version where the publisher supplies no immutable release identifier.

## Contribution registry

| Contribution | Classification | Arda implementation evidence | Known Arda contribution | Attribution state |
|---|---|---|---|---|
| Philosopher-lens governance | `original` Arda integration; historical names are `inspired` | `src/triad.rs`, `src/triad_philosopher.rs`, `src/philosopher_profiles.rs`, `config/governance/philosophers.toml` | Executable Aurelius, Bacon, and Sun Tzu lenses; arbitration; profile maturity; evidence and readiness boundaries | Creator statement confirms the integration as original. Historical anchors are *Meditations* (George Long English edition, 1862), Francis Bacon's *Novum Organum* (1620), and Sun Tzu's *The Art of War* (Lionel Giles English edition, 1910). Those works/editions are public domain in the United States; no translation text, algorithm, or code is copied. |
| Integrated governance pipeline | `original` integration over the separately recorded concepts | `src/lib.rs`, `src/resonance.rs`, `src/readiness.rs`, `config/governance/chains.toml` | Typed composition of philosophical evaluation, resonance, evidence maturity, receipts, and scoped autonomy readiness | Creator statement plus live implementation mapping; no unrecorded external framework was found in source, plans, git history, or reference-tree search. |
| JouleWork | `inspired` (creator statement used “adapted”; implementation audit narrows current code to inspiration) | `src/joulework.rs`, resonance and consumer integrations | Arda task/resource profile, honesty semantics, serialized governance use, and runtime integrations | Brian Roemmele, “Wages for AI Workers? The JouleWork Revolution, and the Birth of a New Economic Paradigm,” Read Multiplex, 2026-01-31, <https://readmultiplex.com/2026/01/31/wages-for-ai-workers-the-joulework-revolution-and-the-birth-of-a-new-economic-paradigm/>. Corroboration: David Galipeau, “From Wages to Watts,” written 2026-07-07, posted 2026-07-14, SSRN 7069098, <https://doi.org/10.2139/ssrn.7069098>. Boundary/terms below. |
| Love Equation compatibility proxy | `original` compatibility heuristic under a legacy borrowed name | `src/love_equation.rs` | Impact/reach divided by energy/time; explicit metadata says it is not canonical Love Dynamics | The heuristic is not present in the identified upstream source. It is retained only for serialized/API compatibility and is deprecated. The borrowed name is attributed through the canonical Love Dynamics record below. |
| Love Dynamics | `adapted` | `src/love_dynamics.rs`, `src/triad_philosopher.rs` | Bounded one-step cooperation/defection projection and philosopher arbitration integration | Brian Roemmele, “How One Starry Night In 1978, Thinking About Alien Intelligence, I Solved The AI ‘Alignment Problem’ With The Love Equation,” Read Multiplex, 2025-12-20, <https://readmultiplex.com/2025/12/20/how-one-starry-night-in-1978-thinking-about-alien-intelligence-i-solved-the-ai-alignment-problem-with-the-love-equation/>. Boundary/terms below. |
| Resonance and phi-harmonic composition | `conventional` weighted scoring plus original Arda integration | `src/resonance.rs` | Typed composition, compatibility/live Triad source disclosure, Arda task mapping | Source/plan/history audit found no specific external equation or copied implementation. The golden ratio is a mathematical constant; local ratios, weights, and missing-signal rules are Arda policy. |
| Bacon-Lite validation and ledger | `inspired` historical label plus original Arda implementation | `src/bacon_lite.rs` | Empirical evidence gate and machine/human ledger outputs | Francis Bacon's *Novum Organum* (1620) is the historical empirical-method influence. No modern implementation, scoring scheme, prose, or code was found to be adapted. |
| Game-theory selection | `conventional` | `src/game_theory.rs` | Capability/action-class filtering and governance-aware deterministic selection | Uses the generic mathematical field name only; source audit found no specific external algorithm, payoff table, text, or code. |
| Readiness and independent-review receipts | `original` Arda policy integration | `src/readiness.rs` | Conservative maturity levels, independent receipt validation, scoped autonomy boundary | Source/plan/history audit found no specific external schema or copied implementation. |
| Audio and vision coherence | `conventional` advisory normalization | `src/audio.rs`, `src/vision.rs`, `src/environmental.rs` | Typed optional advisory inputs to resonance | Local bounded arithmetic and quality/freshness metadata; no copied thresholds, equations, models, clinical claims, or source implementation found. |
| Solar/geomagnetic signal | `dependency` on live NOAA data plus original advisory mapping | `src/solar.rs`, `src/environmental.rs` | NOAA Kp/Dst retrieval, caching/quality projection, and an Arda advisory multiplier | SWPC Data Service documentation: <https://www.spaceweather.gov/content/data-access>. NOAA Open Data terms checked 2026-07-25: <https://registry.opendata.aws/noaa-space-weather/>. Live products are not version-pinned; concrete URLs are in `src/solar.rs`. NOAA data are open for public use; NOAA requests attribution for unaltered data. Do not imply endorsement or call transformed output unaltered NOAA data. |
| Rust implementation produced in the agentic development environment | `ai-assisted` | Git history and development sessions | Human-directed integration, review, and acceptance through Arda tests | Exact historical tool/model identities are not recoverable from repository evidence. Record as unknown rather than human-only; this is an AI-assistance accreditation gap, not an external-source/license gap. |

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

## Exact adaptation and notice records

### JouleWork

- **Upstream definition consulted:** Roemmele's 2026-01-31 article embeds the definition
  `JW = E × κ × W`, with energy `E`, normalized productive output `W`, and normalization
  coefficient `κ`; it also discusses wages, tokenization, pricing, and an efficiency ratio.
- **Retained by Arda:** the name and high-level premise that work/resource estimates and
  observations should be visible and efficiency-sensitive.
- **Not retained:** the upstream equation, normalization coefficient, wage unit, tokens,
  pricing, economic agency, thermodynamic inevitability claims, and illustrative numbers.
- **Arda adaptation:** `profile_joulework` compares local task estimate/actual fields,
  computes symmetric honesty and variance diagnostics, discloses measurement provenance,
  and forbids default estimates from acting as autonomy truth.
- **Terms/notice:** Read Multiplex identifies its site content as copyrighted. The consulted
  pages expose no permissive content license or express reuse grant; no more specific
  permission rule is asserted here. No upstream text, media, or code is incorporated. Keep
  creator/title/URL attribution in this registry; no upstream copyright notice is copied
  into a repository `NOTICE`. Re-review before adding the upstream equation or economic
  mechanism.

### Love Equation and Love Dynamics

- **Upstream definition consulted:** Roemmele's 2025-12-20 article defines
  `dE/dt = β(C - D)E`: emotional complexity/empathy `E` grows when cooperation `C`
  exceeds defection `D`, scaled by `β`.
- **Retained by Arda:** the equation and cooperation-versus-defection interpretation.
- **Arda adaptation:** `evaluate_love_dynamics` normalizes caller-provided values, takes
  one bounded Euler step over explicit `delta_time`, classifies growing/stable/decaying,
  and supplies advisory evidence to philosopher arbitration. It does not independently
  authorize or block runtime work.
- **Compatibility distinction:** `src/love_equation.rs` is an independently written
  impact/reach/energy/time task-value heuristic. It is explicitly deprecated and labeled
  “not canonical Love Dynamics”; it must not be represented as Roemmele's equation.
- **Terms/notice:** the crate includes attribution plus the public mathematical
  relationship, but no upstream prose, media, or code. On 2026-07-25 the project creator
  completed the human release review and approved public distribution of this narrow
  algorithmic adapter on the stated basis that both algorithms are open source and public.
  That approval does not authorize copying protected prose, media, or source code beyond
  the current adapter boundary.

### Other identified external surfaces

- Historical philosopher names/works are attribution anchors only; local policies do not
  copy public-domain translation text.
- NOAA is a live data-service dependency. Attribute NOAA when redistributing unaltered
  source data, identify Arda projections as transformed, and never imply NOAA endorsement.
- No other specific external equation, algorithm, schema, prompt, code, or asset was found
  in the crate source, its three source plans, git history, or the Annunimas reference-tree
  search performed for this release gate.

## AI-assistance record

For existing implementation history, repository and session evidence establishes
AI-assisted development under human direction, but does not preserve a reliable exact
tool/model mapping per module. Those identities are recorded as `unknown`. The project
creator reviewed and approved this source mapping for release on 2026-07-25. This document
records that human release decision; it does not provide legal advice.

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

- [x] Every non-original concept in the contribution registry has an exact source/version
  or live-service retrieval boundary.
- [x] Every adapted or dependency item records the observable license/terms and notice
  requirements; no external code, prose, media, schema, or asset is incorporated.
- [x] Source comments link to this document where an equation or algorithm would
  otherwise appear internally invented.
- [x] Public README language matches the originality boundary above.
- [x] Serialized policy/version names identify Arda compatibility/policy semantics and do
  not claim upstream endorsement.
- [x] No upstream artifact requiring copied notice text is distributed by this crate;
  creator/title/URL and NOAA notice obligations are preserved here.
- [x] `FIRST_CLASS_CHECKLIST.md` includes provenance review in its release gate.
- [x] This document is `active` because the creator classifications and upstream records
  are present, and human release review was approved by the project creator on 2026-07-25.
