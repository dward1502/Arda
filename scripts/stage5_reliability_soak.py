#!/usr/bin/env python3
"""Bounded Stage 5 reliability soak for deterministic Workbench fault fixtures."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import time
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

CONTRACT = "arda.stage5.reliability-soak.v1"
MAX_FAILURES = 20
SCENARIOS = (
    ("repeated-cancellation", ("cargo", "test", "-p", "arda-engine", "--test", "harness_runs", "cancel_is_idempotent_and_mutations_require_typed_envelopes", "--", "--exact")),
    ("adapter-crash-hang", ("cargo", "test", "-p", "arda-engine", "--test", "project_adapter_jsonl", "adapter_cancellation_terminates_and_reaps_process", "--", "--exact")),
    ("model-timeout", ("cargo", "test", "-p", "arda-engine", "--test", "project_adapter_jsonl", "adapter_timeout_terminates_and_reaps_process", "--", "--exact")),
    ("large-noisy-output", ("cargo", "test", "-p", "arda-engine", "--test", "project_adapter_jsonl", "adapter_rejects_oversized_noisy_output_without_unbounded_buffering", "--", "--exact")),
    ("network-loss", ("cargo", "test", "-p", "arda-engine", "harness::tests::models_proxy_reports_network_loss_without_false_completion", "--", "--exact")),
    ("disk-pressure", ("cargo", "test", "-p", "arda-engine", "--test", "run_recovery", "disk_pressure_write_failure_does_not_publish_partial_result", "--", "--exact")),
    ("corrupted-tail", ("cargo", "test", "-p", "arda-engine", "--test", "run_recovery", "corrupt_or_truncated_journal_tail_fails_visibly", "--", "--exact")),
    ("checkpoint-restart", ("cargo", "test", "-p", "arda-engine", "--test", "workbench_boundary_recovery", "restart_at_every_graph_boundary_preserves_exact_once_mutation", "--", "--exact")),
)


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def percentile(values: list[float], fraction: float) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, int((len(ordered) - 1) * fraction))]


def tree_measure(path: Path) -> dict[str, int]:
    files = total = 0
    if not path.exists():
        return {"files": 0, "bytes": 0}
    for candidate in path.rglob("*"):
        try:
            if candidate.is_file() and not candidate.is_symlink():
                files += 1
                total += candidate.stat().st_size
        except FileNotFoundError:
            continue
    return {"files": files, "bytes": total}


@dataclass
class ScenarioState:
    runs: int = 0
    passed: int = 0
    durations_ms: list[float] = field(default_factory=list)


def render_report(*, started_at: str, finished_at: str, requested_seconds: float,
                  elapsed_seconds: float, states: dict[str, ScenarioState],
                  failures: list[dict[str, Any]], protected_before: dict[str, int],
                  protected_after: dict[str, int], max_growth_files: int = 0,
                  max_growth_bytes: int = 0) -> dict[str, Any]:
    scenarios = {}
    for name, state in states.items():
        durations = state.durations_ms
        scenarios[name] = {
            "runs": state.runs,
            "passed": state.passed,
            "failed": state.runs - state.passed,
            "latency_ms": {
                "min": min(durations) if durations else None,
                "p50": percentile(durations, 0.50),
                "p95": percentile(durations, 0.95),
                "max": max(durations) if durations else None,
            },
        }
    total_runs = sum(state.runs for state in states.values())
    total_failed = sum(state.runs - state.passed for state in states.values())
    growth = {key: protected_after[key] - protected_before[key] for key in protected_before}
    return {
        "contract": CONTRACT,
        "status": "pass" if (
            total_runs > 0
            and total_failed == 0
            and growth["files"] <= max_growth_files
            and growth["bytes"] <= max_growth_bytes
        ) else "fail",
        "started_at_utc": started_at,
        "finished_at_utc": finished_at,
        "requested_duration_seconds": requested_seconds,
        "elapsed_seconds": elapsed_seconds,
        "summary": {"runs": total_runs, "passed": total_runs - total_failed, "failed": total_failed},
        "scenarios": scenarios,
        "protected_state": {
            "before": protected_before,
            "after": protected_after,
            "growth": growth,
            "maximum_growth": {"files": max_growth_files, "bytes": max_growth_bytes},
        },
        "failures": failures[-MAX_FAILURES:],
        "limits": {"retained_failures": MAX_FAILURES, "subprocess_output": "sha256-only"},
    }


def run_soak(root: Path, duration_seconds: float, interval_seconds: float,
             command_timeout_seconds: float, protected_path: Path,
             max_growth_files: int = 0, max_growth_bytes: int = 0) -> dict[str, Any]:
    started_at = utc_now()
    started = time.monotonic()
    deadline = started + duration_seconds
    states = {name: ScenarioState() for name, _ in SCENARIOS}
    failures: list[dict[str, Any]] = []
    before = tree_measure(protected_path)
    cycle = 0
    while cycle == 0 or time.monotonic() < deadline:
        name, command = SCENARIOS[cycle % len(SCENARIOS)]
        state = states[name]
        command_started = time.monotonic()
        try:
            completed = subprocess.run(command, cwd=root, capture_output=True, timeout=command_timeout_seconds, check=False)
            output = completed.stdout + completed.stderr
            returncode = completed.returncode
        except subprocess.TimeoutExpired as error:
            output = (error.stdout or b"") + (error.stderr or b"")
            returncode = 124
        elapsed_ms = round((time.monotonic() - command_started) * 1000, 3)
        state.runs += 1
        state.durations_ms.append(elapsed_ms)
        if returncode == 0:
            state.passed += 1
        else:
            failures.append({"scenario": name, "returncode": returncode, "output_sha256": hashlib.sha256(output).hexdigest(), "at_utc": utc_now()})
            failures[:] = failures[-MAX_FAILURES:]
        cycle += 1
        remaining = deadline - time.monotonic()
        if remaining > 0 and interval_seconds > 0:
            time.sleep(min(interval_seconds, remaining))
    elapsed = time.monotonic() - started
    return render_report(started_at=started_at, finished_at=utc_now(), requested_seconds=duration_seconds,
                         elapsed_seconds=elapsed, states=states, failures=failures,
                         protected_before=before, protected_after=tree_measure(protected_path),
                         max_growth_files=max_growth_files, max_growth_bytes=max_growth_bytes)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--duration-seconds", type=float, required=True)
    parser.add_argument("--interval-seconds", type=float, default=30.0)
    parser.add_argument("--command-timeout-seconds", type=float, default=180.0)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--protected-path", type=Path)
    parser.add_argument("--max-protected-growth-files", type=int, default=0)
    parser.add_argument("--max-protected-growth-bytes", type=int, default=0)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    protected = args.protected_path or root / "core" / "state"
    report = run_soak(
        root,
        args.duration_seconds,
        args.interval_seconds,
        args.command_timeout_seconds,
        protected,
        args.max_protected_growth_files,
        args.max_protected_growth_bytes,
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    temporary = args.output.with_suffix(args.output.suffix + ".tmp")
    temporary.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    os.replace(temporary, args.output)
    print(json.dumps({"status": report["status"], **report["summary"]}, sort_keys=True))
    return 0 if report["status"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
