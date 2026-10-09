"""Owned pre-initialization checks; deliberately accepts JSON (a YAML subset)."""
import json
import os
import re
import stat


def _unique_fields(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate profile field")
        result[key] = value
    return result


def validate_profile(home):
    for name in (".env", ".op.env", "auth.json", "active_profile", "managed-settings.json"):
        if os.path.lexists(os.path.join(home, name)):
            raise ValueError("ambient profile authority is forbidden")
    plugins = os.path.join(home, "plugins")
    if os.path.lexists(plugins) and (os.path.islink(plugins) or os.listdir(plugins)):
        raise ValueError("profile plugins are forbidden")
    fd = os.open(os.path.join(home, "config.yaml"), os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
    try:
        if not stat.S_ISREG(os.fstat(fd).st_mode) or not os.fstatvfs(fd).f_flag & os.ST_RDONLY:
            raise ValueError("profile must be a readonly regular file")
        with os.fdopen(os.dup(fd), "rb") as handle:
            raw = handle.read(65537)
    finally:
        os.close(fd)
    if len(raw) > 65536 or b"${" in raw:
        raise ValueError("profile exceeds bounds or expands environment")
    config = json.loads(raw, object_pairs_hook=_unique_fields)
    return validate_profile_config(config)


def validate_profile_config(config):
    """Validate the captured policy independently of filesystem admission."""
    custom = config["custom_providers"]
    if type(custom) is not list or len(custom) != 1:
        raise ValueError("one Manwe gateway profile is required")
    model = config["model"]["default"]
    context = config["model"]["context_length"]
    provider = custom[0]["extra_body"]["routing"]["force_provider_id"]
    origin = custom[0]["extra_body"]["routing"]["inference_origin"]
    if type(model) is not str or not re.fullmatch(r"[A-Za-z0-9._/-]{1,128}", model):
        raise ValueError("invalid model identifier")
    if type(provider) is not str or not provider.strip():
        raise ValueError("an explicit provider is required")
    if origin not in ("local", "cloud"):
        raise ValueError("an explicit local or cloud origin is required")
    if type(context) is not int or not 256 <= context <= 262144:
        raise ValueError("invalid context budget")
    expected = {
        "model": {"provider": "retained-manwe", "default": model, "context_length": context},
        "custom_providers": [{"name": "retained-manwe", "base_url": "http://127.0.0.1:7171/v1", "api_mode": "chat_completions", "model": model, "extra_body": {"routing": {"local_only": origin == "local", "inference_origin": origin, "origin_preference": origin, "force_provider_id": provider, "force_model_id": model, "allow_forced_provider_fallback": False, "tool_use_required": True}}}],
        "fallback_providers": [],
        "terminal": {"backend": "local", "cwd": "auto", "timeout": 180, "persistent_shell": False},
        "memory": {"memory_enabled": False, "user_profile_enabled": False, "provider": ""},
        "compression": {"enabled": False},
        "auxiliary": {"background_review": {"enabled": False}},
        "plugins": {"enabled": []}, "mcp_servers": {},
        "security":{"redact_secrets":True}, "database":{"journal_mode":"wal"},
    }

    if json.dumps(config, sort_keys=True) != json.dumps(expected, sort_keys=True):
        raise ValueError("profile is outside the retained Manwe contract")
    return config
