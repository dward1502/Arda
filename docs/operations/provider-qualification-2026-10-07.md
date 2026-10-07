# R06 provider qualification — October 7, 2026

R06 remains open, with rollout and bounded live-provider qualification now recorded.
The health repair is installed, three Hermes auxiliary endpoints are repaired,
and subscription tool use, emulated streamed-tool replay, native OpenRouter
streaming and a Hermes auxiliary request passed. Local inference was unavailable.
No real-channel message was sent; canonical fallback/budget gates remain open.

## Initial installed observations (before rollout)

Evidence: `target/qualification/r06-20261007/installed-snapshot.json`.
It contains allowlisted route metadata, credential-variable presence (never
credential values), configuration and installed-binary digests, timings and units.

| Surface | Observation |
| --- | --- |
| Installed ownership | Arda PID 973416, Manwe child PID 973433; root restart count 0. |
| `/paths` | HTTP 200 in 9 ms; active dynamic config is repository `config/manwe.providers.toml`. |
| `/health`, `/v1/models` | Both timed out at 5 seconds; an earlier health request also timed out at 10 seconds. Live provider health/catalog could not be reconciled. |
| Configured capacity | 22 providers; 8 enabled: five edge routes, OpenRouter, Google and `openai_sub`. Enabled configuration is not proof of availability. |
| Credential prerequisites | `OPENROUTER_API_KEY` and `GEMINI_API_KEY` were present in Manwe's environment. Presence does not prove validity, quota or spending authority. Subscription authentication was not exported or tested. |
| Hermes primary/delegation | Both configured for `openai-codex` / `gpt-6-astra`; primary endpoint is the configured Codex backend. These are configured identities, not actual-route receipts for this run. |
| Hermes auxiliary routes | Compression, session search and memory flush reference `custom:arda-manwe` at port 5110. No listener exists there; the installed gateway listens on 7171. Correct these through the existing Hermes configuration owner before relying on those functions. |
| Static gateway file | Repository `manwe.toml` contains `api_key = null`, invalid TOML. Current loader code falls back on malformed static configuration; `/paths` independently confirms the dynamic catalog path. No static config was changed here. |
| Offline capacity | `mesh-llm.service` and its probe remain inactive. No provider was enabled to obtain a passing result. |

## Health/diagnostic boundary repair

Source inspection found `/health` calling full `status()` diagnostics while
holding the provider write lock. That work reads complete event histories,
counts malformed records and runs external runtime probes. `/v1/models` needs
the same provider lock. Installed histories totalled 311,085,710 bytes, consistent
with a costly diagnostic path; this is a source-supported explanation, not a
claim that an installed rollout has proved the sole cause.

Health now uses a consistent provider/config-generation snapshot and releases
its locks before reading a bounded tail: at most 256 KiB for up to 48 recent valid
records. The existing health response fields and readiness decision remain;
`recent_event_sample_complete` and `recent_event_sample_limit` disclose whether
the recent-event sample covers the requested window. Truncated or unreadable
history is not silently represented as a complete zero-failure audit.

Full `/status` diagnostics retain their existing fields and full-history audit,
but execute on a blocking worker after the provider snapshot is released. No
routing policy, provider selection, fallback authority, quota or receipt changed.
Full diagnostics remain potentially expensive; this repair isolates their cost
from readiness and catalog lock ownership.

## Qualification matrix

| Requirement | Current evidence | Remaining gate |
| --- | --- | --- |
| Health and unavailable capacity | Isolated candidate returns HTTP 503 promptly with all fixture providers disabled; catalog stays responsive. | Installed checks now pass; retain unavailable-capacity fixture coverage. |
| Tool use / streamed tool replay | Existing Manwe source regressions pass. | Subscription tool loop and emulated streamed replay pass; native OpenRouter streaming passes. Local route remains unavailable. |
| Timeout / quota / failure | Source timeout, quota and failure-classification regressions pass; installed health/catalog timeouts captured. | Missing/local-only routes refuse with HTTP 502. Upstream timeout/quota and retry-count qualification remain open. |
| Fallback / privacy / budget | Existing routing, local-only and fallback policy regressions pass. | Admitted canonical work proving authorized fallback and refusal without authority; measured retry/token/cost evidence. |
| Channel duplicates / cancellation / reconnect / acknowledgement | No live-channel test performed. | A bounded real-channel fixture with recipient and delivery evidence. |

Validation: 303 Manwe library tests passed, including five new health/tail
regressions; strict library Clippy and workspace formatting passed. The isolated
candidate used copies of both installed histories, disabled all fixture providers,
made no inference requests and wrote no package diagnostic output. Three health
responses took 3.8–4.6 ms; three catalog responses took 1.2–1.7 ms in the final
run. Timings and candidate binary/source identities are retained with the
qualification evidence. The candidate process was stopped and its fixture removed.

The source baseline was `e9478631`; validation includes the existing working-tree
provider changes. The commit includes only the health repair and this R06 record.
This initial fixture alone did not establish live catalog/route agreement.
The following rollout supersedes that limitation for the sampled routes;
P2–P6 and G2.1–G2.7 remain open.


## Installed rollout and live checks

Evidence directory: `target/qualification/r06-rollout-20261007/` (local, ignored).
The release build passed (`cargo build --release -p manwe --bin manwe -j2`).
The prior executable was preserved as `manwe.before` before building. Only the
supervised Manwe child received SIGTERM; Arda's supervisor restarted it from the
release path. Root PID 973416 remained active with zero root restarts; Manwe
changed from PID 973433 to 1189229. The executable read through `/proc/1189229/exe`
matched SHA-256 `5e2190e9426804c5d63d3a9ab189948a26adddbca732d1b1a5d69c3cae2693d6`.
This binary includes the existing working-tree provider changes, as did the prior
303-test qualification; it is not a claim of a clean-tree reproducible release.

Immediately after rollout, `/health` returned 200 in 10.175 ms and `/v1/models`
in 0.899 ms. After all live checks, three health requests returned 200 in
1.160–10.582 ms; three catalog requests returned 200 in 0.591–0.754 ms with
127 entries. Catalog entries do not imply that every model is usable.
The root and Manwe PIDs remained stable. Both mesh services remained inactive.

Exactly three URLs in `~/.hermes/config.yaml` changed from port 5110 to 7171:
`auxiliary.compression`, `auxiliary.session_search`, and
`auxiliary.flush_memories`. A parsed-config comparison checked that no other
setting changed. The original configuration is retained privately with mode 0600;
never stage or publish that backup. The installed Hermes runtime resolved all
three clients to `http://127.0.0.1:7171/v1/`, model `gpt-6-astra`. One synthetic
session-search-client request then passed in 2.859 seconds, with actual route
`openai_sub/gpt-6-astra`, route ID `aea36621aaf0d2c4`, and 33 reported total tokens.
This exercised the client transport, not a search of user sessions or a memory
flush. Existing long-lived client instances were not inspected or restarted.

| Live check | Result and actual-route evidence |
| --- | --- |
| Subscription tool loop | PASS: `openai_sub/gpt-6-astra`; route IDs `50cd34bf1e30fa7b`, `fb8bc877a6bca6c0`; reported route latencies 2.544 and 6.260 seconds. A random receipt generated only after the tool call was consumed exactly. |
| Streamed tool replay | PASS: same provider/model; routes `76fe4ab98fec09c5`, `ac09df2690c2b910`. Both responses explicitly carried `x-manwe-stream-emulated: true`, valid SSE termination and linked tool-result consumption. This is buffered stream emulation, not native subscription streaming. |
| Native streaming | PASS: pinned `openrouter/nvidia/nemotron-3-super-120b-a12b:free`, route `bfe3a2a15f83926d`; first event 507 ms, complete in 1.091 seconds, exact marker and `[DONE]`. Upstream reported 28 prompt + 59 completion = 87 tokens, including 51 reasoning tokens, and cost 0. |
| Native subscription streaming | HTTP 502 in 10.541 ms. Route policy excludes native streaming for `codex_responses` and `hermes_agent_cli`; no native-stream success is claimed. |
| Local tool use | HTTP 502 for pinned `edge_core/Qwen3.5-9B-Q4_K_M`; subsequent local-only request rejected unavailable capacity. No local service was enabled. |
| Missing-route/local-only refusal | HTTP 502 with no actual-route receipt and forced fallback disabled. This bounded refusal is not proof of the entire canonical privacy/fallback matrix. |

All successful checks pinned provider/model and disabled forced-provider fallback.
The tool loops capped each response at 256 tokens; marker requests at 64 tokens.
The free-route cost is provider-reported, not an independent billing audit.
Subscription cost and upstream retry accounting were not established. No paid
fallback, artificial quota exhaustion or real-channel messaging was performed.

Remaining: upstream timeout/quota and retry accounting; admitted canonical work
with privacy, budget and fallback evidence; local-capacity qualification when
available; real-channel duplicates, cancellation, reconnect and acknowledgement.
See the [storage audit](storage-retention-2026-10-07.md) for the separate history
retention and corruption findings. No history was deleted during rollout.
