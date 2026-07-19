# Code/Agentic lane — Qwen3-Coder-30B-A3B validation checklist

Target node: `node-backbone-gemma4-coder`
Current: `gemma4-12b-coder-q4km` on `:8094`
Hardware: RTX 2080 Super, 28 GPU layers, 8K context
OS: annunimas-server / eregion distrobox
Service: `llama-server-gemma4-coder.service`

## 1. Pre-flight hardware check

On `annunimas-server` via SSH:
- [ ] `nvidia-smi` — confirm RTX 2080 Super detected, VRAM total ~8GB
- [ ] `df -h /home/annunimas-server/models` — confirm ≥45GB free for 30B-MoE GGUF
- [ ] `systemctl --user status llama-server-gemma4-coder.service` — stop/disable current lane before swap

## 2. Model selection

Primary target:
- `Qwen/Qwen3-Coder-30B-A3B-Instruct`
- GGUF source: `unsloth/Qwen3-Coder-30B-A3B-Instruct-GGUF`
- Quant target: Q4_K_M or Q5_K_M (balances quality/VRAM for 2080 Super)

Fallback if 30B-A3B too large:
- `Qwen/Qwen3-Coder-Next` (dense, smaller) or distilled 9B coder variant
- `unsloth/Qwen3-Coder-Next-MLX-8bit` style if Metal/CPU fallback needed

## 3. Download + verify

On `annunimas-server`:
- [ ] Download GGUF to `/var/home/annunimas-server/models/`
- [ ] Confirm file size matches expected Q4_K_M ~26-30GB range
- [ ] Run `llama-cli` dry-run with `--n-gpu-layers 28` and `-ngl 28` to detect OOM
- [ ] If OOM, drop to Q3_K_M or reduce context to 4096

## 4. Service command rewrite

Replace `llama-server-gemma4-coder.service` with:
- [ ] Model path pointing to new GGUF
- [ ] `--ctx-size 8192` (can reduce to 4096 if VRAM tight)
- [ ] `--n-gpu-layers 28` or lower based on dry-run
- [ ] `--port 8094` unchanged so fleet.toml URL stays valid
- [ ] `--jinja` for Qwen3 chat template compliance
- [ ] Metrics enabled if monitoring needed

## 5. Startup validation

- [ ] `systemctl --user daemon-reload`
- [ ] `systemctl --user start llama-server-gemma4-coder.service`
- [ ] `journalctl -u llama-server-gemma4-coder.service -f` — watch for OOM/load errors
- [ ] Wait for `server is listening on :8094`
- [ ] `curl -s http://127.0.0.1:8094/v1/models` — confirm model id
- [ ] `curl -s http://127.0.0.1:8094/health` — HTTP 200

## 6. Quality smoke test

- [ ] Short completion: `curl -s http://127.0.0.1:8094/v1/chat/completions` with system+user prompt
- [ ] Confirm response contains coherent code/answer in <10s
- [ ] If latency >20s or truncated, reduce context width or GPU layers

## 7. Fleet + provider wiring

- [ ] Update `config/fleet.toml` for `node-backbone-gemma4-coder`:
  - `expected_models`: `["Qwen3-Coder-30B-A3B-Instruct"]` or chosen alias
  - `runtime_model_alias`: canonical alias used by manwe provider id
- [ ] Update `data/plutus/runtime_status.json` if model metadata is surfaced there
- [ ] No `manwe/src/provider.rs` code changes yet — just make sure `/v1/models` returns what fleet expects

## 8. Rollback criteria

Roll back to `gemma4-12b-coder-q4km` if:
- [ ] VRAM OOM persists at Q4_K_M with 28 GPU layers
- [ ] Tokens/sec < 8 on 2080 Super at 8K context
- [ ] Service crashes within 10 minutes of idle/load
- [ ] `/v1/chat/completions` returns malformed output

Rollback command:
- [ ] `systemctl --user stop llama-server-gemma4-coder.service`
- [ ] Restore previous service file with `gemma4-12b-coder-q4km` path
- [ ] Restart and verify `/v1/models`

## 9. Manwe integration gate

After code lane is stable:
- [ ] Confirm `manwe` `/v1/models` lists code lane provider
- [ ] Confirm HUD shows new model id under Fleet → Providers
- [ ] Confirm arda-hud offline/fallback path doesn’t crash on missing model id
