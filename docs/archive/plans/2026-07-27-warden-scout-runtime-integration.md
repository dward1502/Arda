# Warden Scout Runtime Integration

**Status:** Complete — deployed, centrally reachable, and verified
**Owner:** Warden outpost with Arda/Varda governance
**Target:** `node-pi5-warden` (`numenor@warden`, aarch64)

## Objective

Turn `arda-outpost-scout` from a library into an always-on Warden capability that can:

1. run bounded repository surveys on the Warden Pi5,
2. perform bounded internet discovery through a local SearXNG service,
3. persist every survey/research result as an advisory `OutpostObservation`,
4. answer scoped recall queries over Warden's local observation memory,
5. expose a Tailscale-only HTTP surface for Arda consumers,
6. feed discovered source candidates into ATHENA/`arda-varda` without granting task or policy authority.

## Architecture decision

The Pi5 is the correct execution host for collection and lightweight querying because it is always on, already enrolled as `node-pi5-warden`, has a live local LLM lane, and is explicitly assigned bounded internet/repository scout work in `config/fleet.toml`.

Authority remains split:

- **Warden Pi5:** collection, search discovery, local advisory memory, health, and recall.
- **`arda-vaire`:** canonical observation-memory envelope and recall metadata.
- **ATHENA / `arda-varda`:** source ingestion, crawling, digestion, knowledge governance, and any later policy-readiness workflow.
- **Manwe/council/operator:** decides when advisory evidence warrants further work.

Search results must never mutate task queues or become policy-ready automatically.

## Runtime topology

```text
Arda/council/Manwe
        |
        | Tailscale HTTP: search, survey, recall, health
        v
Warden Pi5: arda-outpost-scout runtime
        |-- local SearXNG :18080
        |-- local arda-vaire root
        |-- bounded scheduled research topics
        `-- advisory observations + ingestion receipts

Selected source candidates
        |
        v
ATHENA / arda-varda ingest/crawl/governance
```

## Implementation slices

### P0 — Warden research contract

**Complete.**

- Add typed SearXNG request/result/report models.
- Enforce bounded result counts and request timeouts.
- Convert reports to advisory `OutpostObservation` records with query, provider, URLs, snippets, timestamps, and provenance.
- Add deterministic fixture tests with a local mock HTTP server.

### P1 — Runnable outpost service

**Complete.**

- Add an `arda-outpost-scout` binary.
- Expose:
  - `GET /health`
  - `POST /search`
  - `POST /survey`
  - `POST /recall`
  - `POST /research/run`
- Persist successful observations through `ObservationMemoryBridge`.
- Return structured unavailable/degraded states instead of silently dropping evidence.
- Bind to the configured Tailscale address, not the public internet.

### P2 — Recurring research tasks

**Complete.**

- Add a versioned Warden research-topic config.
- Run only enabled, bounded topics at a conservative interval.
- Keep source discovery advisory and deduplicated by normalized URL/query window.
- Record receipts and failure observations.

### P3 — Deployment

**Complete.**

- Build an aarch64 binary without requiring a Rust toolchain on Warden.
- Deploy binary, config, and user-systemd unit to `numenor@warden`.
- Deploy SearXNG with loopback-only port `18080` and JSON output enabled.
- Enable linger-backed `arda-warden-scout.service`.
- Verify health, live search, persistence, recall, service restart, and negative public-bind posture.

### P4 — Arda consumer handoff

**Complete for bounded observation and recall handoff.** The Arda harness now
discovers Warden's `scout_url` from `config/fleet.toml` (with
`ARDA_WARDEN_SCOUT_URL` as an override) and exposes `/v1/scout/health`,
`/v1/scout/search`, and `/v1/scout/recall`. Automatic ATHENA ingestion remains
intentionally excluded: source promotion requires a separate governed selection
step rather than allowing Warden to turn discovery directly into learned or
policy-ready knowledge.

- Register Warden's scout endpoint in fleet/runtime configuration.
- Add a narrow consumer/client path for Manwe/council/ATHENA to request search or recall.
- Forward selected URLs to ATHENA's governed ingest/crawl path only; do not write task queues from Warden.
- Capture end-to-end request, observation, memory, and consumer receipts.

## Safety and resource bounds

- Tailscale-only API bind.
- SearXNG bound to Pi loopback only.
- Search limit capped at 10 results per query.
- Scheduled topic count capped and interval no faster than hourly.
- HTTP connect/read/total timeouts required.
- No credential files or search history committed to the repository.
- No automatic task promotion, policy promotion, software installation, or destructive action from search results.
- Pi local model remains optional for summarization; raw source evidence and provenance must survive independently of model output.

## Acceptance evidence

- Focused Rust tests pass for protocol, scout, Vaire, and Varda integration surfaces.
- Workspace `cargo check` passes.
- Warden responds on its Tailscale scout endpoint.
- A live internet query returns provenance-bearing results.
- The resulting observation has a Vaire memory id and is retrievable through `/recall`.
- SearXNG is not reachable through Warden's non-loopback port `18080`.
- Restarting the Warden service preserves recallable observations.
- No task queue or policy-ready ledger changes during the acceptance run.

## Verified closeout — 2026-07-27

- `cargo check --workspace`: pass.
- `cargo test -p arda-engine`: 9 unit tests and 1 integration test passed.
- `cargo test -p arda-outpost-scout`: 20 tests passed across unit, memory,
  observation, research, runtime API, runtime CLI, and survey suites.
- Strict Clippy passed for `arda-engine`, the root `arda` binary, and
  `arda-outpost-scout` (the scout command retains explicit allowances for three
  existing style-only lints).
- Live Arda harness test bound `127.0.0.1:17878`, discovered
  `http://100.110.85.37:8092`, and successfully proxied health, search, and
  recall to Warden.
- Live search `Arda bounded scout verification` returned one bounded result and
  memory receipt `mem_871dc4acab804f788f6d4a57a0df628a`.
- Warden scout restart completed with `active` state; after readiness, recall
  returned `available` with the persisted record, proving restart persistence.
- Warden service state: scout and SearXNG services active/enabled; research and
  survey timers enabled.
- Socket posture: scout listens on Tailscale `100.110.85.37:8092`; SearXNG
  listens only on `127.0.0.1:18080`. A workstation probe to
  `100.110.85.37:18080` was correctly refused.
- SHA-256 values for `core/state/queue_active.json`,
  `core/state/queue_summary.json`, and `data/governance/bacon_lite.jsonl` were
  identical before and after the central harness search, proving the acceptance
  query did not promote work or mutate governance state.
