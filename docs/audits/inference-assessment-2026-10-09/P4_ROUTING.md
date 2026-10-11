---
soterion:
  sigil: "SCROLL"
  role: "assessment_evidence"
  owner: "HERMES"
  status: "draft"
  reviewed: "2026-10-09"
---

# Routing recovery and failed tool acceptance

Partial P4 evidence. No replacement model promoted, no model service restarted,
and no task-data tools executed. Manwe mutations wrote normal catalog, health
and routing receipts through its authenticated service API.

## Verified recovery

Independent read-only inspection found the governed Manwe child serving 7171
under Arda on 7878. The parent rechecked live state: catalog generation 2,
Core still at `http://100.105.24.100:9337/v1`, all enabled local providers unhealthy.
The intended provider file already held `http://100.78.138.113:9337/v1`.

After inspecting the provider-file diff, authenticated `POST /reload_config`
returned 200. Readback proved generation 3 and the corrected Core URL.
Authenticated, provider-scoped `POST /probe` returned actual READY completions
from Core, Nanbeige and Bonsai; final route identities matched each target.
Probe attempt records also include skipped fallback candidates. Do not claim
that its attempt list was restricted to the named provider.

- [Health before](p4-health-before.json): one ready provider.
- [Core probe](p4-probe-edge_core.json)
- [Nanbeige probe](p4-probe-edge_backbone_coder.json)
- [Bonsai probe](p4-probe-edge_backbone_long.json)
- [Health after](p4-health-after.json): four ready, four unhealthy, fourteen disabled.

Credentials were resolved from the supervised process's configured credential
source in memory; no token was printed or retained in evidence. No health ledger
was edited directly and no guardrail was lowered.

## Actual routed synthesis and tool boundary

[Routed raw responses and headers](p4-routed-smoke.json) retain forced-local and
automatic-local research synthesis, both returning cited answers from Core.
The automatic request retained `local_only=true`; this was not cloud fallback.
Route headers identify Qwen3.5-9B-Q4_K_M and edge_core on both calls.

An inert `read_file(path, offset)` schema through the execution lane failed:
Manwe selected `tool_oriented`, but the response was Python code, truncated at
128 tokens, without a native call. No generated code was executed. The backend
reported only 32 prompt tokens.

[Direct control](p4-direct-tool-control.json), using the same tool/user schema
with explicit deterministic/thinking-off settings, returned exactly one native
`read_file` call with Cargo.toml and offset 17; prompt size was 306 tokens.
The differing sampling settings mean this is not an exact request equivalence
experiment, but the prompt-count discrepancy strongly motivates tracing schema
transport. The subsequent independent source trace below establishes the loss
in inspected source, but does not prove installed-binary/source equivalence.

The source diagnosis confirmed destructive stripping in all three proxy paths:
`strip_optional_tool_payload` treated `tool_choice:auto` as permission to delete
both fields. The working-tree fix preserves supplied fields and keeps routing
intent separate. An inverted helper regression first failed against the original
implementation. A real loopback HTTP fixture now verifies non-stream and stream
forwarding plus linked tool-result continuation without inference or tool execution.

The complete library suite passes serially: 313 passed, zero failed. A parallel
run had 312 passed and one credential-environment failure in the existing Codex
structured-content test. That run did not establish a clean parallel-suite pass;
the isolation repair and subsequent successful runs are recorded below. Logs were
initially retained in the local scratch directory as
`inference-manwe-tool-tests.log` and `inference-manwe-tool-tests-serial.log`.
Independent review found no production blocker in the transport change and
identified unguarded token mutations in three Codex conversion tests. Those now
test the pure body conversion without credentials. Three subsequent parallel
library runs pass all 313 tests; full `cargo test -p manwe --locked` passes 313
library and three binary tests. Follow-up independent review found no blockers
in this repair scope. Logs and the
operational smoke script are retained beside this note, not only in scratch.

The [isolated candidate tool loop](p4-candidate-tool-loop.json) passes through
the real public HTTP handler using the rebuilt debug executable and live Core.
It emits exactly `read_file(Cargo.toml, offset=17)`, receives six actual repository
lines and completes with a grounded summary. Requests force Core, disallow
fallback and require local-only inference. The script validates the exact tool
and arguments before performing that read; no model-generated code executes.
Candidate state and HOME were isolated, its listener was loopback-only, and the
candidate process was terminated/reaped. Production Manwe was not replaced.
This proves the narrow routed candidate loop, not Hermes harness acceptance,
release qualification, incremental tool-call streaming or production deployment.
The fixture's final-answer automated assertion checks presence/no further calls;
the retained response was also inspected for grounding against the six lines.

## Actual Hermes candidate loop

[Hermes CLI evidence](p4-hermes-candidate-tool-loop.json) retains a genuine
`read_file` tool event, successful tool result and correct final answer for an
isolated file fixture through the repaired candidate and live Core. Parsed JSONL
was checked for exactly one read, exact fixture path, non-error result, correct
owner/port and unchanged fixture. No test expected values were put in the prompt.
The normal file toolset was enabled; observed execution was read-only, not an
OS-enforced read-only sandbox. This was safe-mode with a temporary Hermes HOME,
not acceptance of the full default profile or its much larger prompt/tool set.

Two failed setup attempts are retained: the normal launcher tried preparing a
new dependency environment for the isolated data root and failed building
`python-olm`; directly invoking the already-installed managed environment avoided
unnecessary installation. The second attempt used the wrong endpoint variable
for bare `custom`; inspected resolver source requires `CUSTOM_BASE_URL`, not
`OPENAI_BASE_URL`. Corrected invocation succeeds with no production config edit.
Both candidate and CLI exited. This is still a debug candidate, not deployment.

Full-profile Hermes acceptance, representative code/research tests, voice,
vision, fallback and staged deployment remain open. Read-only scout health is
not research execution. Arda exposes no chat-completions proxy on 7878; governed
chat requests go through Manwe on 7171.

## Configuration ownership established by source review

Provider reload is the authenticated `/reload_config` endpoint in
`crates/spine/runtime/manwe/src/adaptive/transport/http.rs`. Matching provider
health is retained by reload; use governed probes to establish recovered health.
`config/fleet.toml` does not directly hot-reload inference routes. Arda reads its
Warden scout URL at startup; observability rereading the file is not route reload.
The retained fleet bootstrap JSON has a historical `providers[]` shape while
current source expects `targets[]`; changing that ledger is not part of this repair.

Warden's 262144 advertised context versus actual 4096 remains uncorrected and
must not be used to qualify tools. Nanbeige's 32768 and Bonsai's tools=false
likewise must not be bypassed to satisfy the tool-route guardrail. Source paths
alone do not establish installed binary/source equivalence.
