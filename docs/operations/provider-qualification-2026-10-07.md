# R06 provider qualification — October 7, 2026

R06 remains open. Initial route reconciliation found an installed health/catalog
stall before live inference qualification. A source repair passed tests and an
isolated process check; the installed daemon has not been replaced or restarted.
No model request or real-channel message was sent during this qualification.

## Installed observations

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
| Health and unavailable capacity | Isolated candidate returns HTTP 503 promptly with all fixture providers disabled; catalog stays responsive. | Roll out the qualified health repair and repeat installed checks. |
| Tool use / streamed tool replay | Existing Manwe source regressions pass. | Actual pinned-route tool loop and streamed response, with actual provider/model receipts. |
| Timeout / quota / failure | Source timeout, quota and failure-classification regressions pass; installed health/catalog timeouts captured. | Bounded real route failure with recorded attempt count and latency. |
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
P1 is partial because live catalog and actual route agreement remain unproven;
P2–P6 and G2.1–G2.7 are not closed by source tests or this disabled-provider fixture.
