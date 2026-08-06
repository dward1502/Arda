# Stage 5 Support and Release-Blocker Policy

## Privacy and reporting boundary

Arda does not transmit crashes, errors, source, prompts, state, or diagnostics by default. There is no automatic crash-reporting endpoint in the Stage 5 candidate. Reporting is explicit and operator initiated:

```bash
python3 scripts/arda_beta_ops.py diagnostics \
  --root /path/to/Arda \
  --output "$HOME/arda-diagnostics.tar.gz"
```

The operator must inspect every archive before sharing it. Secret-named files, binary config, files over 256 KiB, state payloads, source diffs, provider responses, environment dumps, secret-like values, credentials in URLs, and absolute operator/source paths are excluded or redacted. Redaction is defense in depth; free-form text still requires human review.

## Triage policy

| Severity | Definition | Release effect | Initial response target |
|---|---|---|---|
| S0 | Data loss, secret disclosure, unauthorized mutation, signature bypass, rollback failure, or false terminal success | Block release immediately | Same working day |
| S1 | Supported-profile install/start failure, repeatable crash, inaccessible critical path, or adapter protocol corruption without data loss | Block release unless explicitly removed from the supported matrix | One working day |
| S2 | Recoverable degradation with documented workaround; no authority, privacy, or state-integrity impact | May ship only with a known-issue entry and owner | Three working days |
| S3 | Cosmetic/documentation issue with no operational ambiguity | Does not block | Next maintenance review |

A blocker closes only when a regression test or machine-readable receipt reproduces the fix. Reclassification requires the release maintainer to record rationale and affected support-matrix rows. Silence, a passing happy path, or unavailable external evaluators cannot downgrade a blocker.

## Current known-issue registry

| ID | Severity | Status | Scope and workaround | Closure evidence |
|---|---|---|---|---|
| SEC-AUTH-001 | S0 | Open | Harness has no inbound bearer authentication. It therefore rejects non-loopback binds; do not expose it remotely. | Authenticated endpoint tests and revised threat model |
| PKG-EVIDENCE-001 | S0 | Open | The published `v0.3.0-rc.0` asset set has tag-bound keyless signatures, but the current source and local candidate have not been rebuilt, reconciled, and signed as one final identity. Do not promote the local candidate. | One final source/artifact manifest with normalized package hashes, signatures, and lifecycle receipts |
| SEC-GLIB-001 | S1 | Mitigated; bounded local backport | Launcher and HUD resolve the same vendored `glib 0.18.5` with the exact upstream `VariantStrIter` fix. RustSec scanners omit path dependencies, so the pinned-archive verifier and optimized regression are mandatory compensating controls. The rejected GTK4 fork remains rejected. | `../evidence/stage-5-release-candidate/security/glib-0185-backport-spike-20260805.md`; `scripts/verify_glib_0185_backport.py`; `scripts/tests/test_glib_backport.py`; replace with maintained upstream GTK4 when available |
| TEST-ACT-001 | S3 | Open | Existing Learning Loop tests emit React `act(...)` warnings while passing. | Warning-free focused tests |

## Seeded support exercise

Run:

```bash
python3 scripts/stage5_support_exercise.py \
  --output-dir docs/evidence/stage-5-release-candidate/support
```

The exercise creates isolated profiles and diagnoses these failures using only each redacted archive:

1. missing launcher;
2. unresolved/invalid native launcher dependency surface;
3. missing Manwe provider endpoint.

It fails if a secret canary, credential filename, isolated HOME, or source path survives redaction. The resulting `support-exercise.json` contains bundle hashes, sizes, readiness status, and diagnosis without raw bundle contents.

## Maintainer checklist

1. Record supported profile, release ID, artifact checksum, and signature identity.
2. Reproduce the failure using an isolated profile; never request the operator's full HOME or repository.
3. Ask for a diagnostics archive only after the operator has inspected it.
4. Diagnose from redacted contents before requesting any additional data.
5. Treat terminal-state loss, duplicate mutation, false completion, secret leakage, and authority bypass as S0.
6. Link the fix to an exact test and machine-readable evidence receipt.
7. Update this registry and the Stage 5 evidence packet; do not silently remove unresolved entries.

## Operator terminology

Operator-facing instructions retain Arda's internal identities but always pair
them with functional labels:

- **Workbench** — governed project attachment, run, evidence review, and closeout surface.
- **Manwe** — local provider and inference gateway; a missing Manwe endpoint means provider routing is unavailable.
- **Oromë** — approval and dispatch boundary; it cannot silently replace explicit operator approval.
- **Mirromere** — optional ambient-assistant application.
- **RELIC/CITADEL** — optional runtime-presence and avatar presentation surfaces.

## Support boundaries

The first public cohort supports only `bluefin-lts-10-x86_64`, the profile
declared in the release manifest. The harness remains loopback-only while inbound
authentication is absent. Best-effort help for other distributions, hardware,
external adapters, local model stacks, and experimental Mirromere/RELIC/CITADEL
surfaces does not expand the supported matrix. Diagnostics are local and
operator-initiated; redaction is not permission to share an archive without
inspection. Never imply clinical, safety-critical, unattended-authority, or
general multi-user guarantees.
