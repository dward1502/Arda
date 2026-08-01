---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-21"
---
> Arda-AIPKG: 📜 sovereign package/runtime contract | owner: prometheus | status: active | reviewed: 2026-07-21

# AIPKG Container v0.1

`.aipkg` is the sovereign package format, preflight gate, receipt ledger, and
runtime contract for portable agent tooling. Open standard law; marketplace
economics are explicitly out of scope here.

## Package Identity

- `manifest_version`: currently fixed at `0.1`.
- `package_id`: dotted namespaced identity, e.g. `org.arda.demo`.
- `version`: semver-style package version.
- `package_digest`: `sha256:` prefixed content digest.

## Profiles

- `wasm-wasi`
- `oci-sandboxed`
- `local-sovereign`

## Preflight Mandate

Every package must allow zero-work preflight after compatibility and quote
phases. Preflight produces an `AipkgPreflightReceipt` and must occur before
execution.

## Governance Gates

Five explicit gates must be enabled:
- Triad
- Bacon-lite
- JouleWork budget
- Love equation guard
- Soterion trace

## Receipt Chain

Required receipts:
- preflight
- execution
- validation
- signed attestation

Optional:
- settlement

## Rust Validation Surface

`arda-core` exposes `AipkgManifest::validate()` and `AipkgManifest::preflight_check()`.

## Runtime Dispatch Gate

`loop_engine` treats `Task::aipkg_manifest` as optional. When present, dispatch
runs `manifest.validate()` before bid/triad/council allocation. Valid manifests
are dispatched normally; invalid manifests are recorded as
`aipkg_preflight_blocked` evidence and skipped.

## Spec Maintenance

Run `scripts/aipkg/validate_spec.py` to verify the spec bundle remains aligned
with the code/test surface. The script checks:

- required spec files exist
- JSON schemas parse
- `manifest.example.json` matches required AIPKG manifest fields
- container doc references the current runtime gate and validator script

## Open Questions

1. Marketplace economics surface remains separate from this container spec.
2. Cross-profile signing cadence is still operator-defined.
3. Hardware-backed attestation is deferred.
