---
library_name: transformers
base_model: Qwen/Qwen3.5-9B
base_model_relation: finetune
tags:
- mimo_v2
- agentic
- distillation
- supervised-fine-tuning
- code
- tool-use
license: mit
---

# MiMo-V2.6-Distill-Qwen-9B

MiMo-V2.6-Distill-Qwen-9B is a 9B agentic model developed by Xiaomi MiMo through supervised fine-tuning of [Qwen3.5-9B](https://huggingface.co/Qwen/Qwen3.5-9B) on MiMo-generated data. It covers coding, general-purpose agent tasks, visual coding, and cybersecurity. We release this SFT checkpoint as a starting point for open research in agentic reinforcement learning.

## Evaluation

Results for the released SFT checkpoint, as reported in the MiMo-V2.6 technical report.

| Domain | Benchmark | Metric | Qwen3.5-9B | MiMo-V2.6-Distill-Qwen-9B (SFT) |
| --- | --- | --- | ---: | ---: |
| Code | SWE Verified | avg@3 | 60.0 | **61.1** |
| Code | SWE Pro | avg@3 | 32.0 | **44.6** |
| Code | MiMo Code (mini)† | avg@3 | 19.5 | **51.6** |
| Cyber | MiMo Cyber (mini)† | avg@3 | 5.7 | **31.3** |
| General | AutomationBench v1.0.6 | avg@1 | 5.0 | **30.3** |
| General | Terminal Bench 2.1 | avg@1 | 27.0 | **37.1** |
| General | Toolathlon-Verified | avg@1 | 25.9 | **35.2** |
| General | OfficeQA | avg@1 | 9.0 | **19.5** |
| General | JobBench | avg@1 | 2.6 | **18.3** |
| General | MiMo General (mini)† | avg@1 | 28.5 | **62.2** |
| Visual | MiMo Visual Coding (mini)† | avg@1 | 61.7 | **64.0** |

† Internal evaluation sets.

## Training Data

The weighted SFT data mixture contains 77.4B total tokens, including 27.2B loss-bearing tokens.

| Domain | Total tokens (B) | Token share (%) | Loss-bearing tokens (B) |
| --- | ---: | ---: | ---: |
| Code | 23.2 | 29.9 | 7.3 |
| Cyber | 11.0 | 14.2 | 4.8 |
| General | 22.0 | 28.5 | 5.7 |
| Visual | 21.2 | 27.4 | 9.4 |
| **Total** | **77.4** | **100.0** | **27.2** |

## Quickstart

For text generation, use a recent [SGLang](https://docs.sglang.io/get_started/install.html) build with Qwen3.5 support. The checkpoint includes its tokenizer and MiMo v2.6 chat template.

```bash
sglang serve \
  --model-path XiaomiMiMo/MiMo-V2.6-Distill-Qwen-9B \
  --reasoning-parser mimo \
  --host 0.0.0.0 \
  --port 30000
```

Query the endpoint with thinking explicitly enabled:

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:30000/v1",
    api_key="EMPTY",
)

response = client.chat.completions.create(
    model="XiaomiMiMo/MiMo-V2.6-Distill-Qwen-9B",
    messages=[
        {"role": "user", "content": "What is 15% of 240?"}
    ],
    max_tokens=2048,
    extra_body={"chat_template_kwargs": {"enable_thinking": True}},
)

message = response.choices[0].message
print("Thinking:", getattr(message, "reasoning_content", "") or "")
print("Answer:", message.content or "")
```

## Citation

```bibtex
@misc{mimo2026v26,
  title={MiMo-V2.6: Scaling Reinforcement Learning Towards Self-Improvement},
  author={{Xiaomi MiMo Team}},
  year={2026},
  howpublished={\url{https://huggingface.co/XiaomiMiMo/MiMo-V2.6-Pro-RL}},
}
```