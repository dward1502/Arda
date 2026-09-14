# Governed execution acceptance closeout

Date: 2026-08-27
Objective: `arda-governed-audit-20260827`

## Evidence generations

The acceptance evidence has two deliberately distinct generations. They must not be interpreted as one post-remediation run.

### 1. Governed live execution

The live Workbench run completed the `scope`, `gather`, and `synthesize` leaves under binding `read_only_audit` governance authority. The canonical queue records the full progression in rows 73-90. The terminal synthesized artifact is:

- `docs/audits/2026-08-27-continuous-governed-execution-gap-report.md`
- SHA-256 `63db537c3d44e5cc067016b87260036ff23afcdb4c5178c25fb10967184bde77`

That digest matches the synthesize execution receipt. The report's cited queue snapshot is preserved byte-for-byte at:

- `docs/audits/evidence/2026-08-27-governed-audit-queue-through-synthesize-claim.jsonl`
- SHA-256 `7d25fb0066ba99acb46c7f8bb048ba49652d4e1b054a7a69d60ecb959512d023`

It is exactly canonical queue rows 1-86 at synthesis time. The canonical queue subsequently appended rows 87-90 to close synthesis and now has SHA-256 `980c09f65715946d02585633d93b230222069516daad840054d0aa2bba80b88d`; no history was rewritten.

Only the terminal synthesize artifact digest is asserted as reproducible. Scope and gather receipts referenced intermediate versions of the same mutable report that were not retained, so those intermediate artifact-digest claims are intentionally excluded from the focused evidence set.

### 2. Provenance remediation

The live acceptance checkpoints predate the remediation and therefore truthfully contain embedded `objective_plan` and `objective_plan_validation` fields. They prove the governed execution above; they do not prove the replacement receipt format.

The acceptance run exposed that embedding opaque plan values in `arda-core` was the wrong compatibility and trust boundary. The core schema change was reverted. The remediated Workbench executor now:

- persists a typed plan and validation receipt outside `RunGraph`;
- limits the receipt to 256 KiB;
- rejects unsafe run-ID path components before filesystem access;
- verifies receipt digest, run identity, objective identity, and fresh plan validation on replay;
- binds only the receipt digest into `RunGraph` provenance; and
- reuses the persisted plan across restart instead of recomputing from drifting repository state.

This replacement behavior is covered by the focused Workbench regression. No post-remediation positive objective execution is claimed.

## Acceptance boundary

Positive governance-only execution is verified for the live acceptance generation. The new external objective-plan receipt boundary is source- and test-verified, but not represented as live positive execution evidence. Generated queue projections and unrelated dirty work remain outside the focused change.
