# Arda Company Adapter Protocol v1

Schema: `arda.company-adapter.v1`

Company adapters are supervised, capability-allowlisted boundaries. They use the existing Arda JSONL adapter handshake and add a commercial resource envelope.

Operations:

- `crm.organizations.read`
- `crm.contacts.read`
- `crm.opportunities.read`
- `crm.activities.read`
- `calendar.activities.read`
- `email.context.read`
- `accounting.export.write` (operator approval required)
- `project.issues.read`

Every result contains `adapter`, `adapter_version`, stable `external_id`, `observed_at`, `source_digest`, and `read_only`. Contact locators and credentials must not enter general telemetry. Secrets are supplied only by the adapter-local store or OS keyring and are never part of request arguments, environment passthrough, provenance, logs, or receipts.

Outbound capabilities are denied by default. Every request identifies its target with `resource_id`; for `accounting.export.write`, the approval receipt's exact scope is `accounting_export_write:<resource_id>`. An adapter may advertise a write capability only when the engine allowlist includes it and the request supplies an unexpired explicit operator approval receipt with that exact scope. CRM reference v1 is read-only. Conflict detection, stable-ID deduplication, idempotency, and append-only audit tests are prerequisites for any later CRM write capability.

Terminal results preserve attempted/accepted/delivered/failed truth. An accepted request is not a delivered external communication. External messages are delegated to Oromë rather than sent by a CRM adapter.
