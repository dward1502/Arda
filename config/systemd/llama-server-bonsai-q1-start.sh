#!/usr/bin/env bash
set -euo pipefail

MODEL=/var/home/annunimas-server/models/Bonsai-27B-Q1_0/Bonsai-27B-Q1_0.gguf
BIN=/var/home/annunimas-server/llama.cpp-prism/build-cuda/bin/llama-server

[[ -f "$MODEL" ]] || { echo "ERROR: model not found: $MODEL" >&2; exit 1; }
[[ -x "$BIN" ]] || { echo "ERROR: binary not found: $BIN" >&2; exit 1; }

export CUDA_VISIBLE_DEVICES=0
export LD_LIBRARY_PATH=/var/home/annunimas-server/miniforge3/lib:/usr/local/cuda/lib64:/usr/lib64:${LD_LIBRARY_PATH:-}

exec "$BIN" \
  --model "$MODEL" \
  --alias bonsai-27b-q1_0 \
  --host 0.0.0.0 \
  --port 8097 \
  --ctx-size 100000 \
  --parallel 1 \
  --n-gpu-layers 999 \
  --flash-attn on \
  --cache-type-k q4_0 \
  --cache-type-v q4_0 \
  --jinja \
  --chat-template-kwargs '{"enable_thinking":false}' \
  --metrics \
  --no-webui
