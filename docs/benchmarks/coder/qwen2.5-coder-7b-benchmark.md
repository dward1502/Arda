# Qwen2.5-Coder-7B coder-lane benchmark receipt

Benchmark date: 2026-07-19  
Node: annunimas-server / 100.102.250.115  
Service: llama-server-qwen2.5-coder.service on :8094  
Model: Qwen2.5-Coder-7B-Instruct-Q4_K_M  
Backend: llama.cpp build-cuda2  
GPU: 1x NVIDIA GeForce RTX 2080 SUPER 8GB  
Context: 32768  
GPU layers: 28  

## Smoke benchmark

Prompt: `Write a Python hello world.`  
Response: coherent code block + run instructions  
Latency:
- prompt: 31 tokens, ~60.8 ms, ~181 tok/s
- completion: 108 tokens, ~1548.5 ms, ~69.7 tok/s

## VRAM at idle/load

- GPU0: 6998 MiB free
- GPU1: 222 MiB free

## Verdict

Fit for local coding/tool lane. throughput stays usable for agentic coding. Higher n-gpu-layers can be tested later if VRAM frees on GPU1.
