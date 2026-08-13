# Arda Development Guide

This guide defines contributor-facing repository rules that complement
[`AGENTS.md`](../AGENTS.md). Product identity and evidence vocabulary are owned
by the [Arda 1.0 product doctrine](architecture/ARDA_1_0_PERSONAL_AGENT_ECOSYSTEM.md)
and the [current 0.9 improvement plan](plans/2026-08-12-arda-0.9-baseline-and-improvement-plan.md).

## Active-plan completion claims

Active plans must not use `complete`, `operational`, or `fully integrated` as
unqualified current-state claims. A closeout records exactly what maturity was
proven rather than collapsing source presence, runtime reachability, operator
acceptance, and release support into one status.

Use one of these maturity values:

- `specified`
- `implemented`
- `compile_active`
- `root_composed`
- `operator_reachable`
- `workflow_proven`
- `failure_proven`
- `operator_accepted`
- `release_supported`

Keep the maturity declaration, bounded claim, and evidence link on one Markdown
line so the documentation-health check can evaluate the declaration:

```text
Maturity: workflow_proven — bounded claim — Markdown evidence link on this line
```

`operator_accepted`, `release_supported`, `workflow_proven`, `failure_proven`,
and capability-specific `*_verified` declarations require a native, operator,
or live evidence link. Fixture, author, or proxy evidence cannot close an
independent-user or final-artifact gate.

Historical quotations and preserved records are allowed when the same line is
explicitly labeled `historical`, `quotation`, `previously reported`, or
`archived record`. Phrase future completion conditions as gates or requirements,
not as current-state claims.

Run the contributor gate before closing plan work:

```bash
python scripts/hades_markdown_link_check.py \
  --root docs/plans \
  --out /tmp/arda-doc-health.md \
  --check-completion-language
```

The checker reports `unqualified_completion_claim` and
`missing_evidence_link` with file and line locations. Its focused tests are:

```bash
python -m unittest tests.test_hades_markdown_link_check -v
```
