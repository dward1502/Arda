"""Worker-owned bootstrap; profile validation is prepended by the Rust worker."""
import importlib
import io
import json
import os
import sys

roots = json.loads(sys.argv.pop(1))
mode = sys.argv.pop(1)
os.environ.update(HERMES_SAFE_MODE="1", HERMES_REDACT_SECRETS="true", HERMES_GUEST_ONBOARDING="0")
for root in roots:
    for name in (".env", ".op.env", "auth.json", "config.yaml"):
        if os.path.lexists(os.path.join(root, name)):
            raise RuntimeError("ambient runtime configuration is forbidden")
guard = globals().get("validate_profile")
if not callable(guard):
    raise RuntimeError("owned profile guard is missing")
profile = guard(os.environ["HERMES_HOME"]) if mode != "probe" else None
sys.path[:0] = roots
sys.argv[0] = "hermes"

# Env safe mode preserves the dedicated config, unlike CLI --safe-mode.
env_loader = importlib.import_module("hermes_cli.env_loader")
setattr(env_loader, "load_hermes_dotenv", lambda *args, **kwargs: [])
main = importlib.import_module("hermes_cli.main")
setattr(main, "_start_chat_background_prefetch", lambda: None)

if mode == "chat":
    httpx = importlib.import_module("httpx")
    assert isinstance(profile, dict)
    original_send = httpx.Client.send
    expected_provider = profile["custom_providers"][0]["extra_body"]["routing"]["force_provider_id"]
    expected_model = profile["model"]["default"]

    def observed_send(client, request, *args, **kwargs):
        if request.method == "POST" and str(request.url) != "http://127.0.0.1:7171/v1/chat/completions":
            raise RuntimeError("unexpected inference destination")
        if request.method == "POST":
            body = json.loads(request.content)
            # Governed chat requires observed work, not a text-only assertion.
            # After a real tool result, allow the model to finish normally.
            # Auxiliary requests without tools are not execution turns.
            if (body.get("tools") and not any(message.get("role") == "tool"
                    for message in body.get("messages", []))
                    and body.get("tool_choice") in (None, "auto", "none")):
                body["tool_choice"] = "required"
                headers = dict(request.headers)
                headers.pop("content-length", None)
                request = httpx.Request(request.method, request.url, headers=headers,
                    content=json.dumps(body).encode(), extensions=request.extensions)
        # HTTPX follows redirects inside send, without re-entering this guard.
        kwargs["follow_redirects"] = False
        response = original_send(client, request, *args, **kwargs)
        if request.method == "POST" and 200 <= response.status_code < 300:
            evidence = {key: response.headers.get("x-manwe-" + key, "") for key in ("route-id", "provider-id", "model-id", "route-class")}
            if any(not value or len(value) > 256 or not value.isascii() or any(not 32 <= ord(c) <= 126 for c in value) for value in evidence.values()):
                response.close()
                raise RuntimeError("missing or malformed Manwe route evidence")
            if evidence["provider-id"] != expected_provider or evidence["model-id"] != expected_model or evidence["route-class"] != "tool_oriented":
                response.close()
                raise RuntimeError("Manwe selected an unapproved route")
            print("ARDA_MANWE_ROUTE " + json.dumps(evidence, sort_keys=True), file=sys.stderr, flush=True)
        return response

    httpx.Client.send = observed_send

if mode != "export":
    raise SystemExit(main.main())

# CLI --redact covers only some fields, and session resolution permits prefixes.
# Buffer bounded export output, require the full ID, then redact every field.
class BoundedExport(io.StringIO):
    def __init__(self):
        super().__init__()
        self.byte_count = 0

    def write(self, text):
        self.byte_count += len(text.encode("utf-8"))
        if self.byte_count > 2097152:
            raise RuntimeError("export exceeds retained output budget")
        return super().write(text)

requested_id = next(arg.split("=", 1)[1] for arg in sys.argv if arg.startswith("--session-id="))
output = BoundedExport()
original_stdout = sys.stdout
sys.stdout = output
try:
    try:
        main.main()
    except SystemExit as result:
        if result.code not in (None, 0):
            raise
finally:
    sys.stdout = original_stdout
exported = json.loads(output.getvalue())
if (exported.get("id") or exported.get("session_id")) != requested_id:
    raise RuntimeError("export did not resolve the exact admitted session")
redact_sensitive_text = importlib.import_module("agent.redact").redact_sensitive_text

def redact_all(value):
    if isinstance(value, str):
        return redact_sensitive_text(value, force=True)
    if isinstance(value, list):
        return [redact_all(item) for item in value]
    if isinstance(value, dict):
        return {redact_sensitive_text(key, force=True): redact_all(item) for key, item in value.items()}
    return value

print(json.dumps(redact_all(exported)))
