---
soterion:
  sigil: "SCROLL"
  role: "assessment_evidence"
  owner: "HERMES"
  status: "draft"
  reviewed: "2026-10-09"
---

# Inference P0 review

P0 is partially implemented, not an accepted deployment or benchmark. Live
probes began at 2026-10-10T04:44Z (October 9 PDT). No service, routing,
privilege, model installation or running workload was changed by this review.
No generation benchmarks were run. The next gate requires operator budgets.

## Runtime corrections

Evidence: [fresh process/API probes](p0-live.json),
[core ownership](p0-core-ownership.json),
[server checkout identities](p0-server-checkouts.json).
Reproduce targeted fleet observations with `python p0-probe.py --out PATH` from
this directory; choose a new output path to preserve the original snapshot.
The probe assumes Core is the local host and uses the recorded SSH identities.

- Core: the active model belongs to `mesh-llm.service`, PID 3066405,
  executable `~/.local/bin/llama-server-cuda`, Qwen3.5-9B Q4_K_M,
  context 73728. LM Studio `lms ps --json` returned an empty list. Its llmster
  process remains alive but is not evidence of loaded model weights.
- Core legacy `llama-server.service` remains active as distrobox/conmon and a
  `podman logs` follower. Its cgroup does not show another llama-server.
  This is ownership discovery, not permission to stop the wrapper/container.
- Server: Bonsai uses `llama.cpp-prism/build-cuda/bin/llama-server`,
  context 100000. Nanbeige uses `llama.cpp-nanbeige/build-cuda/bin/llama-server`,
  context 32768, NOT the separately inspected stock checkout.
  Checkout heads are respectively `9fcaed763ccda38ea81068ad9d7f991aaddca451`
  and `c6640a1c0cf7b38df342b67021a3900b04d092e7`; the stock checkout is
  `571d0d540df04f25298d0e159e520d9fc62ed121`.
  A checkout HEAD is not proof of the source used to compile a running binary.
- Beelink: actual executable is `llama.cpp-prism/build-prism/bin/llama-server`,
  serving Ternary-Bonsai-8B-Q2_0 through Vulkan0, context 32768.
- Warden: actual command is Qwen3.5-4B-Q4_K_M with `-c 4096`.
  Both `config/fleet.toml` and `config/manwe.providers.toml` advertise 262144.
  Training context metadata is not deployed capacity. Use 4096 as the review
  ceiling; a later authorized correction must trace the live config consumer,
  not increase the Pi allocation to match the advertisement.
- Laptop: no matching inference process or model API was observed. This does
  not mean SSH failed. CITADEL is excluded, not an assumed spare worker.
- Executable SHA256 values are retained per process. Full model hashes,
  embedded templates, library linkage/build provenance and rollback bundles
  remain P1 work. Empty GGUF maps on `--no-mmap` services do not mean no model.
  Short-lived `/proc` races and expected unavailable ports remain raw errors,
  not successful observations.

## Corrected candidate and compatibility matrix

Pins and publisher-provided LFS sizes/digests are in
[model metadata](p0-model-metadata.json) and
[alternative metadata](p0-alternatives.json). These are remote identities, NOT
locally downloaded/hash-verified weights. Retrieved cards are retained beside
this report under `card-*.md`; they are publisher claims, not fleet measurements.
Runtime source resolutions are in [runtime pins](p0-runtime-pins.json).

| Candidate | Artifact / license | Template, projector and runtime boundary | Disposition |
|---|---|---|---|
| Saluki 27B 1.0 | IQ2-mix; 7898369152 bytes; Apache-2.0 | Qwen3.8 Jinja, enable_thinking toggle; optional F16 or Q8_0 projector. Card says stock llama.cpp but does not pin a minimum commit.[1] | First text-only replacement experiment after baseline; exact runtime/template verification still open |
| Kolibri-1 | Independent Hob-forge Q4_K_M; 47454113472 bytes; Apache-2.0 weights, MIT patch | Embedded Jinja; reasoning_effort none/low/medium/high; text only. Patch on upstream 836d57176dc699a726c55418e4f96b8ca628e1bf; patch itself pinned by HF repository revision.[2][3][13] | Server RAM/offload research lane, not an assumed low-latency winner |
| Clef | bartowski Q4_K_M; 17652432960 bytes; Apache-2.0 | No chat-generation interface (official repo does contain `chat_template.jinja`): /v1/systemone; decision head Q8_0; optional BF16/F16 projector. b11430 resolves to 8345f333951c661d166b00e6f9362e553768f292.[4][5][12] | Separate advisory classifier experiment; not a Hermes chat replacement |
| Mitsuba / HiMitsuba | Artifact and license unresolved | Unauthenticated HF API returned 401 for both investigated repositories; no downloadable artifact/revision/projector verified.[6][7] | Blocked, not selected; a search snippet or historic commit is insufficient |
| Bonsai 2 27B | PTQ1_0 5946648928 bytes or PQ2_0 7206168928 bytes; Apache-2.0 | Requires Prism kernels/Hadamard runtime; stock llama.cpp is not compatible. Optional Q8_0 projector; exact tested runtime pair still needed.[9] | New candidate distinct from installed Bonsai-27B-Q1_0, not an interchangeable file |
| MiMo-V2.6-Distill-Qwen-9B | Upstream MIT; independent Q4_K_M 5629105344 bytes has license metadata 'other' | MiMo template and reasoning parser differ; upstream recommends SGLang. Independent GGUF includes a BF16 projector but local parser compatibility is untested.[10][11] | Candidate for Core-size budget, pending license and tool-parser verification |

Prism branch HEAD resolved to `6684606ac1bfcf8e31465369d9365df03b3af904`.
That is an observation, not a selected/qualified deployment pin.[14]

## Placement corrections and bounded experiment proposal

These are review recommendations, not benchmark results or approved windows.

1. Preserve existing Qwen, Bonsai and Nanbeige as old controls. Compare the same
   model on old/new runtime before claiming model improvements.
2. Do not assume Saluki fits usefully on one 8 GiB server GPU: weight size alone
   excludes KV/cache/workspace and desktop overhead. Begin modest text context
   on one reserved lane with partial offload if necessary, preserving the other
   lane. Dual-GPU trials need a separately approved window.
3. Core GPU already reported 8006 MiB occupied, including 6076 MiB for its model.
   No second model should be loaded alongside it just because its file is small.
4. Clef Q4_K_M exceeds even the server's combined nominal 16 GiB VRAM before
   overhead. Require CPU offload or another measured placement; compare against
   cheaper classifiers before accepting its resident cost.
5. Kolibri's publisher CPU/offload timings use a different DDR5 platform. They
   do not predict the Threadripper's DDR4 performance.[3]
6. Bonsai 2 is a worthwhile low-memory candidate, but the published speed table
   is not a Turing qualification. Keep the working Prism fork untouched.[9]
7. First proposed suite: sanitized tool/no-tool and multi-turn tool fixtures,
   citation-grounded research, a bounded code task, then retrieval. Voice and
   image-to-prompt outcomes stay in scope but need separate fixtures, not a
   claim that chat quality covers them. Context 4K first, concurrency 1; expand
   only after correctness and observed headroom. No latency/energy threshold is
   accepted until the operator sets priorities and budgets.

## Existing harness and remaining gate

`scripts/benchmark_local_model.py` already exists; do not invent a second general
benchmark runner. Inspection of lines 57-82 found that NVIDIA sampling selects
only the first GPU and AMD sampling combines VRAM/GTT totals. Correct and test
per-device attribution before using it for multi-GPU or UMA comparisons. Its
endpoint tests are not actual Hermes/Arda client parsing or routed acceptance.
No benchmark source was changed in this P0 review.

Remaining P0/P1 inputs:
- operator priority order and acceptable interactive/background latency;
- disk/download cap, energy/thermal constraints and allowed maintenance window;
- exact source-to-binary provenance, model/template hashes and rollback capture;
- Saluki runtime pin, Mitsuba artifact access, and MiMo license/parser resolution.

Proposed next scope: retain existing-service baseline and test contract without
upgrades; fix harness measurements with failing fixtures; use sanitized prompts
and sequential runs only within an agreed load budget. No rollout is implied.

## Sources

[1] https://huggingface.co/ConwayResearch/Underdog-Saluki-27B-1.0
[2] https://huggingface.co/Aleph-Alpha/Kolibri-1
[3] https://huggingface.co/Hob-forge/Kolibri-1-GGUF
[4] https://huggingface.co/Cloudflare/clef
[5] https://huggingface.co/bartowski/Cloudflare_clef-GGUF
[6] https://huggingface.co/isichan-ai/Mitsuba_and_HiMitsuba-27B-GGUF
[7] https://huggingface.co/isichan-ai/Mitsuba-ComfyUI-27B-GGUF
[9] https://huggingface.co/prism-ml/Ternary-Bonsai-2-27B-gguf
[10] https://huggingface.co/XiaomiMiMo/MiMo-V2.6-Distill-Qwen-9B
[11] https://huggingface.co/prithivMLmods/MiMo-V2.6-Distill-Qwen-9B-GGUF
[12] https://api.github.com/repos/ggml-org/llama.cpp/commits/b11430
[13] https://api.github.com/repos/ggml-org/llama.cpp/commits/836d571
[14] https://api.github.com/repos/PrismML-Eng/llama.cpp/commits/prism
