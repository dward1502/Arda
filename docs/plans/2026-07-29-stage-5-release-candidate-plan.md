# Arda Stage 5 — Release Candidate Plan

> **For Hermes:** Stage 5 is active following operator acceptance of the Stage 4 evidence on 2026-07-31. Stage 5 converts a useful private beta into an installable, supportable release candidate; it is not a feature-expansion sprint.

**Product/master authority:** [Arda 1.0 Personal Agent Ecosystem Plan](2026-08-08-arda-1.0-personal-agent-ecosystem-plan.md). This Stage 5 plan remains authoritative for Workbench-centered release-candidate packaging, security, reliability, diagnostics, support, and evaluator gates—not product identity.

**System completion coordination:** [Arda System Unification and Usability Plan](2026-08-02-arda-system-unification-and-usability-plan.md).

**Lifecycle status:** ACTIVE as of 2026-07-31.

**Stage objective:** Prove Arda Workbench works across the declared supported profiles, repositories, providers, and failure conditions with supportable installation, upgrades, security boundaries, and documentation. One qualifying independent non-author evaluator receipt is required; broader external-person repetition remains optional.

**Primary product:** Arda Workbench release candidate  
**Secondary betas:** Personal Operations and Warden Research  
**Optional/not Stage 5 blockers:** Mirromere and RELIC/CITADEL feature
expansion; retained operations remain isolated from this release-critical path

## Reconciled Stage 5 authorities and dependencies — 2026-07-31

| Surface | Authority | Stage 5 relationship |
|---|---|---|
| Workbench release candidate | this plan | only release-critical product |
| Warden/Varda durable backend | [governed-learning plan](../archive/2026-07-27-warden-varda-ceo-learning-loop.md) | completed supporting authority; required only if the optional Warden Research recurring-watchlist beta ships |
| Warden Research product/API/HUD | [archived Warden Research record](../archive/2026-07-29-warden-research-application-plan.md) | completed optional secondary beta record; not release-critical |
| Pi5 AArch64 deployment, fleet, SSH, shared recovery | [Pi5 record](../archive/2026-07-23-pi5-outpost-integration-plan.md) | completed supporting infrastructure; required only for a beta that promises physical outpost support |
| RELIC/CITADEL presence/presentation | [retained RELIC/CITADEL operations](../operations/relic-citadel-presence.md) | existing recovery/soak history is nonblocking; feature expansion is an optional master-plan capability, not a Stage 5 gate |

No deferred application/outpost task may delay Workbench packaging, security, reliability, diagnostics, or support work.

---

## Entry criteria

- Maturity: `workflow_proven` — the Stage 4 exit checklist and evidence packet are complete under the operator-approved local-only profile documented in the [archived Stage 4 plan](../archive/2026-07-29-stage-4-private-beta-plan.md); independent evaluator evidence is optional supplementary validation.
- No critical unresolved data-loss, duplicate-mutation, secret-exposure, or authority-bypass issue.
- Run graph and project contracts have a documented compatibility policy.
- One Rust and one Python adapter path are repeatable.

## Exit definition

Stage 5 is complete when a signed release candidate can be installed, upgraded, used, diagnosed, backed up, restored, and rolled back across the declared supported environment matrix while preserving run truth and project source. Automated isolated-profile evidence, operator acceptance, and one qualifying independent non-author evaluator receipt are active gates. Broader external-person repetition remains optional.

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

**Initial supported profile — declared 2026-07-31**

- `bluefin-lts-10-x86_64`: Linux `x86_64`, `/etc/os-release` `ID=centos`,
  `VERSION_ID=10`, and `PRETTY_NAME` beginning with `Bluefin LTS`.
- No other distribution, version, or architecture is claimed by S5-RC0.
- `python3 scripts/arda_beta_ops.py compatibility --root "$ARDA_ROOT" --home "$HOME"`
  is the machine-readable preflight. Install, release-manifest, and upgrade
  commands run the same preflight before mutating installation paths.

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
The conformance fixture integrates a previously unseen repository from the published SDK documentation without editing Arda source. A later external-person walkthrough is desirable but non-gating while no separate evaluator is available.

**Closed 2026-08-05.** A separately sourced checkout of
`BurntSushi/byteorder` at commit
`5a82625fae462e8ba64cec8146b24a372b4d75c6` was onboarded from
`sdk/README.md` with the published Rust contract template, schema-validated,
and executed with the declared `cargo test` command: 802 tests passed and zero
failed without editing Arda source. The machine receipt is
`docs/evidence/stage-5-release-candidate/adapters/external-repository-onboarding.json`.

## Workstream 4 — Security and privacy hardening

### Execution status — 2026-07-31

- Added `SECURITY.md` and `docs/security/stage-5-threat-model.md` with trust
  boundaries, disclosure, incident response, credential rotation, and owned
  release blockers.
- Hardened beta restore so archive regular files must exactly match the
  manifest; unlisted-file injection now fails closed.
- Prohibited every non-loopback harness bind until inbound authentication is
  implemented; mutation routes retain loopback peer validation.
- Added a restrictive launcher CSP with a packaged-config regression test.
- Pinned project-adapter handshake and result provenance to the configured
  adapter identity/version, and added malicious instruction/argument-injection
  regression coverage.
- Fixed the original production npm advisory by updating `postcss` from
  `8.5.16` to `8.5.18`, then closed the follow-up
  `GHSA-fxqj-rqcc-2cmp` / `CVE-2026-69153` by advancing both Tauri applications
  to `postcss >=8.5.23`.
- Fixed `RUSTSEC-2024-0437` through `prometheus 0.14.0` / `protobuf 3.7.2`
  and fixed `RUSTSEC-2026-0221` with `event-listener 5.4.2`.
- Final scans report zero Cargo vulnerabilities and zero production npm
  advisories. `RUSTSEC-2024-0429` is mitigated by a checksum-pinned vendored
  `glib 0.18.5` containing only the exact upstream `VariantStrIter` fix; because
  RustSec scanners omit path dependencies, the source verifier and optimized
  regression are mandatory. Eighteen unmaintained transitive-package warnings
  remain inventory follow-up.
- Machine evidence: `docs/evidence/stage-5-release-candidate/security/`.
- Added the missing top-level MIT `LICENSE`; final signed artifacts remain stale
  until rebuilt after all Stage 5 source and lockfile changes.
- The isolated GTK4/Tauri fork candidate compiled and passed focused native and
  frontend gates, but independent review rejected it: the Tao/Wry/Muda deltas
  contain untested event-loop, unsafe-lifetime, IPC/session, and display-backend
  changes, and the configured tray-icon commit is unavailable. Candidate
  `fd58583` remains isolated and must not be integrated or signed. Stage 5 instead
  uses the bounded, checksum-pinned upstream `glib 0.18.5` safety backport; see
  `docs/evidence/stage-5-release-candidate/security/glib-0185-backport-spike-20260805.md`.

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

**Reliability disposition (2026-08-07):** The uninterrupted 24-hour run from
commit `efd118b5` passed 2,844 of 2,844 scenario executions across all eleven
failure classes. Source identity remained unchanged, protected-state growth was
zero, every latency budget held, and the 64-GiB disk floor was preserved. The
receipt
`docs/evidence/stage-5-release-candidate/reliability/soak-24h-final-efd118b5-20260807.json`
and its adjacent assessment close S5-R1 for that source identity. The necessary
release-version-only commit `6616addd` supersedes it as the `0.3.0-rc.1`
candidate. Its 11/11 smoke passed. The operator then stopped the replacement
86,400-second run and removed this elapsed-time gate from the current
development critical path. The stopped run under
`~/.local/state/arda/stage5-final-6616addd/` is not release evidence. Existing
reliability evidence supports continued implementation; a new final-source
qualification run is needed only if a later public-release policy requires it.

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
Complete automated keyboard/focus/accessibility checks and an operator-run guided setup/recovery walkthrough; resolve all blocking findings. One qualifying independent non-author evaluator must pass the final comprehension gate; moderated multi-person evaluation remains optional supplementary validation.

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
Recurring watchlists may ship only if governed-learning GL-1 through GL-4 and Warden Research WR-1 through WR-3/WR-5 pass budget, pause, source policy, replay, change-detection, contradiction, and prompt-injection gates. Physical Warden support additionally requires Pi5 PI5-1.

### Mirromere and RELIC/CITADEL
Experimental feature flags only. Sensor and kiosk failures must not affect Workbench. Hardware/device support is explicitly listed, not implied. RELIC/CITADEL RC-5 companion/collaboration expansion is deferred until RC-1 through RC-4 independently pass.

## Workstream 9 — Independent evaluator and optional broader beta evidence

One qualifying independent non-author evaluator receipt is release-gating under
`docs/operator/stage-5-independent-evaluator-guide.md`; do not fabricate proxy
sign-off. The broader cohort below remains supplementary and does not block
Stage 5. Continue collecting machine-readable isolated-profile, conformance,
recovery, security, and operator-acceptance evidence independently.

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
- one independent non-author evaluator receipt covering state, authority, evidence quality, and recovery comprehension; broader five-user evaluation remains optional;
- accessibility findings;
- explicit 1.0 blockers.

## Exit criteria

- [ ] Signed, reproducible release candidate exists.
- [x] Upgrade, rollback, backup, and restore pass.
- [x] Rust, Python, and JavaScript adapter conformance passes.
- [x] Automated unseen-project SDK/conformance onboarding passes; external-person repetition is optional.
- [x] Security threat-model gates pass. SEC-GLIB-001 is mitigated by the bounded,
  checksum-pinned upstream backport documented in
  `docs/evidence/stage-5-release-candidate/security/glib-0185-backport-spike-20260805.md`;
  its path-dependency verifier and optimized regression remain mandatory.
- [ ] Reliability/soak budgets pass.
- [x] Automated accessibility checks and operator setup/recovery walkthrough have no blocking usability issue.
- [x] Documentation and diagnostics are supportable.
- [ ] Independent evaluator status is documented honestly; the release remains blocked until one qualifying receipt passes `docs/operator/stage-5-independent-evaluator-guide.md`.
- [x] Remaining 1.0 blockers are finite, owned, and measurable.

## Stage 5 deferrals

- cloud control plane and multi-tenant SaaS;
- plugin marketplace with arbitrary third-party execution;
- autonomous financial/legal commitments;
- general consumer assistant claim;
- clinical or diagnostic sensor claims;
- “all languages and all operating systems” support.

## Smallest release-critical implementation tranche — S5-RC0

Implement **artifact identity plus upgrade/rollback proof** before any optional application expansion.

**Why this is smallest:** Stage 4 already proved native startup, isolated-profile install, backup/restore/reset/uninstall, durable Workbench runs, and restart recovery. Stage 5's first new release-critical gap is turning those pieces into one versioned candidate lifecycle rather than adding product features.

**Owned work**

1. [x] Declare one initial supported Linux profile and fail unsupported profiles with an explicit compatibility result.
2. [x] Extend the existing launcher/beta operations tooling to emit a release manifest containing artifact identity, SHA-256 checksums, build/toolchain inputs, config/state schema versions, and rollback compatibility.
3. [x] Build a candidate artifact, take a pre-upgrade backup, install it over the Stage 4 beta layout, run one deterministic Workbench fixture, then roll back the binary/state and restore.
4. [x] Assert project source is untouched, terminal run truth is preserved, and no mutation or receipt is duplicated across upgrade/rollback.
5. [x] Write machine-readable evidence under `docs/evidence/stage-5-release-candidate/` and link it from the eventual Stage 5 evidence packet.

**Acceptance**

- [x] The manifest and checksum ledger reproduce byte-identically from the same declared inputs. The native candidate embeds its `0.3.0-rc.0` release identity and differs from the Stage 4 binary.
- [x] Unsupported-profile detection fails before partial installation; covered by `test_cli_install_rejects_unsupported_profile_before_partial_install`.
- [x] Upgrade → deterministic run → rollback → restore completes without source loss, terminal-state loss, or duplicate mutation.
- [x] Diagnostics identify the installed artifact, schema, backup, and rollback result without exposing secrets or source content.

**Closed 2026-07-31.** Authoritative machine-readable proof is
`docs/evidence/stage-5-release-candidate/s5-rc0/s5-rc0-summary.json`; the same
directory contains the release manifest, `SHA256SUMS`, compatibility result,
installed-candidate diagnostics, exact-once Workbench result, and rollback
receipt. S5-RC0 closure unblocks the remaining packaging, security,
reliability/accessibility, adapter, and support workstreams; it does not mark
those later gates complete.

Adapter expansion and the 24-hour soak may now proceed. Optional Warden and
RELIC/CITADEL expansion remain outside the Workbench release-critical path.

## Packaging and reproducibility tranche — S5-P0/P1

**Packaging/reproducibility and publisher-identity design complete
2026-08-01.** The release tooling, runbook, license text, machine evidence, and
tag-bound keyless Sigstore workflow are present. Publication remains blocked on
remote repository hardening and a successful signing run over the final
normalized artifacts.

- [x] Built the `0.3.0-rc.0` AppImage, DEB, and RPM from the live launcher.
- [x] Generated a deterministic multi-artifact manifest and `SHA256SUMS` with
  `scripts/arda_release_ops.py`; release identity is
  `sha256:c80fda9abb2e980e4c69f18cd1984dc8772889f7fa49c527782637c864bbc096`.
- [x] Generated a launcher-reachable Cargo/pnpm inventory: 642 components,
  zero missing package-license declarations.
- [x] Proved the canonical AppImage byte-identical across two builds using
  `SOURCE_DATE_EPOCH` and pinned `appimagetool` 1.9.1.
- [x] Repaired the Bluefin RELR packaging failure with a fail-closed direct
  AppImage assembly path that pins and verifies appimagetool 1.9.1 plus the
  type-2 runtime. Two real outputs from the populated RELR-era AppDir were
  byte-identical at
  `7ce0900826d1e099879f6c76c0bed104123eca38bb0dd8cb6ccff85ff1d025bf`;
  six focused packaging/release-ops tests pass. This closes the toolchain defect,
  not the final-source artifact/signing/lifecycle gate.
- [x] Signed every package, SBOM, manifest, and checksum ledger with detached
  Cosign bundles and verified each bundle against the retained RC public key.
- [x] Documented build, verification, key-custody, rotation, and license
  exception policy in
  `docs/operator/release-candidate-packaging-signing.md`.
- [x] Made DEB and RPM byte-reproducible with
  `scripts/arda_reproducible_packages.py`. The original failing A/B pairs
  normalize to byte-identical outputs, and both normalized RPMs pass header and
  payload verification with `rpm -Kv`.
- [x] Added the repository's top-level MIT `LICENSE` text, aligned with the
  workspace package metadata.
- [x] Selected production publisher trust as tag-bound keyless Sigstore using
  GitHub OIDC; added a SHA-pinned workflow, exact asset allowlist, tag/manifest
  checks, identity-bound bundle verification, policy, and decision receipt.
- [x] Activated production trust remotely: the workflow landed through review,
  `production-release` requires maintainer approval, `main` is protected, and
  the first tag-bound signing run passed. The final-source artifact set still
  requires a new signing run and remains owned by the global signed-release exit
  gate and U4 rather than reopening remote trust activation.

Authoritative machine evidence is
`docs/evidence/stage-5-release-candidate/packaging/packaging-summary.json` and
`docs/evidence/stage-5-release-candidate/packaging/linux-package-reproducibility.json`,
`docs/evidence/stage-5-release-candidate/packaging/appimage-relr-preflight-20260805.md`,
and
`docs/evidence/stage-5-release-candidate/packaging/publisher-identity-decision.json`,
with the release manifest, checksums, SBOM, public key, and Sigstore bundles in
the same directory. The repaired package bytes supersede the pre-normalization
DEB/RPM candidate signatures; the global signed-release exit item remains open
until the final normalized artifacts are signed by the tag-bound workflow and
pass the installed lifecycle. The 2026-08-06 remote reconciliation verifies all
six published `v0.3.0-rc.0` signatures but fails closed because those assets bind
to source `28cde28b` with `tracked_worktree_clean=false`, not frozen final source
`efd118b5`; see
`docs/evidence/stage-5-release-candidate/packaging/remote-signed-artifact-reconciliation-20260806.json`.

## Reliability, performance, and accessibility tranche — S5-R1/U1

**S5-R1 accepted for continued development; final `0.3.0-rc.1` qualification deferred on 2026-08-07.** The bounded runner rotates eleven exact
registered tests for repeated cancellation, operator rejection, provider loss,
external process kill/restart, adapter crash, model timeout, oversized/noisy
output, network loss, disk-pressure write failure, corrupted journal tail, and
checkpoint restart. The U3 all-scenario smoke passed 11/11 with zero
protected-state growth. Three authoritative 24-hour runs completed but failed:
the first recorded 40 nonzero exits in 2,494 attempts, the 2026-08-03 rerun
recorded 132 nonzero exits in 2,827 attempts despite a final 17.34-hour
failure-free tail, and `soak-24h-u3-20260805.json` completed 2,850/2,851 attempts
with one genuine model-timeout fixture-startup failure. All remain failed
evidence and cannot be reinterpreted as passes.

The runner now retains a bounded redacted diagnostic tail and explicit root
cause, scenario, command, duration, and output digest for each failure; records
and periodically verifies a source-tree fingerprint; uses a dedicated
`CARGO_TARGET_DIR`; rejects exact filters that select zero tests; and invalidates
the run if a 64-GiB free-space floor is breached. Its evaluator has 11/11
focused unit tests passing. The valid U3 smoke exercised all eleven scenarios
with unchanged source identity and preserved capacity. The incomplete
eight-scenario `20260804-v2` run was stopped without a receipt rather than being
misrepresented as U3 evidence. After correcting the fixture startup race, a
fresh 86,400-second run from commit `efd118b5` completed at
2026-08-07T05:51:54Z. Its valid receipt passed all 2,844 scenario executions with
zero failures, unchanged source identity, zero protected-state growth, preserved
latency budgets, and preserved disk headroom. Finite protected-state growth
ceilings remain 1,000 files and 64 MiB. Because the matching release tag must be
new, release-only commit `6616addd` aligns package, Cargo, lockfile, and Tauri
versions at `0.3.0-rc.1`. Its 11/11 all-scenario smoke passed with unchanged
source identity and zero protected-state growth. A new 86,400-second run began
at 2026-08-07T06:57:37Z from that clean commit; its launch and eventual receipt
are retained under `~/.local/state/arda/stage5-final-6616addd/`. The operator
stopped that replacement run after its green smoke and removed it from the
current development critical path. It emitted no final receipt and must not be
cited as a completed release soak.

Automated Workbench accessibility passed 2/2 for critical/serious axe findings
and keyboard focus order. Reduced-motion, increased-contrast, and forced-color
CSS paths are present. HUD lint, full tests, and build pass. The native X11
walkthrough now passes: cold start was 0.706 seconds, five-minute aggregate RSS
peaked at 440.78 MiB and ended at 422.0 MiB, post-warmup growth was 4.2 MiB, and
WebKit consumed 2.58% of one CPU core. AT-SPI plus physical X11 key events proved
dialog naming, setup/recovery text, initial focus, Tab/Shift+Tab containment,
Escape close, and focus restoration. U1 remains complete. S5-R1 is sufficient
for continued development based on the completed `efd118b5` run and the
`6616addd` smoke; exact final-source release qualification is deferred.

Evidence:

- `docs/evidence/stage-5-release-candidate/reliability/soak-smoke-final.json`
- `docs/evidence/stage-5-release-candidate/reliability/soak-24h.json` (completed failed run; retained)
- `docs/evidence/stage-5-release-candidate/reliability/soak-24h-rerun-20260802.json` (not emitted; interrupted before a valid receipt because `/var` reached 100% capacity)
- `docs/evidence/stage-5-release-candidate/reliability/soak-24h-rerun-20260803.json` (completed failed run; retained)
- `docs/evidence/stage-5-release-candidate/reliability/u3-degradation-smoke-20260804.json` (valid 11/11 U3 matrix smoke)
- `docs/evidence/stage-5-release-candidate/reliability/soak-24h-u3-20260805.json` (completed failed run; retained)
- `docs/evidence/stage-5-release-candidate/reliability/soak-24h-final-efd118b5-20260807.json` (valid final-source pass)
- `docs/evidence/stage-5-release-candidate/reliability/soak-24h-final-efd118b5-20260807-assessment.md`
- `docs/evidence/stage-5-release-candidate/reliability/performance-accessibility.json`
- `docs/evidence/stage-5-release-candidate/reliability/native-launcher-walkthrough.json`
- `docs/evidence/stage-5-release-candidate/reliability/native-launcher-metrics.json`

## Adapter and support tranche — S5-A1/O1

**Complete 2026-08-01.** `scripts/arda_adapter_ops.py` generates deny-by-default
Rust, Python, and JavaScript project contracts and emits one conformance receipt.
All three examples validate; the executable Python reference server, Rust engine
JSONL boundary, and isolated Python/Rust golden repositories pass. Dedicated
Rust and JavaScript SDK packages under `sdk/rust` and `sdk/javascript` provide
bounded 64-KiB JSONL framing, fail-closed envelope/version validation, and
deny-by-default capability negotiation. Their focused suites pass 4/4 each and
the regenerated cross-language conformance receipt passes every command. Python
remains the complete executable reference server; the Rust and JavaScript
packages intentionally provide the published bounded SDK surface rather than
claiming additional adapter execution semantics.

S5-O1 passes its bounded exercise: three isolated failures were diagnosed from
redacted bundle contents alone (missing launcher, native dependency failure,
and missing provider endpoint), with zero secret/path canary leaks. The support
policy defines release-blocker severity, first-cohort response targets,
opt-in-only diagnostics, maintainer workflow, and the current known-issue ledger.

Evidence:

- `docs/evidence/stage-5-release-candidate/adapters/conformance.json`
- `docs/evidence/stage-5-release-candidate/support/support-exercise.json`
- `docs/operator/stage-5-support-policy.md`
