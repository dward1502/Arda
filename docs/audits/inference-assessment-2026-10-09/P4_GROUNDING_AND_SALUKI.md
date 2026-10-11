---
soterion:
  sigil: "SCROLL"
  role: "assessment_evidence"
  owner: "HERMES"
  status: "draft"
  reviewed: "2026-10-10"
---

# Grounding and alternate-candidate continuation

The strengthened offline validator passed independent review with no bounded
blockers. This did not repair Qwen factual accuracy or qualify deployment.

## Qwen grounding experiments (not accepted)

- Initial AGENTS.md grounding advice was NOT injected: --safe-mode disables
  project rules. Recorded upstream grounding flags are false. Both procedural
  recoveries passed, but repetition 2 again invented Ticket #1. Retained files:
  p4-b11430-grounded-recovery-{1,2}.json. This was an ineffective setup, not
  evidence against injected grounding advice.
- The same generic rule was then explicitly prepended to the isolated query.
  It explains LINE_NUM|CONTENT metadata and requires checking asserted source
  facts; it contains no fixture answers. This intentionally changes the request
  policy, not the underlying task prompt or fixtures. Recovery 1 stopped after
  index.txt; recovery 2 passed. Research 1 passed. The candidate was rejected
  early rather than repeating until lucky. Files:
  p4-b11430-grounding-preamble-{recovery-1,recovery-2,research-1}.json.

## Saluki alternate serving pair

The already-staged Underdog-Saluki-27B-1.0-IQ2-mix.gguf runs with the same b11430
runtime, actual context 73728, GPU0 only and 40 GPU-offloaded layers. It was not
misrepresented as the Qwen model; the isolated provider model ID is
saluki-27b-iq2. No production provider or profile was changed.

The first recovery case times out under both thinking and non-thinking modes at
the original 60-second run / 80-second child budget. Captures show native tool
progress, but generation around 2.1 tokens/sec. Non-thinking reaches the index
result and emits the replacement call after the consumer has timed out. This is
an observed latency failure, not evidence of completed recovery. Retained files:
p4-saluki-serial-t0-recovery-1.json and p4-saluki-nonthinking-recovery-1.json.

The separately named long-budget batch FINISHED with 240-second runs and a
260-second child timeout, using [runner](manwe-saluki-longbudget-case.py).
Research 2/2 and recovery 2/2 passed the mechanical checks; parent inspection
found the four final answers grounded. Coding repetition 1 timed out and coding
repetition 2 did not run. This is not 6/6 and not acceptance at the original
budget. See [disposition](p4-saluki-longbudget-disposition.json).

The outer 330-second timeout killed the coding runner during cleanup before its
final JSON write. The retained coding log establishes the 260-second child
failure; no full coding capture is claimed. Before another expensive run, save
per-case/HTTP evidence before bounded cleanup and ensure timeout status propagates
to the outer process exit code. The batch itself returned zero despite its error,
so its JSON, not exit status, determines the failed outcome.

Result: [p4-saluki-longbudget-batch.json](p4-saluki-longbudget-batch.json).
Process proc_f075f6cd5943 has exited. Post-run read-back confirms status=ok for
server ports 8097/8098 and Core 9337; the Saluki transient unit is inactive.
No production model/profile promotion occurred. The operator subsequently
explicitly approved one bounded dual-GPU server test through the authorization
prompt. Both Bonsai and Nanbeige may pause for this experiment; Core and Warden
remain untouched. No permanent allocation change was authorized.

The dual-GPU batch FINISHED (proc_b5b84e5ca262), using
[runner](manwe-saluki-dualgpu-case.py). It restores the original 60-second task /
80-second child budgets, uses all layers on GPUs 0 and 1 with equal tensor split,
and otherwise retains the non-thinking serial/t0 case policy. All six mechanical
cases pass at the original budget. [Summary](p4-saluki-dualgpu-summary.json)
retains per-case elapsed event time and delivery checks. Both recovery answers
name ALPHA correctly; research answers are grounded. Coding implementations pass
the four vectors, but their explanations contain incorrect Python claims:
repetition 2 says the old truthiness filter dropped negative numbers; repetition
1 contradicts itself about booleans. Therefore full factual acceptance is NOT
claimed. Independent review confirms mechanical 6/6 and factual 4/6. It matched
16 native tool calls, reran both strict recovery checks and all four vectors for
each coding artifact. All 22 primary requests delivered cleanly, but six auxiliary
session-title deliveries had broken pipes. No further dual-GPU experiment is
authorized by the already-consumed one-run approval.

Post-run read-back: both incumbent services active; server8097/server8098 and
Core9337 status=ok; candidate unit inactive, MainPID=0. No promotion occurred.

The runner now persists a redacted checkpoint before cleanup; local regression
probes verified persistence, redaction and no mutation of original captures.
The orchestrator exits nonzero on incomplete/failed batches; both incomplete
and complete exit branches were exercised locally. This is harness verification,
not new model acceptance. The candidate has RuntimeMaxSec=1200 and a separate
1260-second timer restores both incumbents; finally also restores and probes them.
Results will be in p4-saluki-dualgpu-batch.json. Review factual answers and exact
restoration read-back before any acceptance or promotion claim.
Historical one-GPU run: inference-assessment-saluki.service used a 2000-second
limit and 2100-second restore timer. It finished and restored Bonsai before the
separately authorized dual-GPU run above.
