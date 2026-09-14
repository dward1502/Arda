#!/usr/bin/env python3
"""Run no-delete HADES nightly operations and write audit receipts."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

CONTRACT = "arda.hades.nightly_operations.v1"
DEFAULT_OUT_ROOT = Path("audit/hades-nightly-runs")


def now_utc() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def default_run_id(now: datetime | None = None) -> str:
    now = now or datetime.now(timezone.utc)
    return f"hades-nightly-{now.strftime('%Y%m%dT%H%M%SZ')}"


def date_run_dir(root: Path, family: str, run_id: str, now: datetime | None = None) -> Path:
    now = now or datetime.now(timezone.utc)
    return root / "audit" / family / now.strftime("%Y-%m-%d") / run_id


def repo_relative(path: Path, root: Path) -> str:
    try:
        return path.resolve().relative_to(root.resolve()).as_posix()
    except ValueError:
        return path.as_posix()


def run_command(command: list[str], root: Path, *, timeout: int = 900) -> dict[str, Any]:
    started = now_utc()
    try:
        completed = subprocess.run(
            command,
            cwd=root,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
            check=False,
        )
        return {
            "command": command,
            "started_at_utc": started,
            "finished_at_utc": now_utc(),
            "exit_code": completed.returncode,
            "timed_out": False,
            "stdout_tail": completed.stdout[-12000:],
            "stderr_tail": completed.stderr[-12000:],
        }
    except subprocess.TimeoutExpired as exc:
        stdout = exc.stdout if isinstance(exc.stdout, str) else ""
        stderr = exc.stderr if isinstance(exc.stderr, str) else ""
        return {
            "command": command,
            "started_at_utc": started,
            "finished_at_utc": now_utc(),
            "exit_code": 124,
            "timed_out": True,
            "stdout_tail": stdout[-12000:],
            "stderr_tail": stderr[-12000:],
        }


def cli_bin() -> str:
    return os.environ.get(
        "ARDA_CLI_BIN",
        str(Path.home() / ".cache/arda-build/target/release/arda-cli"),
    )


def read_json(path: Path) -> dict[str, Any] | None:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError, UnicodeDecodeError):
        return None
    return data if isinstance(data, dict) else None


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def append_jsonl(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(payload, sort_keys=True) + "\n")


def command_succeeded(result: dict[str, Any]) -> bool:
    return result.get("exit_code") == 0 and result.get("timed_out") is False


def run_setup_console_readiness(
    root: Path, setup_out: Path, portability_summary: Path
) -> dict[str, Any]:
    result = run_command(
        [
            "python3",
            "scripts/audit/setup_console_audit.py",
            "--out-dir",
            repo_relative(setup_out, root),
            "--state-path",
            "core/state/setup_console_readiness.json",
            "--portability-receipt",
            repo_relative(portability_summary, root),
        ],
        root,
    )
    result["producer"] = "scripts/audit/setup_console_audit.py"
    result["fallback_used"] = False
    return result


def run_nightly(root: Path, run_id: str, out_dir: Path, now: datetime | None = None) -> dict[str, Any]:
    root = root.resolve()
    now = now or datetime.now(timezone.utc)
    out_dir.mkdir(parents=True, exist_ok=True)

    system_run_id = run_id.replace("hades-nightly-", "system-audit-", 1)
    portability_run_id = run_id.replace("hades-nightly-", "portability-audit-", 1)
    setup_run_id = run_id.replace("hades-nightly-", "setup-console-", 1)
    repeated_run_id = run_id.replace("hades-nightly-", "repeated-audit-", 1)

    portability_out = date_run_dir(root, "portability-audit-runs", portability_run_id, now)
    setup_out = date_run_dir(root, "setup-console-runs", setup_run_id, now)
    repeated_out = date_run_dir(root, "repeated-audit-runs", repeated_run_id, now)
    org_out = out_dir / "organization"

    commands: dict[str, dict[str, Any]] = {}
    commands["system_audit"] = run_command(
        [
            "python3",
            "scripts/audit/system_audit.py",
            "--target-set",
            "phase5",
            "--run-id",
            system_run_id,
        ],
        root,
    )
    commands["portability_audit"] = run_command(
        [
            "python3",
            "scripts/audit/portability_audit.py",
            "--out",
            repo_relative(portability_out, root),
        ],
        root,
    )
    portability_summary = portability_out / "summary.json"
    setup_receipt = setup_out / "setup_console_readiness_receipt.json"
    system_summary = date_run_dir(root, "system-audit-runs", system_run_id, now) / "summary.json"
    commands["setup_console_readiness"] = run_setup_console_readiness(
        root, setup_out, portability_summary
    )
    commands["repeated_audit"] = run_command(
        [
            "python3",
            "scripts/audit/repeated_audit.py",
            "--out",
            repo_relative(repeated_out, root),
            "--run-id",
            repeated_run_id,
            "--portability-summary",
            repo_relative(portability_summary, root),
            "--setup-receipt",
            repo_relative(setup_receipt, root),
            "--system-summary",
            repo_relative(system_summary, root),
        ],
        root,
    )
    commands["rumil_organization_maintenance"] = run_command(
        ["bash", "scripts/rumil_organization_maintenance.sh"],
        root,
    )
    commands["rumil_storage_hygiene_audit"] = run_command(
        [
            "python3",
            "scripts/rumil_storage_hygiene_audit.py",
            "--out-dir",
            repo_relative(out_dir / "storage_hygiene", root),
            "--state-path",
            "core/state/storage_hygiene.json",
        ],
        root,
    )
    commands["queue_active_projection"] = run_command(
        [
            cli_bin(),
            "prometheus",
            "autopilot",
            "status",
            "--root",
            ".",
        ],
        root,
        timeout=120,
    )
    commands["queue_hygiene_projection"] = run_command(
        ["bash", "scripts/monitor_queue_hygiene.sh"],
        root,
        timeout=120,
    )

    org_out.mkdir(parents=True, exist_ok=True)
    for name in ("markdown_link_check_last.md", "storage_hygiene_last.json"):
        source = root / "data/rumil" / name
        if source.exists():
            target = org_out / name
            target.write_bytes(source.read_bytes())

    artifacts = {
        "system_audit_summary": repo_relative(system_summary, root),
        "portability_summary": repo_relative(portability_summary, root),
        "setup_console_receipt": repo_relative(setup_receipt, root),
        "repeated_audit_summary": repo_relative(repeated_out / "summary.json", root),
        "markdown_link_check": repo_relative(org_out / "markdown_link_check_last.md", root),
        "organization_storage_hygiene": repo_relative(org_out / "storage_hygiene_last.json", root),
        "storage_hygiene": repo_relative(out_dir / "storage_hygiene" / "summary.json", root),
        "queue_active": "core/state/queue_active.json",
        "queue_hygiene": "core/state/queue_hygiene.json",
    }

    payload = {
        "contract": CONTRACT,
        "run_id": run_id,
        "generated_at_utc": now_utc(),
        "mode": "no_delete_nightly_operations",
        "mutation_policy": "audit_receipts_only_no_source_config_service_or_queue_mutation",
        "layout": {
            "contract": "arda.audit.run_layout.v1",
            "default_shape": "audit/<family>/YYYY-MM-DD/<run-id>",
            "date_first": True,
        },
        "commands": commands,
        "artifacts": artifacts,
        "status": "pass" if all(command_succeeded(item) for item in commands.values()) else "warn",
    }
    write_json(out_dir / "summary.json", payload)
    write_json(root / "core/state/hades_nightly_operations.json", payload)
    append_jsonl(root / "data/hades/nightly_operations_history.jsonl", payload)
    return payload


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--run-id", default=None)
    parser.add_argument("--out", type=Path, default=None)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    now = datetime.now(timezone.utc)
    root = args.root.resolve()
    run_id = args.run_id or default_run_id(now)
    out_dir = (root / args.out).resolve() if args.out else date_run_dir(root, "hades-nightly-runs", run_id, now)
    summary = run_nightly(root, run_id, out_dir, now)
    print(json.dumps({"status": summary["status"], "summary": repo_relative(out_dir / "summary.json", root)}, indent=2, sort_keys=True))
    return 0 if summary["status"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
