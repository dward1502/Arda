---
soterion:
  sigil: "SCROLL"
  role: "assessment_evidence"
  owner: "HERMES"
  status: "draft"
  reviewed: "2026-10-09"
---

# Release qualification and failed repeatability

## Frozen release

Bundle: `/var/home/mythos/.local/state/arda/deployments/manwe-tools-20261010T055414Z`.
Candidate SHA256: `1616537160a4d932359177df685526013503a285994971623a2852b22fbd022f`.
Frozen source derives from the reviewed Arda source bundle with explicit current
Manwe source/test overlays, not an assumed match to the old Manwe executable.
The bundle retains its manifest, rollback binary, fresh isolated target, logs,
qualification record and dependency-info digests. Release build, full release
tests and all-target Clippy with `-D warnings` passed. This is not deployment
clearance and the candidate is now superseded by the additional source fix below.

## Actual release smoke did not pass

The simple public HTTP native-tool/read/result loop passed, but actual Hermes
CLI behavior was not repeatable:

- [First release run](p4-hermes-release-refusal.json): refused a permitted fixture
  read without calling a tool.
- [Second release run](p4-hermes-release-tool-loop.json): answered `2049, 80`
  without a tool, rather than reading the actual fixture. Marked failed.
- [Upstream capture](p4-hermes-release-capture.json): all four file schemas did
  reach Core. Hermes omitted `tool_choice`, triggering the existing local prompt
  and schema slimming path. The system prompt was visibly truncated and schema
  descriptions/defaults removed. The model again refused the permitted read.

Earlier one-off debug Hermes success remains evidence of that run only; it is
not sufficient evidence of reliable harness operation. Production is unchanged.

## Additional lossless-payload repair

The local structured-payload predicate now protects any nonempty supplied tools,
including requests omitting `tool_choice`. This prevents prompt/schema slimming
without forcing tool execution or changing routing authority. A new regression
failed before this change. Extended loopback HTTP tests cover absent/auto choice,
nonstream/stream dispatch, long system content, schema descriptions/defaults and
linked continuation. Full package tests pass: 314 library and three binary tests.
Independent read-only review found no blocker in the narrow nonempty-tools
preservation change: optional routing and authority remain separate. It is not
in the frozen release. Review identified a separate deployment blocker, verified
against source: `model_supports_request` reduces local required-tool/continuation
context admission using `local_slimmed_tool_context_target`, whose policy/schema
estimates and total are capped (total at 16,000). Those assumptions no longer
match lossless forwarding. Existing execution floors/headroom do not establish
that larger forwarded requests fit, especially with explicit low-context fallback.
Context admission is now repaired in source: both admission and diagnostics use
the full existing estimate as a lower bound, preserving policy floors/headroom.
Independent review found no blocking defect in that fix. Expanded tests cover
undersized explicit targets, estimates above 256k, and large output limits.
Review also exposed Core's omitted-output-budget discrepancy; a new failing
boundary assertion now passes after reserving its downstream 2,048-token default.
Full package tests still pass (314 library, three binary). Independent follow-up
review found no blockers in the output-reserve fix and independently reran the
expanded context regression (one passed). None of these changes is deployed or
release-qualified.
The initial review did not rerun builds/tests; the follow-up ran only the focused
regression. Neither review changed production.

[Lossless debug capture](p4-hermes-lossless-capture.json) confirms an untrimmed
system prompt and all four schemas. Hermes still refused the fixture read.
Therefore prompt slimming is a proven transport contract defect, but is NOT an
established cause of the refusals. Do not tune prompts repeatedly until a pass
and erase the failed observations. Full Hermes/Core qualification remains failed
for these observations; no replacement model or default route is selected.

Capture artifacts withhold private system/developer prompt contents, retaining
hash, character count and trimming indicator. Only synthetic fixture/user/tool
content and operational metadata are retained in repository evidence. Safe-mode
flags do not independently prove all profile context was excluded.

## Deployment boundary

Read-only audit established the installed Arda supervisor can respawn just its
Manwe child after exit; restarting all of `arda.service` is unnecessary. Before
any swap, exact artifact review, a quiet request window, rollback preservation
and child PID/start-identity checks remain required. No graceful request drain
was established. Verification must use Manwe chat on 7171; Arda on 7878 supplies
status, not an assumed chat proxy. No process has been signaled for this repair.
