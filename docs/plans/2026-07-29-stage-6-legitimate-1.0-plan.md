# Arda Stage 6 — Legitimate 1.0 Plan

> **For Hermes:** Stage 6 is the release decision and proof phase. Do not expand scope to satisfy every long-term Arda vision. Ship the smallest complete, supportable promise and preserve later applications as compatible extensions.

**System completion coordination:** [Arda System Unification and Usability Plan](2026-08-02-arda-system-unification-and-usability-plan.md). This Stage 6 plan remains authoritative for 1.0 release gates; the coordination plan adds no product scope.

**Release promise:** Connect a supported software project to a sovereign AI development environment you own. Arda plans, researches when needed, executes bounded work, verifies it with the project's commands, explains the evidence, records every consequential action, and resumes safely.

**Product:** Arda Workbench 1.0 for the declared Linux support matrix  
**Optional first-party extensions:** Warden Research and Personal Operations beta; Mirromere, RELIC/CITADEL, and Company Operations preview unless they independently meet their plans.

---

## Entry criteria

- Stage 5 release-candidate evidence packet is complete.
- Every critical/high defect is resolved or explicitly release-blocking.
- External evaluators have completed repeated useful sessions.
- Installation, upgrade, rollback, backup, restore, and diagnostics are reproducible.
- The project/adapter/run contracts are frozen for the 1.x compatibility window.

## 1.0 scope lock

### Included
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

### Not implied by 1.0
- fully autonomous company operation;
- unrestricted self-modification;
- support for every model/provider/language/OS;
- cloud synchronization or multi-tenancy;
- a clinically meaningful health system;
- general camera surveillance or biometric identity;
- Mirromere or CITADEL hardware as required components.

Any new feature proposed after scope lock must replace an equal or larger release risk, not merely add capability.

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

Required evidence from independent users:
- installation without author intervention;
- integration of an unseen supported repository;
- at least one accepted verified change;
- at least one rejected or revised run;
- recovery from one induced failure;
- return usage after the first session;
- qualitative understanding of approval and evidence surfaces.

**Minimum 1.0 cohort gate**
- at least ten independent evaluators;
- at least five complete a second useful session;
- at least three use Arda on their own project over two or more weeks;
- no unresolved critical usability blocker;
- measured value is reported honestly, including users for whom Arda was slower or unsuitable.

These numbers are release evidence thresholds, not market-success claims.

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

- [ ] 1.0 scope and compatibility policy are frozen.
- [ ] Signed release artifacts pass the supported matrix.
- [ ] Independent security review has no unresolved blocker.
- [ ] Clean install, upgrade, rollback, backup, and restore pass.
- [ ] Rust, Python, and JavaScript unseen-project integrations pass.
- [ ] Failure/recovery matrix has no silent mutation or false completion.
- [ ] Ten-user evidence gate and repeat-use gate pass.
- [ ] Documentation and accessibility gates pass.
- [ ] Licensing, third-party notices, support, and vulnerability processes exist.
- [ ] Every first-party application has an honest release classification.
- [ ] Known limitations are published without marketing inflation.

## Post-1.0 sequence

1. Stabilize the 1.x Workbench compatibility line.
2. Turn repeated Workbench usage into Company Operations evidence and paid service/product experiments.
3. Mature Personal Operations around capture and recovery.
4. Add Mirromere as a consent-governed projection of the same orchestrator.
5. Add RELIC/CITADEL as read-only runtime presence.
6. Expand Warden watchlists and governed self-improvement only as audit evidence supports additional authority.

The 1.0 release is the point where Arda begins serving the creator reliably—not the point where the vision stops growing.
