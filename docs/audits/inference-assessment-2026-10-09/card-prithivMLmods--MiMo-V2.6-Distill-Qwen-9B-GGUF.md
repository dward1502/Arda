---
base_model:
- XiaomiMiMo/MiMo-V2.6-Distill-Qwen-9B
tags:
- text-generation-inference
- llama-cpp
- mimo_v2
- qwen3_5
- agentic
- distillation
- supervised-fine-tuning
- code
- tool-use
license: other
language:
- en
pipeline_tag: image-text-to-text
library_name: transformers
---

# **MiMo-V2.6-Distill-Qwen-9B-GGUF**

> **MiMo-V2.6-Distill-Qwen-9B** is a 9-billion-parameter agentic model from Xiaomi MiMo, built via supervised fine-tuning of Qwen3.5-9B on MiMo-generated data spanning coding, general-purpose agent tasks, visual coding, and cybersecurity, released as an SFT checkpoint intended as a starting point for open research in agentic reinforcement learning. It was trained on a weighted mixture of 77.4B total tokens (27.2B loss-bearing) split roughly evenly across code (29.9%), general (28.5%), visual (27.4%), and cyber (14.2%) domains, and shows dramatic gains over its Qwen3.5-9B base across nearly every reported benchmark — jumping from 5.0 to 30.3 on AutomationBench v1.0.6, 5.7 to 31.3 on internal cyber evaluation, 27.0 to 37.1 on Terminal-Bench 2.1, and 19.5 to 51.6 on internal coding evaluation, alongside solid improvements on SWE-bench Verified (61.1) and SWE-bench Pro (44.6). It ships with its tokenizer and MiMo v2.6 chat template, is served via SGLang with a dedicated `mimo` reasoning parser and explicit thinking-mode control through an OpenAI-compatible API, and is fine-tuned from the Apache-2.0-licensed Qwen3.5-9B base.

## Model Files

| File Name | Quant Type | File Size | File Link | Description |
|-----------|------------|-----------|-----------|-------------|
| MiMo-V2.6-Distill-Qwen-9B.BF16.gguf | BF16 | 17.9 GB | [Link](https://huggingface.co/prithivMLmods/MiMo-V2.6-Distill-Qwen-9B-GGUF/blob/main/MiMo-V2.6-Distill-Qwen-9B.BF16.gguf) | Full BF16 weights. Highest quality, largest file size. |
| MiMo-V2.6-Distill-Qwen-9B.Q3_K_L.gguf | Q3_K_L | 4.93 GB | [Link](https://huggingface.co/prithivMLmods/MiMo-V2.6-Distill-Qwen-9B-GGUF/blob/main/MiMo-V2.6-Distill-Qwen-9B.Q3_K_L.gguf) | Lower quality but usable, good for low RAM availability. |
| MiMo-V2.6-Distill-Qwen-9B.Q3_K_M.gguf | Q3_K_M | 4.62 GB | [Link](https://huggingface.co/prithivMLmods/MiMo-V2.6-Distill-Qwen-9B-GGUF/blob/main/MiMo-V2.6-Distill-Qwen-9B.Q3_K_M.gguf) | Low quality. |
| MiMo-V2.6-Distill-Qwen-9B.Q4_K_M.gguf | Q4_K_M | 5.63 GB | [Link](https://huggingface.co/prithivMLmods/MiMo-V2.6-Distill-Qwen-9B-GGUF/blob/main/MiMo-V2.6-Distill-Qwen-9B.Q4_K_M.gguf) | Good quality, default size for most use cases, *recommended*. |
| MiMo-V2.6-Distill-Qwen-9B.Q4_K_S.gguf | Q4_K_S | 5.35 GB | [Link](https://huggingface.co/prithivMLmods/MiMo-V2.6-Distill-Qwen-9B-GGUF/blob/main/MiMo-V2.6-Distill-Qwen-9B.Q4_K_S.gguf) | Slightly lower quality with more space savings, *recommended*. |
| MiMo-V2.6-Distill-Qwen-9B.Q5_K_M.gguf | Q5_K_M | 6.47 GB | [Link](https://huggingface.co/prithivMLmods/MiMo-V2.6-Distill-Qwen-9B-GGUF/blob/main/MiMo-V2.6-Distill-Qwen-9B.Q5_K_M.gguf) | High quality, *recommended*. |
| MiMo-V2.6-Distill-Qwen-9B.Q5_K_S.gguf | Q5_K_S | 6.31 GB | [Link](https://huggingface.co/prithivMLmods/MiMo-V2.6-Distill-Qwen-9B-GGUF/blob/main/MiMo-V2.6-Distill-Qwen-9B.Q5_K_S.gguf) | High quality, *recommended*. |
| MiMo-V2.6-Distill-Qwen-9B.Q6_K.gguf | Q6_K | 7.36 GB | [Link](https://huggingface.co/prithivMLmods/MiMo-V2.6-Distill-Qwen-9B-GGUF/blob/main/MiMo-V2.6-Distill-Qwen-9B.Q6_K.gguf) | Very high quality, near perfect, *recommended*. |
| MiMo-V2.6-Distill-Qwen-9B.mmproj-bf16.gguf | mmproj-bf16 | 922 MB | [Link](https://huggingface.co/prithivMLmods/MiMo-V2.6-Distill-Qwen-9B-GGUF/blob/main/MiMo-V2.6-Distill-Qwen-9B.mmproj-bf16.gguf) | Multimodal projection file in BF16 format. Used for vision/language models. |

## llama.cpp

LLM inference in C/C++ — https://github.com/ggml-org/llama.cpp