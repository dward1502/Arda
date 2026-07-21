# Provider Catalog Contract

## Source of truth
Active provider entries are generated from `config/fleet.toml` `[[nodes]]` blocks
marked `active` or `active_staging`. Each admission decision is recorded in the
admission shed receipt log.

## Persistent state
- Bootstrap snapshot: `core/state/fleet_bootstrap.json`
- Admission receipts: `data/prometheus/runtime_admission_shed_receipts.jsonl`

## Fleet node mapping rules
A `FleetNode` becomes a provider candidate only when ALL of the following are true:
- `charon_provider_id` is present and non-empty
- `enrollment_status` is `active` or `active_staging`
- `llm_runtime` does not contain `inactive`

Selected model for the candidate:
- Prefer `runtime_model_alias`
- Fallback to the first element of `expected_models`

Normalized base URL:
- `base_url` already contains `host:port` -> `http://host:port[/base_url]`
- Otherwise fall back to `http://127.0.0.1:<runtime_port>`

## Candidate reconciliation
The `/provider_candidates` surface compares configured candidates against the live
`/v1/models` catalog for each node. Differences emit `provider_catalog_reconciled`
state events and append resulting receipts to the admission shed JSONL file.

## Evidence requirements
Every change to the provider catalog schema/contract must include:
- doc update in `docs/contracts/`
- at least one source module reference
- one receipt-producing write path
- one independent review receipt or smoke test
