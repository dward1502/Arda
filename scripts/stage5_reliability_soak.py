#!/usr/bin/env python3
"""Bounded Stage 5 reliability soak for deterministic Workbench fault fixtures."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import time
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

CONTRACT = "arda.stage5.reliability-soak.v1"
MAX_FAILURES = 20
DEFAULT_DIAGNOSTIC_BYTES = 4096
DEFAULT_MINIMUM_FREE_BYTES = 64 * 1024 * 1024 * 1024
DEFAULT_SCENARIO_LATENCY_BUDGET_MS = 180_000
SOURCE_INPUTS = tuple(Path(path) for path in (
    "Cargo.toml", "Cargo.lock", ".cargo", "src", "crates", "outposts", "apps",
    "sdk", "adapters", "config", "tests", "scripts/stage5_reliability_soak.py",
))
SOURCE_EXCLUDED_DIRS = {".git", "target", "node_modules", "dist", ".vite", "__pycache__"}
SOURCE_EXCLUDED_SUFFIXES = {".key", ".pem"}
SCENARIOS = (
    ("repeated-cancellation", ("cargo", "test", "-p", "arda-engine", "--test", "harness_runs", "cancel_is_idempotent_and_mutations_require_typed_envelopes", "--", "--exact")),
    ("operator-rejection", ("cargo", "test", "-p", "arda-engine", "--test", "harness_runs", "operator_rejection_is_durable_and_cannot_authorize_execution", "--", "--exact")),
    ("provider-loss", ("cargo", "test", "-p", "arda-engine", "--test", "hermes_adapter_contract", "missing_provider_executable_is_a_typed_spawn_failure", "--", "--exact")),
    ("process-kill", ("cargo", "test", "-p", "arda-engine", "supervisor::tests::killed_child_is_reaped_and_restarted_with_visible_attribution", "--", "--exact")),
    ("adapter-crash", ("cargo", "test", "-p", "arda-engine", "--test", "project_adapter_jsonl", "adapter_crash_is_a_typed_failure_without_false_completion", "--", "--exact")),
    ("model-timeout", ("cargo", "test", "-p", "arda-engine", "--test", "hermes_adapter_contract", "graph_node_timeout_terminates_and_reaps_hermes", "--", "--exact")),
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


def bounded_diagnostic(output: bytes, *, root: Path, max_bytes: int) -> dict[str, Any]:
    captured = output[-max(0, max_bytes):] if max_bytes else b""
    text = captured.decode("utf-8", errors="replace")
    replacements = (
        (str(root.resolve()), "$ARDA_ROOT"),
        (str(root), "$ARDA_ROOT"),
        (str(Path.home()), "$HOME"),
    )
    for original, replacement in replacements:
        text = text.replace(original, replacement)
    text = re.sub(
        r"(?i)(authorization\s*:\s*bearer\s+)[^\s]+",
        r"\1[REDACTED]",
        text,
    )
    text = re.sub(
        r"(?i)\b(api[_-]?key|token|password|secret|prompt)\s*([:=])\s*[^\s]+",
        r"\1\2[REDACTED]",
        text,
    )
    return {
        "tail": text,
        "captured_bytes": len(captured),
        "total_bytes": len(output),
        "truncated": len(captured) < len(output),
    }


def classify_failure(returncode: int, output: bytes) -> str:
    text = output[-16_384:].decode("utf-8", errors="replace").lower()
    if returncode == 124 or "timed out" in text or "timeout" in text:
        return "timeout"
    if returncode == 125:
        return "test_not_exercised"
    if returncode < 0 or "signal:" in text or "killed" in text:
        return "process_signal"
    if "no space left on device" in text or "disk quota exceeded" in text:
        return "resource_exhaustion"
    if "could not compile" in text or "failed to compile" in text:
        return "compile_failure"
    if "test result: failed" in text or "failures:" in text:
        return "test_failure"
    if "failed to spawn" in text or "no such file or directory" in text:
        return "dependency_unavailable"
    return "unknown_failure"


def source_fingerprint(root: Path, *, inputs: tuple[Path, ...] = SOURCE_INPUTS) -> dict[str, Any]:
    digest = hashlib.sha256()
    files = total = 0
    candidates: set[Path] = set()
    for relative in inputs:
        candidate = root / relative
        if candidate.is_file() or candidate.is_symlink():
            candidates.add(candidate)
        elif candidate.is_dir():
            for nested in candidate.rglob("*"):
                if any(part in SOURCE_EXCLUDED_DIRS for part in nested.relative_to(root).parts):
                    continue
                if nested.name == ".env" or nested.name.startswith(".env.") or nested.suffix in SOURCE_EXCLUDED_SUFFIXES:
                    continue
                if nested.is_file() or nested.is_symlink():
                    candidates.add(nested)
    for candidate in sorted(candidates, key=lambda path: path.relative_to(root).as_posix()):
        relative = candidate.relative_to(root).as_posix().encode("utf-8")
        digest.update(relative + b"\0")
        if candidate.is_symlink():
            content = os.readlink(candidate).encode("utf-8")
        else:
            content = candidate.read_bytes()
        digest.update(content + b"\0")
        files += 1
        total += len(content)
    return {"sha256": digest.hexdigest(), "files": files, "bytes": total}


def soak_environment(cargo_target_dir: Path, *, base: dict[str, str] | None = None) -> dict[str, str]:
    environment = dict(os.environ if base is None else base)
    environment["CARGO_TARGET_DIR"] = str(cargo_target_dir.resolve())
    return environment


@dataclass
class ScenarioState:
    runs: int = 0
    passed: int = 0
    durations_ms: list[float] = field(default_factory=list)


def render_report(*, started_at: str, finished_at: str, requested_seconds: float,
                  elapsed_seconds: float, states: dict[str, ScenarioState],
                  failures: list[dict[str, Any]], protected_before: dict[str, int],
                  protected_after: dict[str, int], max_growth_files: int = 0,
                  max_growth_bytes: int = 0, source_before: dict[str, Any] | None = None,
                  source_after: dict[str, Any] | None = None,
                  storage: dict[str, int] | None = None,
                  invalid_reason: str | None = None,
                  scenario_latency_budget_ms: int = DEFAULT_SCENARIO_LATENCY_BUDGET_MS,
                  diagnostic_max_bytes: int = DEFAULT_DIAGNOSTIC_BYTES) -> dict[str, Any]:
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
            "latency_budget_ms": scenario_latency_budget_ms,
            "latency_budget_preserved": bool(durations) and max(durations) <= scenario_latency_budget_ms,
        }
    total_runs = sum(state.runs for state in states.values())
    total_failed = sum(state.runs - state.passed for state in states.values())
    growth = {key: protected_after[key] - protected_before[key] for key in protected_before}
    source_before = source_before or {}
    source_after = source_after or source_before
    source_unchanged = source_before == source_after
    storage = storage or {"minimum_free_bytes": 0, "minimum_observed_free_bytes": 0}
    storage = {**storage, "floor_preserved": storage["minimum_observed_free_bytes"] >= storage["minimum_free_bytes"]}
    return {
        "contract": CONTRACT,
        "status": "pass" if (
            total_runs > 0
            and total_failed == 0
            and growth["files"] <= max_growth_files
            and growth["bytes"] <= max_growth_bytes
            and source_unchanged
            and storage["floor_preserved"]
            and all(scenario["latency_budget_preserved"] for scenario in scenarios.values())
            and invalid_reason is None
        ) else "fail",
        "validity": "valid" if invalid_reason is None else "invalid",
        "invalid_reason": invalid_reason,
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
        "source_integrity": {"before": source_before, "after": source_after, "unchanged": source_unchanged},
        "storage": storage,
        "failures": failures[-MAX_FAILURES:],
        "limits": {
            "retained_failures": MAX_FAILURES,
            "diagnostic_bundle_bytes": diagnostic_max_bytes,
            "subprocess_output": "bounded-redacted-tail",
        },
    }


def run_soak(root: Path, duration_seconds: float, interval_seconds: float,
             command_timeout_seconds: float, protected_path: Path,
             max_growth_files: int = 0, max_growth_bytes: int = 0,
             cargo_target_dir: Path | None = None, minimum_free_bytes: int = DEFAULT_MINIMUM_FREE_BYTES,
             diagnostic_max_bytes: int = DEFAULT_DIAGNOSTIC_BYTES,
             integrity_check_interval_seconds: float = 300.0) -> dict[str, Any]:
    started_at = utc_now()
    started = time.monotonic()
    deadline = started + duration_seconds
    states = {name: ScenarioState() for name, _ in SCENARIOS}
    failures: list[dict[str, Any]] = []
    before = tree_measure(protected_path)
    source_before = source_fingerprint(root)
    source_after = source_before
    free_bytes = shutil.disk_usage(root).free
    minimum_observed_free_bytes = free_bytes
    invalid_reason = "insufficient_disk_headroom" if free_bytes < minimum_free_bytes else None
    environment = soak_environment(cargo_target_dir or root / "target" / "stage5-reliability-soak")
    next_integrity_check = started + max(0.0, integrity_check_interval_seconds)
    cycle = 0
    while invalid_reason is None and (cycle < len(SCENARIOS) or time.monotonic() < deadline):
        now = time.monotonic()
        free_bytes = shutil.disk_usage(root).free
        minimum_observed_free_bytes = min(minimum_observed_free_bytes, free_bytes)
        if free_bytes < minimum_free_bytes:
            invalid_reason = "disk_floor_breached"
            break
        if now >= next_integrity_check:
            source_after = source_fingerprint(root)
            if source_after != source_before:
                invalid_reason = "source_changed"
                break
            next_integrity_check = now + max(0.0, integrity_check_interval_seconds)
        name, command = SCENARIOS[cycle % len(SCENARIOS)]
        state = states[name]
        command_started = time.monotonic()
        try:
            completed = subprocess.run(command, cwd=root, env=environment, capture_output=True, timeout=command_timeout_seconds, check=False)
            output = completed.stdout + completed.stderr
            returncode = completed.returncode
        except subprocess.TimeoutExpired as error:
            output = (error.stdout or b"") + (error.stderr or b"")
            returncode = 124
        if returncode == 0 and command[0] == "cargo" and b"running 1 test" not in output:
            output += b"\nreliability soak: exact scenario selected zero tests\n"
            returncode = 125
        elapsed_ms = round((time.monotonic() - command_started) * 1000, 3)
        state.runs += 1
        state.durations_ms.append(elapsed_ms)
        if returncode == 0:
            state.passed += 1
        else:
            failures.append({
                "scenario": name,
                "returncode": returncode,
                "root_cause": classify_failure(returncode, output),
                "duration_ms": elapsed_ms,
                "command": list(command),
                "output_sha256": hashlib.sha256(output).hexdigest(),
                "diagnostic": bounded_diagnostic(output, root=root, max_bytes=diagnostic_max_bytes),
                "at_utc": utc_now(),
            })
            failures[:] = failures[-MAX_FAILURES:]
        cycle += 1
        remaining = deadline - time.monotonic()
        if remaining > 0 and interval_seconds > 0:
            time.sleep(min(interval_seconds, remaining))
    elapsed = time.monotonic() - started
    source_after = source_fingerprint(root)
    if source_after != source_before and invalid_reason is None:
        invalid_reason = "source_changed"
    minimum_observed_free_bytes = min(minimum_observed_free_bytes, shutil.disk_usage(root).free)
    return render_report(started_at=started_at, finished_at=utc_now(), requested_seconds=duration_seconds,
                         elapsed_seconds=elapsed, states=states, failures=failures,
                         protected_before=before, protected_after=tree_measure(protected_path),
                         max_growth_files=max_growth_files, max_growth_bytes=max_growth_bytes,
                         source_before=source_before, source_after=source_after,
                         storage={"minimum_free_bytes": minimum_free_bytes,
                                  "minimum_observed_free_bytes": minimum_observed_free_bytes},
                         invalid_reason=invalid_reason,
                         scenario_latency_budget_ms=int(command_timeout_seconds * 1_000),
                         diagnostic_max_bytes=diagnostic_max_bytes)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path)
    parser.add_argument("--duration-seconds", type=float, required=True)
    parser.add_argument("--interval-seconds", type=float, default=30.0)
    parser.add_argument("--command-timeout-seconds", type=float, default=180.0)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--protected-path", type=Path)
    parser.add_argument("--max-protected-growth-files", type=int, default=0)
    parser.add_argument("--max-protected-growth-bytes", type=int, default=0)
    parser.add_argument("--cargo-target-dir", type=Path)
    parser.add_argument("--minimum-free-bytes", type=int, default=DEFAULT_MINIMUM_FREE_BYTES)
    parser.add_argument("--diagnostic-max-bytes", type=int, default=DEFAULT_DIAGNOSTIC_BYTES)
    parser.add_argument("--integrity-check-interval-seconds", type=float, default=300.0)
    args = parser.parse_args()
    root = (args.root or Path(__file__).resolve().parents[1]).resolve()
    protected = args.protected_path or root / "core" / "state"
    report = run_soak(
        root,
        args.duration_seconds,
        args.interval_seconds,
        args.command_timeout_seconds,
        protected,
        args.max_protected_growth_files,
        args.max_protected_growth_bytes,
        args.cargo_target_dir,
        args.minimum_free_bytes,
        args.diagnostic_max_bytes,
        args.integrity_check_interval_seconds,
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    temporary = args.output.with_suffix(args.output.suffix + ".tmp")
    temporary.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    os.replace(temporary, args.output)
    print(json.dumps({"status": report["status"], **report["summary"]}, sort_keys=True))
    return 0 if report["status"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
