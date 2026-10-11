---
soterion:
  sigil: "SCROLL"
  role: "assessment_evidence"
  owner: "HERMES"
  status: "draft"
  reviewed: "2026-10-10"
---

# Strict recovery qualification and review corrections

General qualification remains OPEN. The earlier six-pass result was a mechanical
harness result, not proof of conditional recovery. Independent review found both
recovery sessions requested missing.txt and index.txt in the same assistant turn.
The answers and tools were real, but the ordering assertion was insufficient.
Review also found auxiliary HTTP captures could be incomplete at shutdown.

## Repairs and retained experiments

[strict_recovery.py](strict_recovery.py) checks the actual model request history:
index reads require an already-returned linked missing-file not_found result;
replacement reads require an already-returned linked index naming records.txt.
It rejects the retained original recovery case with the expected ordering error.
The revised recorder joins handler threads before serializing captures.
Follow-up review exposed three additional false positives: wrong full path,
truncation before the replacement result, and failed replacement reads. The
[regression tests](test_strict_recovery.py) reproduced all three failures, then
passed after strengthening exact-path, linked-result and terminal-response checks.
Five tests pass; both retained procedural chains pass the strengthened validator.
The runner now supplies the exact expected fixture root.

[Reviewed acceptance](p4-reviewed-acceptance.json) separates mechanical/factual
scores and primary/auxiliary delivery. All 46 upstream responses returned 200,
but each artifact has one auxiliary broken-pipe delivery error. Primary case
requests have no recorded delivery errors. These are not error-free captures.
One terminal reply lives in reasoning_content rather than content; the procedural
validator accepts terminal text in either channel, not tool-shaped reasoning as
execution. This is not clean reasoning-presentation qualification.

Two new sessions with parallel_tool_calls=false but default sampling passed
only 1/2: the failure stopped after the index and invented a RESOLVED status.
These remain in p4-b11430-serial-recovery-{1,2}.json; they were not discarded.

The next complete six-case batch additionally sets temperature=0. Prompts,
fixtures, budgets and existing assertions are unchanged. All six mechanical
checks pass, both recovery histories pass strict conditional ordering, and all
upstream captures have status or error recorded. See
[p4-b11430-serial-t0-summary.json](p4-b11430-serial-t0-summary.json) and
[runner](manwe-b11430-serial-t0-case.py).

However, recovery repetition 2 reports "Ticket ID | 1 (ALPHA)" when the source
only names ALPHA. The 1 appears to be the read tool's line-number prefix, not a
source fact. Therefore this batch is NOT accepted as all-factual-pass. An
independent follow-up review confirms this failure: factual score is 5/6, with
procedural recovery 2/2. Do not turn mechanical 6/6 into factual
acceptance or repeatedly sample until this error disappears.

Research and coding responses were inspected for the earlier rubric omissions:
energy is reported unknown, obsolete/current sources are explained, and coding
answers explicitly disclose that the model did not run tests. The parent harness
executes four constrained arithmetic assertions per coding case. Protection
assertions cover named fixtures; they are not a general filesystem sandbox.

## Next boundary

Separate native execution success from grounded final-answer accuracy. Retain
this counterexample and strengthen the final-answer rubric before qualifying the
serving pair. No production promotion, profile switch, commit or push occurred.
The authorized server lane was restored after the bounded experiment; the newer
runtime and serial/temperature settings remain isolated assessment settings.
