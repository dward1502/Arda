# U6 audit and preflight — 2026-08-12

Status: blocked before final 1.0 source freeze

## Reconciled closures

- Backend-owned health, Workbench, Research, Personal Operations, and monitor-session authority is implemented.
- Five-monitor native lifecycle acceptance remains closed; it was not reopened.
- Research watchlist pause, restart, resume, retire, and second-restart durability pass in `crates/engine/tests/harness_research_operator.rs`.
- Personal Operations capture/classification, process restart, resume, and daily-brief reconstruction pass in `crates/engine/tests/harness_personal_ops.rs`.
- Root-daemon configured-identity coverage was repaired in `tests/root_daemon.rs`; all five root-daemon tests pass.
- HUD: 131 files / 514 tests pass; lint exits zero with existing warnings; production build passes.
- HUD Tauri PTY tests pass 2/2 and `cargo check` passes.
- Launcher: 4 files / 11 tests pass; lint and production build pass.
- Shared JSON-schema/release-ops suite passes 8/8 in an isolated Python environment.
- `cargo deny check` passes all advisory, ban, license, and source gates.
- `cargo audit` reports zero vulnerabilities and retained unmaintained GTK3-chain warnings already covered by the Stage 5 bounded-support decision.
- Both frontend production audits contain no high or critical vulnerabilities; HUD reports one low and five moderate advisories, launcher reports none.
- Provenance, vendored GLib backport verification, GLib regression tests, seeded support exercise, and docs link/completion-language checks pass.
- The published Stage 5 `v0.3.0-rc.1` source identity is independently reconfirmed: workflow run `31543599410` succeeded at `8a5e3f75db3867803d56c0b3568ec5fc51794349`; the six downloaded artifacts pass `SHA256SUMS` and all six keyless Sigstore bundles verify against the exact tag-bound workflow identity.
  The machine-readable receipt is `stage5-rc1-signature-revalidation-20260812.json`.

## Release blockers that cannot be closed by source-only or author/agent evidence

1. No clean final `1.0.0` source revision exists. Current product manifests remain development/RC versions, and the worktree contains uncommitted plan/archive/evidence state.
2. No final `v1.0.0` signed artifact bytes exist. Stage 5 RC.1 bytes are valid historical evidence, not final 1.0 qualification.
3. Stage 6 requires a qualifying independent non-author evaluator. No such receipt exists; operator or agent evidence cannot substitute.
4. Stage 6 requires an independent release-critical security/code review. Existing Stage 5 threat/audit evidence is useful input but does not prove an independent final-1.0 review.
5. Phone/current-live identity continuity and required whole-system product proofs remain open in the master plan.
6. Native final-1.0 accessibility/performance and exact-artifact lifecycle evidence cannot be produced before final bytes exist.
7. Stage 6 artifact lifecycle, supported matrix, backup/restore/rollback, adapter conformance, fault matrix, and soak must be rerun against the selected final bytes.

## Gate decision

Do not rename current packages to `1.0.0`, create a `v1.0.0` tag, archive Stage 6/U6 authorities, or declare U6 complete. Doing so would fabricate artifact, independent-evaluator, and independent-security evidence. The next legitimate release action is to obtain the two independent receipts, close the required master-plan product proofs, then select and qualify one clean final source identity.
