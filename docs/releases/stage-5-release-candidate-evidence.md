# Stage 5 release-candidate evidence

Generated: 2026-08-05T00:49:08-07:00
Reconciled: 2026-08-10

## Verdict

**Not release-ready.** Stage 5 has passing RC0 upgrade/rollback, the valid
86,400-second S5-R1 soak, automated and native accessibility/performance, adapter
conformance, security scanning, support-exercise evidence, and a successful
tag-bound keyless signing workflow. Publication remains blocked by two required
receipts that do not exist for one final candidate identity: a reconciled signed
artifact lifecycle and a qualifying independent non-author evaluator pass. The
local-only endpoint posture, bounded GLib backport, and local-adapter trust model
remain explicit accepted support boundaries; they do not substitute for either
open release gate.

No row below converts a partial or blocked gate into a pass.

## Gate ledger

| Gate | Status | Evidence | Closure required |
|---|---|---|---|
| S5-RC0 artifact identity and upgrade/rollback | Pass | `../evidence/stage-5-release-candidate/s5-rc0/s5-rc0-summary.json`; `../evidence/stage-5-release-candidate/s5-rc0/rollback-proof.json` | Re-run against final rebuilt artifacts. |
| U4 local installation lifecycle | Pass for unsigned local candidate; final signed gate open | `../evidence/stage-5-release-candidate/reliability/u4-lifecycle-local-20260804.json` | Re-run fresh install, Workbench persistence, upgrade, backup/restore, rollback, diagnostics, safe reset, and uninstall against one reconciled final signed artifact identity. |
| Supported Linux profile | Pass for initial profile | `../evidence/stage-5-release-candidate/s5-rc0/supported-profile.json` | Maintain the declared `bluefin-lts-10-x86_64` boundary. |
| Reliability fault-matrix smoke | Pass | `../evidence/stage-5-release-candidate/reliability/u3-degradation-smoke-20260804.json` | 11/11 runs passed across the complete U3 failure matrix with zero state growth and unchanged source identity. |
| 24-hour soak / S5-R1 | Pass for source `efd118b5` | `../evidence/stage-5-release-candidate/reliability/soak-24h-final-efd118b5-20260807.json`; `../evidence/stage-5-release-candidate/reliability/soak-24h-final-efd118b5-20260807-assessment.md` | The valid run completed 86,400 seconds and passed 2,844/2,844 scenarios. The later `6616addd` smoke passed 11/11, but its stopped elapsed run emitted no receipt and is not cited. Re-run at the next freeze only if then-current release policy requires exact-candidate elapsed qualification. |
| Automated performance | Pass | `../evidence/stage-5-release-candidate/reliability/performance-accessibility.json` | Re-run after final artifact build. |
| Automated accessibility | Pass | `../evidence/stage-5-release-candidate/reliability/performance-accessibility.json` | Two automated Workbench tests passed; critical/serious axe findings were zero. |
| Native performance/accessibility acceptance | Pass | `../evidence/stage-5-release-candidate/reliability/performance-accessibility.json`; `../evidence/stage-5-release-candidate/reliability/native-launcher-walkthrough.json` | X11 cold-start, sustained-idle, AT-SPI/key-event keyboard, setup/recovery, and focus-containment acceptance passed. |
| Security scans and threat regressions | Pass with owned support boundaries | `../evidence/stage-5-release-candidate/security/security-summary.json`; `../evidence/stage-5-release-candidate/security/glib-0185-backport-spike-20260805.md`; `../evidence/stage-5-release-candidate/security/tauri-gtk-migration-preflight-20260805.md`; `../security/stage-5-threat-model.md` | SEC-GLIB-001 is mitigated by the checksum-pinned exact upstream fix in a local `glib 0.18.5` path dependency. The rejected GTK4 fork remains rejected. Path dependencies are omitted by RustSec scanners, so the archive/source verifier and optimized regression are mandatory. |
| Adapter SDK/conformance | Pass | `../evidence/stage-5-release-candidate/adapters/conformance.json`; `../evidence/stage-5-release-candidate/adapters/external-repository-onboarding.json` | Repeat the conformance suite on final source. The separately sourced onboarding gate is closed. |
| Support exercise | Pass | `../evidence/stage-5-release-candidate/support/support-exercise.json`; `../operator/stage-5-support-policy.md` | Repeat against final package if diagnostics format changes. |
| Packaging, SBOM, checksums, signatures | Partial | `../evidence/stage-5-release-candidate/packaging/packaging-summary.json`; [release `v0.3.0-rc.0`](https://github.com/dward1502/Arda/releases/tag/v0.3.0-rc.0); [signing run 30714060617](https://github.com/dward1502/Arda/actions/runs/30714060617) | The remote tag-bound workflow signed and verified all six allowlisted assets. Reconcile local packaging evidence and repeat release lifecycle proof against that exact signed artifact set. |
| Final release approval | Blocked | This ledger | All critical/high blockers and required acceptance evidence must close. |

## Verified evidence summary

### RC0 upgrade and rollback

The RC0 receipt reports:

- distinct Stage 4 and candidate artifact hashes;
- reproducible release manifest and checksum generation;
- candidate version `0.3.0-rc.0` on profile `bluefin-lts-10-x86_64`;
- machine-readable rollback restoring the Stage 4 artifact;
- terminal run truth preserved;
- secret state preserved without archiving it; and
- no source-repository mutation.

The U4 local lifecycle rerun additionally passed fresh install, native startup,
upgrade, backup, redacted diagnostics, rollback, terminal-run persistence,
uninstall, post-uninstall state preservation, and unchanged source identity on
the declared Bluefin LTS profile. This receipt uses the current locally built
executable and explicitly records `final_signed_artifact_exercised=false`; it is
implementation evidence, not closure of the final artifact gate. A narrow
NVIDIA/Wayland explicit-sync startup guard resolved the supported host's default
Wayland protocol-error exit without requiring a user-provided launch override.

### Reliability, performance, and accessibility

The U3 smoke receipt passed 11 of 11 runs across checkpoint restart, corrupted
journal tail, cancellation, operator rejection, provider loss, model timeout,
external process kill/restart, adapter crash, network loss, disk pressure, and
large/noisy output. Protected state growth was zero files and zero bytes.

The hardened evaluator has 11/11 focused unit tests passing. The current smoke
made 11/11 successful attempts with every scenario represented, unchanged
source identity, zero protected-state growth, and preserved disk headroom. The
final-source 24-hour run from `efd118b5` then passed 2,844/2,844 executions across
the same eleven scenarios. Its launch identity matches the receipt's unchanged
before/after source fingerprint; protected-state growth was zero, every latency
budget held, and the 64-GiB disk floor was preserved. Nonzero exits remain
fail-closed with explicit scenario/root-cause attribution and bounded redacted
diagnostics; successful exact filters that select zero tests are rejected as
false passes. Release-only commit `6616addd` aligns the candidate at
`0.3.0-rc.1`; its 11/11 smoke passed. The replacement final-source soak began
at 2026-08-07T06:57:37Z and was intentionally stopped at
2026-08-07T07:28:51Z. It emitted no final receipt and is not release evidence.
The completed `efd118b5` run and current smoke are accepted for continued
development; exact final-source qualification is deferred until an actual
public release requires it.

Automated performance remained within declared budgets:

- maximum observed fault-matrix p95: 356.148 ms (budget 1,000 ms);
- checkpoint restart: 284.754 ms (budget 1,000 ms);
- HUD JavaScript assets: 2,140,046 bytes (budget 3,145,728 bytes); and
- largest redacted diagnostic bundle: 1,530 bytes (budget 1,048,576 bytes).

Automated accessibility passed both tested Workbench states, keyboard focus
order, the four-part plain-language operator summary, failed-run authority and
recovery guidance, reduced-motion, increased-contrast, and forced-colors
assertions. Separate native AT-SPI/XTest acceptance passed dialog naming,
keyboard containment, Escape closure, focus restoration, and setup/recovery
exposure; jsdom alone is not cited as rendered contrast or screen-reader proof.

### Security

Completed controls include:

- loopback-only harness binding while inbound authentication is absent;
- exact backup manifest/member equality and traversal/symlink rejection;
- restrictive launcher CSP with packaged-config regression coverage;
- adapter handshake and result-provenance identity/version pinning;
- malicious instruction/argument-injection regression coverage;
- bounded adapter output, timeout, cancellation, and child reaping;
- approval and exported-tool-evidence enforcement; and
- zero current production npm audit findings plus recorded Rust advisory output.

### Adapter conformance

The consolidated receipt passed the Python protocol suite, engine JSONL process
boundary, Rust SDK, JavaScript SDK, and isolated Python/Rust golden projects. The
Python golden repository is isolated from the Cargo workspace but remains
repository-authored, so it does not by itself satisfy the separately sourced
unseen-repository criterion. That criterion is now closed independently by the
`BurntSushi/byteorder` receipt: the documented Rust template produced a valid
contract, its declared `cargo test` command passed 802 tests with zero failures,
and only the external clone's `arda-project.json` was added. The receipt does not
claim non-author usability acceptance.

### Support

Three seeded incidents—missing launcher, native dependency failure, and missing
provider endpoint—were diagnosed from redacted bundles alone. All bundles passed
the secret-leak check and retained machine-verifiable hashes. Operator guidance
now pairs internal identities with functional labels and states the supported
profile, local-only authentication boundary, diagnostic-sharing caution, and
known limitations. U5 implementation and automated/native infrastructure gates
pass. The neutral protocol and fail-closed record template are prepared in
`docs/operator/stage-5-independent-evaluator-guide.md` and
`docs/operator/templates/stage-5-independent-evaluator-record.json`, but no
qualifying non-author session has occurred, so comprehension acceptance remains
open.

### Packaging

The local packaging evidence records a 642-component SBOM with zero missing
dependency licenses and byte-identical AppImage/normalized DEB/RPM proof. The
published `v0.3.0-rc.0` release has all six allowlisted assets plus detached
keyless Sigstore bundles; GitHub Actions run `30714060617` completed its
identity-bound download, checksum verification, signing, verification, and upload
steps successfully. The remote asset identities differ from the older local
packaging summary, so that summary cannot alone prove the published artifact
lifecycle; regenerate one reconciled evidence packet before promotion.

The 2026-08-06 reconciliation packet is
`docs/evidence/stage-5-release-candidate/packaging/remote-signed-artifact-reconciliation-20260806.json`.
It verifies the remote checksum ledger and all six Sigstore bundles, but fails
closed for final-source promotion: the published tag and manifest bind to
`28cde28b9fd97fdc91de36418da872b1628ae41b`, the manifest records
`tracked_worktree_clean=false`, and the frozen final source is
`efd118b5339f42df133fdfb9d3256c64a02b7e59`. The signed bytes are authentic
published RC bytes, not final-source Stage 5 evidence.

The separate Bluefin RELR packaging defect is repaired in commit `30d0a5f`.
The fail-closed helper pins and verifies appimagetool 1.9.1 and the type-2
runtime, and two real outputs from the populated RELR-era AppDir are
byte-identical at
`7ce0900826d1e099879f6c76c0bed104123eca38bb0dd8cb6ccff85ff1d025bf`.
`python3 -m unittest tests.test_arda_appimage tests.test_arda_release_ops`
passes 6/6. This closes the package-tool implementation defect; the final
source/artifact reconciliation, signing, and installed lifecycle remain owned
by `PKG-EVIDENCE-001`.

## Owned blockers and accepted/deferred boundaries

| ID | Severity | Owner | Required closure |
|---|---|---|---|
| REL-SOAK-002 | Deferred policy check; not a current blocker | Reliability maintainer | S5-R1 passed on `efd118b5`. The operator stopped the later `6616addd` elapsed run and removed it from the development critical path. If policy at the next release freeze requires exact-candidate qualification, start a fresh uninterrupted run; do not resume or reinterpret the stopped launch. |
| USABILITY-EVAL-001 | Release gate | UX/release maintainer | The neutral guide and fail-closed record template are prepared. Have a clean non-author evaluator identify current system state, approval authority, evidence quality, and the next recovery action without source or raw-state access; retain the completed evaluator receipt rather than substituting author or agent self-review. |
| SEC-AUTH-001 | High if remotely exposed | Engine/harness maintainer | Keep all harness binds loopback-only; implement inbound authentication before remote or multi-user exposure. |
| SEC-ADAPTER-001 | High for third-party adapters | Adapter runtime maintainer | Pin adapter artifact digest/signature; self-reported identity/version alone cannot defeat a malicious replacement binary. |
| SEC-GLIB-001 | High advisory; mitigated | Launcher maintainer | Both Tauri consumers resolve the checksum-pinned vendored `glib 0.18.5` with the exact upstream `&mut` fix. Preserve the verifier/regression controls and replace the exception with maintained upstream GTK4 when available. |
| PKG-EVIDENCE-001 | Release gate | Release maintainer | Remote `v0.3.0-rc.0` checksums and all six signatures verify, but its source identity is seven commits behind `efd118b5` and its manifest records a dirty worktree. At the next actual freeze, select one exact clean source identity, satisfy any then-current elapsed-soak policy, publish a tag-bound prerelease, reconcile its exact signed bytes, and run the U4 lifecycle against them. |


## Final approval rule

Stage 5 may be marked complete only after all required receipts are regenerated
from the same final source/artifact identity, all release-gate rows above pass,
and every critical/high item is either closed or accepted through the documented
release policy by the named owner. Until then, the candidate remains local-only
and non-promotable.
