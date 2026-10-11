---
license: apache-2.0
base_model: Aleph-Alpha/Kolibri-1
base_model_relation: quantized
language:
- de
- en
pipeline_tag: text-generation
library_name: gguf
tags:
- gguf
- llama.cpp
- kolibri1
- mixture-of-experts
- reasoning
- tool-calling
---

# Kolibri-1 GGUF

GGUF quantizations of [Aleph-Alpha/Kolibri-1](https://huggingface.co/Aleph-Alpha/Kolibri-1), a 78B-parameter Mixture-of-Experts reasoning model (3.46B active per token) for German and English, released by Aleph Alpha under Apache-2.0.

This is an independent conversion. It is not made or endorsed by Aleph Alpha or the llama.cpp project.

## Files

| File | Quant | Size |
|---|---|---|
| `Q5_K_M/Kolibri-1-Q5_K_M-00001-of-00002.gguf` + `-00002-of-00002.gguf` | Q5_K_M | 55.6 GB (2 parts) |
| `Kolibri-1-Q4_K_M.gguf` | Q4_K_M | 47.5 GB |
| `Kolibri-1-Q2_K.gguf` | Q2_K | 28.6 GB |
| `Kolibri-1-Q3_K_M.gguf` | Q3_K_M | 37.5 GB |
| `Q6_K/Kolibri-1-Q6_K-00001-of-00002.gguf` + `-00002-of-00002.gguf` | Q6_K | 64.2 GB (2 parts) |
| `Q8_0/Kolibri-1-Q8_0-00001-of-00002.gguf` + `-00002-of-00002.gguf` | Q8_0 | 83.1 GB (2 parts) |

For the split files (Q5_K_M, Q6_K, Q8_0), point llama.cpp at the first part. The second part loads automatically.

The weights were converted from Aleph Alpha's FP8 checkpoint (FP8 e4m3 with 128×128 block scales). The converter dequantizes them to BF16, and the Q4_K_M is quantized from that BF16 GGUF. Every quant here is made from that BF16 GGUF, so none is a requant. Q4_K_M, Q5_K_M, Q6_K and Q8_0 use no importance matrix; there is no further training. Q6_K (added 2026-10-05) is made the same way. Q3_K_M and Q2_K (added 2026-10-06) use an importance matrix computed from the Q8_0 on calibration_datav3 (128 x 512 tokens); some rarely-routed experts got partial coverage (93-98%). Q5_K_M (added 2026-10-05) likewise. The Q4_K_M has 903 tensors: 426 Q4_K, 76 Q6_K, and 401 F32 (norms, router gates and expert biases).

## Requires a llama.cpp patch

Stock llama.cpp does not support the `kolibri1` architecture yet. Apply `kolibri1-llama.cpp.patch` (included in this repo) to llama.cpp at upstream commit `836d571`, then build:

```bash
git clone https://github.com/ggml-org/llama.cpp && cd llama.cpp
git checkout 836d571
git am /path/to/kolibri1-llama.cpp.patch
cmake -B build -DCMAKE_BUILD_TYPE=Release   # add -DGGML_CUDA=ON etc. for your GPU
cmake --build build -j --target llama-server llama-cli
```

**Prebuilt instead:** [hob-llama.cpp release `hob-b11434`](https://github.com/Seraphiel102/llama.cpp/releases/tag/hob-b11434) is upstream llama.cpp with this patch already applied, built for Windows (CUDA, Vulkan, ROCm, CPU), Linux and Intel Mac. Pick the archive for your machine, unzip, and run `llama-server` as below.

Ollama, LM Studio and other llama.cpp-based apps will not load this file until they include support for the architecture.

## Running

```bash
./build/bin/llama-server -m Kolibri-1-Q4_K_M.gguf -c 32768 --jinja \
    --temp 1.0 --top-p 0.97 --top-k 128
```

Reasoning effort is set per request:

```json
"chat_template_kwargs": {"reasoning_effort": "low"}
```

Accepted values are `none`, `low`, `medium` and `high`. The chat template embedded in the GGUF handles reasoning (`<think>`) and tool calls.

## What the patch implements

- **Attention:** GQA (48 query and 4 KV heads, head_dim 128) with per-head q/k RMSNorm. Layers repeat a 4:1 pattern: four sliding-window layers (window 513, RoPE) to one full-attention layer (no positional encoding).
- **Experts:** 384 routed experts with top-6 selection, plus one ungated shared expert, in every layer.
- **Router:** experts are selected on `logits + expert_bias` and weighted by the unbiased `sigmoid(logits)`, with no renormalisation. This differs from llama.cpp's existing DeepSeek-V3 sigmoid router, which selects on `sigmoid(logits) + bias`, so the patch adds a new gating mode.
- **Norms:** sandwich norms around both the attention and MoE blocks.
- **Weights:** the converter dequantizes the FP8 block-scaled weights.

## Verification

| Check | Result |
|---|---|
| Tiny random checkpoint with the same architecture and FP8 weights, compared against a PyTorch reference written from Aleph Alpha's vLLM implementation | max abs logit diff 4.3e-6, top-1 40/40, for batch, single-token and 7-token decode |
| Same test using the DeepSeek-style router instead | max diff 4.56, top-1 4/40, so the test can tell the two routers apart |
| Tokenizer vs HF `tokenizers` | identical on 13/13 strings, including umlauts, emoji, CJK and tool-call JSON |
| Chat template vs HF `apply_chat_template` | identical, covering reasoning effort, thinking off, tools and tool results |

**Real model compared against an independent float64 PyTorch reference** (67-token chat prompt, full logits at every position):

| | Top-1 agreement | Mean KL vs reference |
|---|---|---|
| Q8_0 | 67/67 | 0.0048 |
| Q6_K | 67/67 | 0.0037 |
| Q5_K_M | 64/67 | 0.0065 |
| Q4_K_M | 63/67 | 0.0129 |
| Q2_K | 64/67 | 0.1292 |
| Q3_K_M | 66/67 | 0.0340 |
| Same model with the DeepSeek-style router, for comparison | 10/67 | 8.36 |

Q6_K scoring a hair below Q8_0 on this one prompt is within the test's noise; read both as "near-lossless".

**About Hugging Face's hardware badge:** the red marks on the right count only GPU memory. These files run from
system RAM (CPU-only figures below), and with a GPU llama.cpp can hold part of the model in VRAM and the rest in RAM.

**Memory** (CPU, weights loaded without mmap): Q4_K_M uses 46.6 GB RAM, Q8_0 81.6 GB. Q8_0 passed the same four smoke tests as the Q4_K_M below.

**Speed, Q4_K_M** (`llama-bench`, measured 2026-10-07 with nothing else running; an idle check was recorded before
and after each run). Machine: AMD Ryzen 7 7800X3D, 128 GB DDR5-6400 (2 × 64 GB), RTX 5070 12 GB, 8 threads, flash
attention on.

| Setup | Prompt (512 tokens) | Generation (128 tokens) |
|---|---|---|
| CPU only (GPU hidden) | 83 ± 13 tok/s | 12.3 ± 1.4 tok/s |
| GPU for attention + shared expert, all routed experts in RAM (`-ngl 99 --cpu-moe`, KV q8_0) | 723 ± 18 tok/s | 18.0 ± 0.3 tok/s |
| same, after 32,768 tokens of context | 696 ± 12 tok/s | 11.7 ± 1.1 tok/s |

Routed experts are read from RAM for every token, so generation speed depends mostly on memory bandwidth. Slower
RAM will give lower numbers.

**CUDA graphs:** with CUDA graphs on (the default), the GPU + RAM setup gave very unstable prompt speeds (657 ± 247
tok/s), and `llama-bench` aborted in the CUDA-graph update (`ggml-cuda.cu:2720`) at 32K depth. With
`GGML_CUDA_DISABLE_GRAPHS=1` it was stable and did not crash; the GPU rows above use that setting. Our long-context
server runs did not crash with graphs on, but if you see instability, set that variable.

**Q4_K_M on CPU only** (AMD Ryzen 7 7800X3D, 8 threads, 128 GB RAM, no GPU):

| Test | Result |
|---|---|
| German explanation (why the sky is blue) | correct, fluent German |
| English poem | coherent |
| Arithmetic word problem with checkable answer (reasoning on) | correct: 1.5 |
| Tool call (weather in Heidelberg) | correct `get_weather` call with argument `{"city": "Heidelberg"}` |
| 1,393-token German prompt | coherent four-topic summary |

Perplexity on a 7 KB mixed German/English sample (2 × 512-token chunks): Q4_K_M 6.70 ± 0.80, Q8_0 6.73 ± 0.81. The sample is too small to be a benchmark. It only shows the model is not broken.

**Long context, Q4_K_M** (added 2026-10-07). Three facts ("access codes") were hidden at 10%, 50% and 90% depth inside varied real English text, then the model was asked for all three, with reasoning off and temperature 0:

| Context | Codes recalled | Setup |
|---|---|---|
| 16,463 tokens | 3/3 | CPU only |
| 65,915 tokens | 3/3 | CPU only |
| 133,254 tokens | 3/3 | GPU + experts in RAM |
| 253,646 tokens | 3/3 | GPU + experts in RAM |

The GPU + RAM runs used an RTX 5070 (12 GB) with all routed experts kept in system RAM, 6.4 GB VRAM in total at the full 262,144-token context:

```bash
llama-server -m Kolibri-1-Q4_K_M.gguf -c 262144 -np 1 -ngl 99 --cpu-moe \
    -fa on -ctk q8_0 -ctv q8_0 --jinja --temp 1.0 --top-p 0.97 --top-k 128
```

Most layers use a 513-token sliding window, so the KV cache stays small even at full context. A three-fact recall test shows the context works; it is not a long-context benchmark.

## Not verified

- GPU backends: Vulkan and Metal were not tested. CUDA was used only for the long-context runs above (attention on GPU, experts on CPU), not checked against the reference logits.
- Long-context quality beyond the three-fact recall test above.
- Multiple or streamed tool calls in one turn.
- Quality versus the original FP8 model on benchmarks. No benchmark suite was run, and quantization can reduce accuracy.
- Safety behaviour beyond the original model's.

## License

The model weights are Apache-2.0, from [Aleph-Alpha/Kolibri-1](https://huggingface.co/Aleph-Alpha/Kolibri-1). See the original model card for training data, intended use and limitations. The llama.cpp patch is MIT, the same as llama.cpp.
