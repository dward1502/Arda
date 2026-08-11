# arda-rumil

`arda-rumil` is Arda's project-independent, evidence-first audit coordinator. It inventories bounded project roots, runs explicitly allowlisted read-only providers, normalizes findings, compares selected historical baselines, and emits review-only evidence and organization plans.

## Authority boundary

Rúmil observes and coordinates. It does not approve or execute moves, rewrites, archives, deletes, or arbitrary shell commands. Provider programs and arguments are registered by code/profile; requests may only allow or deny registered provider IDs.

## Feature surface

- default: bounded generic inventory and hashing
- `cargo`: Cargo workspace metadata adapter
- `git`: bounded read-only Git adapter
- `provider`: timeout/output-bounded command receipts and selected source excerpts

Organization profiles are project-neutral and opt-in per rule. Planning never moves, rewrites, archives, or deletes files; every run emits a dry-run receipt, including when there are no candidates.

## Warden consumer boundary

`arda-outpost-scout` exposes Rúmil through bounded `/audit` and
`/audit/followup` routes. Project roots are relative to the configured Warden
runtime root, requests expire, budgets are mandatory, and authority remains
`advisory_read_only`. Full packets remain under the audit-owned Warden path;
Vairë receives only compact receipt metadata. Replays are digest-bound and do
not create duplicate receipts or memory records.

## Governed evidence consumers

`RumilEvidenceReference` carries bounded packet identity, digest, completeness,
classification, and degraded-state metadata without source contents or
filesystem access. Mandos preserves tool-backed, heuristic, historical,
partial, and unavailable classifications. Varda emits advisory evaluation
receipts with `execution_authorized: false`. Workbench and HUD projections show
completeness, stale baselines, rejected providers, and missing evidence.

## Project profiles and migration

Built-in TOML profiles under `profiles/` cover Arda, generic Rust, Node,
Python, and mixed projects through one `audit_with_profile` coordinator. A
project owner selects a profile, passes the absolute project root to that
coordinator, and supplies revision identity on the enclosing `AuditRequest`;
the profile declares its relative root, exclusions, providers, budgets,
organization rules, retention, and redaction policy. Invalid/traversing roots,
unknown providers, invalid budgets, and mutation-capable organization settings
fail validation.

Full audit/provider execution is host-side. Pi/Warden consumers receive bounded
references and may use only a profile explicitly declared for Pi inventory.
`import_legacy_hades_findings` creates deterministic historical comparison
baselines with `legacy_source = "hades"`; imported findings are never promoted
to current or tool-backed evidence.

## Verification

```bash
cargo test -p arda-rumil --all-features -- --test-threads=1
cargo test -p arda-rumil --no-default-features -- --test-threads=1
cargo clippy -p arda-rumil --all-targets --all-features --no-deps -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc -p arda-rumil --no-deps
```

See [OWNERSHIP.md](OWNERSHIP.md), [BREAKDOWN.md](BREAKDOWN.md), and [STATUS.md](STATUS.md).
