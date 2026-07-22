#!/usr/bin/env python3
"""Bounded end-to-end production smoke test for Manwe and Hermes delegation."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
import uuid
from pathlib import Path
from typing import Any, Callable

BASE_URL = "http://127.0.0.1:5110"
PORT = 5110
PROVIDER = "edge_core"
MODEL_ID = "LFM2.5-8B-A1B-Q4_K_M"
PINNED_MODEL = f"{PROVIDER}/{MODEL_ID}"
UNAVAILABLE_ROUTE = "smoke_unavailable/LFM2.5-8B-A1B-Q4_K_M"
REPO_ROOT = Path(__file__).resolve().parents[1]
EXPECTED_EXECUTABLE = (REPO_ROOT / "target/release/manwe").resolve()
DELEGATION_ROOT = Path.home() / ".hermes/cache/delegation/live"


class SmokeFailure(RuntimeError):
    """A production smoke-test assertion failed."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SmokeFailure(message)


def pass_check(message: str) -> None:
    print(f"PASS  {message}", flush=True)


def run_command(command: list[str], timeout: float, cwd: Path = REPO_ROOT) -> subprocess.CompletedProcess[str]:
    try:
        return subprocess.run(
            command,
            cwd=cwd,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
            check=False,
        )
    except subprocess.TimeoutExpired as exc:
        raise SmokeFailure(f"command timed out after {timeout:.0f}s: {' '.join(command)}") from exc


def http_json(
    method: str,
    path: str,
    *,
    payload: dict[str, Any] | None = None,
    timeout: float,
) -> tuple[int, Any, dict[str, str]]:
    data = None if payload is None else json.dumps(payload).encode("utf-8")
    request = urllib.request.Request(
        f"{BASE_URL}{path}",
        data=data,
        method=method,
        headers={"Content-Type": "application/json"},
    )
    try:
        response = urllib.request.urlopen(request, timeout=timeout)
    except urllib.error.HTTPError as error:
        response = error
    except (urllib.error.URLError, TimeoutError) as error:
        raise SmokeFailure(f"{method} {path} failed: {error}") from error

    with response:
        body_bytes = response.read()
        status = int(response.status or 0)
        headers = {str(key).lower(): str(value) for key, value in response.headers.items()}
    try:
        body = json.loads(body_bytes)
    except json.JSONDecodeError as error:
        raise SmokeFailure(f"{method} {path} returned non-JSON body: {body_bytes[:200]!r}") from error
    return status, body, headers


def check_listener() -> None:
    result = run_command(["ss", "-H", "-ltnp", "sport", "=", f":{PORT}"], timeout=5)
    require(result.returncode == 0, f"ss failed: {result.stderr.strip()}")
    pids = set(re.findall(r"pid=(\d+)", result.stdout))
    require(bool(pids), f"no process owns TCP port {PORT}")
    require(len(pids) == 1, f"TCP port {PORT} has multiple listener PIDs: {sorted(pids)}")
    pid = next(iter(pids))
    try:
        executable = Path(f"/proc/{pid}/exe").resolve(strict=True)
    except OSError as error:
        raise SmokeFailure(f"cannot resolve executable for port {PORT} PID {pid}: {error}") from error
    require(
        executable == EXPECTED_EXECUTABLE,
        f"port {PORT} listener is {executable}, expected {EXPECTED_EXECUTABLE}",
    )
    pass_check(f"port {PORT} listener PID {pid} executable is {EXPECTED_EXECUTABLE}")


def check_health_and_models() -> None:
    status, health, _ = http_json("GET", "/healthz", timeout=5)
    require(status == 200, f"/healthz returned HTTP {status}: {health}")
    require(health.get("runtime") == "arda-manwe", f"/healthz runtime is {health.get('runtime')!r}")
    require(health.get("port") == PORT, f"/healthz port is {health.get('port')!r}")
    pass_check("/healthz identifies runtime=arda-manwe on port 5110")

    status, models, _ = http_json("GET", "/v1/models", timeout=5)
    require(status == 200, f"/v1/models returned HTTP {status}: {models}")
    entries = models.get("data") if isinstance(models, dict) else None
    if not isinstance(entries, list):
        raise SmokeFailure("/v1/models response has no data list")
    advertised = any(
        isinstance(entry, dict)
        and entry.get("provider") == PROVIDER
        and entry.get("id") == MODEL_ID
        for entry in entries
    )
    require(advertised, f"/v1/models does not advertise {PINNED_MODEL}")
    pass_check(f"/v1/models advertises {PINNED_MODEL}")


def check_explicit_inference(inference_timeout: float) -> None:
    payload = {
        "model": PINNED_MODEL,
        "messages": [{"role": "user", "content": "Reply with exactly MANWE_SMOKE_OK."}],
        "max_tokens": 16,
        "temperature": 0,
    }
    status, body, headers = http_json(
        "POST", "/v1/chat/completions", payload=payload, timeout=inference_timeout
    )
    require(status == 200, f"explicit inference returned HTTP {status}: {body}")
    require(
        headers.get("x-manwe-provider") == PROVIDER,
        f"explicit inference x-manwe-provider is {headers.get('x-manwe-provider')!r}",
    )
    require(
        headers.get("x-manwe-model") == MODEL_ID,
        f"explicit inference x-manwe-model is {headers.get('x-manwe-model')!r}",
    )
    require(isinstance(body, dict) and isinstance(body.get("choices"), list), "inference body has no choices list")
    pass_check(f"explicit inference returned x-manwe-provider={PROVIDER} and x-manwe-model={MODEL_ID}")


def check_fail_closed() -> None:
    payload = {
        "model": UNAVAILABLE_ROUTE,
        "messages": [{"role": "user", "content": "This request must not fall through."}],
        "max_tokens": 8,
    }
    status, body, headers = http_json("POST", "/v1/chat/completions", payload=payload, timeout=10)
    error = body.get("error", {}) if isinstance(body, dict) else {}
    require(status == 503, f"unavailable explicit route returned HTTP {status}, expected 503: {body}")
    require(error.get("code") == "no_compatible_model", f"unexpected fail-closed code: {error}")
    require(error.get("runtime") == "arda-manwe", f"unexpected fail-closed runtime: {error}")
    require(error.get("requested_model") == UNAVAILABLE_ROUTE, f"requested route not preserved: {error}")
    require("x-manwe-provider" not in headers, "unavailable explicit route fell through to a provider")
    pass_check(f"unavailable explicit route {UNAVAILABLE_ROUTE} failed closed with HTTP 503")


def delegation_dirs() -> set[Path]:
    if not DELEGATION_ROOT.is_dir():
        return set()
    return {path for path in DELEGATION_ROOT.glob("deleg_*") if path.is_dir()}


def find_delegation(marker: str, before: set[Path], timeout: float) -> tuple[Path, dict[str, Any], str]:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        for directory in delegation_dirs() - before:
            manifest_path = directory / "manifest.json"
            try:
                manifest_text = manifest_path.read_text(encoding="utf-8")
                manifest = json.loads(manifest_text)
            except (OSError, json.JSONDecodeError):
                continue
            if marker not in manifest_text:
                continue
            tasks = manifest.get("tasks")
            if not isinstance(tasks, list) or len(tasks) != 1:
                raise SmokeFailure(f"delegation {directory.name} did not contain exactly one task")
            log_path = Path(tasks[0].get("log", ""))
            try:
                log_text = log_path.read_text(encoding="utf-8")
            except OSError:
                time.sleep(0.2)
                continue
            if tasks[0].get("status") == "completed" and "end status=completed" in log_text:
                return directory, manifest, log_text
        time.sleep(0.2)
    raise SmokeFailure(f"no completed delegation transcript found for marker {marker} within {timeout:.0f}s")


def transcript_events(log_text: str, event: str) -> list[str]:
    events: list[str] = []
    for line in log_text.splitlines():
        match = re.match(r"^\d{2}:\d{2}:\d{2}\s+([a-z]+)\s*\|\s?(.*)$", line)
        if match and match.group(1) == event:
            events.append(match.group(2))
    return events


def transcript_tool_names(log_text: str) -> list[str]:
    names: list[str] = []
    for payload in transcript_events(log_text, "tool"):
        match = re.match(r"-> ([A-Za-z0-9_]+)\(", payload)
        if match:
            names.append(match.group(1))
    return names


def run_delegation(
    label: str,
    goal: str,
    marker: str,
    timeout: float,
    validate: Callable[[str], None],
) -> None:
    before = delegation_dirs()
    parent_prompt = (
        "This is a production smoke gate. Use delegate_task exactly once with one leaf child. "
        "Pass the child this goal verbatim:\n"
        f"{goal}\n"
        "Wait for the child to complete, then return its result. Do not perform the task yourself."
    )
    result = run_command(["hermes", "--oneshot", parent_prompt], timeout=timeout)
    require(
        result.returncode == 0,
        f"{label} parent Hermes process failed ({result.returncode}): {result.stderr.strip() or result.stdout.strip()}",
    )
    directory, _, log_text = find_delegation(marker, before, timeout=min(10.0, timeout))
    validate(log_text)
    pass_check(f"{label} succeeded ({directory.name})")


def check_delegations(delegate_timeout: float) -> None:
    token = uuid.uuid4().hex
    marker = f"MANWE_SINGLE_READ_{token}"
    with tempfile.TemporaryDirectory(prefix=".manwe-smoke-", dir=REPO_ROOT) as temp_dir:
        fixture = Path(temp_dir) / "single-read.txt"
        fixture.write_text(f"{marker}\n", encoding="utf-8")
        goal = (
            f"Production smoke marker {marker}. Use the native read_file tool to read {fixture}. "
            "Return the marker verbatim. Do not use any other tool."
        )

        def validate_single(log_text: str) -> None:
            tools = transcript_tool_names(log_text)
            require(tools == ["read_file"], f"single-read child tool sequence was {tools}, expected ['read_file']")
            read_results = [
                payload for payload in transcript_events(log_text, "result") if payload.startswith("read_file ok ")
            ]
            require(
                any(marker in payload for payload in read_results),
                "single-read marker missing from native read_file result",
            )

        run_delegation("delegated single-tool read", goal, marker, delegate_timeout, validate_single)

    marker = f"MANWE_SEQUENTIAL_{uuid.uuid4().hex}"
    goal = (
        f"Production smoke marker {marker}. In the repository {REPO_ROOT}, first capture the verbatim "
        "stdout from the native terminal command `pwd`; then use native read_file to quote the exact first "
        "top-level Markdown heading from AGENTS.md. Return the marker and both results verbatim."
    )

    def validate_sequential(log_text: str) -> None:
        tools = transcript_tool_names(log_text)
        require("terminal" in tools, f"sequential child did not call terminal: {tools}")
        require("read_file" in tools, f"sequential child did not call read_file: {tools}")
        require(
            tools.index("terminal") < tools.index("read_file"),
            f"sequential child tool order was {tools}, expected terminal before read_file",
        )
        results = transcript_events(log_text, "result")
        terminal_results = [payload for payload in results if payload.startswith("terminal ok ")]
        read_results = [payload for payload in results if payload.startswith("read_file ok ")]
        require(
            any(f'"output": "{REPO_ROOT}"' in payload for payload in terminal_results),
            "repository path missing from native terminal result",
        )
        require(
            any("# Arda AGENTS.md" in payload for payload in read_results),
            "AGENTS.md heading missing from native read_file result",
        )
        final_results = transcript_events(log_text, "final")
        require(
            any(str(REPO_ROOT) in payload for payload in final_results),
            "repository path missing from completed child result",
        )
        require(
            any("# Arda AGENTS.md" in payload for payload in final_results),
            "AGENTS.md heading missing from completed child result",
        )

    run_delegation(
        "delegated sequential terminal-plus-file task",
        goal,
        marker,
        delegate_timeout,
        validate_sequential,
    )


def self_test() -> None:
    sample = (
        "12:00:00 tool     | -> terminal(pwd)\n"
        "12:00:01 result   | terminal ok\n"
        "12:00:02 tool     | -> read_file(AGENTS.md)\n"
    )
    require(
        transcript_events(sample, "result") == ["terminal ok"],
        "transcript event parser self-test failed",
    )
    require(transcript_tool_names(sample) == ["terminal", "read_file"], "transcript parser self-test failed")
    try:
        require(False, "expected self-test failure")
    except SmokeFailure as error:
        require(str(error) == "expected self-test failure", "failure assertion lost its diagnostic")
    else:
        raise SmokeFailure("failed assertions do not stop the smoke gate")
    print("PASS  smoke-test helper self-test", flush=True)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inference-timeout", type=float, default=120.0)
    parser.add_argument("--delegate-timeout", type=float, default=240.0)
    parser.add_argument("--self-test", action="store_true", help="test local parsing helpers without production probes")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        if args.self_test:
            self_test()
            return 0
        require(args.inference_timeout > 0, "--inference-timeout must be positive")
        require(args.delegate_timeout > 0, "--delegate-timeout must be positive")
        check_listener()
        check_health_and_models()
        check_explicit_inference(args.inference_timeout)
        check_fail_closed()
        check_delegations(args.delegate_timeout)
    except SmokeFailure as error:
        print(f"FAIL  {error}", file=sys.stderr, flush=True)
        return 1
    print("PASS  production Manwe/Hermes smoke gate completed", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
