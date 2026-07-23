# Hermes Agent → Arda Manwe Production Cutover

Status: sections 1–4 complete as of 2026-07-22.

The pinned production path is Hermes delegation → Arda Manwe on port 5110 →
`edge_core/LFM2.5-8B-A1B-Q4_K_M`. Legacy Charon is inactive and masked as
rollback/port-reclamation protection.

## 1. Fresh-process acceptance test — COMPLETE

Acceptance criteria:

- A native `terminal` call executes.
- A native `read_file` call executes after the terminal result.
- The child does not emit descriptive tool-call JSON instead of calling tools.
- The completed result reports `manwe` and `# Arda AGENTS.md`.

Evidence:

- Fresh launcher: `/var/home/mythos/.hermes/hermes-agent/venv/bin/hermes`.
- Loaded source:
  `/var/home/mythos/.hermes/hermes-agent/tools/delegate_tool.py`.
- Effective child allowlist: `terminal`, `file`.
- Manwe `/v1/models` advertised the pinned model and direct inference returned
  HTTP 200.
- A fresh `hermes -z` process delegated a request for the verbatim output of
  `git branch --show-current` and the first top-level `AGENTS.md` heading. The
  delegated goal did not prescribe tool order.
- Canonical transcript:
  `/var/home/mythos/.hermes/cache/delegation/live/deleg_cde5e1a4/task-0.log`.
- Native execution order: `terminal` returned `manwe`, then `read_file`
  returned `# Arda AGENTS.md`; the child completed normally.
- Hermes regression suite: `tests/tools/test_delegate.py` — 163 passed.
- The live gate exposed and fixed ambiguous `execute_code` versus shell/file
  guidance and JSON-string `child_toolsets` parsing. Earlier failed transcripts
  remain diagnostic evidence, not acceptance evidence.

## 2. Replace legacy dependency edges — COMPLETE

Live user-unit consumers now order against `arda-manwe.service` rather than
`annunimas-charon.service`:

- `annunimas-agent-supervisor.service` (`After`, `Wants`)
- `annunimas-ceo-autopilot-supervised.service` (`After`, `Wants`)
- `annunimas-ceo-autopilot.service` (`After`, `Wants`)
- `annunimas-charon-healthcheck.service` (`After`)
- `annunimas-charon-inference-probe.service` (`After`)
- `annunimas-hermes-provider-heartbeat.service` (`After`)
- `annunimas-litellm.service` (`After`)
- `annunimas-update-intel.service` (`After`)

Verification:

- `systemd-analyze --user verify` passed for Manwe and all eight consumers.
- A complete live-unit dependency search found no remaining
  `After`/`Wants`/`Requires`/`BindsTo`/`PartOf` references to
  `annunimas-charon.service`.
- A real daemon reload and restart changed Manwe PID `3299417 -> 3391617` and
  supervisor PID `3001 -> 3391632`; both returned active and LiteLLM remained
  active.
- `default.target` directly wants Manwe, the supervisor, and LiteLLM. No
  physical host reboot was performed in this slice.
- Healthcheck, inference-probe, and Hermes-provider-heartbeat oneshots each
  completed with `Result=success` and `ExecMainStatus=0` through Manwe.
- PID 3391617 (`manwe`) listened on `0.0.0.0:5110`; `/healthz` reported
  `runtime=arda-manwe`; `/v1/models` returned the fleet; explicit pinned-model
  inference returned HTTP 200.
- `annunimas-charon.service` remains masked and inactive solely as
  port/rollback protection. Legacy-named compatibility probes remain named but
  no longer depend on or launch the legacy unit.
- Historical templates under `~/Annunimas/scripts/systemd` were audited but
  not modified because Arda policy forbids editing `~/Annunimas` without an
  explicit cross-repository request.

## 3. Repeatable production smoke test — COMPLETE

Gate: `scripts/smoke_manwe_production.py`

The bounded gate verifies:

- Port 5110 listener executable resolves through `/proc/<pid>/exe` to
  `target/release/manwe`.
- `/healthz` reports `runtime=arda-manwe` and port 5110.
- `/v1/models` advertises
  `edge_core/LFM2.5-8B-A1B-Q4_K_M`.
- Explicit inference returns exact route headers:
  - `x-manwe-provider: edge_core`
  - `x-manwe-model: LFM2.5-8B-A1B-Q4_K_M`
- A delegated native `read_file` task succeeds.
- A delegated sequential native `terminal` then `read_file` task succeeds.
- An unavailable explicit route fails closed with HTTP 503,
  `code=no_compatible_model`, and no provider route header.

Latest verification:

- `python3 scripts/smoke_manwe_production.py --self-test` — PASS.
- `python3 scripts/smoke_manwe_production.py` — PASS.
- Single-tool transcript: `deleg_840e59f0`.
- Sequential transcript: `deleg_c7665cdf`.
- The sequential validator checks native tool order, tool results, and the
  required path/heading in the completed result. It does not fail solely
  because a small local model changes the case of an opaque correlation marker.

## 4. Isolate and commit focused changes — COMPLETE

### Hermes repository

Focused files:

- `hermes_cli/config.py`
- `tools/delegate_tool.py`
- `tests/tools/test_delegate.py`

Verification:

- `venv/bin/python -m pytest tests/tools/test_delegate.py -q`
- Result: 163 passed.
- `git diff --cached --check` passed before commit.

Commit:

- `36c74f076 fix(delegation): enforce native child tool execution`

Unrelated Hermes modifications remained unstaged.

### Arda repository

Focused production-cutover files:

- `config/fleet.toml`
- `crates/spine/runtime/manwe/src/config.rs`
- `crates/spine/runtime/manwe/src/main.rs`
- `crates/spine/runtime/manwe/src/provider.rs`
- `crates/spine/runtime/manwe/src/adaptive/service/route_policy_tests.rs`
- `scripts/smoke_manwe_production.py`
- `docs/plans/HERMES_AGENT_FLEET.md`

Verification:

- `cargo fmt -p manwe -- --check` — PASS.
- `cargo test -p manwe` — PASS: 19 binary tests.
- `cargo test -p manwe --features adaptive` — PASS: 157 adaptive library
  tests plus 19 binary tests.
- Production smoke gate — PASS.

The focused Arda commit deliberately excludes unrelated dirty-tree work,
including `Cargo.lock`, Manwe documentation consolidation/deletions, Mandos,
Aule, Varda, Athena, generated runtime state, and other project changes.

## 5. Bounded stability soak — PENDING

Run 20–50 low-concurrency requests without stressing the VRAM-constrained
`edge_core` lane:

- Repeated single-tool delegates.
- Repeated two-step delegates.
- One representative larger tool schema.
- Provider/model route receipt capture.
- Service restart followed by acceptance.
- Confirmation that no legacy unit can reclaim port 5110.

## 6. Documentation and rollback closeout — PENDING

Record and verify:

- Production service and listener identity.
- Effective Hermes configuration keys.
- Canonical acceptance transcripts.
- Why legacy Charon remains masked.
- Exact rollback unit path:
  `/var/home/mythos/.config/systemd/user/annunimas-charon.service.rollback`.
- Exact rollback commands.
- Any remaining legacy compatibility naming or consumer migration.

## 7. Broaden local delegation qualification — PENDING

Qualify each lane independently against the same native-tool acceptance gate:

- `edge_beelink_light` for cheaper/light work.
- `edge_backbone` for high-quality local reasoning without tools.
- `edge_core` for local tool/code execution.
- `local/auto` for fail-closed local selection.

Do not move Hermes away from the known-good pinned `edge_core` route until each
candidate lane passes independently.
