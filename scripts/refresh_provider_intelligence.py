#!/usr/bin/env python3
"""Refresh provider intelligence overlays for Manwe cloud providers."""

from __future__ import annotations

import json
import os
import tomllib
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ENV_PATH = ROOT / "config" / ".env"
ENV_PATHS = [
    ENV_PATH,
    ROOT / "config" / "runtime.generated.env",
    ROOT / "config" / "offsite_operator.env",
    Path.home() / ".hermes" / ".env",
]
INTELLIGENCE_PATH = Path(
    os.getenv(
        "ARDA_PROVIDER_INTELLIGENCE_PATH",
        str(ROOT / "core" / "state" / "provider_intelligence.json"),
    )
)
PROVIDER_CONFIG_PATH = Path(
    os.getenv(
        "ARDA_MANWE_PROVIDER_CONFIG",
        str(ROOT / "config" / "manwe.providers.toml"),
    )
)
RUNTIME_GOVERNOR_BUDGET_PATH = Path(
    os.getenv(
        "ARDA_RUNTIME_GOVERNOR_BUDGET",
        str(ROOT / "config" / "runtime_governor_budget.toml"),
    )
)
VIRTUAL_MODEL_IDS = {
    "openrouter": {"openrouter/auto"},
}
OPENROUTER_FREE_MODEL_IDS = {"openrouter/free"}
OPENROUTER_NON_FREE_DEFAULT_BLOCKLIST = {
    "openrouter/owl-alpha",
    "google/lyria-3-clip-preview",
    "google/lyria-3-pro-preview",
}


def utc_now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def openrouter_configured_daily_soft_cap(default: int = 2000) -> int:
    if not RUNTIME_GOVERNOR_BUDGET_PATH.exists():
        return default
    try:
        with RUNTIME_GOVERNOR_BUDGET_PATH.open("rb") as fh:
            config = tomllib.load(fh)
    except Exception:  # noqa: BLE001
        return default
    provider_budget = (config.get("providers") or {}).get("openrouter") or {}
    monthly_soft = provider_budget.get("monthly_requests_soft_cap")
    if not isinstance(monthly_soft, int) or monthly_soft <= 0:
        return default
    return max(1, monthly_soft // 30)


def openrouter_free_daily_limit(is_free_tier: bool | None, credits_data: dict | None) -> int:
    credits_total = 0.0
    if credits_data and isinstance(credits_data.get("data"), dict):
        raw_total = credits_data["data"].get("total_credits")
        if isinstance(raw_total, (int, float)):
            credits_total = float(raw_total)
    configured_cap = openrouter_configured_daily_soft_cap(default=1000)
    if is_free_tier and credits_total < 10.0:
        # OpenRouter can return a low per-model/per-window hint even when the
        # operator intentionally wants the free pool treated as a broader spillover
        # lane. Keep the configured soft cap as Manwe's durable budget truth and
        # let runtime failures/cooldowns handle actual exhausted models.
        return min(1000, configured_cap)
    return min(1000, configured_cap)


def load_env_file(path: Path) -> None:
    if not path.exists():
        return
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        line = line.removeprefix("export ").strip()
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        key = key.strip()
        value = value.strip().strip("'").strip('"')
        if key and key not in os.environ:
            os.environ[key] = value
    if not os.getenv("OPENAI_API_KEY") and os.getenv("OPEN_AI_KEY"):
        os.environ["OPENAI_API_KEY"] = os.environ["OPEN_AI_KEY"]
    if not os.getenv("GOOGLE_API_KEY") and os.getenv("GEMINI_API_KEY"):
        os.environ["GOOGLE_API_KEY"] = os.environ["GEMINI_API_KEY"]
    if not os.getenv("GLM_API_KEY") and os.getenv("ZAI_API_KEY"):
        os.environ["GLM_API_KEY"] = os.environ["ZAI_API_KEY"]
    if not os.getenv("ZAI_API_KEY") and os.getenv("GLM_API_KEY"):
        os.environ["ZAI_API_KEY"] = os.environ["GLM_API_KEY"]
    if not os.getenv("MOONSHOT_API_KEY") and os.getenv("KIMI_API_KEY"):
        os.environ["MOONSHOT_API_KEY"] = os.environ["KIMI_API_KEY"]
    if not os.getenv("KIMI_API_KEY") and os.getenv("MOONSHOT_API_KEY"):
        os.environ["KIMI_API_KEY"] = os.environ["MOONSHOT_API_KEY"]


def load_env_files(paths: list[Path]) -> None:
    for path in paths:
        load_env_file(path)


def fetch_json(url: str, *, bearer: str | None = None) -> dict:
    req = urllib.request.Request(url)
    req.add_header("Accept", "application/json")
    req.add_header("User-Agent", "arda-provider-intelligence/1.0")
    if bearer:
        req.add_header("Authorization", f"Bearer {bearer}")
    with urllib.request.urlopen(req, timeout=20) as resp:
        return json.loads(resp.read().decode("utf-8"))


def offline_mode() -> bool:
    return os.getenv("ARDA_PROVIDER_INTELLIGENCE_OFFLINE", "").lower() in {
        "1",
        "true",
        "yes",
    }


def safe_fetch_json(url: str, *, bearer: str | None = None) -> tuple[dict | None, str | None]:
    if offline_mode():
        return None, "offline_mode"
    try:
        return fetch_json(url, bearer=bearer), None
    except urllib.error.HTTPError as exc:
        try:
            body = exc.read().decode("utf-8", errors="replace")
        except Exception:
            body = ""
        return None, f"http_{exc.code}:{body[:240]}"
    except Exception as exc:  # noqa: BLE001
        return None, str(exc)


def is_zeroish(value: object) -> bool:
    if value is None:
        return True
    text = str(value).strip()
    return text in {"", "0", "0.0", "0.00", "0.000", "0.0000", "0.000000"}


def is_text_model(model: dict) -> bool:
    arch = model.get("architecture") or {}
    outputs = arch.get("output_modalities") or []
    if not outputs:
        return True
    return "text" in outputs


def infer_capable_tasks(model: dict, provider_id: str | None = None) -> list[str]:
    supported = {str(item).lower() for item in (model.get("supported_parameters") or [])}
    tasks = {"chat", "summary", "background"}
    model_id = str(model.get("id", "")).lower()
    provider_id = (provider_id or "").lower()

    code_tokens = ("coder", "code", "codestral", "devstral", "starcoder", "codegemma", "qwen3-coder", "cobuddy", "ring-2.6")
    reasoning_tokens = ("reason", "think", "r1", "magistral", "nemotron", "glm", "gpt-oss", "ring-2.6", "kimi")
    research_tokens = ("medium", "ultra", "large", "v3.1", "v3.2", "4.5", "4.6", "4.7", "5.1", "1t", "m2.5", "m2.7")

    if "tools" in supported or "tool_choice" in supported:
        tasks.add("code")
    if any(token in model_id for token in code_tokens):
        tasks.update({"code", "reasoning"})
    if any(token in model_id for token in reasoning_tokens):
        tasks.add("reasoning")
    if "reasoning" in supported or "include_reasoning" in supported:
        tasks.add("reasoning")
    context = int(model.get("context_length") or model.get("max_context_length") or 0)
    if context >= 64000 or any(token in model_id for token in research_tokens):
        tasks.add("research")

    if provider_id == "mistral":
        if "codestral" in model_id or "devstral" in model_id:
            tasks.update({"code", "reasoning", "research"})
        if "mistral-medium" in model_id or "magistral" in model_id:
            tasks.update({"reasoning", "research"})
    elif provider_id == "zai":
        if model_id.startswith("glm-"):
            tasks.update({"code", "reasoning", "research"})
    elif provider_id == "nvidia":
        if any(token in model_id for token in ("deepseek-ai/deepseek-coder", "starcoder", "codegemma")):
            tasks.update({"code", "reasoning"})
        if any(token in model_id for token in ("nemotron-ultra", "deepseek-ai/deepseek-v3", "llama-3.3-70b-instruct")):
            tasks.update({"code", "reasoning", "research"})
    elif provider_id == "cerebras":
        if any(token in model_id for token in ("qwen-3", "qwen3", "gpt-oss", "glm")):
            tasks.update({"code", "reasoning", "research"})
    elif provider_id == "groq":
        if any(token in model_id for token in ("qwen", "gpt-oss", "compound", "llama-3.3-70b")):
            tasks.update({"code", "reasoning", "research"})
    elif provider_id == "minimax":
        if any(token in model_id for token in ("minimax-m", "m2", "m3")):
            tasks.update({"code", "reasoning", "research"})
    elif provider_id == "kimi":
        if any(token in model_id for token in ("kimi", "moonshot")):
            tasks.update({"code", "reasoning", "research"})
    elif provider_id == "opencode":
        if model_id.endswith("-free") or any(token in model_id for token in ("deepseek", "mimo", "qwen3.6", "minimax", "nemotron")):
            tasks.update({"code", "reasoning", "research"})

    return sorted(tasks)


def is_free_model(model: dict) -> bool:
    model_id = str(model.get("id", ""))
    model_key = model_id.lower()
    if model_key in OPENROUTER_NON_FREE_DEFAULT_BLOCKLIST:
        return False
    if model_key in OPENROUTER_FREE_MODEL_IDS:
        return True
    if model_id.endswith(":free"):
        return True
    pricing = model.get("pricing") or {}
    return all(
        is_zeroish(pricing.get(field))
        for field in (
            "prompt",
            "completion",
            "request",
            "image",
            "web_search",
            "internal_reasoning",
            "input_cache_read",
            "input_cache_write",
        )
    )


def model_overlay(model: dict, *, default: bool = False) -> dict:
    return {
        "id": model["id"],
        "capable_tasks": infer_capable_tasks(model),
        "context_window": int(model.get("context_length") or 8192),
        "is_default": default,
    }


def context_window_hint(model: dict, default_value: int) -> int:
    for key in ("context_length", "max_context_length", "input_token_limit", "max_input_tokens"):
        value = model.get(key)
        if isinstance(value, int) and value > 0:
            return value
    model_id = str(model.get("id", "")).lower()
    if "ring-2.6-1t" in model_id:
        return 262144
    if any(token in model_id for token in ("gemini", "gpt-5.5", "claude-opus", "claude-sonnet")):
        return max(default_value, 200000)
    if any(token in model_id for token in ("minimax", "kimi", "qwen3.6", "glm-5")):
        return max(default_value, 128000)
    if any(token in model_id for token in ("qwen-3", "qwen3", "gpt-oss", "llama-3.3-70b")):
        return max(default_value, 131072)
    return default_value


def extract_models(payload: dict | None) -> list[dict]:
    if not payload:
        return []
    if isinstance(payload.get("data"), list):
        return [item for item in payload["data"] if isinstance(item, dict) and item.get("id")]
    if isinstance(payload.get("models"), list):
        out: list[dict] = []
        for item in payload["models"]:
            if isinstance(item, dict) and item.get("id"):
                out.append(item)
            elif isinstance(item, str) and item.strip():
                out.append({"id": item.strip()})
        return out
    return []


def model_overlay_openaiish(model: dict, *, provider_id: str, default: bool = False, default_context: int = 8192) -> dict:
    return {
        "id": model["id"],
        "capable_tasks": infer_capable_tasks(model, provider_id),
        "context_window": context_window_hint(model, default_context),
        "is_default": default,
    }


def preferred_model_order(provider_id: str) -> list[str]:
    provider_id = provider_id.lower()
    if provider_id == "mistral":
        return [
            "mistral-small-latest",
            "mistral-small-2603",
            "mistral-medium-latest",
            "mistral-medium-2508",
            "mistral-medium-2505",
            "mistral-medium",
            "codestral-latest",
            "codestral-2508",
            "devstral-latest",
            "devstral-medium-latest",
        ]
    if provider_id == "zai":
        return [
            "glm-5.1",
            "glm-5",
            "glm-5-turbo",
            "glm-4.7",
            "glm-4.6",
            "glm-4.5",
            "glm-4.5-air",
        ]
    if provider_id == "nvidia":
        return [
            "openai/gpt-oss-120b",
            "openai/gpt-oss-20b",
            "meta/llama-3.3-70b-instruct",
            "deepseek-ai/deepseek-v3.1-terminus",
            "qwen/qwen3-coder-480b-a35b-instruct",
            "qwen/qwen3.5-397b-a17b",
            "mistralai/devstral-2-123b-instruct-2512",
            "mistralai/mistral-large-3-675b-instruct-2512",
            "moonshotai/kimi-k2.5",
            "nvidia/llama-3.3-nemotron-super-49b-v1.5",
            "deepseek-ai/deepseek-v3.2",
            "deepseek-ai/deepseek-v3.1-terminus",
            "bigcode/starcoder2-15b",
            "deepseek-ai/deepseek-coder-6.7b-instruct",
        ]
    if provider_id == "cerebras":
        return [
            "qwen-3-235b-a22b-instruct-2507",
            "gpt-oss-120b",
            "zai-glm-4.7",
            "llama3.1-8b",
        ]
    if provider_id == "groq":
        return [
            "llama-3.3-70b-versatile",
            "qwen/qwen3-32b",
            "openai/gpt-oss-120b",
            "groq/compound",
            "openai/gpt-oss-20b",
            "llama-3.1-8b-instant",
            "meta-llama/llama-4-scout-17b-16e-instruct",
        ]
    if provider_id == "minimax":
        return [
            "MiniMax-M2.7",
            "MiniMax-M2.7-highspeed",
            "MiniMax-M2.5",
            "minimax-m2.7",
            "minimax-m2.5",
            "minimax-m3",
        ]
    if provider_id == "kimi":
        return [
            "kimi-k2.6",
            "kimi-k2.5",
            "kimi-k2",
            "moonshot-v1-128k",
            "moonshot-v1-32k",
            "moonshot-v1-8k",
        ]
    if provider_id == "opencode":
        return [
            "deepseek-v4-flash-free",
            "mimo-v2.5-free",
            "qwen3.6-plus-free",
            "minimax-m3-free",
            "nemotron-3-super-free",
            "deepseek-v4-flash",
            "mimo-v2.5",
            "qwen3.6-plus",
            "minimax-m2.7",
            "minimax-m2.5",
            "kimi-k2.6",
            "kimi-k2.5",
            "glm-5.1",
            "glm-5",
            "gpt-5.5",
            "claude-sonnet-4-6",
        ]
    return []


def sort_models_for_provider(provider_id: str, models: list[dict]) -> list[dict]:
    preferred = preferred_model_order(provider_id)
    index = {model_id.lower(): position for position, model_id in enumerate(preferred)}

    def sort_key(model: dict) -> tuple[int, int, str]:
        model_id = str(model.get("id", "")).lower()
        preferred_rank = index.get(model_id, len(preferred) + 100)
        context_rank = -context_window_hint(model, 8192)
        return (preferred_rank, context_rank, model_id)

    return sorted(models, key=sort_key)


def dedupe_models_by_id(models: list[dict]) -> list[dict]:
    seen: set[str] = set()
    unique: list[dict] = []
    for model in models:
        model_id = str(model.get("id", "")).strip()
        if not model_id:
            continue
        key = model_id.lower()
        if key in seen:
            continue
        seen.add(key)
        unique.append(model)
    return unique


def free_model_sort_key(model: dict) -> tuple[int, int, str]:
    model_id = str(model.get("id", "")).lower()
    ring_rank = 0 if "ring-2.6" in model_id else 1
    return (ring_rank, -context_window_hint(model, 8192), model_id)


def load_existing_intelligence(path: Path) -> dict:
    if not path.exists():
        return {
            "schema_version": "arda.provider-intelligence.v1",
            "generated_at_utc": utc_now(),
            "authority": "manual_seed",
            "providers": {},
        }
    return json.loads(path.read_text(encoding="utf-8"))


def configured_model_ids(provider_id: str, config_path: Path | None = None) -> list[str]:
    config_path = config_path or PROVIDER_CONFIG_PATH
    if not config_path.exists():
        return []
    try:
        config = tomllib.loads(config_path.read_text(encoding="utf-8"))
    except Exception:
        return []
    providers = config.get("provider") or []
    if not isinstance(providers, list):
        return []
    for provider in providers:
        if not isinstance(provider, dict) or provider.get("id") != provider_id:
            continue
        models = provider.get("model") or []
        if not isinstance(models, list):
            return []
        return [
            model["id"]
            for model in models
            if isinstance(model, dict) and isinstance(model.get("id"), str) and model["id"].strip()
        ]
    return []


def stale_configured_models(provider_id: str, fetched_models: list[dict]) -> list[str]:
    if not fetched_models:
        return []
    live_ids = {str(model.get("id", "")).strip().lower() for model in fetched_models}
    virtual_ids = {model_id.lower() for model_id in VIRTUAL_MODEL_IDS.get(provider_id, set())}
    stale = []
    for model_id in configured_model_ids(provider_id):
        normalized = model_id.strip().lower()
        if normalized in virtual_ids:
            continue
        if normalized and normalized not in live_ids:
            stale.append(model_id)
    return sorted(stale, key=str.lower)


def refresh_openrouter(existing: dict) -> dict:
    providers = existing.setdefault("providers", {})
    providers.pop("openrouter_free", None)
    previous = providers.get("openrouter") if isinstance(providers.get("openrouter"), dict) else {}
    key = os.getenv("OPENROUTER_API_KEY", "").strip()

    models_data, models_error = safe_fetch_json("https://openrouter.ai/api/v1/models")
    key_data = None
    key_error = None
    credits_data = None
    credits_error = None
    if key:
        key_data, key_error = safe_fetch_json("https://openrouter.ai/api/v1/key", bearer=key)
        credits_data, credits_error = safe_fetch_json("https://openrouter.ai/api/v1/credits", bearer=key)

    all_models = (models_data or {}).get("data") or []
    text_models = [model for model in all_models if is_text_model(model)]
    free_models = [model for model in text_models if is_free_model(model)]
    free_models.sort(key=free_model_sort_key)

    free_limit_daily = None
    free_limit_minute = 20
    is_free_tier = None
    limit_remaining = None
    usage_daily = None
    if key_data and isinstance(key_data.get("data"), dict):
        key_info = key_data["data"]
        is_free_tier = key_info.get("is_free_tier")
        limit_remaining = key_info.get("limit_remaining")
        usage_daily = key_info.get("usage_daily")
        explicit_limit = key_info.get("limit")
        if isinstance(explicit_limit, int) and explicit_limit > 0:
            free_limit_daily = explicit_limit
        elif is_free_tier:
            free_limit_daily = openrouter_free_daily_limit(is_free_tier, credits_data)
        else:
            free_limit_daily = openrouter_free_daily_limit(is_free_tier, credits_data)

    refreshed = utc_now()
    discovered_models = [
        {
            "id": "openrouter/auto",
            "capable_tasks": ["code", "research", "reasoning", "chat", "summary", "background"],
            "context_window": max([128000] + [context_window_hint(model, 0) for model in free_models[:24]]),
            "is_default": True,
        }
    ] + [
        model_overlay_openaiish(
            model,
            provider_id="openrouter",
            default=False,
            default_context=128000,
        )
        for index, model in enumerate(free_models[:24])
    ]
    if models_error and previous.get("models"):
        selected_models = previous["models"]
    else:
        selected_models = discovered_models

    healthy = (previous.get("healthy") if models_error else True)
    if models_error and healthy is False and selected_models:
        healthy = None
    providers["openrouter"] = {
        "access_tier": "mixed",
        "quality_band": "high",
        "enabled": bool(key),
        "healthy": healthy,
        "requests_per_minute": free_limit_minute if is_free_tier else None,
        "requests_per_day": free_limit_daily,
        "refreshed_at_utc": refreshed,
        "models": selected_models,
        "metadata": {
            "source": "openrouter",
            "models_total": len(all_models),
            "text_models_total": len(text_models),
            "free_models_total": len(free_models),
            "stale_configured_models": stale_configured_models("openrouter", text_models),
            "limit_remaining": limit_remaining,
            "usage_daily": usage_daily,
            "credits": credits_data,
            "key_info": key_data,
            "errors": {
                "models": models_error,
                "key": key_error,
                "credits": credits_error,
            },
        },
    }
    return existing


def refresh_opencode(existing: dict) -> dict:
    providers = existing.setdefault("providers", {})
    previous = providers.get("opencode") if isinstance(providers.get("opencode"), dict) else {}
    key = os.getenv("OPENCODE_API_KEY", "").strip()
    models_data, models_error = safe_fetch_json("https://opencode.ai/zen/v1/models", bearer=key or None)
    if (models_error or not extract_models(models_data)) and key:
        models_data, models_error = safe_fetch_json("https://opencode.ai/zen/v1/models")
    fetched_models = extract_models(models_data)
    selected_models = sort_models_for_provider("opencode", fetched_models)
    fallback_models = [
        {"id": "deepseek-v4-flash-free"},
        {"id": "mimo-v2.5-free"},
        {"id": "qwen3.6-plus-free"},
        {"id": "minimax-m3-free"},
        {"id": "nemotron-3-super-free"},
    ]
    if models_error and previous.get("models"):
        overlay_models = previous["models"]
    else:
        overlay_models = [
            model_overlay_openaiish(
                model,
                provider_id="opencode",
                default=index == 0,
                default_context=128000,
            )
            for index, model in enumerate((selected_models or fallback_models)[:32])
        ]
    healthy = previous.get("healthy") if models_error else bool(fetched_models)
    if models_error and overlay_models:
        healthy = None
    providers["opencode"] = {
        "access_tier": "free_cloud",
        "quality_band": "high",
        "enabled": bool(key),
        "healthy": healthy,
        "requests_per_minute": 120,
        "requests_per_day": 100000,
        "refreshed_at_utc": utc_now(),
        "models": overlay_models,
        "metadata": {
            "source": "opencode",
            "base_url": "https://opencode.ai/zen/v1",
            "models_total": len(fetched_models),
            "stale_configured_models": stale_configured_models("opencode", fetched_models),
            "errors": {"models": models_error},
        },
    }
    return existing


def refresh_openai_compatible_provider(existing, provider_id, env_key, base_url, access_tier, quality_band, rpm, rpd, fallback_models):
    providers = existing.setdefault("providers", {})
    env_keys = [env_key] if isinstance(env_key, str) else list(env_key)
    key = next((os.getenv(item, "").strip() for item in env_keys if os.getenv(item, "").strip()), "")
    models_data, models_error = (None, None)
    if key:
        models_data, models_error = safe_fetch_json(f"{base_url.rstrip('/')}/models", bearer=key)
    fetched_models = extract_models(models_data)
    selected_models = dedupe_models_by_id(
        sort_models_for_provider(provider_id, fetched_models if fetched_models else fallback_models)
    )
    default_context = int(fallback_models[0].get("context_length") or fallback_models[0].get("context_window") or 8192)
    stale_models = stale_configured_models(provider_id, fetched_models)
    providers[provider_id] = {
        "access_tier": access_tier,
        "quality_band": quality_band,
        "enabled": bool(key),
        "healthy": bool(key) and models_error is None,
        "requests_per_minute": rpm,
        "requests_per_day": rpd,
        "refreshed_at_utc": utc_now(),
        "models": [
            model_overlay_openaiish(
                model,
                provider_id=provider_id,
                default=index == 0,
                default_context=default_context,
            )
            for index, model in enumerate(selected_models[:24])
        ],
        "metadata": {"source": provider_id, "base_url": base_url, "api_key_env": env_keys[0], "models_total": len(fetched_models), "stale_configured_models": stale_models, "errors": {"models": models_error}},
    }
    return existing


def main() -> int:
    if os.getenv("ARDA_PROVIDER_INTELLIGENCE_SKIP_ENV", "").lower() not in {
        "1",
        "true",
        "yes",
    }:
        load_env_files(ENV_PATHS)
    existing = load_existing_intelligence(INTELLIGENCE_PATH)
    existing["schema_version"] = "arda.provider-intelligence.v1"
    existing["generated_at_utc"] = utc_now()
    existing["authority"] = "refresh_provider_intelligence.py"
    refreshed = refresh_openrouter(existing)
    refreshed = refresh_opencode(refreshed)
    refreshed = refresh_openai_compatible_provider(
        refreshed,
        "mistral",
        "MISTRAL_API_KEY",
        "https://api.mistral.ai/v1",
        "mixed",
        "high",
        60,
        1000,
        [{"id": "mistral-small-2603", "context_length": 256000, "supported_parameters": ["tools", "reasoning"]}],
    )
    refreshed = refresh_openai_compatible_provider(
        refreshed,
        "zai",
        "ZAI_API_KEY",
        "https://api.z.ai/api/paas/v4",
        "paid_cloud",
        "high",
        60,
        10000,
        [{"id": "glm-5", "context_length": 200000, "supported_parameters": ["tools", "reasoning"]}],
    )
    refreshed = refresh_openai_compatible_provider(
        refreshed,
        "nvidia",
        "NVIDIA_API_KEY",
        "https://integrate.api.nvidia.com/v1",
        "mixed",
        "high",
        60,
        1000,
        [{"id": "qwen/qwen3-coder-480b-a35b-instruct", "context_length": 128000, "supported_parameters": ["tools", "reasoning"]}],
    )
    refreshed = refresh_openai_compatible_provider(
        refreshed,
        "cerebras",
        "CEREBRAS_API_KEY",
        "https://api.cerebras.ai/v1",
        "free_cloud",
        "high",
        30,
        1000000,
        [{"id": "qwen-3-235b-a22b-instruct-2507", "context_length": 131072, "supported_parameters": ["tools", "reasoning"]}],
    )
    refreshed = refresh_openai_compatible_provider(
        refreshed,
        "groq",
        "GROQ_API_KEY",
        "https://api.groq.com/openai/v1",
        "free_cloud",
        "high",
        30,
        14400,
        [{"id": "llama-3.3-70b-versatile", "context_length": 131072, "supported_parameters": ["tools", "reasoning"]}],
    )
    refreshed = refresh_openai_compatible_provider(
        refreshed,
        "minimax",
        "MINIMAX_API_KEY",
        "https://api.minimax.io/v1",
        "paid_cloud",
        "high",
        60,
        10000,
        [{"id": "MiniMax-M2.7", "context_length": 128000, "supported_parameters": ["tools", "reasoning"]}],
    )
    refreshed = refresh_openai_compatible_provider(
        refreshed,
        "kimi",
        ["MOONSHOT_API_KEY", "KIMI_API_KEY"],
        "https://api.moonshot.ai/v1",
        "paid_cloud",
        "high",
        60,
        10000,
        [{"id": "kimi-k2.6", "context_length": 256000, "supported_parameters": ["tools", "reasoning"]}],
    )
    INTELLIGENCE_PATH.parent.mkdir(parents=True, exist_ok=True)
    INTELLIGENCE_PATH.write_text(json.dumps(refreshed, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(str(INTELLIGENCE_PATH))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
