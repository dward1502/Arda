# Arda Stage 6 — Legitimate 1.0 Plan

> **For Hermes:** Stage 6 is the release decision and proof phase. The [Arda 1.0 Personal Agent Ecosystem Plan](2026-08-08-arda-1.0-personal-agent-ecosystem-plan.md) now owns product scope and required vertical proofs. This plan owns artifact, compatibility, safety, quality, support, independent-user, and release-decision gates; it must not substitute Workbench-only evidence for the master-plan product proofs.

**System completion coordination:** [Arda System Unification and Usability Plan](2026-08-02-arda-system-unification-and-usability-plan.md). This Stage 6 plan remains authoritative for release qualification, not product identity.

**Frontend convergence authority:** [HUD Frontend–Backend Contract Convergence and 1.0 Closeout](2026-08-06-hud-frontend-backend-contract-convergence-plan.md) and its [live integration audit](../audits/2026-08-06-hud-frontend-backend-integration.md).

**Release promise:** Operate one local-first personal agent ecosystem from phone and desktop: preserve personal and project context, compose bounded workers and optional capabilities, complete verified external-project work, provide governed proactive assistance, explain evidence, request appropriate authority, and resume safely.

**Product:** Arda 1.0 personal-agent-ecosystem base for the declared Linux support matrix; Workbench remains the release-critical artifact path<br>
**Included HUD projections:** system/runtime health, Workbench, recovery/diagnostics, Research, Personal Operations, and five independently claimable upper monitors.
**Optional/not base blockers:** Mirromere hardware, RELIC/CITADEL feature expansion, Company Operations pilots, live x402/payment execution, and validated-device ingestion. Their attachment contracts and truthful status follow the master plan.

**Execution state (2026-08-12):** Stage 6 is active. The first qualification
tranche hardened `scripts/arda_release_ops.py`: release source identity now
covers the root runtime, launcher and HUD, workspace crates, SDKs, schemas,
configuration, vendored dependencies, release tooling, service composition, and
the tag-bound signing workflow. Manifest generation fails closed when tracked
release source is dirty. Focused tests pin every covered production surface.
This makes a future clean-source freeze trustworthy; it does not freeze the
current dirty worktree, create final 1.0 bytes, or satisfy any artifact matrix.

---

## Entry criteria

- Entry requires the Stage 5 release-candidate evidence packet to be complete.
- Every critical/high defect is resolved or explicitly release-blocking.
- One qualifying independent non-author evaluation has passed without author/agent substitution.
- Installation, upgrade, rollback, backup, restore, and diagnostics are reproducible.
- The project/adapter/run contracts are frozen for the 1.x compatibility window.

## 1.0 scope lock

### Included
- capability-composed objectives over one task, memory, governance, communications, and receipt model;
- one phone-accessible authenticated operator session sharing canonical lineage with desktop surfaces;
- Personal Operations capture, context resume, reminders, and bounded proactive assistance;
- useful optional local-inference/council participation with explicit non-approval semantics;
- a governed external-adapter and assimilation proof;
- Linux install and launcher;
- project attach/import;
- Rust, Python, JavaScript/TypeScript adapters;
- objective, run graph, authority preview, approval;
- bounded code/tool execution;
- diff and project-native verification;
- routing/provider/model/tool/cost provenance;
- checkpoint/restart/recovery;
- cited bounded research when requested;
- acceptance, rejection, revision, and exportable receipt bundle;
- local-first storage, backup/restore, diagnostics, and documented privacy controls.
- authoritative HUD projections for health, Workbench, recovery, Research, and Personal Operations;
- five independently claimable upper monitors with same-session workstation continuity;
- explicit loading, healthy, stale, partial, degraded, unavailable, and failed states;
- a measured HUD/runtime performance baseline with release-blocking regressions corrected.

### Not implied by 1.0
- fully autonomous company operation;
- unrestricted self-modification;
- support for every model/provider/language/OS;
- cloud synchronization or multi-tenancy;
- a clinically meaningful health system;
- general camera surveillance or biometric identity;
- Mirromere or CITADEL hardware as required components.

Any new feature proposed after scope lock must replace an equal or larger release risk, not merely add capability.

The master convergence plan's P0–P6, P9, and required P10 vertical proofs are
entry criteria for the corrected 1.0 release candidate. Optional payment,
validated-device, Mirromere, and Company distributions remain separately gated.

## Release workstream 1 — Contract and compatibility freeze

**Deliverables**
- supported 1.x contract/version policy;
- migration and deprecation rules;
- canonical schema fixtures;
- adapter conformance release suite;
- protocol fuzz/property tests where boundary risk warrants;
- documented event/replay and receipt guarantees.

**Gate S6-C1**
A 1.0-compatible adapter survives patch/minor upgrades according to policy, or receives a precise compatibility error without unsafe fallback.

The contract freeze also covers Tauri/HTTP request and response schemas,
backend-issued operator identity and mutation authority, stable lineage IDs, and
SSE cursor/reconnect semantics. React cannot mint approval decisions, canonical
run topology, completion, receipts, or aggregate health.

## Release workstream 2 — Final safety case

Document and verify:
- human authority and approval boundaries;
- tool/process/filesystem/network sandbox posture;
- model/provider trust assumptions;
- repository/web prompt-injection posture;
- data classification, retention, deletion, and export;
- secret handling;
- outpost and adapter trust/revocation;
- failure, cancellation, resume, and rollback semantics;
- optional telemetry and consent.

**Gate S6-S1**
Commission an independent code/security review of the release-critical path. Resolve all release-blocking findings and publish a summarized threat model and limitations.

## Release workstream 3 — Final quality matrix

Run the release artifact, not only source builds, against:
- every supported OS/hardware profile;
- local-only and remote-provider modes;
- each supported project adapter;
- clean install and upgrade from the release candidate;
- provider loss, network loss, process kill, disk pressure, corrupt tail, adapter crash, failed tests, cancellation, and operator rejection;
- backup/restore and rollback;
- long-running mixed workload soak.

**Gate S6-Q1**
There is no known reproducible path to silent mutation, false completion, unrecoverable project-source loss, secret disclosure, or receipt/run-state inconsistency in the supported matrix.

## Release workstream 4 — User outcome proof

Required evidence from one qualifying independent non-author evaluator:
- installation without author intervention;
- integration of an unseen supported repository;
- at least one accepted verified change;
- at least one rejected or revised run;
- recovery from one induced failure;
- qualitative understanding of approval and evidence surfaces.

**Minimum 1.0 evaluator gate**
- one qualifying evaluator completes the Stage 5 guide against the final artifact bytes;
- no author, agent, or proxy review is substituted for that evaluator;
- no unresolved critical usability blocker;
- findings and limitations are reported honestly.

Broader cohort and repeat-use evidence is valuable post-1.0 product evidence,
not a reason to keep the creator-owned 1.0 release permanently open.

## Release workstream 4A — HUD convergence and native acceptance

Execute the integration matrix one vertical workflow at a time after Stage 5
closes. Prove backend state → transport → rendering → operator action → backend
mutation → durable receipt → restart recovery. Accept authored HUD geometry and
the five monitor/workstation sessions in the native application. Keep World View
display-only.

Measure cold launch, first useful render, idle CPU/RSS, Three.js frame/GPU cost,
React commits, SSE visible latency, workstation opening latency, media growth,
duplicate polling, and long-session growth before optimizing.

## Release workstream 5 — Documentation and product identity

**Required documents/surfaces**
- concise product promise and limitations;
- installation and first-run guide;
- 15-minute first verified change tutorial;
- Workbench operator guide;
- project contract and adapter SDK reference;
- provider/model configuration;
- backup/restore/upgrade/rollback;
- privacy/security model;
- troubleshooting and diagnostics;
- architecture and receipt semantics;
- accessibility options;
- known issues and support channels;
- contributor and responsible-disclosure paths.

Remove or clearly label stale architecture, legacy Annunimas paths, prototype ports, and historical system claims from the default documentation route.

## Release workstream 6 — Distribution and lifecycle

**Deliverables**
- versioned signed artifacts and checksums;
- SBOM and third-party notices;
- reproducible build instructions;
- release channel and update policy;
- support window and compatibility promise;
- rollback artifact retention;
- vulnerability response and key rotation;
- opt-in crash/diagnostic process;
- end-of-life policy for 1.x.

## Release workstream 7 — Commercial foundation

1.0 does not require a mature company platform, but it requires a viable way to learn and earn without violating sovereignty.

**Deliverables**
- explicit open-source/product license decision;
- clear free versus paid/support boundaries if any;
- no hidden mandatory cloud dependency;
- consent-based feedback channel;
- paid pilot/support offer suitable for first external clients;
- pricing experiments tracked as proposals and outcomes, not invented revenue;
- operator capacity and support limits.

**Recommended initial offer**
A paid, operator-assisted Arda Workbench integration for selected software projects, with adapter setup, governed workflow design, and measurable delivery outcomes. This produces revenue and real product evidence without pretending 1.0 is a turnkey autonomous company.

## Release workstream 8 — First-party application status

At release freeze, classify every application as:
- **1.0 supported**;
- **beta, opt-in, data-compatible**;
- **preview, isolated, no support guarantee**;
- **research/prototype, not distributed**.

A preview cannot read private state or activate sensors merely because Workbench is installed. Extension failures never downgrade the core Workbench safety guarantees.

## Final gate commands

The exact closeout script should be created before RC freeze and must invoke the live, supported checks. Minimum source gates:

```bash
cargo metadata --no-deps --format-version=1
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-features -- --test-threads=1
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo deny check
pnpm --dir apps/arda-hud test
pnpm --dir apps/arda-hud lint
pnpm --dir apps/arda-hud build
pnpm --dir apps/arda-launcher test
pnpm --dir apps/arda-launcher lint
pnpm --dir apps/arda-launcher build
python3 scripts/hades_markdown_link_check.py --root docs --out /tmp/arda-1.0-links.json
```

Also require signed-artifact verification, install/upgrade/rollback matrix, adapter conformance, security regression, recovery/fault injection, backup/restore, and soak reports.

## Release evidence packet

Create immutable release artifacts at closeout:
- `docs/releases/1.0/RELEASE_EVIDENCE.md`
- `docs/releases/1.0/SUPPORTED_MATRIX.md`
- `docs/releases/1.0/KNOWN_LIMITATIONS.md`
- `docs/releases/1.0/SECURITY_AND_PRIVACY.md`
- `docs/releases/1.0/THIRD_PARTY_NOTICES.md`

Each claim references commands, artifact hashes, run IDs, evaluator evidence, or an explicit limitation.

## Stage 6 exit criteria

- [ ] 1.0 scope and compatibility policy are frozen. Scope is declared above;
  the source-identity and clean-tree freeze gate is implemented, but no final
  clean source revision or `1.0.0` artifact bytes have been selected.
- [ ] Signed release artifacts pass the supported matrix.
- [ ] Independent security review has no unresolved blocker.
- [ ] Clean install, upgrade, rollback, backup, and restore pass.
- [ ] Rust, Python, and JavaScript unseen-project integrations pass.
- [ ] Failure/recovery matrix has no silent mutation or false completion.
- [ ] The qualifying independent non-author evaluator gate passes.
- [ ] Frontend/backend contract convergence passes without React-owned authority.
- [ ] Five-monitor native visual/workstation acceptance passes.
- [ ] The performance baseline is published and release-blocking regressions are corrected.
- [ ] Documentation and accessibility gates pass.
- [ ] Licensing, third-party notices, support, and vulnerability processes exist.
- [ ] Every first-party application has an honest release classification.
- [ ] Known limitations are published without marketing inflation.

## Post-1.0 sequence

1. Stabilize the 1.x Workbench compatibility line.
2. Gather broader cohort and repeat-use evidence without rewriting 1.0 history.
3. Consider Company Operations pilots only when a real operator-selected engagement exists.
4. Advance Mirromere or RELIC/CITADEL only through a bounded master-plan
   decision; shipping those optional distributions may occur after the base.
5. Expand Warden watchlists and governed self-improvement only as audit evidence supports additional authority.

The 1.0 release is the point where Arda begins serving the creator reliably—not the point where the vision stops growing.
