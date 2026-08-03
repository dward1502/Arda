# `arda-rumil` Contract Outline

**Status:** Archived design baseline for `2026-08-03-arda-rumil-project-audit-plan.md`; live authority is the `arda-rumil` crate
**Contract namespace:** `arda.rumil.*`

This file preserves the pre-implementation packet outline as historical design evidence. It is not a current Rust schema claim; exact field types and compatibility behavior are authoritative in the live `arda-rumil` crate and its contract tests.

## Contract rules

- All contracts are versioned and reject unknown fields unless a later compatibility rule explicitly permits them.
- All paths inside packets are project-relative POSIX paths.
- Absolute host paths may appear only in local operator logs, never in portable packets.
- Every packet carries `generated_at_utc`, `project_id`, `source_revision`, `policy_id`, and `authority`.
- `authority` is advisory/read-only for Rúmil output.
- Large source content is referenced by bounded excerpt ID and digest, not embedded by default.
- A packet must disclose exclusions, truncation, provider failures, and unavailable capabilities.

## `arda.rumil.audit-request.v1`

Purpose: request a bounded audit against an explicitly selected project root/profile.

Required concepts:

```text
request_id
project_id or project_root_identity
profile_id
source_revision expectation, if known
requested_capabilities
root_policy
path_exclusions
file_count_budget
byte_budget
source_excerpt_budget
command_timeout
provider_allowlist
redaction_policy
prior_audit_id, if comparison is requested
requested_by
expires_at_utc
authority = advisory
```

The request must not contain arbitrary shell commands. Providers are selected by named capability/profile.

## `arda.rumil.audit-report.v1`

Purpose: durable top-level audit result.

Required concepts:

```text
audit_id
project_id
project_kind
root_identity
source_revision
profile_id
generated_at_utc
completed_at_utc
completeness
capabilities_requested
capabilities_completed
capabilities_unavailable
inventory_summary
tree_digest
file_records or bounded file-record artifact reference
package_records
module_records
dependency_graph_reference
command_receipts
finding_references
organization_plan_reference
comparison_reference
exclusions
truncation
warnings
errors
authority = advisory
```

Completeness must be one of:

```text
complete
partial
structure_only
failed
not_requested
```

`complete` is only allowed when all required profile capabilities completed within policy and no undisclosed truncation occurred.

## `arda.rumil.file-record.v1`

Purpose: bounded inventory entry.

```text
path
kind: file | directory | symlink | unreadable | excluded
size_bytes
content_sha256, when read and allowed
mime_or_extension
executable
symlink_target_digest, when allowed
source_excerpt_ids, optional
redaction_state
observed_at_utc
```

No record may imply that content was inspected when only metadata was observed.

## `arda.rumil.command-receipt.v1`

Purpose: prove what an approved provider command actually did.

```text
command_id
provider_id
argv_digest
working_directory_relative
policy_id
started_at_utc
finished_at_utc
exit_code
stdout_digest
stderr_digest
stdout_bytes_retained
stderr_bytes_retained
truncated
timeout
status: completed | failed | timed_out | denied | unavailable
tool_version
configuration_digest
authority = advisory
```

Raw stdout/stderr is stored separately under policy, not automatically in the portable packet.

## `arda.rumil.finding.v1`

Purpose: normalized finding from a tool, heuristic, comparison, or organization rule.

```text
finding_id
audit_id
category
severity
status: new | persistent | changed | resolved | stale | unverifiable
confidence_class: tool_backed | source_backed | heuristic | historical | unavailable
path_or_scope
summary
recommendation
evidence_refs
provider_id
source_command_id, optional
prior_finding_id, optional
review_required
mutation_allowed = false
```

A finding without evidence must say `confidence_class = unavailable` or `heuristic` and explain why.

## `arda.rumil.organization-plan.v1`

Purpose: review-only organization proposal.

```text
plan_id
audit_id
profile_id
scope
candidates[]
no_delete = true
no_move = true
no_rewrite = true
operator_review_required = true
mutation_authorized = false
generated_at_utc
```

Each candidate includes:

```text
candidate_id
path
candidate_type
risk
recommended_action
evidence_refs
affected_paths
rollback_note
```

A future mutation system may consume this packet, but Rúmil does not perform the mutation.

## `arda.rumil.comparison.v1`

Purpose: compare a current audit to a prior audit for the same project identity.

```text
comparison_id
current_audit_id
prior_audit_id
identity_match
revision_relation
new_findings
persistent_findings
changed_findings
resolved_findings
stale_findings
unverifiable_findings
baseline_warnings
```

Identity matching must not rely only on a local absolute path. It should use project-provided identity, repository remote/revision where available, and an explicit operator override when necessary.

## `arda.rumil.memory-observation.v1`

Purpose: bounded handoff to `arda-vaire`.

This is not a copy of the full audit packet. It contains only:

```text
observation_id
source_audit_id
project_id
source_revision
summary
completeness
finding_counts
comparison_digest
receipt_refs
eligible_tags
retention_class
provenance
```

No raw source excerpts or unrestricted file trees should be written to Vairë by default.

## `arda.rumil.legacy-hades-import.v1`

Purpose: preserve historical HADES evidence during migration.

```text
import_id
legacy_contract
legacy_path
legacy_sha256
legacy_generated_at_utc
mapped_project_id
mapped_rumil_audit_id
mapping_quality
unmapped_fields
historical_only = true
native_rumil_evidence = false
```

This prevents old HADES reports from being silently relabeled as Rúmil-native audits.

## Compatibility and security tests

The implementation must test:

- round-trip serialization;
- malformed packets;
- unknown fields;
- unsupported future versions;
- path traversal attempts;
- absolute-path leakage;
- secrets/credential path exclusion;
- oversize packet/excerpt rejection;
- missing evidence disclosure;
- partial/failed provider disclosure;
- duplicate audit request idempotency;
- replay of the same audit;
- legacy HADES import without native-contract confusion.
