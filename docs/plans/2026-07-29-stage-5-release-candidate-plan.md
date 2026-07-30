# Arda Stage 5 — Release Candidate Plan

> **For Hermes:** Begin only after Stage 4 evidence is complete. Stage 5 converts a useful private beta into an installable, supportable release candidate; it is not a feature-expansion sprint.

**Stage objective:** Prove Arda Workbench works for multiple independent users, repositories, providers, and failure conditions with supportable installation, upgrades, security boundaries, and documentation.

**Primary product:** Arda Workbench release candidate  
**Secondary betas:** Personal Operations and Warden Research  
**Optional projections:** Mirromere and RELIC/CITADEL, isolated from the release-critical path

---

## Entry criteria

- Stage 4 exit checklist and evidence packet are complete.
- No critical unresolved data-loss, duplicate-mutation, secret-exposure, or authority-bypass issue.
- Run graph and project contracts have a documented compatibility policy.
- One Rust and one Python adapter path are repeatable.

## Exit definition

Stage 5 is complete when a signed release candidate can be installed, upgraded, used, diagnosed, backed up, restored, and rolled back by independent evaluators across a supported environment matrix, while preserving run truth and project source.

## Workstream 1 — Supported product matrix

Declare and test:
- supported Linux distribution/version set;
- CPU/GPU minimum and recommended profiles;
- local-only and remote-provider modes;
- Rust, Python, and JavaScript/TypeScript project adapters;
- NVIDIA and CPU-only inference posture where applicable;
- optional outpost and projection features.

**Gate S5-M1**
Unsupported combinations fail with an honest compatibility message, not a partial silent install.

## Workstream 2 — Packaging, signing, upgrade, rollback

**Deliverables**
- reproducible release build;
- package/AppImage or declared canonical Linux distribution artifact;
- checksums and signature verification;
- config/state schema migrations;
- pre-upgrade backup;
- rollback to prior binary and compatible state;
- release notes generated from verified changes;
- SBOM and third-party license inventory.

**Gate S5-P1**
From the prior beta version: upgrade, complete a run, rollback, and restore state without losing project source or duplicating work.

## Workstream 3 — Adapter SDK and external project onboarding

**Deliverables**
- stable `arda.project-contract.v1` documentation;
- Rust, Python, and JavaScript reference adapters;
- adapter conformance suite;
- project template generator;
- capability/security guidance;
- compatibility matrix and deprecation rules;
- example repositories runnable in CI.

**Gate S5-A1**
An evaluator integrates an unseen repository by reading the SDK documentation, without editing Arda source or receiving private coaching.

## Workstream 4 — Security and privacy hardening

Threat-model and test:
- malicious repository instructions;
- prompt injection from web/evidence;
- path traversal and symlink escape;
- command and argument injection;
- child-process escape and cancellation;
- secret discovery/exfiltration;
- adapter impersonation/version downgrade;
- receipt tampering/replay;
- malicious or stale outpost;
- exposed HUD/harness endpoints;
- unsafe backup/diagnostic bundle.

**Deliverables**
- security model and trust boundaries;
- dependency and license scanning;
- secret handling/keyring integration;
- local endpoint authentication where exposure is possible;
- responsible disclosure channel;
- security incident and credential-rotation procedure.

**Gate S5-S1**
All critical/high findings are fixed or release-blocked with an explicit mitigation and owner; there is no undocumented exception.

## Workstream 5 — Reliability and performance

**Scenarios**
- 24-hour mixed-run soak;
- repeated cancellation/restart;
- provider failover;
- network loss;
- corrupted/truncated tail record;
- disk pressure;
- model timeout;
- adapter crash/hang/noisy output;
- large repository and test output;
- state compaction and retention.

**Budgets to define from measured baseline**
- cold start;
- idle resource use;
- HUD interaction latency;
- event-to-projection latency;
- run state growth;
- checkpoint/recovery duration;
- diagnostic bundle size;
- maximum unbounded queue/log growth: zero.

**Gate S5-R1**
No lost terminal state, duplicate mutation, unexplained completion, or unbounded state growth during the soak matrix.

## Workstream 6 — UX and accessibility

**Deliverables**
- first-run guided setup;
- sample/sandbox project;
- keyboard-complete Workbench path;
- screen-reader labels and focus management;
- reduced-motion/high-contrast options;
- clear model/provider/offline state;
- plain-language authority and approval explanations;
- “what happened?” and “what should I do?” recovery summaries;
- import/export and safe reset.

**Gate S5-U1**
Run moderated evaluations with at least five participants, including at least one technical user new to Arda and one accessibility-focused pass. Resolve all blocking usability findings.

## Workstream 7 — Support and operations

**Deliverables**
- installation, upgrade, backup/restore, troubleshooting, adapter, security, and architecture docs;
- structured/redacted diagnostics command;
- known-issues registry;
- release-blocker triage policy;
- crash/error reporting that is opt-in and locally inspectable;
- support response expectations for the first public cohort.

**Gate S5-O1**
A maintainer can diagnose the seeded failure set from a redacted bundle without requesting secrets or raw personal data.

## Workstream 8 — Secondary application betas

### Personal Operations
Private opt-in beta may ship if capture, reminder delivery truth, calendar conflict handling, encryption/redaction, and restore are independently proven. It cannot block Workbench startup.

### Warden Research
Recurring watchlists may ship if budget, pause, source policy, change detection, and contradiction-aware brief tests pass.

### Mirromere and RELIC/CITADEL
Experimental feature flags only. Sensor and kiosk failures must not affect Workbench. Hardware/device support is explicitly listed, not implied.

## Workstream 9 — External beta evidence

Recruit a small, consent-based cohort representing:
- existing Rust project;
- Python project;
- JavaScript/TypeScript project;
- local-model use;
- remote-provider use;
- CPU-constrained machine;
- at least one non-author operator.

Track:
- installation completion;
- time to first verified change;
- approval comprehension;
- run success/recovery;
- defects and severity;
- repeat usage;
- time saved or lost;
- reasons for abandonment.

Do not collect source, prompts, personal records, or telemetry by default.

## Stage 5 release gates

```bash
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
python3 scripts/hades_markdown_link_check.py --root docs --out /tmp/arda-stage5-links.json
```

Add artifact/signature, migration/rollback, adapter conformance, soak, and security scripts as they are implemented. Never list a gate as passing before the script exists and runs.

## Stage 5 evidence packet

Create `docs/releases/stage-5-release-candidate-evidence.md` at closeout with:
- release artifact identity/checksum/signature;
- environment/support matrix;
- upgrade/rollback and backup/restore evidence;
- security review and unresolved risk ledger;
- soak/performance results;
- adapter conformance results;
- external evaluator findings;
- accessibility findings;
- explicit 1.0 blockers.

## Exit criteria

- [ ] Signed, reproducible release candidate exists.
- [ ] Upgrade, rollback, backup, and restore pass.
- [ ] Rust, Python, and JavaScript adapter conformance passes.
- [ ] Independent unseen-project onboarding passes.
- [ ] Security threat-model gates pass.
- [ ] Reliability/soak budgets pass.
- [ ] Five-user evaluation has no blocking usability issue.
- [ ] Documentation and diagnostics are supportable.
- [ ] At least three external evaluators return for a second useful session.
- [ ] Remaining 1.0 blockers are finite, owned, and measurable.

## Stage 5 deferrals

- cloud control plane and multi-tenant SaaS;
- plugin marketplace with arbitrary third-party execution;
- autonomous financial/legal commitments;
- general consumer assistant claim;
- clinical or diagnostic sensor claims;
- “all languages and all operating systems” support.
