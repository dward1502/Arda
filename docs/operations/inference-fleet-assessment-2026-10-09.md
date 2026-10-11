# Inference fleet assessment - 2026-10-09

Historical snapshot (2026-10-09): read-only hardware audit and sourced research complete; replacement deployment and comparative benchmarks had NOT yet been performed. Recommendations below were candidates, not measured winners. No service or model changes were made during that initial audit. Subsequent experiments, deployment and current acceptance status belong to [the active inference assessment plan](../plans/INFERENCE_ASSESSMENT.md).

## Verified hardware and current deployment

Live SSH/system/GPU/API inspection covered core, server, Beelink, laptop and Warden. CITADEL remains outside usable capacity until its boot issue is resolved.

| Host | Hardware observed | Present inference | Recommended role to test |
|---|---|---|---|
| Core | Ryzen 5 5600X, 32GB class RAM, RTX3080 10GB | Qwen3.5-9B Q4_K_M on 9337; configured 73728 context; 8117 MiB total GPU use at audit | Interactive general agent and short research synthesis; one resident generation model |
| Server | Threadripper2950X 16 cores, 128GB class RAM, two RTX2080Super 8GB | Bonsai27B Q1 on 8097 and Nanbeige4.2-3B Q4 on 8098; model APIs respond | Independent worker lanes normally; scheduled large MoE/offload experiments |
| Beelink | Ryzen AI9 HX PRO370, Radeon890M, 29000458240 bytes OS-visible RAM | Prism Ternary-Bonsai8B Q2 on 9337 | Background extraction/summarization; CPU retrieval/TTS; test Vulkan vs HIP |
| Laptop | i5-13420H, 7917580288 bytes system RAM, RTX3050 6GB | No responding model endpoint on checked ports; Qwen3.5-4B file and inactive service exist | Optional voice/vision and compact offline assistant, not required fleet capacity |
| Warden | Pi5 Cortex-A76 4 cores, 8453947392 bytes RAM | Qwen3.5-4B CPU, actual service context 4096; scout and SearXNG active | Retrieval/scout/control, bounded short local inference |

Server GPU topology is PHB, not NVLink. Two 8GB devices are not a transparent 16GB GPU. Beelink reports a 4GiB VRAM allocation and 30GiB GTT limit; GTT is an addressable shared-memory limit, not additional physical RAM. Do not sum these with OS RAM.

Important drift: Warden inventory advertises a 262144 model/context capacity but the actual service is configured at 4096. Core also has an active legacy llama-server service wrapper and llmster processes; this does not establish duplicate loaded models. Inspect loaded processes/GPU allocations before any benchmark, rather than killing services by name.

### Runtime age and provenance

- Core checkout: March 15 source; running service binary previously identified itself as b8668/b2f6a68ae. Checkout date is not proof of running executable provenance.
- Server stock checkout: July 18; checked build-cuda2 binary reports 10068 (571d0d540). Prism binary reports 1 (9fcaed7).
- Beelink stock checkout: March 15, but active Prism checkout/binary is July 18 / 9fcaed7.
- Laptop checkout: March 19; currently no loaded inference service on checked ports.
- Warden checkout: March 13; active CPU service.

These justify a runtime refresh experiment. They do not establish that every installed model is inferior merely because it is older. Preserve exact binaries/templates as baselines and test replacement candidates separately.

## Priority candidate models

### 1. Saluki: first new general-agent trial

ConwayResearch/Underdog-Saluki-27B-1.0 is a Qwen3.8-27B low-bit derivative, Apache2. Its IQ2-mix GGUF is 7.89GB decimal. Current card documents stock llama.cpp with Jinja template, thinking control, and optional 629MB Q8 or 928MB F16 vision projector. This corrects launch text that called it text-only.

Test first on server, with an explicit GPU split/offload plan, then on core during an isolated window. Core already uses about 8GB VRAM with the current service/desktop; do not load Saluki alongside it. Full GPU fit at useful context is unverified. Begin text-only, modest context, and thinking off for direct tool calls.

Publisher reports 88/120 selected BFCL tasks and 30/50 SWE-bench issues. These are publisher subsets, not a complete independent benchmark. Strict parallel tool-call result of 42/100 cautions against treating the model as a reliable autonomous executor without local validation.

Sources: https://underdog.ai/saluki
https://huggingface.co/ConwayResearch/Underdog-Saluki-27B-1.0

### 2. Kolibri: server-only large-model experiment

Aleph-Alpha/Kolibri-1: 78B total, 3.46B active MoE, Apache2. Native context is 262144, with a validated extension to 1M; neither is a practical fleet promise. Active parameters reduce per-token computation, not weight residency requirements.

Community GGUF Q4_K_M is 47.5GB; Q5_K_M 55.6GB; Q2_K 28.6GB. Server RAM makes Q4/Q5 plausible. Core, Beelink and laptop are not recommended hosts. Use CPU/RAM expert offload, measure prompt processing as well as generation, and start with much shorter context. The 2950X memory subsystem may dominate latency.

Crucially: community conversion requires its specific Kolibri llama.cpp patch/fork. Stock llama.cpp or ik_llama.cpp support must not be assumed. Official vLLM plugin support does not imply the official FP8 kernels work on Turing GPUs. Treat the patched runtime as an isolated experimental service and verify numerical/template behavior.

Sources: https://huggingface.co/Aleph-Alpha/Kolibri-1
https://huggingface.co/Hob-forge/Kolibri-1-GGUF
https://github.com/Aleph-Alpha/aleph-alpha-inference

### 3. Clef: typed decision/classification worker

Cloudflare/clef is not a chat generator. It reads state plus typed questions and returns option probabilities. Potential jobs: triage, classification, relevance, routing and bounded workflow decisions. Do not use its output as independent authority for consequential execution.

The current bartowski GGUF documents architecture clef, Q8 decision-head tensors, llama.cpp b11430 or later and /v1/systemone. This is not a drop-in /v1/chat/completions replacement. Q4_K_M is 17.65GB before context/buffers/projector; server hybrid offload is the first placement. Beelink may be memory-feasible but its current available RAM and latency leave less margin. Do not assume backbone-only GGUF files preserve the decision model.

Benchmark calibration, abstention and task accuracy against a compact classifier/general model before dedicating this much memory. No generation tokens/second metric is appropriate for its decision endpoint.

Sources: https://huggingface.co/Cloudflare/clef
https://huggingface.co/bartowski/Cloudflare_clef-GGUF
https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md

### 4. Mitsuba: image-to-prompt specialist, availability unresolved

User-supplied release and indexed card describe a Qwen3.8-27B derivative specialized for ComfyUI image/video prompts, recommended 7.3GB PQ2_0 weights, optional smaller PTQ1_0 with vision trade-offs, PrismML runtime and thinking off. Indexed card recommends a 16GB GPU. Direct retrieval of both original and renamed repositories returned 404 during this assessment; artifact availability, projector requirements and current license remain unverified.

Treat as a specialist candidate, not coding/general-agent replacement. Server is the first fit experiment if multi-GPU vision support is confirmed; Beelink is a potential shared-memory alternative. Core is tight and laptop not a first target. Generate prompts remotely rather than keeping this VLM and an image generator resident together on core. Evaluate image grounding, retained composition/details, hallucinated objects, prompt controllability and final generation usefulness. Claims of superiority over Bonsai/Qwen remain publisher claims.

Discovery URLs: https://huggingface.co/isichan-ai/Mitsuba-ComfyUI-27B-GGUF
https://huggingface.co/isichan-ai/Mitsuba_and_HiMitsuba-27B-GGUF

## Supporting specialists and controls

These are established comparison candidates, NOT a list of the newest releases or a justification to replace current models with older ones.

- Keep existing Qwen3.5-9B/4B, Bonsai and Nanbeige as measured controls.
- Qwen3-Coder-30B-A3B-Instruct: Apache2, 30.5B total/3.3B active, Q4 around18.6GB. Server offload coding challenger; full two-GPU residency is not expected at that quant.
- Qwen2.5-Coder7B and Qwen3-8B: optional older control models only, not recommended upgrades based on novelty.
- Qwen3-Embedding-0.6B and Qwen3-Reranker-0.6B: Apache2 retrieval pipeline candidates. Preserve prescribed pooling/query instructions and yes/no rerank scoring. CPU first if latency permits; avoid consuming a whole GPU for light retrieval.
- Whisper-large-v3-turbo with faster-whisper/CTranslate2: ASR comparator to existing voice arrangements, not a claim it is newer or better. Test local microphone recordings and silence hallucinations.
- Kokoro82M: CPU TTS candidate, Apache2 weights; audit voice/frontend licensing separately.
- Gemma3-4B-it: independent-family vision control under custom Gemma terms, not first tool-agent choice.

Sources:
https://huggingface.co/Qwen/Qwen3-Coder-30B-A3B-Instruct
https://huggingface.co/Qwen/Qwen3-Embedding-0.6B
https://huggingface.co/Qwen/Qwen3-Reranker-0.6B
https://huggingface.co/openai/whisper-large-v3-turbo
https://github.com/SYSTRAN/faster-whisper
https://huggingface.co/hexgrad/Kokoro-82M
https://huggingface.co/google/gemma-3-4b-it

## Serving software strategy

1. Upstream llama.cpp is the common baseline: CPU, CUDA, Vulkan/HIP, hybrid offload, familiar API and broad GGUF coverage. Pin the exact tested commit/build; do not automatically overwrite the working binaries.
2. Keep PrismML separate for Bonsai/Mitsuba formats requiring it. An upstream upgrade does not necessarily replace that fork. Validate vision projector and quant support independently.
3. ik_llama.cpp is a high-priority server experiment for AVX2 CPU plus Turing CUDA and MoE offload. It is not the preferred 890M GPU backend; Vulkan/ROCm performance is not its focus. Kolibri architecture support is a separate prerequisite.
4. ExLlamaV3/TabbyAPI is a secondary experiment for NVIDIA/EXL3 models. Current research reports experimental Turing support and AVX2 offload; confirm exact model and release compatibility before selecting it. It is not a GGUF substitute.
5. vLLM/SGLang are targeted options when concurrent throughput warrants their overhead. Framework support for a GPU does not establish support for all attention/quantization kernels. Do not assume native BF16/FP8 support on2080s. Avoid making these the default on an8GB-RAM laptop.
6. Beelink: compare Vulkan first against CPU, then HIP/ROCm with verified driver/kernel compatibility. Official Ryzen support has evolved, so blanket claims that890M cannot use ROCm are stale. Exact HX PRO370 SKU, Fedora host and quantized inference kernels still need checking against AMD's validated stack.
7. KTransformers now has an AVX2 path, but its documented24GB+ GPU recommendation is outside this fleet's individual cards. Lower priority.

Prefer routing complete requests among independent nodes over splitting one model across mixed machines. Server-local GPU splitting is reasonable; cross-node RPC is an experiment only after network bandwidth/latency testing. Keep RPC private. Do not infer usable distributed performance from summed VRAM. exo/distributed-llama merit investigation only for a concrete capacity need, not fleet-wide replacement.

Primary software references:
https://github.com/ggml-org/llama.cpp
https://github.com/PrismML-Eng/llama.cpp
https://github.com/ikawrakow/ik_llama.cpp
https://github.com/turboderp-org/exllamav3
https://github.com/theroyallab/tabbyAPI
https://docs.vllm.ai/en/latest/getting_started/installation/gpu/
https://docs.sglang.io/docs/get-started/install
https://rocm.docs.amd.com/projects/radeon-ryzen/en/latest/docs/compatibility/compatibilityryz/native_linux/native_linux_compatibility.html
https://github.com/kvcache-ai/ktransformers
https://github.com/ggml-org/llama.cpp/tree/master/tools/rpc

## Evaluation and rollout gates

Order:
1. Capture baseline outputs, executable hashes/build IDs, model hashes, templates, service configuration, resources and endpoint behavior. Resolve wrapper/process provenance and exposure before changing it.
2. Build a pinned current llama.cpp in a separate prefix. Verify old models first to isolate runtime gains/regressions from model changes. Preserve Prism services.
3. Saluki versus current Qwen/Bonsai/Nanbeige on fixed tool/research/code tasks; start server, then isolated core trial.
4. Add small embeddings/rerank and ASR/TTS comparisons where they remove generative work.
5. Kolibri server experiment with its required patch; Clef decision adapter experiment; Mitsuba only after artifact/support verification.
6. Promote only a passing candidate with a rollback and a verified service restart. Do not upgrade all nodes simultaneously.

Measure actual useful work:
- Exact Hermes/Arda tool schemas: nested arguments, no-tool decisions, multi-turn call IDs, streaming, failures, duplicate side effects, prompt injection in retrieved text. Score final state, not JSON validity alone.
- Coding: held-out local repository tasks, patch validity, compilation/tests, unintended edits, elapsed time.
- Research: fixed conflicting source packs, citation entailment, omission/unsupported claims and abstention.
- Retrieval: BM25 versus embedding versus reranking on labeled queries; Recall@k/nDCG and p95 latency.
- Vision: OCR/screenshots/tables and real image-to-prompt cases. Test projector/runtime together.
- Operational metrics: cold/warm load, time to first useful token/call, prompt throughput, generation rate, successful task time, peak RAM/VRAM, swap, GPU/CPU temperature, throttling, energy where measurable.
- Context4K/8K/16K and concurrency1/2/4 only where memory permits; expand context after correctness and capacity pass. Do not advertise1M context from a model card as local acceptance.

Remaining uncertainties: measured DDR4 bandwidth, sustained power/thermal limits, cross-node throughput, exact loaded LM Studio workloads, binary provenance on every service, supported large-model forks on these GPUs, comparative task quality, and Mitsuba download availability. No throughput winner or final optimal deployment is claimed without these tests.
