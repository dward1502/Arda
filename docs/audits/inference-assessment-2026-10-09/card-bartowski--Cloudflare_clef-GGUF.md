---
quantized_by: bartowski
pipeline_tag: image-text-to-text
base_model: Cloudflare/clef
license: apache-2.0
tags:
- clef
- cloudflare
- systemone
- qwen3.8
- post-train
- image-text-to-typed-output
- multimodal
- structured-output
- classification
- custom-code
base_model_relation: quantized
---

## Llamacpp Static Quantizations of clef by Cloudflare

Using <a href="https://github.com/ggml-org/llama.cpp/">llama.cpp</a> release <a href="https://github.com/ggml-org/llama.cpp/releases/tag/b11430">b11430</a> for quantization.

Original model: https://huggingface.co/Cloudflare/clef

**Model details:**
- Parameter count: 27B (source checkpoint)
- Input support: text, image (with mmproj file) - [details](#multimodal)
- Decision model: yes (clef) - [details](#decision-model)
- Speculative decoding: no
- imatrix: no (not possible for a decision model) - [details](#decision-model)

[How to run](#how-to-run)

## Decision model

This is a decision model, not a text generator: it reads a state (text or JSON) together with a schema of typed questions and hands back a probability for every allowed option of every question, all in a single forward pass. It never generates text, so there's no chat, no completion endpoint, and no prompt format to worry about.

llama.cpp serves it through llama-server's `/v1/systemone` endpoint (the [TypeSafe System One API](https://docs.typesafe.ai/api)). The full request and response shapes are in the [llama-server README](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md), but here's what a request looks like:

```
curl http://127.0.0.1:8080/v1/systemone \
    -H "Content-Type: application/json" \
    -d '{
        "state": "Customer message: I was charged twice for my order last week and nobody has replied.",
        "questions": {
            "route": {
                "type": "choice",
                "instructions": "Which team should handle this?",
                "criteria": {"billing": null, "shipping": null, "technical": null}
            },
            "angry": {
                "type": "noul",
                "instructions": "Is the customer angry?"
            },
            "urgency": {
                "type": "score",
                "instructions": "How urgent is this?",
                "criteria": ["can wait", "this week", "today", "right now"]
            }
        }
    }'
```

**Decision head.** The decision head - the tensors that turn the backbone's final hidden states into actual answers - is stored at Q8_0 in every quantized file here. It's a small part of the model, it's the part doing the answering, and none of the usual measurements work on this model (see below), so there's no way to check what quantizing it would cost. Keeping it at Q8_0 is the safe call.

**No imatrix.** There's no imatrix for this model, and there can't be one: llama.cpp's imatrix tool works by watching the token probabilities a model produces as it generates text, and a decision model doesn't produce any, so there's nothing to calibrate against. Every quantized file here is therefore a static quant, made without an imatrix. The files below 4 bits per weight (Q3_K_L, Q3_K_M, IQ3_M, Q3_K_S and Q2_K) will lose noticeably more quality than an imatrix quant of the same size would, so if you can, stick to Q4_K_M or larger. The formats that cannot be made at all without an imatrix (IQ1, IQ2, IQ3_XS, IQ3_XXS and Q2_K_S) are not offered. Perplexity and KL divergence can't be measured either, for the same reason, so the descriptions in the file table below are the usual guidance for each format rather than measurements of this particular model.

**Don't know which to choose?** Grab [Q4_K_M](https://huggingface.co/bartowski/Cloudflare_clef-GGUF/blob/main/Cloudflare_clef-Q4_K_M.gguf) (17.65GB) - usually a good mix of size and performance. Download instructions available [here](#downloading-using-the-hugging-face-cli)

## Available files:

| Filename | Quant type | File Size | Split | Description |
| -------- | ---------- | --------- | ----- | ----------- |
| [Cloudflare_clef-bf16.gguf](https://huggingface.co/bartowski/Cloudflare_clef-GGUF/tree/main/Cloudflare_clef-bf16) | bf16 | 54.06GB | true | Full BF16 weights. |
| [Cloudflare_clef-Q8_0.gguf](https://huggingface.co/bartowski/Cloudflare_clef-GGUF/blob/main/Cloudflare_clef-Q8_0.gguf) | Q8_0 | 28.80GB | false | Extremely high quality, generally unneeded but max available quant. |
| [Cloudflare_clef-Q6_K_L.gguf](https://huggingface.co/bartowski/Cloudflare_clef-GGUF/blob/main/Cloudflare_clef-Q6_K_L.gguf) | Q6_K_L | 23.96GB | false | Uses Q8_0 for embed and output weights. Very high quality, near perfect, *recommended*. |
| [Cloudflare_clef-Q6_K.gguf](https://huggingface.co/bartowski/Cloudflare_clef-GGUF/blob/main/Cloudflare_clef-Q6_K.gguf) | Q6_K | 23.34GB | false | Very high quality, near perfect, *recommended*. |
| [Cloudflare_clef-Q5_K_M.gguf](https://huggingface.co/bartowski/Cloudflare_clef-GGUF/blob/main/Cloudflare_clef-Q5_K_M.gguf) | Q5_K_M | 20.64GB | false | High quality, *recommended*. |
| [Cloudflare_clef-Q5_K_S.gguf](https://huggingface.co/bartowski/Cloudflare_clef-GGUF/blob/main/Cloudflare_clef-Q5_K_S.gguf) | Q5_K_S | 19.57GB | false | High quality, *recommended*. |
| [Cloudflare_clef-Q4_K_L.gguf](https://huggingface.co/bartowski/Cloudflare_clef-GGUF/blob/main/Cloudflare_clef-Q4_K_L.gguf) | Q4_K_L | 18.60GB | false | Uses Q8_0 for embed and output weights. Good quality, *recommended*. |
| [Cloudflare_clef-Q4_1.gguf](https://huggingface.co/bartowski/Cloudflare_clef-GGUF/blob/main/Cloudflare_clef-Q4_1.gguf) | Q4_1 | 17.71GB | false | Legacy format, similar performance to Q4_K_S but with improved tokens/watt on Apple silicon. |
| [Cloudflare_clef-Q4_K_M.gguf](https://huggingface.co/bartowski/Cloudflare_clef-GGUF/blob/main/Cloudflare_clef-Q4_K_M.gguf) | Q4_K_M | 17.65GB | false | Good quality, default size for most use cases, *recommended*. |
| [Cloudflare_clef-Q4_K_S.gguf](https://huggingface.co/bartowski/Cloudflare_clef-GGUF/blob/main/Cloudflare_clef-Q4_K_S.gguf) | Q4_K_S | 16.59GB | false | Slightly lower quality with more space savings, *recommended*. |
| [Cloudflare_clef-IQ4_NL.gguf](https://huggingface.co/bartowski/Cloudflare_clef-GGUF/blob/main/Cloudflare_clef-IQ4_NL.gguf) | IQ4_NL | 16.30GB | false | Similar to IQ4_XS, but slightly larger. |
| [Cloudflare_clef-Q4_0.gguf](https://huggingface.co/bartowski/Cloudflare_clef-GGUF/blob/main/Cloudflare_clef-Q4_0.gguf) | Q4_0 | 16.19GB | false | Legacy format, kept for compatibility with older tools. |
| [Cloudflare_clef-IQ4_XS.gguf](https://huggingface.co/bartowski/Cloudflare_clef-GGUF/blob/main/Cloudflare_clef-IQ4_XS.gguf) | IQ4_XS | 15.56GB | false | Decent quality, smaller than Q4_K_S with similar performance, *recommended*. |
| [Cloudflare_clef-Q3_K_L.gguf](https://huggingface.co/bartowski/Cloudflare_clef-GGUF/blob/main/Cloudflare_clef-Q3_K_L.gguf) | Q3_K_L | 15.15GB | false | Lower quality but usable, good for low RAM availability. Made without an imatrix: expect a larger quality loss than usual at this size. |
| [Cloudflare_clef-Q3_K_M.gguf](https://huggingface.co/bartowski/Cloudflare_clef-GGUF/blob/main/Cloudflare_clef-Q3_K_M.gguf) | Q3_K_M | 14.48GB | false | Low quality. Made without an imatrix: expect a larger quality loss than usual at this size. |
| [Cloudflare_clef-IQ3_M.gguf](https://huggingface.co/bartowski/Cloudflare_clef-GGUF/blob/main/Cloudflare_clef-IQ3_M.gguf) | IQ3_M | 13.78GB | false | Medium-low quality, new method with decent performance comparable to Q3_K_M. Made without an imatrix: expect a larger quality loss than usual at this size. |
| [Cloudflare_clef-Q3_K_S.gguf](https://huggingface.co/bartowski/Cloudflare_clef-GGUF/blob/main/Cloudflare_clef-Q3_K_S.gguf) | Q3_K_S | 13.59GB | false | Low quality, *not* recommended. Made without an imatrix: expect a larger quality loss than usual at this size. |
| [Cloudflare_clef-Q2_K.gguf](https://huggingface.co/bartowski/Cloudflare_clef-GGUF/blob/main/Cloudflare_clef-Q2_K.gguf) | Q2_K | 11.71GB | false | Very low quality but surprisingly usable. Made without an imatrix: expect a larger quality loss than usual at this size. |

Download a specific file:

```
hf download bartowski/Cloudflare_clef-GGUF --include "Cloudflare_clef-Q4_K_M.gguf" --local-dir ./
```

## Downloading using the Hugging Face CLI

<details>
  <summary>Click to view download instructions</summary>

First, make sure you have the Hugging Face CLI installed:

```
pip install -U "huggingface_hub[cli]"
```

Download a specific file:

```
hf download bartowski/Cloudflare_clef-GGUF --include "Cloudflare_clef-Q4_K_M.gguf" --local-dir ./
```

The files marked `true` in the Split column above are stored as multiple parts in a folder. To download all the parts to a local folder, run:

```
hf download bartowski/Cloudflare_clef-GGUF --include "Cloudflare_clef-bf16/*" --local-dir ./
```

You can either specify a new local-dir (Cloudflare_clef-bf16) or download them all in place (./)

</details>

## How to run

These quants run with [llama.cpp](https://github.com/ggml-org/llama.cpp) - installable in one line via [llama.app](https://llama.app/):

```
curl -LsSf https://llama.app/install.sh | sh
llama-server -hf bartowski/Cloudflare_clef-GGUF:Q4_K_M
```

llama-server exposes this model on the `/v1/systemone` endpoint only - there's no chat web UI for a decision model. See [Decision model](#decision-model) above for an example request.

These quants were made with llama.cpp release b11430 - if this model's architecture is newly supported, you'll need that release or newer to run them.

## Multimodal

This model supports image input. Alongside the quants, this repo includes the multimodal projector files [mmproj-Cloudflare_clef-bf16.gguf](https://huggingface.co/bartowski/Cloudflare_clef-GGUF/blob/main/mmproj-Cloudflare_clef-bf16.gguf) and [mmproj-Cloudflare_clef-f16.gguf](https://huggingface.co/bartowski/Cloudflare_clef-GGUF/blob/main/mmproj-Cloudflare_clef-f16.gguf), which pair with any quant above.

llama.cpp downloads the mmproj automatically when using `-hf` as shown above; if you're loading files manually, pass it with `--mmproj`.

## Embed/output weights

Some of these quants (Q3_K_XL, Q4_K_L etc) are the standard quantization method with the embeddings and output weights quantized to Q8_0 instead of what they would normally default to.

## ARM/AVX information

llama.cpp automatically "repacks" weights into an interleaved layout at load time for faster inference on ARM and AVX machines - details in [this PR](https://github.com/ggml-org/llama.cpp/pull/9921). This once required downloading special Q4_0_4_4/4_8/8_8 files; those are long gone. Online repacking now covers Q4_0, IQ4_NL, and most K-quants, so no special quant choice is needed for CPU inference.

## Which file should I choose?

<details>
  <summary>Click here for details</summary>

An older (early 2024) but still useful write-up with charts comparing quant performances is provided by Artefact2 [here](https://gist.github.com/Artefact2/b5f810600771265fc1e39442288e8ec9)

The first thing to figure out is how big a model you can run. To do this, you'll need to figure out how much RAM and/or VRAM you have.

If you want your model running as FAST as possible, you'll want to fit the whole thing on your GPU's VRAM. Aim for a quant with a file size 1-2GB smaller than your GPU's total VRAM.

If you want the absolute maximum quality, add both your system RAM and your GPU's VRAM together, then similarly grab a quant with a file size 1-2GB Smaller than that total.

Hugging Face can also do this math for you: add your hardware in your [Local Apps settings](https://huggingface.co/settings/local-apps) and the model page will show which files fit.

Next, you'll need to decide if you want to use an 'I-quant' or a 'K-quant'.

If you don't want to think too much, grab one of the K-quants. These are in format 'QX_K_X', like Q5_K_M.

If you want to get more into the weeds, you can check out this extremely useful feature chart:

[llama.cpp feature matrix](https://github.com/ggml-org/llama.cpp/wiki/Feature-matrix)

But basically, if you're aiming for below Q4, and you're running cuBLAS (Nvidia) or rocBLAS (AMD), you should look towards the I-quants. These are in format IQX_X, like IQ3_M. These are newer and offer better performance for their size.

These I-quants can also be used on CPU, but will be slower than their K-quant equivalent, so speed vs performance is a tradeoff you'll have to decide.

</details>

## Credits

Thank you ZeroWw for the inspiration to experiment with embed/output.

Want to support my work? Visit my ko-fi page here: https://ko-fi.com/bartowski
