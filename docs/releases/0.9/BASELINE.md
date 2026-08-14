---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "release_baseline"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-08-14"
---

> 🜏 Soterion: 📜 release_baseline | owner: HADES | status: active | reviewed: 2026-08-14

# Arda 0.9 Baseline

**Version:** `0.9.0`<br>
**Profile:** personal/internal baseline<br>
**Release posture:** self-qualified, local-first, unsigned<br>
**Supported package profile:** x86_64 Bluefin LTS 10<br>
**Canonical root:** `/var/home/mythos/Eregion/Arda`

## What 0.9 means

Arda 0.9 is the operator's whole-system maturity checkpoint. It establishes a
stable answer to “where does Arda stand?” after the Workbench, HUD authority,
release engineering, recovery, Research, and Personal Operations implementation
tranches. The jump from `0.3.0-rc.2` to `0.9.0` records product convergence; it
does not imply eight prior public minor releases.

This is suitable for the operator's own use and disclosed alpha handoff. It is
not independently flow-reviewed, signed, or qualified as public production
`1.0.0` bytes.

## Product boundary

Arda is one local-first governed personal-agent ecosystem. The base composes a
Rust runtime, Manwë routing, governed execution, durable receipts and recovery,
a native launcher, and the HUD. `queue.jsonl` is the canonical queue ledger;
`queue_active.json` and `queue_summary.json` are generated read projections.

The supported 0.9 profile is single-operator and loopback-only. Remote or
multi-user exposure is not supported. Payments/Web3, broader Company Operations,
Mirromere, RELIC/CITADEL expansion, extra devices, and broader ecosystem
integrations are optional or deferred and do not block this baseline.

## Capability baseline

| Capability | 0.9 posture | Evidence boundary |
|---|---|---|
| Root runtime and service composition | implemented and tested | Root daemon and workspace package gates; deployment state remains environment-specific. |
| Workbench governed loop | workflow-proven | Attach, plan, approve, execute, verify, durable receipt, restart recovery, and replay protection are exercised. |
| HUD authority boundary | implemented and tested | Rust owns health, Workbench, Research, Personal Operations, errors, identity, and monitor sessions; React is not authority. |
| Five upper monitor sessions | operator-accepted implementation | Native session ownership, workstation continuity, and restart recovery passed; future information-display refinement is not a 0.9 blocker. |
| Research/watchlists | bounded workflow-proven slice | Authenticated question/watchlist lifecycle and restart recovery pass; broader research expansion remains optional. |
| Personal Operations | implemented; operator acceptance open | Automated contracts, restart, privacy, export/deletion, and HUD paths exist. Genuine usefulness/burden dogfood remains open through 2026-08-17. |
| Launcher and Linux packaging | package-proven | AppImage, DEB, and RPM have been built; the 0.9 package is an unsigned personal baseline. |
| Provider/local inference routing | compile/runtime-supported | Manwë is canonical. Individual providers and useful council roles remain configuration- and evidence-dependent. |
| Phone access | architectural/partial | Phone/desktop shared canonical-state proof is deferred; no 0.9 supported-phone claim is made. |
| Optional applications/outposts | not base-supported | Retained only where independently useful; absence does not block 0.9. |

## Verification snapshot

The pre-baseline verification completed on 2026-08-12 records:

- HUD: 514 tests across 131 files, lint, build, native PTY 2/2, and Tauri check passed;
- root daemon: 5/5 tests passed;
- Manwë: 287/287 tests passed;
- launcher: frontend 11/11 and Rust 14/14 tests passed, with lint/build/format passing;
- release and beta operations: 22/22 tests passed;
- engine all-features, Cargo policy, provenance, GLib backport, SBOM, checksums,
  clean-profile install/uninstall, and restart-recovery gates passed;
- Rust audit found zero known vulnerabilities; launcher production dependencies
  reported none; HUD production dependencies retained 1 low and 5 moderate
  findings for assessment, with no high/critical finding reported.

The HUD findings in that 2026-08-12 snapshot were resolved on 2026-08-13 by
upgrading to `mermaid 11.16.1` and transitive `dompurify 3.4.13`; the post-change
production audit reports zero findings.

Detailed command and blocker evidence is retained in
[`u6-audit-and-preflight-20260812.md`](../../evidence/stage-6-1.0/u6-audit-and-preflight-20260812.md).
This baseline does not turn historical test output into exact-byte qualification
of a later artifact.

## Known limitations

1. The 0.9 artifact is unsigned and self-qualified.
2. Independent flow/evaluator review is intentionally omitted for this personal
   baseline and must not be represented as completed.
3. Personal Operations has not yet received the operator's sustained-use verdict.
4. Phone/desktop canonical-state, some whole-system vertical proofs, and a useful
   independent local-inference council role remain unproven.
5. The supported package matrix is intentionally narrow; other Linux profiles
   provide compatibility feedback only.
6. Runtime endpoints are loopback-only; network exposure and multi-user use are
   unsupported.
7. Older generated/vendored documentation contains a known stale-link backlog;
   active plan and baseline links are held to the current documentation gate.
8. Exact signed-byte lifecycle, final accessibility/performance qualification,
   and an independent release-critical security review are deferred to a future
   public/final release decision.
9. The current-source replacement HUD candidate is not qualified for a 0.9
   artifact rebuild: its hardware-backed X11 path rendered black, its rendering
   fallback exceeded the historical native resource budgets, and native WebKit
   exposed no HUD content semantics. The measured
   [0.9 native baseline](../../evidence/0.9/hud-native-performance-accessibility-baseline-20260814.json)
   is blocker evidence, not a regression claim against the previously selected
   0.9 artifact bytes.

## Review policy

For 0.9, automated gates, operator acceptance, and transparent limitations are
sufficient. External alpha feedback is welcome evidence but is not an entry or
exit gate. No author or agent may describe 0.9 as independently qualified.

A future public/final `1.0.0` remains fail-closed on a clean source identity,
OIDC/Sigstore-signed exact artifacts, supported-matrix lifecycle qualification,
independent release-critical security/code review, genuine operator acceptance,
and the required whole-system proofs. Those requirements are deferred, not
waived or marked complete.

## Remaining work authority

Current work is limited to the finite items in
[`../../plans/2026-08-12-arda-0.9-baseline-and-improvement-plan.md`](../../plans/2026-08-12-arda-0.9-baseline-and-improvement-plan.md).
Future `1.0` vision and qualification material is retained under
`docs/archive/deferred/1.0/` and holds no active 0.9 planning authority.
