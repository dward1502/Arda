#!/usr/bin/env python3
"""Run benchmark candidates serially with exactly one model server resident."""

from __future__ import annotations

import argparse
import json
import os
import signal
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


def run(command: list[str], *, check: bool = True, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    print("+", " ".join(command), flush=True)
    return subprocess.run(command, check=check, text=True, capture_output=True, env=env)


def wait_for_health(url: str, process: subprocess.Popen[str], timeout: float) -> float:
    started = time.monotonic()
    last_error = "not attempted"
    while time.monotonic() - started < timeout:
        if process.poll() is not None:
            raise RuntimeError(f"server exited with status {process.returncode}")
        try:
            with urllib.request.urlopen(url.rstrip("/") + "/health", timeout=2) as response:
                body = response.read().decode(errors="replace")
                if response.status == 200 and '"ok"' in body:
                    return time.monotonic() - started
        except (urllib.error.URLError, TimeoutError, ConnectionError) as exc:
            last_error = str(exc)
        time.sleep(0.25)
    raise TimeoutError(f"health endpoint did not become ready in {timeout}s: {last_error}")


def stop_process(process: subprocess.Popen[str] | None) -> None:
    if process is None or process.poll() is not None:
        return
    try:
        os.killpg(process.pid, signal.SIGTERM)
        process.wait(timeout=30)
    except (ProcessLookupError, subprocess.TimeoutExpired):
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.wait(timeout=10)


def wait_for_port_available(host: str, port: int, timeout: float = 90) -> None:
    started = time.monotonic()
    last_error = "not attempted"
    while time.monotonic() - started < timeout:
        try:
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
                probe.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
                probe.bind((host, port))
            return
        except OSError as exc:
            last_error = str(exc)
            time.sleep(0.25)
    raise TimeoutError(f"port {host}:{port} was not released in {timeout}s: {last_error}")


def stop_services(systemctl: list[str], units: list[str]) -> None:
    for unit in units:
        result = run(systemctl + ["stop", unit], check=False)
        if result.returncode and "not loaded" not in result.stderr and "not found" not in result.stderr:
            raise RuntimeError(f"failed to stop {unit}: {result.stderr.strip()}")
    time.sleep(2)


def live_llama_processes() -> list[str]:
    result = subprocess.run(["ps", "-eo", "pid=,comm=,args="], text=True, capture_output=True, check=True)
    return [line.strip() for line in result.stdout.splitlines() if "llama-server" in line and "run_model_shootout" not in line]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--only", action="append", default=[], help="Run only named candidate; repeatable")
    args = parser.parse_args()
    config: dict[str, Any] = json.loads(args.config.read_text())

    receipt_dir = Path(config["receipt_dir"]).expanduser()
    receipt_dir.mkdir(parents=True, exist_ok=True)
    benchmark_script = Path(config["benchmark_script"]).expanduser()
    if not benchmark_script.is_file():
        raise FileNotFoundError(benchmark_script)

    candidates = config["candidates"]
    if args.only:
        selected = set(args.only)
        candidates = [candidate for candidate in candidates if candidate["name"] in selected]
        missing = selected - {candidate["name"] for candidate in candidates}
        if missing:
            raise ValueError(f"unknown candidates: {sorted(missing)}")
    for candidate in candidates:
        for key in ("model_path", "binary"):
            path = Path(candidate[key]).expanduser()
            if not path.is_file():
                raise FileNotFoundError(f"{candidate['name']} {key}: {path}")
            candidate[key] = str(path)

    systemctl = config.get("systemctl", ["systemctl", "--user"])
    stop_services(systemctl, config.get("service_units", []))
    leftovers = live_llama_processes()
    if leftovers:
        raise RuntimeError("isolation check failed; llama-server remains resident:\n" + "\n".join(leftovers))

    environment = os.environ.copy()
    environment.update(config.get("environment", {}))
    summary: dict[str, Any] = {
        "schema_version": 1,
        "host": config["host_label"],
        "started_at": datetime.now(timezone.utc).isoformat(),
        "config": str(args.config),
        "results": [],
    }
    summary_path = receipt_dir / "shootout-summary.json"

    for candidate in candidates:
        process: subprocess.Popen[str] | None = None
        name = candidate["name"]
        safe_name = re_safe(name)
        receipt = receipt_dir / f"{safe_name}.json"
        server_log = receipt_dir / f"{safe_name}.server.log"
        print(f"\n=== {config['host_label']}: {name} ===", flush=True)
        command = [
            candidate["binary"],
            "--model", candidate["model_path"],
            "--host", "127.0.0.1",
            "--port", str(config["port"]),
            "--ctx-size", str(config["context_size"]),
            "-ngl", str(candidate.get("gpu_layers", config.get("gpu_layers", 999))),
            "--no-mmap",
            "--flash-attn", "on",
            "--metrics",
            "-a", candidate["model_id"],
            *config.get("server_args", []),
            *candidate.get("server_args", []),
        ]
        result: dict[str, Any] = {"name": name, "model_id": candidate["model_id"], "receipt": str(receipt), "server_log": str(server_log)}
        try:
            wait_for_port_available("127.0.0.1", config["port"])
            with server_log.open("w") as log:
                launched = time.monotonic()
                process = subprocess.Popen(
                    command,
                    stdout=log,
                    stderr=subprocess.STDOUT,
                    text=True,
                    env=environment,
                    start_new_session=True,
                )
                load_seconds = wait_for_health(f"http://127.0.0.1:{config['port']}", process, config.get("load_timeout", 300))
                result["load_seconds"] = load_seconds
                result["pid"] = process.pid
                benchmark = [
                    sys.executable,
                    str(benchmark_script),
                    "--url", f"http://127.0.0.1:{config['port']}",
                    "--model", candidate["model_id"],
                    "--host-label", config["host_label"],
                    "--output", str(receipt),
                    "--pid", str(process.pid),
                    "--load-seconds", str(load_seconds),
                    "--contexts", ",".join(str(value) for value in config["performance_contexts"]),
                    "--long-context", str(config["long_context"]),
                    "--repetitions", str(config.get("repetitions", 3)),
                    "--long-context-repetitions", str(config.get("long_context_repetitions", 1)),
                    "--long-context-max-seconds", str(config.get("long_context_max_seconds", 300)),
                    "--timeout", str(config.get("request_timeout", 900)),
                    "--generation-gate", str(config["gates"]["generation_tps"]),
                    "--prompt-8k-gate", str(config["gates"]["prompt_8k_tps"]),
                    "--tool-gate", str(config["gates"]["tool_validity"]),
                    "--correctness-gate", str(config["gates"]["task_correctness"]),
                    "--memory-margin-gib", str(config["gates"]["memory_margin_gib"]),
                ]
                bench_result = run(benchmark, check=False, env=environment)
                result["benchmark_exit_code"] = bench_result.returncode
                result["benchmark_stdout"] = bench_result.stdout[-5000:]
                result["benchmark_stderr"] = bench_result.stderr[-5000:]
                if bench_result.returncode != 0:
                    raise RuntimeError(f"benchmark exited {bench_result.returncode}: {bench_result.stderr[-2000:]}")
                parsed = json.loads(receipt.read_text())
                result["summary"] = parsed["summary"]
                result["wall_seconds"] = time.monotonic() - launched
                result["status"] = "completed"
        except Exception as exc:
            result["status"] = "failed"
            result["error"] = f"{type(exc).__name__}: {exc}"
            print(result["error"], file=sys.stderr, flush=True)
        finally:
            stop_process(process)
            try:
                wait_for_port_available("127.0.0.1", config["port"])
            except TimeoutError as exc:
                result["isolation_cleanup_error"] = str(exc)
            leftovers = live_llama_processes()
            if leftovers:
                result.setdefault("isolation_cleanup_error", leftovers)
            summary["results"].append(result)
            summary_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")

    summary["finished_at"] = datetime.now(timezone.utc).isoformat()
    summary_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"summary": str(summary_path), "results": [{"name": row["name"], "status": row["status"], "score": row.get("summary", {}).get("weighted_score")} for row in summary["results"]]}, indent=2))
    return 0 if all(row["status"] == "completed" for row in summary["results"]) else 1


def re_safe(value: str) -> str:
    return "".join(character.lower() if character.isalnum() else "-" for character in value).strip("-")


if __name__ == "__main__":
    raise SystemExit(main())
