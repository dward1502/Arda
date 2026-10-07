#!/usr/bin/env python3
"""Exercise a real Manwe executable against isolated loopback fault fixtures.

No production configuration, credentials, model service or project state is used.
This qualifies gateway transport behavior, not canonical task/budget authority.
"""
import argparse
import contextlib
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import socket
import subprocess
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import urllib.error
import urllib.request


MODEL = "r06-fixture:free"
LOCAL = "edge_guardhouse"
CLOUD = "openrouter"
MARKER = "R06_FAULT_CHECK"


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


@contextlib.contextmanager
def upstream(mode):
    calls = []

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_args):
            pass

        def reply(self, status, value):
            data = json.dumps(value).encode()
            try:
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)
            except (BrokenPipeError, ConnectionResetError):
                pass  # Expected after the gateway's timeout.

        def do_GET(self):
            self.reply(200, {"object": "list", "data": [{"id": MODEL}]})

        def do_POST(self):
            body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            marked = MARKER in json.dumps(body.get("messages", []))
            if marked:
                calls.append({"model": body.get("model"), "max_tokens": body.get("max_tokens")})
                if mode == "timeout":
                    time.sleep(30)
                elif mode != "ok":
                    status, code = {
                        "quota": (429, "rate_limit_exceeded"),
                        "billing": (402, "insufficient_quota"),
                        "server": (503, "service_unavailable"),
                        "payload": (400, "invalid_request_error"),
                    }[mode]
                    self.reply(status, {"error": {"message": code, "type": code, "code": code}})
                    return
            self.reply(200, {"id": "fixture-completion", "object": "chat.completion",
                             "model": MODEL, "choices": [{"index": 0, "finish_reason": "stop",
                             "message": {"role": "assistant", "content": "R06_OK"}}],
                             "usage": {"prompt_tokens": 4, "completion_tokens": 2,
                                       "total_tokens": 6}})

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    worker = threading.Thread(target=server.serve_forever, daemon=True)
    worker.start()
    try:
        yield f"http://127.0.0.1:{server.server_port}/v1", calls
    finally:
        server.shutdown()
        server.server_close()
        worker.join(timeout=2)


def request(base, path, body=None, timeout=40):
    data = None if body is None else json.dumps(body).encode()
    req = urllib.request.Request(base + path, data=data,
                                 headers={"Content-Type": "application/json"})
    # Do not inherit shell HTTP proxy settings for the loopback fixture.
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    try:
        response = opener.open(req, timeout=timeout)
    except urllib.error.HTTPError as error:
        response = error
    with response:
        return response.status, json.load(response)


def provider_config(repo, local_url, cloud_url, local_enabled):
    defaults = (repo / "crates/spine/runtime/manwe/src/adaptive/service/full/bootstrap_defaults.rs").read_text()
    ids = set(re.findall(r'^            id: "([^"]+)"', defaults, re.M)) | {LOCAL, CLOUD}
    blocks = []
    for identity in sorted(ids):
        enabled = identity == CLOUD or (identity == LOCAL and local_enabled)
        blocks.append(f'''[[provider]]
id = {json.dumps(identity)}
enabled = {str(enabled).lower()}
base_url = {json.dumps(cloud_url if identity == CLOUD else local_url)}
driver = "openai_compat"
access_tier = "{'free_cloud' if identity == CLOUD else 'local'}"
requests_per_minute = 120
[[provider.model]]
id = "{MODEL}"
capable_tasks = ["chat", "background"]
context_window = 32768
is_default = true
cost_per_million_tokens_in = 0.0
cost_per_million_tokens_out = 0.0
capabilities = {{ tools = true, streaming = true }}
''')
    return "\n".join(blocks)


def run_case(args, name, mode, local_enabled=True, allow_fallback=False, local_only=False,
             privacy_location="routing", privacy_alias=False):
    output = args.output / name
    output.mkdir()
    with tempfile.TemporaryDirectory(prefix="arda-r06-fault-") as temporary, \
            upstream(mode) as (local_url, local_calls), upstream("ok") as (cloud_url, cloud_calls):
        root = Path(temporary)
        state = root / "data/manwe"
        state.mkdir(parents=True)
        config = root / "providers.toml"
        config.write_text(provider_config(args.repo, local_url, cloud_url, local_enabled))
        static = root / "manwe.toml"
        static.write_text('bind = "127.0.0.1"\nport = 7171\n')
        with socket.socket() as sock:
            sock.bind(("127.0.0.1", 0))
            port = sock.getsockname()[1]
        base = f"http://127.0.0.1:{port}"
        env = {"PATH": os.defpath, "HOME": str(root), "ARDA_ROOT": str(root),
               "ARDA_MANWE_STATE_DIR": str(state), "ARDA_MANWE_PROVIDER_CONFIG": str(config),
               "ARDA_FLEET_BOOTSTRAP_STATE": str(root / "absent.json"), "RUST_LOG": "error"}
        with (output / "process.log").open("w") as log:
            process = subprocess.Popen([str(args.binary), "--config", str(static),
                                        "--bind", "127.0.0.1", "--port", str(port)],
                                       env=env, cwd=root, stdout=log, stderr=log)
            try:
                deadline = time.monotonic() + 10
                while True:
                    try:
                        _, providers = request(base, "/providers", timeout=1)
                        break
                    except (OSError, ValueError):
                        if process.poll() is not None or time.monotonic() > deadline:
                            raise RuntimeError("Fixture gateway failed to start")
                        time.sleep(0.05)
                enabled = [p for p in providers["providers"] if p["enabled"]]
                assert {p["id"] for p in enabled} == ({LOCAL, CLOUD} if local_enabled else {CLOUD})
                assert all(p["base_url"] in {local_url, cloud_url} for p in enabled)
                routing = {"force_provider_id": LOCAL, "force_model_id": MODEL,
                           "allow_forced_provider_fallback": allow_fallback,
                           "inference_origin": "auto"}
                body = {"model": MODEL, "stream": False, "max_tokens": 16,
                        "messages": [{"role": "user", "content": MARKER}], "routing": routing}
                privacy = {"privacy_tier": "confidential"} if privacy_alias else {"local_only": local_only}
                if privacy_location == "body":
                    body.update(privacy)
                elif privacy_location == "routing":
                    routing.update(privacy)
                elif privacy_location == "extra_body":
                    body["extra_body"] = privacy
                else:
                    body["extra_body"] = {"routing": privacy}
                start = time.monotonic()
                status, response = request(base, "/v1/chat/completions", body)
                elapsed = round((time.monotonic() - start) * 1000, 3)
                time.sleep(0.25)  # Let the existing async event writer flush its receipt.
                write_json(output / "response.json", response)
                for filename in ["state.jsonl", "tool_fit_ledger.jsonl"]:
                    if (state / filename).exists():
                        shutil.copyfile(state / filename, output / filename)
                ledger_path = output / "tool_fit_ledger.jsonl"
                ledger = [json.loads(line) for line in ledger_path.read_text().splitlines()] if ledger_path.exists() else []
                expected_provider = CLOUD if not local_enabled and allow_fallback and not local_only else LOCAL
                succeeds = mode == "ok" and (local_enabled or (allow_fallback and not local_only))
                expected_counts = (1, 0) if local_enabled else ((0, 1) if succeeds else (0, 0))
                actual = response.get("_manwe_route", {})
                passed = (status == (200 if succeeds else 502)
                          and (len(local_calls), len(cloud_calls)) == expected_counts)
                if succeeds:
                    passed = passed and actual.get("provider_id") == expected_provider and actual.get("model_id") == MODEL
                    passed = passed and response["choices"][0]["message"]["content"] == "R06_OK"
                else:
                    passed = passed and not actual
                expected_class = {"quota": "rate_or_retry_error", "billing": "billing_or_credit_error",
                                  "server": "provider_server_error", "payload": "client_payload_error",
                                  "timeout": "request_timeout"}.get(mode)
                if expected_class:
                    passed = passed and len(ledger) == 1 and ledger[0].get("outcome_class") == expected_class
                if mode == "timeout":
                    passed = passed and 24000 <= elapsed <= 35000
                result = {"case": name, "ok": bool(passed), "http_status": status,
                          "elapsed_ms": elapsed, "upstream_requests": {LOCAL: local_calls, CLOUD: cloud_calls},
                          "routing": routing, "privacy": privacy, "privacy_location": privacy_location,
                          "outcome_classes": [row.get("outcome_class") for row in ledger],
                          "actual_route": actual, "usage": response.get("usage"),
                          "real_model_tokens": 0, "real_provider_spend": 0,
                          "scope": "isolated gateway; synthetic upstream, not canonical work"}
                write_json(output / "result.json", result)
                return result
            finally:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--case", action="append", help="Run only the named case(s)")
    args = parser.parse_args()
    args.binary = args.binary.resolve(strict=True)
    args.output = args.output.resolve()
    args.output.mkdir(parents=True, exist_ok=False)
    cases = [("success", "ok", {}), ("quota", "quota", {}),
             ("billing", "billing", {}), ("server", "server", {}),
             ("payload", "payload", {}), ("timeout", "timeout", {}),
             ("fallback_refused", "ok", {"local_enabled": False}),
             ("fallback_allowed", "ok", {"local_enabled": False, "allow_fallback": True}),
             ("privacy_refused", "ok", {"local_enabled": False, "allow_fallback": True, "local_only": True})]
    for location in ["body", "extra_body", "extra_body.routing"]:
        cases.append(("privacy_" + location.replace(".", "_"), "ok", {
            "local_enabled": False, "allow_fallback": True, "local_only": True,
            "privacy_location": location}))
    cases.append(("privacy_alias", "ok", {"local_enabled": False, "allow_fallback": True,
                                         "local_only": True, "privacy_alias": True}))
    if args.case:
        unknown = set(args.case) - {row[0] for row in cases}
        if unknown:
            parser.error("Unknown case(s): " + ", ".join(sorted(unknown)))
        cases = [row for row in cases if row[0] in args.case]
    results = []
    for name, mode, options in cases:
        result = run_case(args, name, mode, **options)
        results.append(result)
        print(f"{name}: {'PASS' if result['ok'] else 'FAIL'} HTTP {result['http_status']} {result['elapsed_ms']} ms", flush=True)
    summary = {"binary_sha256": hashlib.sha256(args.binary.read_bytes()).hexdigest(),
               "ok": all(row["ok"] for row in results), "cases": results}
    write_json(args.output / "summary.json", summary)
    return 0 if summary["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
