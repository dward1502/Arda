import importlib.util
from pathlib import Path


SCRIPT_PATH = Path(__file__).resolve().parents[1] / "scripts" / "refresh_provider_intelligence.py"
SPEC = importlib.util.spec_from_file_location("refresh_provider_intelligence", SCRIPT_PATH)
refresh = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(refresh)


def test_opencode_prefers_current_free_deepseek_model():
    models = [
        {"id": "nemotron-3-super-free"},
        {"id": "deepseek-v4-flash-free"},
        {"id": "mimo-v2.5-free"},
        {"id": "qwen3.6-plus-free"},
    ]

    sorted_models = refresh.sort_models_for_provider("opencode", models)

    assert sorted_models[0]["id"] == "deepseek-v4-flash-free"
    overlay = refresh.model_overlay_openaiish(
        sorted_models[0],
        provider_id="opencode",
        default=True,
        default_context=128000,
    )
    assert overlay["context_window"] == 128000
    assert {"code", "reasoning", "research"}.issubset(set(overlay["capable_tasks"]))


def test_openrouter_free_sort_prefers_ring_before_context_only():
    ring = {"id": "inclusionai/ring-2.6-1t:free", "context_length": 262144}
    broad = {"id": "google/gemini-3.1-flash-lite", "context_length": 1048576}

    assert sorted([broad, ring], key=refresh.free_model_sort_key)[0]["id"] == ring["id"]


def test_direct_provider_preferences_cover_cerebras_groq_minimax_kimi():
    cases = [
        ("cerebras", [{"id": "llama3.1-8b"}, {"id": "qwen-3-235b-a22b-instruct-2507"}], "qwen-3-235b-a22b-instruct-2507"),
        ("groq", [{"id": "openai/gpt-oss-120b"}, {"id": "llama-3.3-70b-versatile"}], "llama-3.3-70b-versatile"),
        ("minimax", [{"id": "MiniMax-M2.5"}, {"id": "MiniMax-M2.7"}], "MiniMax-M2.7"),
        ("kimi", [{"id": "moonshot-v1-128k"}, {"id": "kimi-k2.6"}], "kimi-k2.6"),
    ]

    for provider_id, models, expected in cases:
        sorted_models = refresh.sort_models_for_provider(provider_id, models)
        assert sorted_models[0]["id"] == expected
        overlay = refresh.model_overlay_openaiish(
            sorted_models[0],
            provider_id=provider_id,
            default=True,
            default_context=128000,
        )
        assert {"code", "reasoning", "research"}.issubset(set(overlay["capable_tasks"]))


def test_stale_configured_models_ignores_live_and_virtual_ids(tmp_path):
    config_path = tmp_path / "manwe.providers.toml"
    config_path.write_text(
        """
[[provider]]
id = "openrouter"

  [[provider.model]]
  id = "openrouter/auto"

  [[provider.model]]
  id = "deepseek/deepseek-v4-flash:free"

  [[provider.model]]
  id = "retired/model:free"
""",
        encoding="utf-8",
    )

    live_models = [{"id": "deepseek/deepseek-v4-flash:free"}]
    original_path = refresh.PROVIDER_CONFIG_PATH
    refresh.PROVIDER_CONFIG_PATH = config_path
    try:
        stale = refresh.stale_configured_models("openrouter", live_models)
    finally:
        refresh.PROVIDER_CONFIG_PATH = original_path

    assert stale == ["retired/model:free"]
