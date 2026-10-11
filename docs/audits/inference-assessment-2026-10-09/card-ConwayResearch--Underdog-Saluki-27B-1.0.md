---
license: apache-2.0
base_model:
  - Qwen/Qwen3.8-27B
  - ISTA-DASLab/Qwen3.8-27B-GSQ-RCO-GGUF
base_model_relation: quantized
pipeline_tag: text-generation
language:
  - en
library_name: gguf
tags:
  - gguf
  - llama.cpp
  - 2-bit
  - tool-calling
  - function-calling
  - agents
  - qwen3.8
---

# Underdog Saluki 27B 1.0

Qwen3.8-27B in under 8 GB, tuned to keep tool calling intact. A standard GGUF for stock llama.cpp.

| Tool calling (Underdog Bench) | Parallel tool calls | Average retention | Size |
|---|---|---|---|
| **88** vs 84 for the full model | **42** vs 35 for the full model | **96%** across 9 benchmarks | **7.89 GB** vs 54 GB |

| | |
|---|---|
| Base model | Qwen3.8-27B |
| File | `Underdog-Saluki-27B-1.0-IQ2-mix.gguf`, 7.89 GB |
| Runtime | stock llama.cpp, and apps built on it |
| Modality | text, plus images with the optional vision add-on |
| Vision add-on | `mmproj-Underdog-Saluki-27B-1.0-F16.gguf` (928 MB) or `-Q8_0.gguf` (629 MB) |
| Thinking | on by default, switchable per request, same as Qwen3.8 |

## Quickstart

```bash
huggingface-cli download ConwayResearch/Underdog-Saluki-27B-1.0 Underdog-Saluki-27B-1.0-IQ2-mix.gguf --local-dir .
```

```bash
llama-server -m Underdog-Saluki-27B-1.0-IQ2-mix.gguf --jinja -ngl 99 -fa on -c 32768
```

**Add vision.** Download the vision add-on too, and pass it with `--mmproj`. Then send images through the OpenAI chat API as usual.

```bash
huggingface-cli download ConwayResearch/Underdog-Saluki-27B-1.0 mmproj-Underdog-Saluki-27B-1.0-F16.gguf --local-dir .
```

```bash
llama-server -m Underdog-Saluki-27B-1.0-IQ2-mix.gguf --mmproj mmproj-Underdog-Saluki-27B-1.0-F16.gguf --jinja -ngl 99 -fa on -c 32768
```

`--jinja` turns on the Qwen3.8 chat template, which handles tool calls and thinking. The server speaks the OpenAI chat API on port 8080.

| Use | Settings |
|---|---|
| General use, reasoning, instructions | thinking on (default), temperature 0.6, top_p 0.95, top_k 20 |
| Fast, direct tool calls | thinking off with `"chat_template_kwargs": {"enable_thinking": false}`, temperature 0 |

## Benchmarks

Saluki ran in stock llama.cpp. Where we ran the full-size Qwen3.8-27B ourselves, it used the same harness and settings.

**Tool calling.** Underdog Bench is 120 tasks from the Berkeley Function Calling Leaderboard (BFCL v4), frozen before any model was tested. Thinking off, temperature 0.

| Model | Size | Passed (of 120) |
|---|---|---|
| **Underdog Saluki 27B 1.0** | 7.89 GB | **88** |
| Qwen3.8-27B, full size | 54 GB | 84 |
| Bonsai 2 | 5.95 GB | 70 |

**Parallel tool calls.** 100 BFCL v4 parallel tasks, official checker, thinking off: Saluki 42, full size 35.

**Agents, instructions, coding and reasoning.**

| Benchmark | Saluki | Full size |
|---|---|---|
| SWE-bench Verified (50 issues fixed) | 30 | 33 |
| IFEval (prompt-loose) | **93.5** | 91.5 (public) |
| IFBench (prompt-loose) | **72.7** | 71.0 (public) |
| MBPP+ | 78.0 | 83.9 (public) |
| MuSR | 67.5 | 79.6 (public) |
| AIME 2025 (avg@4) | 79.2 | 96.7 (public) |
| AIME 2026 (avg@4) | 80.0 | 94.6 (public) |

Public scores come from a different harness than ours. 120 tasks is a modest test, and a few tasks of any gap is run-to-run variation.

## Limitations

- Keeps about 82 to 85% of the full model's competition math score.
- Weakest on letter-level instruction puzzles (palindromes, vowel rules, alphabetical order).
- About a fifth of parallel-call replies have small formatting slips.
- With thinking on, it often reasons at length before answering.
- Vision is an add-on. The main file is text only; add the 0.6 to 0.9 GB vision file when you need images.

## Credits and license

Apache 2.0. Built on **Qwen3.8-27B** by the Qwen team and **Qwen3.8-27B-GSQ-RCO-GGUF** by ISTA-DASLab, both Apache 2.0. Underdog Bench tasks come from the Berkeley Function Calling Leaderboard. See `NOTICE`.

Built by Underdog.
