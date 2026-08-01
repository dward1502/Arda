# Security policy

## Supported versions

Security fixes are provided for the current release candidate and the most recent stable release, when one exists. Older development snapshots are unsupported.

## Report a vulnerability

Do not open a public issue containing exploit details, secrets, private data, or a working proof of concept.

1. Use GitHub private vulnerability reporting for `dward1502/Arda`:
   <https://github.com/dward1502/Arda/security/advisories/new>
2. Include the affected version or commit, impact, reproduction conditions, and the least-sensitive proof needed to validate the report.
3. If private reporting is unavailable, open a minimal public issue requesting a private security contact. Do not include vulnerability details.

The maintainer should acknowledge a report within three business days, provide a triage decision within seven business days, and coordinate disclosure after a fix or documented mitigation is available.

## Release security boundaries

- The Stage 5 Workbench harness is local-only. Non-loopback binds fail closed until inbound authentication is implemented.
- Loopback is not an authorization boundary against other processes running as the same user. Do not run untrusted local programs in the release profile.
- Repository text, research excerpts, model output, and adapter output are untrusted data. They cannot grant execution authority or replace approval receipts.
- Secrets must be supplied through operator-controlled environment or credential storage and must not enter logs, backup archives, diagnostics, receipts, or release artifacts.
- Published artifacts require a manifest, SHA-256 checksum, and detached signature bundle. Production releases use keyless Sigstore signing from `.github/workflows/release-sign.yml`, bound to the release-tag workflow identity and GitHub's OIDC issuer. The `production-release` environment must require maintainer approval; the workflow must verify the tag, manifest source commit, and checksums before signing.

## Incident response

1. Contain: stop the affected harness and adapters; disable network exposure; preserve read-only logs, manifests, receipts, and process metadata.
2. Revoke: rotate provider/API tokens, harness credentials when introduced, signing credentials, and any exposed service credentials.
3. Eradicate: patch the root cause and add a regression test before restoring service.
4. Recover: restore from a verified backup or signed artifact; rerun checksum, signature, dependency, and deterministic Workbench gates.
5. Review: record impact, timeline, affected releases, evidence, and follow-up owner without storing secret values.

Credential rotation must issue the replacement first, update consumers, verify service health, revoke the old credential, and confirm that logs and diagnostics contain no credential material.
