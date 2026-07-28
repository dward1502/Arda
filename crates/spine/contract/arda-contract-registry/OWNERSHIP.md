# arda-contract-registry ownership

## Registry-crate authority

`arda-contract-registry` owns the typed, read-only interpretation of the
canonical contract registry artifact:

- `ContractRegistry` and `TrackDefinition` schema types;
- the canonical relative path `core/state/contract_registry.json`;
- typed missing-file versus malformed-JSON load errors;
- loading from an explicit file path or an explicit workspace root;
- schema-level enumeration such as `track_ids()`.

The crate deliberately exposes no mutation or persistence API. Governance
processes that produce or approve the canonical JSON artifact remain outside
this library.

## Artifact authority

`core/state/contract_registry.json` is the repository-owned canonical artifact.
Its own `authority` field identifies the governing source. The integration
smoke test reads it without mutation and verifies that declared source paths and
schema identifiers still resolve in the live workspace.

## Consumer authority

`arda-launcher` is the only direct Cargo consumer. It owns:

- discovery of the active workspace root;
- onboarding-specific pass/warn/fail projection;
- Tauri payloads and timestamps;
- checks for receipt-store availability and UI readiness.

Launcher code must use `ContractRegistry::load_from_root` rather than duplicate
file reading or JSON parsing. Governance/core crates do not currently consume
this crate and must not be described as direct consumers without new Cargo and
source evidence.

## Test boundary

Parser and error behavior uses temporary explicit fixtures. The one live
integration suite is intentionally repository-coupled acceptance evidence and
must remain read-only; its run is verified against a before/after SHA-256 hash
of the canonical artifact.
