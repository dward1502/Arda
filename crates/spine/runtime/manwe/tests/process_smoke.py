#!/usr/bin/env python3
"""Process-level smoke tests for the canonical static and full adaptive Manwe runtimes."""

from __future__ import annotations

import json
import os
import socket
import subprocess
import tempfile
import threading
import time
import urllib.error
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

CRATE = Path(__file__).resolve().parents[1]
REPO = CRATE.parents[3]
BINARY = REPO / "target" / "debug" / "manwe"
TOKEN = "MANWE_FULL_SMOKE_OK"
CRATE_LOCAL_OUTPUT_ROOTS = (
    CRATE / "core",
    CRATE / "data",
    CRATE / "docs" / "operator" / "library",
)


def output_snapshot() -> dict[str, bytes]:
    return {
        str(path.relative_to(CRATE)): path.read_bytes()
        for root in CRATE_LOCAL_OUTPUT_ROOTS
        if root.exists()
        for path in root.rglob("*")
        if path.is_file()
    }


def free_port() -> int:
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


class ControlledUpstream(BaseHTTPRequestHandler):
    model = "smoke-model"

    def _json(self, payload: dict) -> None:
        body = json.dumps(payload).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self) -> None:  # noqa: N802
        self._json(
            {
                "object": "list",
                "data": [{"id": self.model, "object": "model", "owned_by": "smoke"}],
            }
        )

    def do_POST(self) -> None:  # noqa: N802
        self.rfile.read(int(self.headers.get("content-length", "0")))
        self._json(
            {
                "id": "smoke-completion",
                "object": "chat.completion",
                "model": self.model,
                "choices": [
                    {
                        "index": 0,
                        "message": {"role": "assistant", "content": TOKEN},
                        "finish_reason": "stop",
                    }
                ],
            }
        )

    def log_message(self, format: str, *args: object) -> None:
        _ = (format, args)
        return


def request(port: int, path: str, payload: dict | None = None) -> tuple[int, dict, dict[str, str]]:
    data = None if payload is None else json.dumps(payload).encode()
    req = urllib.request.Request(
        f"http://127.0.0.1:{port}{path}",
        data=data,
        headers={"content-type": "application/json"} if data else {},
    )
    with urllib.request.urlopen(req, timeout=5) as response:
        return response.status, json.load(response), dict(response.headers.items())


def wait_ready(port: int) -> None:
    deadline = time.monotonic() + 15
    while time.monotonic() < deadline:
        try:
            request(port, "/v1/capabilities")
            return
        except (OSError, urllib.error.URLError):
            time.sleep(0.1)
    raise AssertionError(f"Manwe did not become ready on port {port}")


def stop(process: subprocess.Popen[str]) -> None:
    process.terminate()
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=5)


def assert_common_surfaces(port: int, mode: str) -> None:
    status, health, _ = request(port, "/healthz")
    assert status == 200, health
    status, models, _ = request(port, "/v1/models")
    assert status == 200 and models["object"] == "list" and models["data"], models
    status, capabilities, _ = request(port, "/v1/capabilities")
    assert status == 200 and capabilities["mode"] == mode, capabilities


def run_static(root: Path, upstream_port: int) -> None:
    port = free_port()
    root.mkdir(parents=True)
    config = root / "manwe.toml"
    config.write_text(
        "\n".join(
            [
                'bind = "127.0.0.1"',
                f"port = {port}",
                'default_provider = "smoke"',
                "",
                "[providers.smoke]",
                f'base_url = "http://127.0.0.1:{upstream_port}/v1"',
                'models = ["smoke-model"]',
            ]
        )
    )
    malformed_fleet = root / "fleet.toml"
    malformed_fleet.write_text("[[nodes]\n")
    env = os.environ.copy()
    env["ARDA_ROOT"] = str(root)
    env["ARDA_MANWE_FLEET_CONFIG"] = str(malformed_fleet)
    env["ANNUNIMAS_CHARON_FLEET_CONFIG"] = str(root / "ignored-legacy-fleet.toml")
    process = subprocess.Popen(
        [str(BINARY), "--config", str(config), "--bind", "127.0.0.1", "--port", str(port)],
        cwd=CRATE,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    try:
        wait_ready(port)
        assert_common_surfaces(port, "static")
        _, capabilities, _ = request(port, "/v1/capabilities")
        assert capabilities["config_source"] == "file", capabilities
        assert capabilities["config_path"] == str(config), capabilities
        assert capabilities["fleet_config_path"] == str(malformed_fleet), capabilities
        assert capabilities["route_receipts"] == str(
            root / "data" / "manwe" / "route_receipts.jsonl"
        ), capabilities
        assert capabilities["catalog_generation"] == 1, capabilities
        assert capabilities["fleet_providers"] == 0, capabilities
        status, body, _ = request(
            port,
            "/v1/chat/completions",
            {"model": "smoke-model", "messages": [{"role": "user", "content": "smoke"}]},
        )
        assert status == 200 and body["choices"][0]["message"]["content"] == TOKEN, body
    finally:
        stop(process)


def run_static_fallback_matrix(root: Path) -> None:
    cases = {
        "missing": (None, "embedded_missing"),
        "malformed": ("[providers\n", "embedded_malformed"),
        "partial": ('bind = "127.0.0.1"\n', "embedded_empty"),
    }
    for name, (contents, expected_source) in cases.items():
        case_root = root / name
        case_root.mkdir(parents=True)
        config = case_root / "manwe.toml"
        if contents is not None:
            config.write_text(contents)
        fleet = case_root / "fleet.toml"
        if name != "missing":
            fleet.write_text("[[nodes]\n" if name == "malformed" else "")
        port = free_port()
        env = os.environ.copy()
        env["ARDA_ROOT"] = str(case_root)
        if name == "missing":
            env.pop("ARDA_MANWE_FLEET_CONFIG", None)
            env["ANNUNIMAS_CHARON_FLEET_CONFIG"] = str(fleet)
        else:
            env["ARDA_MANWE_FLEET_CONFIG"] = str(fleet)
            env["ANNUNIMAS_CHARON_FLEET_CONFIG"] = str(case_root / "ignored-legacy.toml")
        process = subprocess.Popen(
            [str(BINARY), "--config", str(config), "--bind", "127.0.0.1", "--port", str(port)],
            cwd=CRATE,
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
        try:
            wait_ready(port)
            _, capabilities, _ = request(port, "/v1/capabilities")
            assert capabilities["config_source"] == expected_source, capabilities
            assert capabilities["fleet_config_path"] == str(fleet), capabilities
            assert capabilities["fleet_providers"] == 0, capabilities
            assert capabilities["catalog_generation"] == 1, capabilities
        finally:
            stop(process)


def run_adaptive(root: Path, upstream_port: int) -> None:
    port = free_port()
    config_dir = root / "config"
    config_dir.mkdir(parents=True)
    (config_dir / "manwe.providers.toml").write_text(
        "\n".join(
            [
                "[[provider]]",
                'id = "smoke"',
                'name = "Controlled Smoke Provider"',
                f'base_url = "http://127.0.0.1:{upstream_port}/v1"',
                "enabled = true",
                'access_tier = "local"',
                'quality_band = "high"',
                "requests_per_minute = 60",
                "",
                "  [[provider.model]]",
                '  id = "smoke-model"',
                '  capable_tasks = ["code", "research", "reasoning", "chat", "summary", "background"]',
                "  context_window = 65536",
                "  is_default = true",
                "  capabilities = { tools = true, structured_output = true, streaming = true }",
            ]
        )
    )
    env = os.environ.copy()
    env.pop("ARDA_MANWE_STATE_DIR", None)
    env.pop("ARDA_MANWE_HOME", None)
    env.pop("ARDA_MANWE_PROVIDER_CONFIG", None)
    env.pop("ANNUNIMAS_CHARON_PROVIDER_CONFIG", None)
    env.update({"ARDA_HOME": str(root), "ARDA_ROOT": str(root)})
    process = subprocess.Popen(
        [str(BINARY), "--adaptive", "--bind", "127.0.0.1", "--port", str(port)],
        cwd=CRATE,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    try:
        wait_ready(port)
        assert_common_surfaces(port, "adaptive")
        status, capabilities, _ = request(port, "/v1/capabilities")
        assert capabilities["runtime"] == "full_governed", capabilities
        assert capabilities["governance"] is True and capabilities["quota_mesh"] is True
        assert capabilities["config_source"] == "provider_file", capabilities
        assert capabilities["config_path"] == str(config_dir / "manwe.providers.toml"), capabilities
        assert capabilities["catalog_generation"] == 2, capabilities
        status, body, headers = request(
            port,
            "/v1/chat/completions",
            {"model": "smoke-model", "messages": [{"role": "user", "content": "smoke"}]},
        )
        assert status == 200 and body["choices"][0]["message"]["content"] == TOKEN, body
        assert headers.get("x-manwe-route-id"), headers
        assert headers.get("x-manwe-provider-id") == "smoke", headers
        state_dir = root / "data" / "manwe"
        state = (state_dir / "state.jsonl").read_text()
        governance = (state_dir / "governance_events.jsonl").read_text()
        assert '"event":"route_selected"' in state, state
        assert '"verdict":"selected"' in governance, governance
    finally:
        stop(process)


def main() -> None:
    subprocess.run(["cargo", "build", "-p", "manwe", "--features", "adaptive"], cwd=REPO, check=True)
    before_outputs = output_snapshot()
    upstream_port = free_port()
    server = ThreadingHTTPServer(("127.0.0.1", upstream_port), ControlledUpstream)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        with tempfile.TemporaryDirectory(prefix="manwe-process-smoke-") as tmp:
            root = Path(tmp)
            run_static(root / "static", upstream_port)
            run_static_fallback_matrix(root / "static-fallbacks")
            run_adaptive(root / "adaptive", upstream_port)
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)
    assert output_snapshot() == before_outputs, "Manwe recreated or modified crate-local output"
    print("PASS: Manwe static and full governed adaptive process smoke tests")


if __name__ == "__main__":
    main()
