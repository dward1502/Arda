---
soterion:
  sigil: "SCROLL"
  role: "workstation_audit"
  owner: "HERMES"
  status: "implemented_phase_7_native_acceptance_pending"
  reviewed: "2026-08-16"
---

> 🜏 Soterion: 📜 Human, Business, and Personal workstation audit | owner: HERMES | status: Phase 7 implemented, native acceptance pending | reviewed: 2026-08-16

# Lower Monitor 05 — Human, Business, and Personal

## Audit status

> Implementation update: Phase 7 replaced the audited stacked modules with the canonical three-horizon continuity owner. See [`HUMAN_BUSINESS_PERSONAL.md`](HUMAN_BUSINESS_PERSONAL.md). The audit evidence below records the pre-implementation state and remains useful as provenance.

- Investigation date: 2026-08-16.
- Authority standard: current code, persisted state, source derivations, module composition, and rendered fields.
- Documentation is context only until verified.
- This was a current-state audit at capture time, not an approved redesign.
- No application code or runtime state was changed by the audit itself.

## Physical surface and route

```text
boardroom.lower.right_wrap
  -> view_desk_aux
  -> human_business_personal
  -> Human + Business Workstation
```

The source-zone name and source-panel contract promise three concerns:

1. human notes/readable context;
2. business operations;
3. personal growth/context.

The active module profile opens only:

1. `human_realm`
2. `business`

There is no Personal tab in this workstation.

## Lower physical instrument

The physical lower screen uses the `human` role and `living_contours` topology. However, its signal input is only:

```text
documents = getHumanDocs(bundle).length
notes = getHumanNotes(bundle).length
```

It does not include:

- business opportunities, commitments, engagements, drafts, or experiments;
- client work or business mode;
- personal priorities, time constraints, values, family commitments, research, or creative domains;
- appointments, health, household, relationships, communications, or personal operations.

Current `human_context.json` provides zero human docs and zero notes. Consequently the physical “Human + Business + Personal” signal can be nearly empty even though business and personal projections contain data.

## Tab 1 — Human Realm

### Current layout

The tab is a generic readable-layer card containing:

1. Docs — first four records;
2. Notes — first three records;
3. count chips for docs, notes, summaries, and Arandur;
4. Human Source Freshness;
5. Plan Shelf — plan root and first four plans.

It has no list/detail interaction, search, filtering, opening, editing, or contextual actions.

### Current source

Primary bundle source:

```text
core/state/human_context.json
  -> bundle.humanContext
  -> getHumanDocs() / getHumanNotes()
  -> HumanRealmModule
```

Current projection:

- generated 2026-07-14;
- `docs_total`: 0;
- `notes_total`: 0;
- `summaries_total`: 0;
- `arandur_docs_total`: 0;
- `library_docs_total`: 1;
- rendered Docs and Notes lists: empty.

### Fallback derivation defect

If `human_context.json` is absent, `deriveHumanContext()` scans `docs/arda` once and reuses the same tree for:

- docs;
- notes;
- summaries;
- library.

It does not read the declared `docs/operator/notes` source for note content. It also uses `docs/operator/philosophy.md` as both Arandur index and Arandur thoughts. Therefore its categories are aliases rather than independent information classes.

The current dedicated snapshot exists, so this fallback is not active now. It remains a future correctness risk.

### Plan Shelf overlap

The Human Realm tab includes up to four plans. Planning and queue already have a dedicated domain, and the Command Core GO button routes there. This is likely cross-domain duplication rather than human-context detail.

## Tab 2 — Business

### Current layout

The tab is one long `ModuleCard` containing:

1. Highest-value next operator action;
2. Active paid and client work;
3. Commitment Ledger;
4. Opportunity Board;
5. Experiment Panel;
6. Drafts awaiting approval;
7. Value Evidence;
8. Mode/client/state-key counters;
9. Company View preview;
10. Business Source Freshness;
11. raw Client Paths;
12. raw State Keys.

The domain concepts are stronger than in the generic Systems tab, but they are still stacked into one long page. No master/detail relationship links an opportunity, engagement, commitment, experiment, or draft to one coherent detail view.

### Primary runtime source

```text
core/state/business_runtime.json
  -> bundle.businessRuntime
  -> App field extraction / companyOpsFromProjection()
  -> BusinessModule
```

Current projection:

- generated 2026-07-14;
- mode: `full`;
- client records: 3;
- state keys: 11;
- three client paths;
- six highlighted state keys;
- Company View preview begins with document frontmatter rather than useful body text;
- no `company_ops` field.

The three projected client paths point into `data/business/clients`, but that directory is currently absent. The snapshot therefore advertises client records whose underlying files no longer exist in the current workspace.

### Company operations

The fallback derives `company_ops` from:

```text
data/business/company-ops.json
```

That file is absent. The Business module receives an empty normalized snapshot, so:

- no scored opportunities;
- no active engagements;
- no commitments;
- no experiments;
- no external drafts;
- no value evidence.

The default “Review evidence before creating commercial work” text is a UI fallback, not an observed recommendation.

### Business derivation sources

`deriveBusinessRuntime()` also looks for:

- `docs/operator/company-view.md` — present;
- `data/business/soterion-business.json` — missing;
- `data/business/clients` — missing;
- `data/business/company-ops.json` — missing.

Because the dedicated `business_runtime.json` snapshot exists, the module looks populated at the summary level even though its underlying operational data is absent.

## Personal projection — loaded but not shown here

```text
core/state/personal_runtime.json
  -> bundle.personalRuntime
  -> PersonalGrowthModule
```

The current projection exists and includes:

- identity, role, and location;
- human priorities and values;
- eight research domains;
- four creative domains;
- one personal document;
- onboarding/context preview;
- time-priority and personal-governance context.

`PersonalGrowthModule` is registered in the global module registry, but `human_business_personal` does not request it. The current workstation therefore omits the most directly personal information promised by its source-zone name.

The separate `personal_growth` source zone requests `personal_operations`, `personal_growth`, and `human_realm`, but that zone is not assigned to a lower monitor.

## Declared source contract

Current persisted source map for `human_business_personal`:

### Primary

- `core/state/human_context.json` — present, generated 2026-07-14.

### Supplemental

| Source | Current state |
|---|---|
| `docs/operator/index.md` | Present |
| `docs/operator/onboard.md` | Present |
| `docs/operator/company-view.md` | Present |
| `config/business.toml` | Present according to source-map provenance; content not directly rendered by these tabs |
| `data/business/soterion-business.json` | Missing |
| `docs/operator/notes` | Missing |
| `data/personal` | Present with one events JSONL file |
| `core/personal` | Missing |

`business_runtime.json` and `personal_runtime.json`, despite being the actual tab/runtime inputs, are not listed as primary or supplemental sources for this combined section. They belong to separate source zones. The combined section contract therefore does not accurately describe everything its modules consume.

## Current truth classification

| Feed/content | Classification |
|---|---|
| Human docs and notes | Snapshot present but counts/lists empty |
| Human fallback categories | Derived aliases over one `docs/arda` tree |
| Plan Shelf | Derived workspace inventory; duplicated planning concern |
| Business summary | July snapshot actively displayed |
| Projected client records | Snapshot references absent current files |
| Company operations | Source missing; empty normalized projection |
| Personal runtime | Populated snapshot loaded but unused by this workstation |
| Physical human signal | Uses only empty human docs/notes counts |
| Business/personal physical signal | Not connected |
| Editing/action controls | None in Human tab; Business subcomponents are primarily display surfaces |

## Existing strengths

1. The Business module has real domain concepts: opportunities, commitments, experiments, engagements, drafts, and evidence.
2. Forecast and realized value are explicitly distinguished.
3. External drafts carry approval-required semantics.
4. Human, Business, and Personal remain recognizable conceptual domains in source and module definitions.
5. The lower instrument has a distinct organic visual topology rather than reusing Fleet/Routing geometry.

## Current defects

1. The combined workstation promises three domains but displays only two.
2. The lower signal ignores Business and Personal entirely.
3. Human Realm is empty despite readable operator documents existing elsewhere.
4. Human fallback docs, notes, summaries, and library are aliases over one tree.
5. Plan Shelf duplicates the Planning domain.
6. Business summary projects three client records whose files are absent.
7. Company operations are empty because the operational source is missing.
8. Company View preview exposes frontmatter instead of useful context.
9. Source-map declarations omit the two runtime files directly consumed by the active tabs.
10. No list/detail structure supports quick scanning and deep reading.
11. There is no coherent personal-operations surface here for life continuity, health, family, household, time, or commitments.

## Intended information responsibility to retain

The final ownership is not decided, but this lower domain should support the user's real life rather than only software/business data. Candidate responsibilities include:

- human priorities, time, health/energy, family, household, and continuity;
- commitments and things requiring personal attention;
- business opportunities, clients, delivery, drafts, receipts, and realized value;
- relevant personal context without dumping raw identity records;
- clear boundaries between private personal context and business operations;
- fast at-a-glance signal plus readable detail on focus.

The Human, Business, and Personal concerns may require different visual compositions even if they remain one physical workstation. They do not have to be forced into one generic tab/card pattern.

## Interaction implications to retain, not implement yet

- Human/personal attention items can use a quiet priority list with selected detail.
- Business opportunities, engagements, commitments, and drafts naturally fit list/detail navigation.
- Sensitive personal context should be summarized purposefully, not exposed as raw state fields.
- A Business draft should link to its governance requirement without duplicating the Governance queue.
- Personal and Business can have different visual systems inside the same workstation.
- The physical lower signal needs an explicit choice of which cross-domain attention metric it owns.

## Cross-reference candidates

Do not resolve until all records exist:

- Business drafts/approvals versus Governance.
- Plans and commitments versus Planning/Command Core.
- Client receipts and evidence versus Evidence/Trust roles.
- Personal priorities versus Daily Command/Now.
- Human notes versus Knowledge/Memory.
- Communications with people versus Routing and Communications.
- Personal runtime versus currently unassigned `personal_growth` role.
- Private life continuity versus generic task/project projections.

## Evidence anchors

- `core/state/arda_boardroom_slots.json`
- `core/state/arda_source_map.json`
- `apps/arda-hud/src/App.tsx`
- `apps/arda-hud/src/components/arda/modules/HumanRealmModule.tsx`
- `apps/arda-hud/src/components/arda/modules/BusinessModule.tsx`
- `apps/arda-hud/src/components/arda/modules/PersonalGrowthModule.tsx`
- `apps/arda-hud/src/lib/ardaSource.ts`
- `apps/arda-hud/src/lib/ardaSurfaces.ts`
- `apps/arda-hud/src/lib/companyOps.ts`
- `apps/arda-hud/src/scene/boardroom/boardroomHudInstruments.ts`
- `apps/arda-hud/src/scene/boardroom/lowerInstrumentSignal.ts`
- current source files listed above

## Verification required after later approved changes

- Source-truth fixtures for human, business, and personal projections.
- Tests preventing stale client paths from appearing current.
- Tests separating docs, notes, summaries, and library sources.
- Lower-signal semantic tests that include the approved cross-domain attention metrics.
- Privacy/redaction tests for personal detail.
- List/detail interaction and keyboard-accessibility tests.
- Native visual acceptance and responsiveness tests.
- Focused Vitest, TypeScript, Rust, and Tauri build gates.

## Phase 10 closeout status

Continuity implementation, inventory reconciliation, and focused contracts are green within the 142-file, 576-test Phase 10 suite, and the optimized Tauri build passed. Native three-horizon, missing-reference, privacy-summary, source/timestamp, keyboard, reduced-motion, screenshot, and frame-rate checks remain blocked because the current release exposed no controllable native window. See [`ACCEPTANCE_MATRIX.md`](ACCEPTANCE_MATRIX.md) and [`VERIFICATION_CLOSEOUT.md`](VERIFICATION_CLOSEOUT.md).
