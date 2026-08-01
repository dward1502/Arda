# Stage 5 threat model and security gate

## Supported profile and assets

This gate covers the x86_64 Bluefin LTS 10 Workbench release candidate, its local harness, attached project source, run graph/journal, approval and execution receipts, backup state, provider credentials, and signed release artifacts.

## Trust boundaries

| Boundary | Untrusted input | Enforced control |
|---|---|---|
| Repository / model | source text, instructions, model output | typed run graph, explicit approval receipt, bounded toolset, actual exported tool evidence |
| Harness HTTP | local requests | loopback-only bind and mutation peer check; typed envelopes and idempotency keys |
| Adapter process | config, cwd, environment, output | schema validation, pinned declared identity/version, cwd/environment allowlists, byte/time limits, cancellation and process reaping |
| Journal / receipts | replayed or malformed records | strict schemas, contiguous sequence, semantic ordering, digest/parent checks, visible failure |
| Backup / restore | tar members and manifest | traversal/symlink rejection, exact manifest/member equality, per-file SHA-256, staged atomic replacement |
| Scout / remote presence | URLs, excerpts, capabilities | private-target rejection, bounded excerpts, explicit enrollment/capability checks |
| Release delivery | artifacts and metadata | SHA-256 ledger, detached signature bundles, pinned build-tool digest |

## Threat-case evidence

The Stage 5 engine gate exercises:

- untyped browser/command fields rejected without execution;
- executable, cwd, and environment boundary violations rejected;
- malicious instruction text remains one bounded process argument and cannot
  inject CLI options or shell commands;
- adapter handshake and result provenance must match the configured identity
  and version;
- adapter timeout and cancellation terminate and reap the child;
- approval required before provider spawn;
- claimed evidence cannot replace exported tool evidence;
- vendor session state rejected from canonical results;
- malformed, truncated, out-of-sequence, and non-contiguous journal input fails visibly;
- restart after mutation does not repeat an idempotency key;
- non-enrolled or wrong-capability remote presence is unauthorized;
- private research targets are rejected;
- run identifiers cannot traverse outside storage;
- backup traversal, symlink, tampering, and unmanifested-file injection are rejected;
- non-loopback harness binds fail closed.

## Known limits and release blockers

1. `SEC-AUTH-001` — inbound bearer authentication is not implemented. Mitigation: the engine now rejects every non-loopback bind, mutations also validate loopback peers, and the initial profile is single-user/local-only. Owner: engine/harness maintainer. Required before remote or multi-user exposure.
2. `SEC-ADAPTER-001` — configured adapter identity/version is enforced, but a
   malicious replacement binary can self-report the expected values. Mitigation:
   the Stage 5 profile executes only operator-installed local adapters through
   canonical executable paths and deny-by-default capabilities. Owner: adapter
   runtime maintainer. Third-party adapter distribution requires artifact digest
   or signature pinning before enablement.
3. `SEC-SIGN-001` — RC signatures use an ephemeral encrypted local key. Mitigation: checksums and bundles are independently verifiable for this candidate. Owner: release maintainer. Production release requires operator-owned hardware/KMS custody and rotation.
4. `PKG-STALE-001` — the top-level MIT `LICENSE` now exists, but the signed RC
   artifacts predate the license and subsequent security/reliability changes.
   Owner: release maintainer. Final publication requires a clean rebuild, new
   SBOM/manifest/checksums, and fresh signatures.
5. `SEC-GLIB-001` — Tauri's GTK dependency resolves `glib 0.18.5`, which carries
   `RUSTSEC-2024-0429`. Arda does not directly call the affected
   `VariantStrIter` API. Owner: launcher maintainer. Mitigation is local-only use;
   closure requires the coordinated Tauri/GTK upgrade to `glib >=0.20.0`.

## Gate commands

```text
cargo test -p arda-engine
python3 -m unittest tests.test_arda_beta_ops -v
pnpm audit --prod --json
cargo audit
pnpm --dir apps/arda-launcher run lint
pnpm --dir apps/arda-launcher run test
pnpm --dir apps/arda-launcher run build
```

A critical or high advisory fails release unless fixed or entered above with a concrete mitigation and owner. Scanner output and the machine summary belong in `docs/evidence/stage-5-release-candidate/security/`.
