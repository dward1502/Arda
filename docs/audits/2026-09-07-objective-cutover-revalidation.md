---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "verification_receipt"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-09-07"
---

# Objective cutover revalidation — 2026-09-07

Scope: [objective runtime cutover](../plans/2026-09-01-arda-objective-runtime-cutover.md).
This is a partial source repair receipt, not full cutover or installed acceptance.

## Reproduced and repaired

Behavior-focused RED runs reproduced idle-poll exhaustion, loss of a successful
sibling's receipts after another leaf failed, acceptance of a cyclic DAG,
revision after execution began, malformed receipt digest acceptance,
same-objective equal-root double claiming, missing expired verify-stage reclaim,
and premature failure of an active fifth attempt when a projection reopened the
store. Each regression subsequently passed through the production ObjectiveStore
or ObjectiveRuntime API. Test receipt fixtures now use real SHA-256 values;
existing context-outcome binding coverage is retained.

Retries remain bounded by persisted per-leaf claims, not daemon polling rounds.
Exhaustion is reconciled in the claim transaction after expiry; reader opens no
longer mutate objective lifecycle. This does not yet prove provider-level exact-once
recovery: the Workbench run ID still contains the incremented attempt.

## Executed verification

| Command | Observed result |
|---|---|
| `cargo test -p arda-engine --test objective_store --test objective_runtime` | 13 store + 3 runtime tests pass |
| `cargo test -p arda-engine` | 271 passed; 0 failed; 4 ignored (ignored tests not acceptance) |
| `cargo test -p arda-aule --features full-cli` | 393 passed; 0 failed |
| `cargo clippy -p arda-engine --all-targets -- -D warnings` | exit 0 |
| `cargo clippy -p arda-aule --features full-cli --all-targets -- -D warnings` | exit 0 |
| `cargo build --release -p arda --bin arda` | exit 0 |
| `cargo build --release -p arda-aule --features full-cli --bin arda-cli` | exit 0 |

Local execution logs: `/tmp/arda-cutover-engine-tests.log`,
`/tmp/arda-cutover-aule-tests.log`, `/tmp/arda-cutover-engine-clippy.log`,
`/tmp/arda-cutover-aule-clippy.log`, `/tmp/arda-cutover-daemon-build.log`,
`/tmp/arda-cutover-cli-build.log`. These temporary logs are supporting local
output, not immutable installed receipts.

## Installed observation and candidate identity

- Existing `arda.service`: active/running, PID 1829 at inspection.
- `GET http://127.0.0.1:7878/health`: `ok`; not proof of objective readiness.
- Retired queue service and timer: `LoadState=not-found`, `ActiveState=inactive`.
- No Arda daemon/CLI installation was performed. The later provider-only restart is recorded below.
- Candidate builds include the pre-existing unstaged Hermes adapter PATH edit;
  they are workspace build evidence, not exact-isolated-commit artifacts.

| Artifact | SHA-256 |
|---|---|
| `target/release/arda` | `8c89340e43df2995bd42cef177c7dbc69e10391f4c39ef274eb389bbfae6e18c` |
| `target/release/arda-cli` | `f070fca2b3613a4dcd1b3a3d8525b3311741d00a1f5be11e748cc352a8f61cfc` |
| Installed `~/.local/bin/arda` | `afb5096ab05b14653dec1b91af1d57a99bcceee79698c513311f0f357014ae11` |
| Installed `~/.local/bin/arda-cli` | `ee7c08bf35b5c0f2c22e3b48a18e2c5930ed27e10f1a46c8a230201abefd28ca` |

The differing hashes explicitly leave source/install equality open.

## Provider repair and independent review follow-up

The earlier empty/timeout review attempts were not approvals. The standalone
Codex CLI was the wrong transport and did not establish a Hermes auth failure.

Manwë-routed independent review of source/config/plan patch
`98c1b041b91dea2cfad572a18ef3d18f7aca1cd6bab92ce57418840e4482ab2e`
returned `APPROVE` through `openai_sub/gpt-6-astra`. The review first identified
connection timeouts incorrectly sharing generation-deadline treatment; that
finding was fixed before approval. This hash precedes this evidence-note update.

Provider repair scope:
- `edge_core` defaults thinking off while preserving explicit template overrides,
  caps ordinary output at 2048 tokens, and receives a 120-second deadline.
- Post-connect request deadlines are classified separately from endpoint transport
  failure; connection timeouts retain transport-failure handling.
- Responses SSE accepts CRLF framing and terminal-only output snapshots; malformed
  or empty streams no longer silently become successful empty completions.
- The configured subscription model is aligned with the active Hermes model,
  `gpt-6-astra`. This is a catalog update, not automatic model synchronization.
  The ChatGPT endpoint remains intentional for this subscription driver, not a
  global router destination. No OpenRouter billing change was made.

Installed provider verification on port 7171:
- Only the supervised Manwë child was restarted: PID 1914 exited; PID 267807
  started under the same Arda parent PID 1829. Arda itself was not restarted.
- An authenticated `/probe` recovered the stale unhealthy `edge_core` state by
  actual inference (`ANKH`), not by manually rewriting provider intelligence.
- Forced local and subscription requests returned `READY` in 1.01s and 4.85s.
- `model=auto` selected `edge_core/Qwen3.5-9B-Q4_K_M`, returning `READY.`.
- A 16,929-input-token scoped diff returned a substantive three-bullet local
  summary in 9.75s, with 287 completion tokens and `finish_reason=stop`.
- Manwë all-target tests, strict Clippy, and release build passed.

Raw smoke/review records are in `/tmp/manwe-repair-*.json`,
`/tmp/manwe-core-recovery.json`, and `/tmp/manwe-scoped-review.json`.
The Engine repair remains source/build verified, not installed cutover acceptance.

## Remaining acceptance boundaries

1. Stable RunStore recovery identity across interruption, including provider
   completion before SQLite persistence and exhaustion at the final claim.
2. Phase 4 notification, due-schedule consumption, bounded shutdown, startup
   reconciliation, and objective runtime health/status integration.
3. Physical-root aliases: current exclusion compares stored root strings.
4. Legacy producer freeze. The queue already had 551 added lines at entry and
   grew during verification. Test emissions and installed producers must be
   distinguished; no queue history or runtime state was deleted to manufacture
   a freeze claim.
5. Exact reviewed candidate deployment, forced restart/replay, authenticated
   controls/projections, and final Phase 8 closure.
6. Milestone 4/5 human-visible real-project overlap and operator continuity
   acceptance remain separately owned by their existing plans.

Unrelated adapter changes, live SQLite data, queue/projections, and generated
runtime artifacts are excluded from the scoped source/doc repair.
