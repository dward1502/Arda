# Stage 5 release-candidate evidence

Generated: 2026-07-31T23:34:43-07:00

## Verdict

**Not release-ready.** Stage 5 has passing RC0 upgrade/rollback, bounded smoke,
automated and native accessibility/performance, adapter conformance, security
scanning, support-exercise evidence, and a successful tag-bound keyless signing
workflow. Publication remains blocked by the failed long-soak, release-evidence
reconciliation to the remotely signed artifacts, the Tauri/GTK GLib advisory, and
the explicit local-only security boundary.

No row below converts a partial or blocked gate into a pass.

## Gate ledger

| Gate | Status | Evidence | Closure required |
|---|---|---|---|
| S5-RC0 artifact identity and upgrade/rollback | Pass | `../evidence/stage-5-release-candidate/s5-rc0/s5-rc0-summary.json`; `../evidence/stage-5-release-candidate/s5-rc0/rollback-proof.json` | Re-run against final rebuilt artifacts. |
| Supported Linux profile | Pass for initial profile | `../evidence/stage-5-release-candidate/s5-rc0/supported-profile.json` | Maintain the declared `bluefin-lts-10-x86_64` boundary. |
| Reliability fault-matrix smoke | Pass | `../evidence/stage-5-release-candidate/reliability/soak-smoke-final.json` | 12/12 runs passed across eight fault scenarios. |
| 24-hour soak | Failed; rerun paused | `../evidence/stage-5-release-candidate/reliability/soak-24h.json` | The completed run made 2,494 attempts with 40 nonzero exits; zero protected-state growth alone is insufficient. The 2026-08-02 rerun was stopped before a receipt when the shared `/var` volume reached 100% capacity. Free verified-safe capacity, then obtain a fresh zero-failure 24-hour receipt. |
| Automated performance | Pass | `../evidence/stage-5-release-candidate/reliability/performance-accessibility.json` | Re-run after final artifact build. |
| Automated accessibility | Pass | `../evidence/stage-5-release-candidate/reliability/performance-accessibility.json` | Two automated Workbench tests passed; critical/serious axe findings were zero. |
| Native performance/accessibility acceptance | Pass | `../evidence/stage-5-release-candidate/reliability/performance-accessibility.json`; `../evidence/stage-5-release-candidate/reliability/native-launcher-walkthrough.json` | X11 cold-start, sustained-idle, AT-SPI/key-event keyboard, setup/recovery, and focus-containment acceptance passed. |
| Security scans and threat regressions | Pass with owned blockers | `../evidence/stage-5-release-candidate/security/security-summary.json`; `../security/stage-5-threat-model.md` | Close or explicitly retain every blocker listed below. |
| Adapter SDK/conformance | Pass for repository-owned suites | `../evidence/stage-5-release-candidate/adapters/conformance.json` | Repeat on final source; add a separately sourced unseen repository before claiming the onboarding exit criterion. |
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

### Reliability, performance, and accessibility

The bounded smoke receipt passed 12 of 12 runs across checkpoint restart,
corrupted journal tail, cancellation, model timeout, adapter crash/hang,
network loss, disk pressure, and large/noisy output. Protected state growth was
zero files and zero bytes in the short soak.

Automated performance remained within declared budgets:

- maximum observed fault-matrix p95: 356.148 ms (budget 1,000 ms);
- checkpoint restart: 284.754 ms (budget 1,000 ms);
- HUD JavaScript assets: 2,099,896 bytes (budget 3,145,728 bytes); and
- largest redacted diagnostic bundle: 1,530 bytes (budget 1,048,576 bytes).

Automated accessibility passed both tested Workbench states, keyboard focus
order, reduced-motion, increased-contrast, and forced-colors assertions. Manual
native acceptance is still required because jsdom cannot prove rendered color
contrast or screen-reader behavior.

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
unseen-repository criterion.

### Support

Three seeded incidents—missing launcher, native dependency failure, and missing
provider endpoint—were diagnosed from redacted bundles alone. All bundles passed
the secret-leak check and retained machine-verifiable hashes.

### Packaging

The local packaging evidence records a 642-component SBOM with zero missing
dependency licenses and byte-identical AppImage/normalized DEB/RPM proof. The
published `v0.3.0-rc.0` release has all six allowlisted assets plus detached
keyless Sigstore bundles; GitHub Actions run `30714060617` completed its
identity-bound download, checksum verification, signing, verification, and upload
steps successfully. The remote asset identities differ from the older local
packaging summary, so that summary cannot alone prove the published artifact
lifecycle; regenerate one reconciled evidence packet before promotion.

## Owned blockers

| ID | Severity | Owner | Required closure |
|---|---|---|---|
| REL-SOAK-001 | Release gate | Reliability maintainer | The first 24-hour receipt failed with 40 nonzero exits. The 2026-08-02 rerun was stopped without a receipt because `/var` reached 100% capacity. Free verified-safe capacity, then complete a fresh 24-hour zero-failure run with bounded protected-state growth and retain enough diagnostics to classify any future failure. |
| ENV-DISK-001 | Release gate | Operator/runtime maintainer | `/var` had 926 MiB free (100% used) after workspace verification; `target/` alone measured 147 GiB. Restore safe free capacity without deleting evidence or operator state before rebuilding or rerunning the soak. |
| ADAPTER-UNSEEN-001 | Release gate | Adapter SDK maintainer | Onboard and execute a separately sourced unseen repository through the documented SDK path. |
| SEC-AUTH-001 | High if remotely exposed | Engine/harness maintainer | Keep all harness binds loopback-only; implement inbound authentication before remote or multi-user exposure. |
| SEC-ADAPTER-001 | High for third-party adapters | Adapter runtime maintainer | Pin adapter artifact digest/signature; self-reported identity/version alone cannot defeat a malicious replacement binary. |
| SEC-GLIB-001 | High advisory | Launcher maintainer | Coordinate Tauri/GTK migration to `glib >=0.20.0`, then rerun Cargo and native gates. |
| PKG-EVIDENCE-001 | Release gate | Release maintainer | Regenerate local manifest/SBOM/checksum/reproducibility and release-lifecycle evidence for the exact remotely signed `v0.3.0-rc.0` asset identities. |

## Final approval rule

Stage 5 may be marked complete only after all required receipts are regenerated
from the same final source/artifact identity, all release-gate rows above pass,
and every critical/high item is either closed or accepted through the documented
release policy by the named owner. Until then, the candidate remains local-only
and non-promotable.
