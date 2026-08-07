# Arda Company Operations Implementation Plan

> **Lifecycle:** Deferred outside the Arda 1.0 release scope on 2026-08-06. The implemented internal-alpha evidence is retained; the real-engagement pilot gates remain honest future evidence and do not block 1.0.

> **For Hermes:** Build this as a governed application over Workbench, Personal Operations, Warden Research, Oromë, Aulë, and Economics. Do not create an unconstrained “CEO agent” or let proposals become executable work without receipts and approval.

**Goal:** Help the operator convert ideas, client needs, research, and development capacity into revenue-producing products and services while preserving human authority over commitments, money, external communications, publication, and deployment.

**Architecture:** Company Operations is an operator cockpit and proposal engine, not a sovereign business actor. It projects CRM, opportunities, products, experiments, client commitments, costs, and outcomes through versioned adapters. Aulë coordinates proposals and metrics, Warden/Varda provide evidence, Workbench executes approved product work, Oromë sends approved communications, and Economics records cost/value.

**Tech stack:** Existing Aulë/CEO and economics surfaces, Rust contracts, HUD React modules, adapter sidecars for CRM/accounting/calendar/email, append-only decision and outcome receipts.

**Target stage:** Stage 5 internal alpha; Stage 6 commercial-operations beta  
**Primary business objective:** recover operator time and turn Arda into a force multiplier for paid external work.

**Implementation status (2026-08-04):** Stage 5 internal-alpha implementation complete. Stage 6 pilot evidence remains operator-gated: no real client record, external message, monetary commitment, or autonomous charge was fabricated or performed to close this plan.

### Implemented evidence

- Phase 0: `arda-core::company_ops` now owns versioned records, privacy/authority classes, confidence ranges, approval-gated commitments, receipt-backed realized value, config loading, Workbench objective proposals, and client-delivery validation. `spec/company-ops/v1/company-ops.schema.json`, `config/business/company-operations.toml`, and `docs/migrations/company-operations-legacy-config.md` define the external and migration boundaries.
- Phase 1: `arda-aule::company_ops` provides a locked append-only JSONL store, idempotency, deterministic replay, forecast/time/evidence/risk scoring, reviewed-outcome feedback for related organizations, redacted projections, and canonical `data/business/{opportunities,drafts,commitments,experiments,outcomes,company-ops}.json` outputs.
- Phase 2: `arda-engine::adapters::company` provides capability-allowlisted CRM, calendar, email, project/issue, and accounting-export operations plus a read-only, stable-ID CRM reference implementation. Write approvals are bound to an exact operation/resource scope. `arda-orome::commercial` requires the approved proposal ID, scope, price, due date, and expiry to match before external transport and preserves attempted/accepted/delivered/failed truth.
- Phase 3: the Business module loads Aulë's canonical summary projection and renders the next action, active paid/client engagements, commitments, opportunities, experiments, approval drafts, receipt-backed expected-versus-realized value, and cost/time constraints through dedicated tested panels.
- Phases 4–5: bounded experiments can become `WorkbenchObjectiveProposal` records only with matching approval receipts; delivery bundles enforce acceptance evidence, scope/support boundaries, change/overrun fields, and invoice-export-only behavior.

### Verification evidence

- `cargo test -p arda-core --test company_ops -- --test-threads=1`: 7 passed.
- `cargo test -p arda-aule --all-features -- --test-threads=1`: package suite passed, including 183 library tests and 4 Company Operations tests.
- `cargo test -p arda-economics --all-features -- --test-threads=1`: 34 passed, 1 ignored operator-scale test.
- `cargo test -p arda-orome --all-features -- --test-threads=1`: 86 library tests and all integration tests passed, including 2 commercial-delivery tests.
- `cargo test -p arda-engine --test company_adapter_contract -- --test-threads=1`: 3 passed.
- `cargo check --workspace --all-targets`: passed.
- `apps/arda-hud`: `pnpm test` passed 100 files / 392 tests; `pnpm lint` passed with 105 pre-existing warnings and 0 errors; `pnpm build` passed.

### Remaining Stage 6 operator-gated pilot evidence

- Represent one operator-selected real client engagement through a configured adapter without exposing restricted data.
- Approve one generated Workbench objective and verify its actual deliverable receipt.
- Explicitly approve one external draft, dispatch it through Oromë, and capture real provider delivery truth.
- Record one reviewed outcome and feed the resulting evidence back into scoring.

---

## Verified starting point

- `apps/arda-hud/src/components/arda/modules/BusinessModule.tsx` currently displays mode, client records, paths, and state keys.
- `config/business/ceo_startup.yaml` contains legacy names, paths, ports, and machine assumptions; treat it as migration evidence, not live authority.
- Aulë already has autopilot, execution-intent, knowledge-triage, telemetry, and operator projection machinery.
- `arda-economics` owns cost, resource, and value-related records.
- Oromë owns message routing and delivery truth.
- Warden/Varda learning work can produce cited opportunity evidence but cannot create executable work directly.
- Personal runtime evidence identifies client relationships and time as high-value constraints; private identity data must not leak into product telemetry.

## Product promise

> Show the operator which client, product, or revenue action is most valuable now; explain the evidence and cost; prepare the work; and execute only the approved parts through normal Arda gates.

## Authority matrix

| Action | Default authority |
|---|---|
| Read internal project/client state | read-only |
| Research market/product question | bounded automatic observation |
| Draft proposal/email/SOW | proposal only |
| Create internal experiment task | review required |
| Send external message | explicit confirmation/allowlist plus delivery receipt |
| Promise scope/date/price | explicit operator approval |
| Spend money or purchase service | prohibited until a separate financial contract |
| Deploy or publish product | explicit release approval |
| Sign legal agreement | prohibited |

## Phase 0 — Define commercial records and privacy boundaries

### Task 0.1: Add business operations contracts

**Files**
- Create: `crates/spine/governance/arda-core/src/company_ops.rs`
- Modify: `crates/spine/governance/arda-core/src/lib.rs`
- Create: `spec/company-ops/v1/company-ops.schema.json`
- Test: `crates/spine/governance/arda-core/tests/company_ops.rs`

**Types**
- `Organization`, `ContactReference`, `ClientEngagement`
- `Opportunity`, `ProductHypothesis`, `RevenueExperiment`
- `ProposalDraft`, `Commitment`, `OutcomeReceipt`
- `ValueEstimate`, `OperatorTimeBudget`, `ConfidenceRange`
- source/evidence, privacy, authority, expiry, and adapter provenance

**Acceptance**
- Estimates cannot be serialized as realized revenue.
- A draft cannot become a commitment without an approval receipt.
- Contact and health/personal fields are redacted from general telemetry.

### Task 0.2: Retire legacy startup config as authority

**Files**
- Audit: `config/business/ceo_startup.yaml`
- Create: `config/business/company-operations.toml`
- Create: `docs/migrations/company-operations-legacy-config.md`

Map each still-useful setting to a current Arda service/adapter or mark it retired. Do not preserve dead mission-control paths merely for narrative continuity.

## Phase 1 — Opportunity and commitment ledger

### Task 1.1: Implement event store and projections

**Files**
- Create: `crates/spine/observability/arda-aule/src/company_ops/mod.rs`
- Create: `crates/spine/observability/arda-aule/src/company_ops/store.rs`
- Create: `crates/spine/observability/arda-aule/src/company_ops/projection.rs`
- Test: package-local replay and privacy tests.

**Canonical projections**
- `data/business/opportunities.json`
- `data/business/commitments.json`
- `data/business/experiments.json`
- `data/business/outcomes.json`

**Acceptance**
- Pipeline values show ranges/confidence and distinguish lead, qualified, proposed, won, lost, delivered, invoiced, and paid.
- Replay is deterministic and append-only.

### Task 1.2: Add operator time and value scoring

Integrate `arda-economics` without reducing every decision to money. Score urgency, expected value range, operator-time cost, strategic fit, family/time constraints, reversibility, evidence quality, and commitment risk. Display components and uncertainty.

## Phase 2 — External system adapters

### Task 2.1: Define a company adapter protocol

**Files**
- Create: `spec/company-adapter/v1/protocol.md`
- Create: `crates/engine/src/adapters/company.rs`
- Test: `crates/engine/tests/company_adapter_contract.rs`

Support CRM, calendar, email, accounting export, and issue/project systems through supervised HTTP/MCP/JSONL adapters. Secrets remain in adapter-local stores or OS keyrings.

### Task 2.2: Build one CRM reference adapter

Select after the external-product licensing review. Begin read-only with organizations, contacts, opportunities, activities, and stable external IDs. Add outbound writes only after conflict, deduplication, and audit tests.

### Task 2.3: Connect communications through Oromë

Drafts include source context, audience, risk, commitments, and approval requirements. External send uses Oromë and preserves attempted/accepted/delivered/failed truth.

## Phase 3 — Company Operations HUD

### Task 3.1: Upgrade the Business module

**Files**
- Modify: `apps/arda-hud/src/components/arda/modules/BusinessModule.tsx`
- Add: `apps/arda-hud/src/components/arda/modules/BusinessModule.test.tsx`
- Create: `apps/arda-hud/src/components/business/OpportunityBoard.tsx`
- Create: `apps/arda-hud/src/components/business/CommitmentLedger.tsx`
- Create: `apps/arda-hud/src/components/business/ExperimentPanel.tsx`
- Create: `apps/arda-hud/src/components/business/ValueEvidencePanel.tsx`
- Create: `apps/arda-hud/src/lib/companyOps.ts`

**First screen**
- highest-value next operator action;
- commitments due soon;
- active paid/client work;
- product experiments and evidence;
- drafts awaiting approval;
- expected versus realized value;
- time/cost consumption.

**Acceptance**
- Forecast, proposal, commitment, invoice, and payment are visually distinct.
- Every recommendation links to evidence and assumptions.

## Phase 4 — Revenue experiment loop

```text
problem evidence -> product hypothesis -> smallest paid experiment
                 -> operator approval -> Workbench implementation
                 -> outreach draft -> approved send -> outcome
                 -> economics + learning -> continue/pivot/stop
```

### Task 4.1: Generate bounded experiment proposals

Warden/Varda evidence may produce a proposal containing customer problem, evidence, offer, target audience, build estimate, success threshold, maximum spend/time, expiry, and stop condition.

### Task 4.2: Execute approved build work through Workbench

Company Operations never edits product code itself. It creates a reviewed objective attached to an external or Arda project contract.

### Task 4.3: Record outcomes, including failure

Capture reply, meeting, trial, sale, loss reason, delivery cost, and operator assessment. Do not train or promote from sparse outcomes without review.

## Phase 5 — Client delivery cockpit

- statement-of-work decomposition;
- acceptance criteria and scope boundary;
- client communication drafts;
- deliverable/test/receipt bundle;
- change request and overrun warning;
- handoff and support boundary;
- invoice export, not autonomous charging.

## Verification ladder

```bash
cargo test -p arda-core --test company_ops -- --test-threads=1
cargo test -p arda-aule --all-features -- --test-threads=1
cargo test -p arda-economics --all-features -- --test-threads=1
cargo test -p arda-orome --all-features -- --test-threads=1
cargo test -p arda-engine --test company_adapter_contract -- --test-threads=1
cd apps/arda-hud && pnpm test && pnpm lint && pnpm build
```

## Release acceptance

- One real client engagement is represented without exposing secrets or personal data.
- One opportunity becomes an approved Workbench task and a verified deliverable.
- One external draft is approved and sent with delivery truth.
- Expected and realized value remain distinct.
- Operator can identify the highest-value next action in under one minute.
- The system measurably reduces status reconstruction and administrative time during a four-week dogfood period.

## Commercial guardrail

The first monetization target is improved operator throughput and paid Workbench-assisted delivery. Do not delay that outcome to automate a fictional zero-human company. Company Operations earns more authority only after repeated, audited, reversible success.
