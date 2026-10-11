---
license: apache-2.0
library_name: llama.cpp
pipeline_tag: text-generation
tags:
- ternary
- 2-bit
- gguf
- llama-cpp
- cuda
- metal
- on-device
- hybrid-attention
- prismml
- bonsai
base_model:
- Qwen/Qwen3.8-27B
---

<p align="center">
  <img src="./assets/bonsai-logo.svg" width="280" alt="Bonsai">
</p>

<p align="center">
  <a href="https://prismml.com"><b>Prism ML Website</b></a> &nbsp;|&nbsp;
  <a href="https://github.com/PrismML-Eng/Bonsai-demo/blob/main/bonsai-2-27b-whitepaper.pdf"><b>Whitepaper</b></a> &nbsp;|&nbsp;
  <a href="https://github.com/PrismML-Eng/Bonsai-demo"><b>Demo &amp; Examples</b></a> &nbsp;|&nbsp;
  <a href="https://discord.gg/prismml"><b>Discord</b></a>
</p>

# Bonsai 2 27B — GGUF

Full 27B-class reasoning in ternary transformer weights, for llama.cpp (CUDA, Metal, CPU)

> **\~9.3x** smaller than FP16 (ideal) | **98.2%** of FP16 intelligence retained | **\~47 tok/s** on an Apple M5 Max laptop

## Highlights

- **\~5.9 GB** language model (down from \~54 GB FP16) — full 27B-class reasoning on a standard laptop or a single GPU
- **98.2% of FP16 intelligence retained**: 84.78 average across 14 thinking-mode benchmarks — far above the conventional IQ2_XXS build (72.59) at about 82% of its footprint, and within 0.4 points of UD-Q4_K_XL at three times the footprint
- **Retains thinking, reasoning, and agentic behavior** deep in the sub-4-bit regime, where conventional low-bit representations collapse: math within half a point of full precision (96.57), coding level with the baseline (89.42), agentic tool calling at 74.92
- **End-to-end ternary language weights** across embeddings, attention projections, MLP projections, and LM head, at a *true* 1.72 bits per weight — no high-precision escape hatches behind a low-bit label; the vision tower ships as a separate Q8_0 mmproj pack
- **262K-token context** on-device, kept practical by the Qwen3.8-27B hybrid-attention backbone (\~75% linear attention)
- **Two GGUF packings** with custom ternary hybrid-attention kernels for llama.cpp (CUDA, Metal) — **PTQ1_0** packs trits densely (1.75 bits/weight, 5.95 GB), **PQ2_0** stores each trit in a 2-bit slot (2.13 bits/weight, 7.21 GB); packed weights are consumed directly, never expanded back to FP16
- **MLX companion**: also available as [Ternary-Bonsai-2-27B-mlx-2bit](https://huggingface.co/prism-ml/Ternary-Bonsai-2-27B-mlx-2bit) for native Apple Silicon inference

## Resources

- **[Whitepaper](https://github.com/PrismML-Eng/Bonsai-demo/blob/main/bonsai-2-27b-whitepaper.pdf)** — full methodology, benchmarks, and measurement notes
- **[Demo & examples](https://github.com/PrismML-Eng/Bonsai-demo)** — **the source of truth for running these models**: tested setup for every backend, pinned binaries, serving, benchmarking and integration, kept current as the runtimes move
- **Low-bit kernels**: [llama.cpp fork](https://github.com/PrismML-Eng/llama.cpp) (CUDA + Metal) · [MLX fork](https://github.com/PrismML-Eng/mlx) (Apple Silicon) · [mlx-swift fork](https://github.com/PrismML-Eng/mlx-swift) (iOS/macOS)
- **[Known issues](https://huggingface.co/prism-ml/Ternary-Bonsai-2-27B-gguf/blob/main/KNOWN_ISSUES.md)** — problems we know about, with workarounds and fix status
- **[Discord](https://discord.gg/prismml)** — join the community for support, discussion, and updates

## Model Overview

| Item              | Specification                                                                                    |
| :---------------- | :----------------------------------------------------------------------------------------------- |
| Base model        | Derived from Qwen3.8-27B, a 27B hybrid-attention causal language model (architecture unchanged)  |
| Parameters        | 27.36B total — 24.35B language backbone (64 blocks) + 2.54B embedding/LM head + 0.46B vision tower (27 blocks) |
| Architecture      | Hybrid attention (\~75% linear / \~25% full attention), SwiGLU MLP, RoPE, RMSNorm                   |
| Context length    | 262K tokens (inherited from the base model; kept practical on-device by the predominantly linear-attention backbone) |
| Weight format     | Ternary g128: {−1, 0, +1} weights with FP16 group-wise scaling, packed as **PTQ1_0** (dense trits) or **PQ2_0** (2-bit slots) |
| Weight basis      | Blockwise Hadamard rotation (block 1024, fixed ±1 signs) folded into the stored weights; the matching transform is applied to activations at runtime |
| Low-bit coverage  | Embeddings, attention projections, MLP projections, LM head                                       |
| Vision tower      | optional \~0.63 GB mmproj pack (Q8_0), loaded only for image input                                 |
| Deployed size     | **5.95 GB** (PTQ1_0) or **7.21 GB** (PQ2_0); 5.8 GB ideal at 1.72 bits/weight — see below           |
| Backends          | llama.cpp (CUDA, Metal, CPU)                                                                      |
| License           | Apache 2.0                                                                                        |

## Weight Representation: Ternary g128

Each weight takes a value from {−1, 0, +1}, with one shared FP16 scale factor for every group of 128 weights. A ternary value carries log₂3 ≈ 1.585 bits of information, so the effective storage cost of the format is **\~1.71 bits/weight** (ternary code + 16-bit scale amortized over 128 weights); counting the small set of tensors held above the ternary representation brings the model as a whole to **1.72 bits/weight** — an idealized \~9.3x reduction vs FP16.

The weights are stored in a **rotated basis**: each matrix is transformed blockwise by an orthogonal Hadamard rotation before the ternary assignment, and the runtime applies the matching transform to activations. The rotation is folded into the stored weights offline, so it costs no extra bits and no extra weight traffic; the packed model declares its rotation as metadata, so a runtime either applies the matching transform or refuses to load the file.

### Memory Requirement

| Format                          | True bits/weight | Size        | Reduction   |
| :------------------------------ | ---------------: | ----------: | ----------: |
| FP16 (baseline)                 | 16.0             | \~54 GB      | 1.0x        |
| Ternary g128 (ideal)            | 1.72             | 5.8 GB      | \~9.3x       |
| **GGUF PTQ1_0** (dense trits)   | **1.75**         | **5.95 GB** | **\~9.0x**   |
| **GGUF PQ2_0** (2-bit slots)    | **2.13**         | **7.21 GB** | **\~7.5x**   |

Practical deployment needs packing formats that efficient kernels can consume, and this repo ships two: **PTQ1_0** packs trits densely and lands essentially on the information-theoretic target, while **PQ2_0** stores each trit in a 2-bit slot, trading footprint for cheaper unpacking. Neither is uniformly faster — see the throughput table below for where each wins. These sizes describe the language model alone, the only component that must stay resident for text inference; 26.2M parameters (**0.0976%** of the language model — the recurrent state path of the linear-attention layers, plus the normalization weights) remain in higher precision and are counted in the 1.72 figure.

### Shipped Components

The vision tower ships alongside the language model as an optional component (on-disk sizes):

| Component      | Pack                               | Size     | Residency                          |
| :------------- | :--------------------------------- | -------: | :--------------------------------- |
| Language model | ternary g128 (PTQ1_0)              | 5.95 GB  | resident                            |
| Language model | ternary g128 (PQ2_0)               | 7.21 GB  | resident                            |
| Vision tower   | mmproj (Q8_0)                      | 0.63 GB  | optional — multimodal input only    |
| Vision tower   | mmproj BF16 (reference)            | 0.93 GB  | optional                            |

The Q8_0 file carries the vision tower in an 8-bit container. It is usually offloaded, loaded only when an image actually arrives, so text-only serving never pays for it.

## Best Practices

### Generation Parameters

We recommend using the following sets of sampling parameters for generation:
> - Thinking Mode: `temperature=1.0`, `top_p=0.95`, `top_k=20`, `min_p=0.05`, `presence_penalty=0.0`, `repetition_penalty=1.0`
> - Instruct (or non-thinking) mode: `temperature=0.7`, `top_p=0.80`, `top_k=20`, `min_p=0.0`, `presence_penalty=1.5`, `repetition_penalty=1.0`

`min_p=0.05` drops tokens far less likely than the top choice. In our tests it scored at least as
well as `min_p=0.0` and followed instructions more reliably, so we recommend it for thinking mode.
It is also llama.cpp's default: the GGUF files carry `top_k`, `top_p` and `temperature`
(`general.sampling.*`) but not `min_p`, so llama.cpp applies 0.05 without being told. The other
values match the base model's own `generation_config.json`. The reported benchmark results were
measured with the same settings, except `min_p=0.0`.

**The model uses `xhigh` reasoning effort by default; use `medium` for shorter responses and a balance of speed and accuracy. `low` reasoning effort is not supported and when selected the model will behave close to `xhigh`.**

### System Prompt

You can use a simple system prompt such as:

```
You are a helpful assistant
```

### Choosing a Packing

**PQ2_0** is the faster decode on H100, A100, and the Blackwell cards, is faster at prompt processing everywhere, and is the pack measured on Apple Silicon. **PTQ1_0** is the faster decode on the Ada-generation cards and the L4, and is the pick wherever memory is tightest. See the throughput table below.

## Quickstart

> **[PrismML-Eng/Bonsai-demo](https://github.com/PrismML-Eng/Bonsai-demo) is the source of truth for
> running these models.** It carries the tested setup for every backend, pins a known-good binary
> release, and is kept current as the kernels move. Where anything here disagrees with it, it is right.

### These files need our llama.cpp build

The ternary hybrid-attention kernels live in the
[PrismML-Eng/llama.cpp](https://github.com/PrismML-Eng/llama.cpp) fork. **Stock llama.cpp will not
run these files.** It rejects `PQ2_0` and `PTQ1_0` as unknown types, and it loads `Q2_0` without any
warning and produces garbage, because it has no Hadamard activation runtime. Use a binary from the
fork.

```bash
# prebuilt, pick the archive for your platform
# https://github.com/PrismML-Eng/llama.cpp/releases/latest
tar -xzf llama-<tag>-bin-<platform>.tar.gz -C bin --strip-components=1

# or build it
git clone https://github.com/PrismML-Eng/llama.cpp && cd llama.cpp
cmake -B build -DGGML_CUDA=ON && cmake --build build -j    # drop -DGGML_CUDA=ON on macOS, Metal is default
```

```bash
hf download prism-ml/Ternary-Bonsai-2-27B-gguf Ternary-Bonsai-2-27B-PQ2_0.gguf --local-dir .
```

```bash
./bin/llama-cli -m Ternary-Bonsai-2-27B-PQ2_0.gguf \
    -ngl 99 -fa on -c 32768 \
    --temp 1.0 --top-p 0.95 --top-k 20 --min-p 0.05 \
    -p "Explain quantum computing in simple terms." -n 16384
```

The binary is `./bin/llama-cli` from an extracted release archive, or `./build/bin/llama-cli` if you
built the fork yourself. `-ngl 99` offloads every layer, `0` is CPU-only; `-c` sets the context, up
to 262144. `-n 16384` leaves room for the thinking trace: the template defaults to `xhigh` reasoning
effort, so a small cap such as 256 tokens ends generation mid-thought, before any answer.
`--min-p 0.05` is llama.cpp's default; it is written out so the command states every recommended
setting.

This is a reasoning model and it thinks by default. For the server, tool calling, reasoning budgets,
image input with the `mmproj` file, and speculative decoding, follow
[Bonsai-demo](https://github.com/PrismML-Eng/Bonsai-demo), which ships run scripts that pick the
right flags for your hardware.

## Cross-Platform Throughput

`tg128` is token-generation throughput over 128 generated tokens (the memory-bandwidth-bound, interactive phase); `pp512` is prompt-processing throughput over 512 input tokens (the compute-bound phase). Both in tokens/s, measured with `llama-bench` on these GGUF packs (custom low-bit kernels), at batch size 1 and depth 0 with no vision tower. NVIDIA energy is board power including HBM/GDDR.

| Platform                     | PQ2_0 TG128 | PQ2_0 PP512 | PQ2_0 J/tok | PTQ1_0 TG128 | PTQ1_0 PP512 | PTQ1_0 J/tok |
| :--------------------------- | ----------: | ----------: | ----------: | -----------: | -----------: | -----------: |
| RTX 5090 (32 GB)             | **129.9**   | 3893        | **1.95**    | 120.5        | 1805         | 2.15         |
| RTX PRO 6000 Blackwell       | 124.8       | **4020**    | 2.49        | 117.9        | 1972         | 2.77         |
| H100 SXM (80 GB)             | 113.9       | 2830        | 2.69        | 86.9         | 1237         | 3.18         |
| RTX 6000 Ada (48 GB)         | 82.8        | 2431        | 2.51        | **90.4**     | 1657         | 2.49         |
| RTX 4090 (24 GB)             | 81.2        | 3124        | 2.99        | **91.1**     | 1645         | 2.58         |
| L40S (48 GB)                 | 74.4        | 2868        | 3.24        | **81.8**     | 1543         | 2.82         |
| A100 SXM (80 GB)             | 73.9        | 1328        | 3.43        | 54.7         | 706          | 4.28         |
| L4 (24 GB, 72 W)             | 29.8        | 777         | 2.42        | **32.1**     | 467          | **2.25**     |
| Laptop (Apple M5 Pro, Metal) | 28.1        | 387         | —           | —            | —            | —            |

On the laptop the FP16 baseline (\~54 GB) does not fit at all — the meaningful statement is not a speedup ratio but that a 27B model runs interactively on an everyday laptop. The measured decode streams \~204 GB/s of weights on the M5 Pro, confirming the memory-bandwidth-dominated profile that the low-bit representation is built to exploit. The M5 Pro figure is measured on a quiet machine; this laptop swings \~4% with background load.

The two packings are a genuine trade rather than a strict ordering. PTQ1_0 moves 17% less weight data per step, but unpacking dense trits costs arithmetic, so it wins on the Ada-generation parts and the L4 — where memory is the binding constraint — and loses on H100, A100, and the Blackwell cards, where batch-1 decode is limited by instruction throughput and launch overhead instead. Prompt processing, being compute-bound, favors PQ2_0 everywhere.

The Apple row carries no per-token energy figure because the two platforms' instrumentation does not enclose the same components: `nvidia-smi` includes the card's HBM/GDDR, while Apple's `powermetrics` reports CPU, GPU, and ANE with no DRAM rail. What the measurement does support is absolute draw: the M5 Pro decodes at **27.5 W** on the GPU rail and 34.1 W across CPU and GPU, against 300–455 W of board power for the NVIDIA cards above.

### Additional Apple Platforms

Measured on the earlier pre-rotation build and reported pending re-measurement on the current stack (llama.cpp Metal backend):

| Platform                     | Footprint | TG128 (tok/s) | PP512 (tok/s) |
| :--------------------------- | --------: | ------------: | ------------: |
| Laptop (Apple M5 Max, Metal) | 7.2 GB    | 47.0          | 765           |
| Laptop (Apple M5 Pro, Metal) | 7.2 GB    | 28.7          | 393           |
| Laptop (Apple M4 Pro, Metal) | 7.2 GB    | 18.0          | 125           |

On the wider M5 Max the model reaches \~47 tok/s; on the M4 Pro, prefill (\~125 tok/s) rather than decode is the practical limit for very long prompts.

## Benchmarks

Evaluated with EvalScope + vLLM on NVIDIA H100 under identical infrastructure, decoding, and scoring, in **thinking mode** — where the model's full reasoning is exercised and the sub-4-bit collapse of conventional methods is most visible. 14 benchmarks across six skill categories. Bit-widths are true averages; "vs FP16" is relative to the Qwen3.8-27B FP16 reference.

| Variant                              | True bpw | Footprint  | Thinking avg | vs FP16    |
| :----------------------------------- | -------: | ---------: | -----------: | ---------: |
| Qwen3.8-27B FP16                     | 16.0     | 54 GB      | 86.32        | 100%       |
| Qwen3.8-27B UD-Q4_K_XL ("4-bit")     | 5.2      | 17.6 GB    | 85.18        | 98.7%      |
| Qwen3.8-27B IQ2_XXS ("2-bit")        | 2.16     | 7.27 GB    | 72.59        | 84.1%      |
| **Bonsai 2 27B**                     | **1.72** | **5.95 GB** | **84.78**    | **98.2%**  |

At 5.95 GB, Bonsai 2 27B outscores the sub-4-bit conventional build by more than twelve points at about 82% of its size, and comes within 0.4 points of UD-Q4_K_XL at a third of its footprint.

The aggregate gap also understates *how* the conventional builds fail: their degradation is selective, concentrated on the benchmarks that demand sustained chains of reasoning. IQ2_XXS falls to 57.5 on AIME26 and 56.4 on LiveCodeBench while still scoring 88.93 on MMLU-Redux — which is why casual testing misses the collapse. Bonsai 2 holds exactly these benchmarks, scoring 95.83 and 90.07. The previous Bonsai 27B report showed the same pattern on a second model family, Gemma-4-31B, so the collapse is a property of the methods rather than of one base model.

### By Skill Category

| Category                | Benchmarks                          | FP16      | Bonsai 2 27B |
| :---------------------- | :---------------------------------- | --------: | -----------: |
| Knowledge & reasoning   | MMLU-Redux, MuSR                    | 85.55     | 79.86        |
| Math                    | GSM8K, MATH-500, AIME25, AIME26     | 97.06     | 96.57        |
| Coding                  | HumanEval+, MBPP+, LiveCodeBench    | 89.07     | 89.42        |
| Instruction following   | IFEval, IFBench                     | 81.25     | 82.66        |
| Agentic / tool calling  | BFCL v3                             | 76.74     | 74.92        |
| Vision                  | MMMU-Pro, OCR Bench v2              | 71.36     | 66.19        |
| **Overall (14)**        |                                     | **86.32** | **84.78**    |

The reasoning backbone comes through intact: math falls only from 97.06 to 96.57, coding is level with the baseline, and instruction following is slightly ahead of it. The remaining gap is concentrated in the most demanding categories — knowledge and reasoning, and vision.

### Full Per-Benchmark Results

<details>
<summary>Expand full per-benchmark results (thinking mode)</summary>

| Benchmark              | FP16      | UD-Q4_K_XL | IQ2_XXS   | Bonsai 2 27B |
| :--------------------- | --------: | ---------: | --------: | -----------: |
| MMLU-Redux             | 91.46     | 93.35      | 88.93     | 89.09        |
| MuSR                   | 79.63     | 73.01      | 66.99     | 70.63        |
| GSM8K                  | 97.19     | 96.66      | 89.90     | 96.66        |
| MATH-500               | 99.80     | 99.40      | 84.60     | 98.80        |
| AIME25                 | 96.67     | 92.91      | 66.67     | 95.00        |
| AIME26                 | 94.58     | 93.00      | 57.50     | 95.83        |
| HumanEval+             | 93.29     | 95.73      | 91.46     | 95.12        |
| MBPP+                  | 83.86     | 83.86      | 78.89     | 83.07        |
| LiveCodeBench          | 90.05     | 87.96      | 56.40     | 90.07        |
| IFEval                 | 91.50     | 88.83      | 84.03     | 91.31        |
| IFBench (prompt-loose) | 71.00     | 65.65      | 53.76     | 74.00        |
| BFCL v3                | 76.74     | 75.05      | 70.28     | 74.92        |
| MMMU-Pro               | 81.73     | 81.73      | 65.19     | 75.49        |
| OCR Bench v2           | 60.99     | 65.45      | 61.70     | 56.88        |
| **Average (14)**       | **86.32** | **85.18**  | **72.59** | **84.78**    |

</details>

## Intelligence Density

Intelligence density captures the ratio of a model's capability to its deployed size:

```
D = -log2(1 - score/100) / size_GB
```

| Variant                                  | Size (GB) | Benchmark avg | Intelligence Density (1/GB) |
| :--------------------------------------- | --------: | ------------: | --------------------------: |
| **Bonsai 2 27B**                         | **5.95**  | **84.78**     | **0.457**                   |
| Ternary Bonsai 27B (previous release)    | 5.75      | 80.98         | 0.416                       |
| Qwen3.8-27B IQ2_XXS                      | 7.27      | 72.59         | 0.257                       |
| Qwen3.8-27B UD-Q4_K_XL                   | 17.56     | 85.18         | 0.157                       |
| Qwen3.8-27B FP16                         | 54.66     | 86.32         | 0.053                       |

Bonsai 2 27B delivers about **1.8x** the density of the densest conventional build here (IQ2_XXS at 0.257) and nearly **9x** FP16. Each stored gigabyte is translated into far more usable intelligence. For Bonsai 2 27B and the conventional builds, sizes are the on-disk sizes of the published files. Against the previous Bonsai 27B release, density rises from 0.416 to 0.457, a 9.9% gain; that row is recomputed on these same 14 benchmarks for a like-for-like comparison.

## Use Cases

- **Laptop-local 27B agents**: full 27B reasoning and tool use on a standard laptop at \~28 tok/s, with the 262K context available for long-document analysis, full-repository code work, and other tasks that depend on holding a large working set in context
- **Privacy-sensitive and offline settings**: on-device execution keeps prompts and data on the device by construction, and works with intermittent or no connectivity
- **Single-GPU and commodity-GPU serving**: 27B-class quality from a single consumer or entry-level datacenter GPU — \~130 tok/s on an RTX 5090, \~30 tok/s on a 72 W L4 — with headroom for larger batches, longer contexts, or co-resident models
- **Quality-first low-bit deployment**: 98.2% of the full-precision model's benchmark average at roughly a ninth of its size

## Limitations

- **The quality–footprint trade-off**: the ternary model retains 98.2% of the full-precision average, and the gap is modest and predictable — the reasoning core (math, coding) stays within a few points of baseline, with the difference concentrated in the most demanding categories
- **Native low-bit kernels**: the dense PTQ1_0 packing (1.75 bits/weight, 5.95 GB) now exists, but unpacking trits costs arithmetic — it is faster on Ada-class and smaller accelerators and slower on Ampere, Hopper, and Blackwell, where batch-1 decode is not bandwidth-starved; returning the footprint advantage as latency on every target is an active engineering target

## Citation

If you use Bonsai 2 27B, please cite:

```bibtex
@techreport{bonsai2_27b,
    title   = {Bonsai 2 27B: A 27B Ternary Reasoning Model},
    author  = {Prism ML},
    year    = {2026},
    month   = {September},
    url     = {https://prismml.com}
}
```

## Contact

For questions, feedback, or collaboration inquiries: **contact@prismml.com**
