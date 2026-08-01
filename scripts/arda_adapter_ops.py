#!/usr/bin/env python3
"""Generate project-contract templates and execute Stage 5 adapter conformance."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from uuid import UUID, uuid4

CONTRACT = "arda.stage5.adapter-conformance.v1"
KINDS = {
    "rust": {
        "adapter": "cargo",
        "program": "cargo",
        "args": ["test"],
        "artifact": ["binary", "target/debug/app"],
    },
    "python": {
        "adapter": "python",
        "program": "python3",
        "args": ["-m", "pytest"],
        "artifact": ["wheel", "dist/project.whl"],
    },
    "javascript": {
        "adapter": "node",
        "program": "pnpm",
        "args": ["test"],
        "artifact": ["bundle", "dist/index.js"],
    },
}
NAME_PATTERN = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$")


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def project_contract(kind: str, name: str, project_id: str | None = None,
                     declared_at: str | None = None) -> dict[str, Any]:
    if kind not in KINDS:
        raise ValueError(f"unsupported project kind: {kind}")
    if not NAME_PATTERN.fullmatch(name):
        raise ValueError("name must be 1-128 safe filename characters")
    identifier = str(UUID(project_id)) if project_id else str(uuid4())
    config = KINDS[kind]
    artifact_id, artifact_path = config["artifact"]
    return {
        "schema_version": "arda.project-contract.v1",
        "identity": {"project_id": identifier, "name": name, "kind": kind},
        "workspace": {"root": "."},
        "runtime": {"adapter": config["adapter"]},
        "commands": [{"id": "test", "program": config["program"], "args": config["args"], "working_dir": "."}],
        "checks": [{"id": "test", "command": "test"}],
        "artifacts": [{"id": artifact_id, "path": artifact_path}],
        "permissions": {
            "authority": "approval_required",
            "network": {"allow": False},
            "filesystem": {"write": True},
            "secrets": {"env_names": []},
        },
        "rollback": {"strategy": "git_revert"},
        "memory": {"scope": "project"},
        "provenance": {"declared_by": "arda", "declared_at": declared_at or utc_now(), "stage": 5},
    }


def command_receipt(root: Path, command: tuple[str, ...], timeout: float = 300) -> dict[str, Any]:
    started = datetime.now(timezone.utc)
    try:
        completed = subprocess.run(command, cwd=root, capture_output=True, timeout=timeout, check=False)
        output = completed.stdout + completed.stderr
        returncode = completed.returncode
    except subprocess.TimeoutExpired as error:
        output = (error.stdout or b"") + (error.stderr or b"")
        returncode = 124
    elapsed = (datetime.now(timezone.utc) - started).total_seconds()
    return {
        "command": list(command),
        "returncode": returncode,
        "elapsed_seconds": elapsed,
        "output_sha256": hashlib.sha256(output).hexdigest(),
    }


def schema_receipt(root: Path) -> dict[str, Any]:
    try:
        from jsonschema import Draft202012Validator, FormatChecker
    except ModuleNotFoundError as error:
        return {"status": "fail", "error": f"jsonschema unavailable: {error}"}
    schema_path = root / "spec/project-contract/v1/project-contract.schema.json"
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    examples = sorted((schema_path.parent / "examples").glob("*.json"))
    errors: dict[str, list[str]] = {}
    for example in examples:
        payload = json.loads(example.read_text(encoding="utf-8"))
        found = sorted(str(item) for item in validator.iter_errors(payload))
        if found:
            errors[str(example.relative_to(root))] = found
    return {
        "status": "pass" if not errors and {item.stem for item in examples} == {
            "rust-project", "python-project", "javascript-project"
        } else "fail",
        "validated_examples": [str(item.relative_to(root)) for item in examples],
        "errors": errors,
    }


def conformance(root: Path) -> dict[str, Any]:
    schema = schema_receipt(root)
    commands = [
        command_receipt(root, (sys.executable, "-m", "unittest", "sdk.python.tests.test_conformance", "-v")),
        command_receipt(root, ("cargo", "test", "-p", "arda-engine", "--test", "project_adapter_jsonl", "--", "--test-threads=1")),
        command_receipt(root, ("cargo", "test", "-p", "arda-project-adapter-sdk")),
        command_receipt(root, ("npm", "test", "--prefix", "sdk/javascript")),
        command_receipt(root, ("cargo", "test", "-p", "arda-engine", "--test", "workbench_python_golden", "--test", "workbench_rust_golden", "--", "--test-threads=1")),
    ]
    passed = schema["status"] == "pass" and all(item["returncode"] == 0 for item in commands)
    return {
        "contract": CONTRACT,
        "status": "pass" if passed else "fail",
        "generated_at_utc": utc_now(),
        "schema": schema,
        "commands": commands,
        "limitations": [
            "Python is the complete executable reference server; Rust and JavaScript packages provide bounded framing, envelope validation, and capability negotiation.",
            "The unseen-repository fixture covers an isolated Python repository; a separately sourced external repository remains unavailable.",
        ],
    }


def write_json(path: Path, payload: dict[str, Any], force: bool = False) -> None:
    if path.exists() and not force:
        raise FileExistsError(f"refusing to overwrite existing file: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    temporary.replace(path)


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    template = subparsers.add_parser("template")
    template.add_argument("--kind", choices=sorted(KINDS), required=True)
    template.add_argument("--name", required=True)
    template.add_argument("--project-id")
    template.add_argument("--declared-at")
    template.add_argument("--output", type=Path, required=True)
    template.add_argument("--force", action="store_true")
    check = subparsers.add_parser("conformance")
    check.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    try:
        if args.command == "template":
            payload = project_contract(args.kind, args.name, args.project_id, args.declared_at)
            write_json(args.output, payload, args.force)
            print(json.dumps({"status": "pass", "output": str(args.output)}, sort_keys=True))
            return 0
        payload = conformance(root)
        write_json(args.output, payload, True)
        print(json.dumps({"status": payload["status"], "output": str(args.output)}, sort_keys=True))
        return 0 if payload["status"] == "pass" else 1
    except (FileExistsError, ValueError) as error:
        print(json.dumps({"status": "fail", "error": str(error)}, sort_keys=True), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
