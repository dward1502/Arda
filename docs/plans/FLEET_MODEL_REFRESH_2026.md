# Arda fleet model refresh plan (July 2026)

Source of truth fleet config: `config/fleet.toml`

Current live fleet serving these models:
- node-core-hub: LFM2.5-8B-A1B-Q4_K_M
- node-pi5-warden: Qwen3.5-4B-Q4_K_M.gguf
- node-ser9-worker: Ternary-Bonsai-8B-Q2_0 (active, Prism-ML llama.cpp fork)
- node-backbone-server: lfm2.5-8b-a1b-q4km
- node-backbone-gemma4-coder: qwen2.5-coder-7b-q4km
- node-backbone-vision: Qwen2.5-VL-7B-Instruct
- node-backbone-bonsai27: ternary-bonsai-27b-q2_0 (active, Prism-ML llama.cpp fork, :8095)
- node-ser9-carnice: Carnice-9b-Q6_K — inactive
- node-laptop: voice/ASR only, not inference
- node-pi5-citadel-avatar: avatar service, not LLM lane

HuggingFace evidence as of 2026-07-21:
- Prism-ML Bonsai 27B: true ternary GGUF `Q2_0_g128`, ~7.2 GB, ~94.6% of FP16 average, 262K ctx, requires Prism-ML llama.cpp fork (`llama.cpp-prism`) with `GGML_VULKAN` or `GGML_CUDA`; stock llama.cpp rejects ternary layout.
- Prism-ML Bonsai 27B 1-bit MLX: binary pack for Apple MLX/iOS, ~3.9 GB, ~89.5% of FP16, 262K ctx; phone-only lane, not server-grade.
- Top downloaded/general 2026 GGUF/base models: Qwen3.5-9B, Gemma-4-12B/26B-A4B/31B, Llama-4-Scout-17B-16E, Qwen3-Coder-Next, Qwen3-Coder-30B-A3B, DeepSeek-V4-Flash, Hy3/A4B variants, Qwen2.5-VL-7B.
- Strong multimodal GGUF options exist around Qwen2.5-VL and Gemma-4-family image-text models.
- Best coding/agentic GGUF density: Qwen3-Coder-Next, Qwen3-Coder-30B-A3B, Gemma-4-12B agentic variants.
- Best general/reasoning GGUF density: Qwen3.5-9B, DeepSeek-V4-Flash-class quantizations, Hy3-A4B.

## Node curation notes before mapping

Current “why” looks broad/redundant:
- Both backbone-server and core-hub serve LFM-style fast lanes; not clearly differentiated.
- ser9-worker and pi5-warden both serve Qwen3.5-4B-class light models; overlap.
- gemma4-coder lane is good if intent is code; otherwise redundant with fast general lane.
- vision lane is valid and should be kept, but Qwen2.5-VL-7B is no longer top-visited in 2026 rankings.
- Multiple co-located models on annunimas-server fragment VRAM and can cap context for the strongest candidate (Ternary Bonsai 27B on :8095 is stuck at 8192 ctx).

## Proposed new model purpose plan

Rename/repurpose nodes around actual operator need, not just “has llama.cpp”:
1. Fast general lane: highest-quality general chat/reasoning for daily use.
2. Code/agentic lane: best-in-class tool use / coding assistant available locally.
3. Edge/light lane: smallest usable general model for warden/guardhouse duty.
4. Vision/multimodal lane: local vision model for UI/media inspection.
5. Avatar/product lane: keep Pi5 for product display only unless re-enrolled later.

Proposed node assignments:

- node-backbone-server → Fast General
- node-backbone-gemma4-coder → Code/Agentic
- node-core-hub → Fast General secondary / failover or specialized reasoning lane
- node-ser9-worker → Edge/light
- node-pi5-warden → Edge/light
- node-backbone-vision → Vision/multimodal
- node-ser9-carnice → second Code/Agentic lane when reactivated
- node-laptop → local_placeholder / offline fallback only
- node-pi5-citadel-avatar → avatar only

## Candidate models by lane with justification

### Fast General / High-quality local chat

Top candidates:
- `Qwen/Qwen3.5-9B` — highest downloads among 2026 small chat GGUF basemodels.
- `unsloth/Qwen3.5-9B-GGUF` / `lmstudio-community/Qwen3.5-9B-GGUF` — ready GGUF.
- `prism-ml/Ternary-Bonsai-27B-gguf` — strongest reasoning density per live evidence; replaces multiple smaller models on one lane if VRAM is retuned.
- `DeepSeek-V4-Flash` quantization-family — strong reasoning per HF metadata.
- `google/gemma-4-12B-it` / `unsloth/gemma-4-12b-it-GGUF` — strong general multimodal text.

Preferred operator direction:
- Prefer one strong lane over multiple mid models: if Bonsai 27B is validated at 32K+ ctx on the strongest available GPU, consolidate general/coding/edge onto Bonsai and disable weaker duplicates.
- If VRAM prevents 32K+, pick the single best GGUF per lane purpose and disable inactive duplicates rather than running multiple underutilized services.
- Vision lane should remain separate if multimodal usage is active; otherwise defer to service absence to reclaim VRAM.

Preferred:
- backbone-server: `Qwen3.5-9B` GGUF Q4/Q5 for 32K context.
- backbone-gemma4-coder OR node-core-hub: `Qwen3.5-9B` or `DeepSeek-V4-Flash`-class if 3080 has VRAM headroom.

### Code / Agentic

Top candidates:
- `Qwen/Qwen3-Coder-Next` — top-downloaded coder base in 2026.
- `Qwen/Qwen3-Coder-30B-A3B-Instruct` — MoE; strong for code at lower active size.
- `google/gemma-4-12B-it` agentic community finetunes — good if VRAM allows.
- `unsloth/Qwen3-Coder-30B-A3B-Instruct-GGUF` — community GGUF for 30B-A3B.
- `prism-ml/Ternary-Bonsai-27B-gguf` — high code/tool-use score if unified general lane is acceptable.

Preferred:
- If Bonsai 27B is the unified lane, disable backbone-vision and backbone-gemma4-coder when not in active use to reclaim VRAM for 32K+ ctx on the same system.
- If coder split is preserved: backbone-gemma4-coder: `Qwen2.5-Coder-7B-Instruct` GGUF Q4_K_M with 28 GPU layers on a single RTX 2080 Super at 32K context. Defer `Qwen3-Coder-30B-A3B` until GPU/VRAM upgrade/next hardware pass.
- When ser9-carnice reactivates: lighter coder model can mirror validation.

### Edge / Light

Top candidates:
- `Qwen3.5-4B` class remains usable, but 2026 better choices are lighter but smarter.
- `google/gemma-3-4b-it` or `google/gemma-4-E2B-it` — smaller MoE-style Gemma options.
- `Qwen3.5-9B` is too large for Pi5; prefer 4B-class or lighter.
- `gemma-4-E2B-it` GGUF / `gemma-4-E4B-it` GGUF are better density than plain 4B.

Preferred:
- ser9-worker: `Ternary-Bonsai-8B-Q2_0` is now active on Beelink; keep unless VRAM/throughput regresses or a stronger lightweight candidate is validated.
- pi5-warden: Qwen3.5-4B-Q4_K_M.gguf validated at 65536 context on aarch64/Raspberry Pi 5 8GB; tested performance was ~14.2 prompt tok/s, ~4.0 predicted tok/s, ~3.5 GB memory during serve; 131072 remains unvalidated on this device

### Vision / Multimodal

Top candidates:
- `Qwen2.5-VL-7B-Instruct` — current, still reasonable in 2026.

Preferred:
- backbone-vision: stay with Qwen2.5-VL-7B unless we validate Gemma-4 multimodal on the same lane.

## Inference purpose rethink

Possible new intents:
- Governance writeback: some lanes should produce definitive plutus/mandos updates, not just answer chat.
- Embedding/routing: instead of chat on every node, have some nodes run embeddings, rerankers, or classifier shards.
- Tool-use only: dedicating a lane to function-calling / tool execution is more useful than duplicate chat.

Recommendation:
- If Bonsai 27B can sustain 32K+ ctx on annunimas-server, consider disabling weaker duplicate lanes (LFM25 fast lane, Gemma4 coder, possibly vision) to reclaim VRAM and context headroom rather than adding more models. Stronger single-model performance beats multiple fragmented services.
- Otherwise decide which lane is primary/failover, which node owns governance state updates, and which node answers human chat.
- Then match that intent to candidate models above.

## Recommended next steps

1. Confirm lane purpose map in `docs/plans/ARDA_REMAINING_WORK.md`.
2. For each node, pick one candidate from above and testllama.cpp startup latency/memory locally.
3. Update `config/fleet.toml` `expected_models` and `runtime_model_alias` after validation.
4. Keep `node-laptop` as offline voice node unless an explicit chat model seat is needed.
5. Last: extend `manwe/src/provider.rs` to read `config/fleet.toml` and reflect model aliases in `/v1/models`.
