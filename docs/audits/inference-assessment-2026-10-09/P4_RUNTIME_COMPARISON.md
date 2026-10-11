---
soterion:
  sigil: "SCROLL"
  role: "assessment_evidence"
  owner: "HERMES"
  status: "draft"
  reviewed: "2026-10-10"
---

# Same-weight newer-runtime representative comparison

All six planned cases passed on the isolated b11430 serving pair: research 2/2,
missing-file recovery 2/2, coding repair 2/2. Aggregate and per-case raw captures
are linked by [p4-b11430-summary.json](p4-b11430-summary.json).
The [runner](manwe-b11430-case.py) preserves the original prompts, assertions,
60-second run budget, 80-second process timeout, four file tools, thinking=true,
and frozen Manwe candidate. Only the upstream address and output names change.
Independent review found conditional-recovery and recorder gaps; see
[corrections and stricter experiments](P4_STRICT_RECOVERY.md). The six-pass count
is the original mechanical score, not full behavioral acceptance. This does not
close full-profile or rollout gates.

## Qualified pair and limits

- Model: Qwen3.5-9B-Q4_K_M.gguf, identical source/destination SHA-256
  `03b74727a860a56338e042c4420bb3f04b2fec5734175f4cb9fa853daf52b7e8`.
- Server executable: retained b11430 CUDA distribution, SHA-256
  `6b8a4e71a274fd7b8048b2b0b84d065262f313f7f1b985934fe403247b2b2c6e`.
- Server GPU0 only; 73728 context, parallel=1, full GPU offload, q4_0 KV,
  flash attention, --jinja, embedded template, explicit request thinking=true.
- Original older Core runtime result remains [1/6](P4_REPRESENTATIVE.md).
  Hardware, serving runtime and template/parser defaults differ; this is evidence
  for the tested pair, not isolated proof of one parser defect or a speed comparison.
- Coding assertions executed the constrained fixture function in the harness.
  Hermes correctly disclosed that its own four-tool environment cannot run tests.
- Initial 16384-context attempt failed router admission and is retained in
  [p4-b11430-recovery-1.json](p4-b11430-recovery-1.json). Context metadata was not
  inflated: the successful server was restarted with the actual 73728 window.
- An attempt made during server loading returned 503 and is retained in
  [p4-b11430-fullctx-startup-failure.json](p4-b11430-fullctx-startup-failure.json).
  Health was checked before the six-case run. Neither setup failure is scored
  as a completed behavioral case; no failed completed case was discarded.

## Restoration and next gate

Only authorized server Bonsai GPU0 was paused. A timed restoration guard was
armed before stopping it. After testing, the candidate was stopped, Bonsai was
restarted, and the guard was disarmed. Read-back health returned status=ok on
server ports 8097 and 8098 and unchanged Core port 9337. Production model weights,
service definitions, Manwe binary and Hermes defaults were not replaced.

Remaining: independent review of captured behavior and scoring, then qualify
full-profile Hermes and the intended installed serving pair before any rollout.
A six-case synthetic suite is not general reliability or whole-plan completion.
