---
soterion:
  sigil: "SCROLL"
  role: "assessment_evidence"
  owner: "HERMES"
  status: "draft"
  reviewed: "2026-10-09"
---

# Thinking-enabled representative continuation assessment

Result: NOT qualified for general agent work. The frozen candidate
`fce5727e2bf66d3f821e904c2f98ee75cba936b3ebed1708b35965bc5f58b774`
was exercised with the actual Hermes CLI, four file tools and explicit thinking.
Six isolated synthetic cases (three task types, two fresh sessions each) used a
60-second run budget and 80-second child timeout. Aggregate and full transcripts:
[p4-representative-summary.json](p4-representative-summary.json), individual
`p4-representative-{research,recovery,coding}-{1,2}.json` files.

- Research conflict resolution: 1/2 passed. Passing run read both dated sources,
  cited the current owner/port, explained supersession, and correctly reported
  energy measurements unknown. Failed run read only the old source.
- Missing-file recovery: 0/2 passed. Both read the absent file but stopped before
  reading the index and replacement, despite describing the next action.
- Coding repair: 0/2 passed. Both read the specification but stopped before reading
  the implementation or applying a fix. No coding-test pass is claimed.

The first all-in-one batch had one research failure and a recovery timeout before
the outer tool timed out. Its partial log is retained separately as
[p4-initial-interrupted-suite.log](p4-initial-interrupted-suite.log); it is not
counted among the six completed cases. Subsequent cases persist independently.

## Specific failure boundary

The first coding continuation capture contains all four tool schemas, linked
assistant/tool history, and `enable_thinking: true`. Upstream returns
`finish_reason: stop`, empty content, and a proposed read_file XML-like tool call
inside `reasoning_content`, not a native `tool_calls` array. Hermes ends with a
statement of the proposed action; no second read occurs. This is a concrete
upstream output/parser boundary failure, not evidence of schema loss. Do not
execute tool-shaped reasoning text or force a success by inventing tool results.
The observations do not distinguish model behavior from old serving-runtime
parser/template behavior; that is the next controlled comparison.

## Safety and limits

Only synthetic scratch fixtures were used; protected fixture contents were
checked and candidate processes stopped. A final process check found no candidate
Manwe left running. No production settings, services, model weights or defaults
were changed. This is a small task suite, not a broad benchmark or full-profile
acceptance. The earlier three single-file passes remain valid but insufficient.
The coding fixture validator rejects unexpected syntax/calls before evaluating
its narrow arithmetic function; both runs failed before reaching that validator.

Next: compare this same continuation case against a pinned newer compatible
runtime in the authorized isolated server lane, retaining native-call semantics,
rather than changing production or parsing hidden reasoning as executable tools.
