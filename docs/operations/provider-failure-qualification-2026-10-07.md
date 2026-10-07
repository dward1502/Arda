# R06 failure qualification and privacy repair — October 7, 2026

The next failure-matrix slice exposed a real HTTP-boundary defect: OpenAI request
normalization omitted `local_only` and the supported `privacy_tier` alias. With
the requested local provider disabled and fallback permitted, the installed
binary selected a cloud-labelled fixture despite `local_only: true`. The test
used synthetic text and loopback-only upstreams; no private content or real
cloud provider was involved. This finding supersedes any inference that the
earlier pinned-route refusal alone proved local-only enforcement.

## Boundary repair

First extracted existing body-option normalization from `http.rs` into
`http/request_options.rs`, preserving existing option handling byte-for-byte.
Then added the two missing privacy selectors at each supported location:
top-level body, `routing`, `extra_body`, and `extra_body.routing`. Header handling,
route policy and other option precedence remain with their existing owners.
No fallback authority, quota, privacy policy or provider configuration was widened.

The old installed binary failed all four local-only layouts and the
`privacy_tier: confidential` case. The repaired release refused all five with
HTTP 502 and zero inference requests to either fixture. Permitted fallback
without a privacy restriction still succeeded, providing a positive control.

Validation: 306 Manwe library tests passed, including three new normalization
regressions; strict library Clippy and workspace formatting passed. The release
build used cached dependencies offline. Existing unrelated working-tree provider
changes were included in the tested binary, as in the earlier rollout; this is
not a claim of a clean-tree reproducible release.

## Reproducible process matrix

`scripts/qualification/check_manwe_failures.py` starts a fresh process of the
specified real executable for each scenario. It uses temporary HOME/root/state,
an explicit environment without credentials, loopback fixture endpoints for all
configured providers, and verifies the enabled catalog before each request.
It stops each child and its fixtures afterward. Production configuration and
histories are never loaded by the fixtures.

| Scenario | Repaired release observation |
| --- | --- |
| Successful pinned local fixture | HTTP 200; one local request, zero cloud-labelled requests; actual provider/model match. |
| Upstream 429 | HTTP 502 after one request; `rate_or_retry_error`; no cloud fallback. |
| Upstream 402 | HTTP 502 after one request; `billing_or_credit_error`; no cloud fallback. |
| Upstream 503 | HTTP 502 after one request; `provider_server_error`; no cloud fallback. |
| Upstream 400 | HTTP 502 after one request; `client_payload_error`; no cloud fallback. |
| Upstream deadline | HTTP 502 in 25.065 seconds; exactly one request; `request_timeout`; no cloud fallback. |
| Local provider disabled, fallback forbidden | HTTP 502; zero upstream inference requests. |
| Local provider disabled, fallback permitted | HTTP 200 through the cloud-labelled fixture; exactly one request with matching actual-route receipt. |
| Local provider disabled, fallback permitted, local-only privacy | HTTP 502 with zero upstream requests for each of four body layouts. |
| Same outage with confidential privacy alias | HTTP 502 with zero upstream requests. |

All **13 scenarios passed**. Non-timeout cases completed in 61.9–67.0 ms.
Failure classification was asserted against the gateway's tool-fit ledger,
independently of fixture request counters. Requests capped output at 16 tokens.
Successful fixtures report six synthetic tokens; no real model tokens or provider
spend occurred. Those invented fixture usage values are not billing evidence.

Reproduce with a fresh output directory:

```bash
python3 scripts/qualification/check_manwe_failures.py \
  --binary target/release/manwe \
  --output /tmp/manwe-failure-qualification
```

Use repeated `--case NAME` arguments for targeted checks. Existing output
directories are refused to avoid overwriting prior evidence. The timeout case
uses the real 25-second interactive deadline for `edge_guardhouse`.

## Installed rollout

The tested binary replaced `target/release/manwe` atomically after preserving
the prior binary as a rollback copy. Only the supervised Manwe child restarted:
PID 1189229 became 1356962; Arda root PID 973416 remained active with zero root
restarts. Installed executable SHA-256, read through `/proc/1356962/exe`, matched
the qualified release:
`288c6c95fb11139ad9409e7ba761203e8319a7079fd93b3562af6e7d801bb641`.

After rollout `/health` returned 200 in 1.415 ms and `/v1/models` returned 200 in
0.582 ms. A synthetic installed request combining `local_only: true` with an
explicit OpenRouter pin returned 502 in 15.518 ms with no actual-route receipt.
Both mesh services remained inactive. The temporary build directory was removed
after rollout, reclaiming 1,368,981,504 allocated bytes; binaries, rollback and
qualification receipts remain.

Local evidence directories under `target/qualification/`:

- `r06-failures-20261007`: initial nine-case discovery, including the privacy failure.
- `r06-privacy-before-20261007`: all five privacy cases failing on the old binary.
- `r06-privacy-after-20261007`: all 13 cases passing on the repaired binary.
- `r06-privacy-rollout-20261007`: build/test logs, rollback binary, installed identity,
  health/refusal checks and build cleanup record.

## Acceptance boundary and next step

This qualifies the HTTP gateway's controlled error classification, deadline,
attempt bounds and privacy/fallback admission. It does not establish real-provider
quota/billing accounting or retries after a successful first route selection.
The permitted-fallback case starts with a disabled local provider; it does not
demonstrate failover after an upstream request has already failed.

The canonical G2 matrix remains open. Next trace admitted resident requirements
through the placement and adapter receipt boundary (P2/P3), then qualify canonical
privacy, cost ceilings and authorized fallback using an isolated admitted task.
Local-capacity execution remains unavailable while mesh is intentionally off.
Real-channel duplication, cancellation, reconnect and acknowledged delivery
still need an explicitly authorized recipient and transport fixture.
