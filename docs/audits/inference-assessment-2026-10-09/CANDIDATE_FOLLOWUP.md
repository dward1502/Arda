---
soterion:
  sigil: "SCROLL"
  role: "assessment_evidence"
  owner: "HERMES"
  status: "draft"
  reviewed: "2026-10-09"
---

# Candidate provenance follow-up

Independent read-only research rechecked publisher metadata and pinned sources.
These are remote artifacts, not additional local qualification. No new weights
were downloaded and no candidate was promoted. Parent retrieval independently
confirmed Clef's template filename and the Bonsai demo download pin below.

| Candidate | Immutable sources and executable boundary | Disposition |
|---|---|---|
| Saluki | [1336c0b5d74dfe6ad7f793577092f5effd9f97c6](https://huggingface.co/ConwayResearch/Underdog-Saluki-27B-1.0/tree/1336c0b5d74dfe6ad7f793577092f5effd9f97c6), Apache-2.0; `Underdog-Saluki-27B-1.0-IQ2-mix.gguf`; embedded Qwen3.8 Jinja. Optional F16/Q8_0 projector. No publisher minimum runtime commit; existing local b11430 evidence is the tested pair. | Broader text/task/resource qualification needed. No full-residency assumption on one 8-GiB card. |
| Kolibri | [Official 35bc4d3be745502227a67247de77d70e691614ee](https://huggingface.co/Aleph-Alpha/Kolibri-1/tree/35bc4d3be745502227a67247de77d70e691614ee); [GGUF/patch 40eaa5cf6cfca3afaf8b8832bac962652edb0bde](https://huggingface.co/Hob-forge/Kolibri-1-GGUF/tree/40eaa5cf6cfca3afaf8b8832bac962652edb0bde). Apache-2.0 weights, MIT patch; `Kolibri-1-Q4_K_M.gguf`. Patched llama.cpp base `836d57176dc699a726c55418e4f96b8ca628e1bf`; embedded reasoning-effort template. Text-only. | Official FP8 GPU deployment is unsuitable for fleet cards; patched Server RAM/offload remains untested, not rejected. Patch numerical correctness, CUDA graphs, memory and DDR4 throughput need measurement. |
| Clef | [Official 0b331204bb13fbd2ca93a64df1956f5b55478ce5](https://huggingface.co/Cloudflare/clef/tree/0b331204bb13fbd2ca93a64df1956f5b55478ce5); [GGUF 9d3b28af7cfe8b3575e9f06ea42f12ed4ccf3f3c](https://huggingface.co/bartowski/Cloudflare_clef-GGUF/tree/9d3b28af7cfe8b3575e9f06ea42f12ed4ccf3f3c). Apache-2.0, `Cloudflare_clef-Q4_K_M.gguf`, Q8_0 decision head. b11430 `8345f333951c661d166b00e6f9362e553768f292`, `/v1/systemone`, nonstreaming, complete prompt within ubatch. Optional F16/BF16 projector. | Not a chat replacement or full-GPU Q4 placement. Advisory offload experiment still needs adapter/calibration/abstention and smaller-classifier comparison. Official repository DOES contain `chat_template.jinja`; the prior 'no chat template' wording was wrong. |
| Mitsuba / HiMitsuba | [Original API](https://huggingface.co/api/models/isichan-ai/Mitsuba-ComfyUI-27B-GGUF), [alternate API](https://huggingface.co/api/models/isichan-ai/Mitsuba_and_HiMitsuba-27B-GGUF). Research again observed 401 and no public author-model results. | Artifact provenance/access blocked. No verified immutable revision, license, projector/template or runtime pair. 401 does not distinguish private, renamed or removed. |
| Bonsai 2 | [b072e1d3b35a0a630cece372c2127528e0994386](https://huggingface.co/prism-ml/Ternary-Bonsai-2-27B-gguf/tree/b072e1d3b35a0a630cece372c2127528e0994386), Apache-2.0; `Ternary-Bonsai-2-27B-PTQ1_0.gguf` or `Ternary-Bonsai-2-27B-PQ2_0.gguf`. Optional Q8_0/BF16 projector. Prism Hadamard/sign-flip support required. [Download script](https://github.com/PrismML-Eng/Bonsai-demo/blob/b306f4e13a260f2954e33acf25b2a0136877eb33/scripts/download_binaries.sh) pins `prism-b10770-6684606`, commit `6684606ac1bfcf8e31465369d9365df03b3af904`. | Isolated Server experiment possible; not stock llama.cpp, not a qualified replacement. Template defaults xhigh; medium offers shorter reasoning and high is documented problematic. Turing correctness/headroom/tools remain untested. |
| MiMo 9B | [Official 2367e865d009c13ac81713a2878291d33ab28177](https://huggingface.co/XiaomiMiMo/MiMo-V2.6-Distill-Qwen-9B/tree/2367e865d009c13ac81713a2878291d33ab28177); [GGUF 2ed8dfba210e13c96c959580c4368fab1d8f0ba0](https://huggingface.co/prithivMLmods/MiMo-V2.6-Distill-Qwen-9B-GGUF/tree/2ed8dfba210e13c96c959580c4368fab1d8f0ba0). Upstream MIT versus quantizer metadata 'other'; `MiMo-V2.6-Distill-Qwen-9B.Q4_K_M.gguf`, optional BF16 projector. MiMo template/tool serialization, not ordinary Qwen substitution; SGLang mimo reasoning parser recommendation lacks exact minimum revision. | Resolve quant licensing and embedded parser/runtime pair before qualification. Core-sized is not authority to replace/co-load on Core; use the one reserved Server lane. Audio template markers do not establish audio inference. |

## Documentation drift and scope

Bonsai's demo README references an older release than its download script.
[Backend support](https://github.com/PrismML-Eng/Bonsai-demo/blob/b306f4e13a260f2954e33acf25b2a0136877eb33/BACKEND-SUPPORT.md)
reports Vulkan PQ2/PTQ support shipped; older 'source only' wording is not a
current categorical blocker. Beelink device/driver execution remains unmeasured.
The [Prism release](https://github.com/PrismML-Eng/llama.cpp/releases/tag/prism-b10770-6684606)
provides a CUDA 12.8 x64 archive; remote release digest is provenance metadata,
not proof of installation or execution on Turing.

One Server lane may pause. Dual-GPU comparison requiring both production lanes
to stop exceeds that window; no such experiment was performed. Research does
not close baseline, runtime, candidate or staged-rollout acceptance gates.
