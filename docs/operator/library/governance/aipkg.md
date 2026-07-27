# AIPKG Operator Guide

Use this guide before authoring, validating, or signing `.aipkg` manifests.

## Safety note

This tooling is assistance-only. Package execution permission remains with the
operator. Governance gates are advisory checks, not clinical or diagnostic
measurement.

## Quick start

1. Define `manifest_version`, `package_id`, `version`, `package_digest`, and
   `runtime_profile`.
2. Enable every governance gate.
3. Require preflight, execution, validation, and signed attestation.
4. Run `AipkgManifest::validate()`.
5. Attach the manifest to a `Task` via `Task::aipkg_manifest` when the task
   represents an `.aipkg` workload.
6. Record `preflight_check` receipt before execution.

## Dispatch preflight gate

In `loop_engine`, `Task::aipkg_manifest` is optional and backward-compatible.
When present, dispatch runs `manifest.validate()` before bid/triad/council
allocation:

- valid manifest -> `DispatchPass::aipkg_preflight_passed += 1` and normal dispatch
- invalid manifest -> `DispatchPass::aipkg_preflight_blocked` records evidence and the task is skipped

This makes package compliance an enforced runtime gate for bundled tasks.

## Profiles

| Profile | Meaning |
|--|--|
| `wasm-wasi` | Portable WebAssembly workload |
| `oci-sandboxed` | Container with explicit sandbox boundary |
| `local-sovereign` | Local machine execution with operator trust |

## Governance gates

Every package must enable:
- Triad
- Bacon-lite
- JouleWork
- Love equation guard
- Soterion trace

## Receipt chain

- preflight
- execution
- validation
- signed attestation
- settlement is optional

## See also

- `spec/aipkg/v0.1/AIPKG-CONTAINER-v0.1.md`
- `crates/spine/governance/arda-core/src/loop_engine.rs`
- `docs/plans/AIPKG.md`
