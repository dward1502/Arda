#!/usr/bin/env python3
"""Seed and diagnose Stage 5 support failures from redacted bundles only."""

from __future__ import annotations

import argparse
import contextlib
import hashlib
import importlib.util
import json
import os
import shutil
import sys
import tarfile
import tempfile
from pathlib import Path
from typing import Any, Iterator

ROOT = Path(__file__).resolve().parents[1]
BETA_OPS_PATH = ROOT / "scripts" / "arda_beta_ops.py"
SPEC = importlib.util.spec_from_file_location("arda_beta_ops_support", BETA_OPS_PATH)
assert SPEC and SPEC.loader
beta_ops = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = beta_ops
SPEC.loader.exec_module(beta_ops)

CONTRACT = "arda.stage5.support-exercise.v1"
SECRET_CANARIES = ("support-secret-token", "support-url-password")


@contextlib.contextmanager
def provider_endpoint(value: str | None) -> Iterator[None]:
    names = ("MANWE_BASE_URL", "ARDA_MANWE_BASE_URL")
    previous = {name: os.environ.get(name) for name in names}
    try:
        for name in names:
            os.environ.pop(name, None)
        if value is not None:
            os.environ["MANWE_BASE_URL"] = value
        yield
    finally:
        for name, old in previous.items():
            if old is None:
                os.environ.pop(name, None)
            else:
                os.environ[name] = old


def diagnose(payload: dict[str, Any]) -> str:
    failed = {
        item["check_id"]
        for item in payload.get("readiness", {}).get("checks", [])
        if item.get("status") == "fail"
    }
    if "launcher.installed" in failed:
        return "missing-launcher"
    if "launcher.dynamic_libraries" in failed:
        return "native-dependency-failure"
    if "provider.manwe_endpoint" in failed:
        return "missing-provider-endpoint"
    return "unclassified"


def inspect_bundle(bundle: Path, forbidden: tuple[str, ...]) -> tuple[dict[str, Any], list[str]]:
    with tarfile.open(bundle, "r:gz") as archive:
        names = archive.getnames()
        diagnostics_file = archive.extractfile("diagnostics.json")
        if diagnostics_file is None:
            raise RuntimeError("diagnostics.json missing from bundle")
        payload = json.load(diagnostics_file)
        all_text = []
        for member in archive.getmembers():
            if not member.isfile():
                continue
            source = archive.extractfile(member)
            if source is not None:
                all_text.append(source.read().decode("utf-8", errors="replace"))
    joined = "\n".join(all_text)
    leaks = [value for value in forbidden if value and value in joined]
    leaks.extend(name for name in names if "credential" in name.lower() or ".env" in name.lower())
    return payload, sorted(set(leaks))


def seed_config(layout: Any) -> None:
    layout.config.mkdir(parents=True, exist_ok=True)
    (layout.config / "settings.toml").write_text(
        "API_TOKEN=support-secret-token\nendpoint=https://operator:support-url-password@example.invalid\n",
        encoding="utf-8",
    )
    (layout.config / "credentials.json").write_text(
        '{"token":"support-secret-token"}\n', encoding="utf-8"
    )


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def execute(output_dir: Path) -> dict[str, Any]:
    output_dir.mkdir(parents=True, exist_ok=True)
    launcher = ROOT / "target/release/arda-launcher"
    if not launcher.is_file():
        raise RuntimeError(f"native launcher missing: {launcher}")
    scenarios = (
        ("missing-launcher", "valid", None),
        ("native-dependency-failure", "invalid", "http://127.0.0.1:7171"),
        ("missing-provider-endpoint", "valid", None),
    )
    receipts = []
    with tempfile.TemporaryDirectory(prefix="arda-stage5-support-") as raw:
        base = Path(raw)
        for expected, launcher_mode, endpoint in scenarios:
            home = base / expected
            home.mkdir()
            layout = beta_ops.layout_from_args(
                argparse.Namespace(home=str(home), root=str(ROOT))
            )
            seed_config(layout)
            if expected != "missing-launcher":
                layout.install_lib.mkdir(parents=True)
                installed = layout.install_lib / "arda-launcher"
                if launcher_mode == "valid":
                    shutil.copy2(launcher, installed)
                else:
                    installed.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
                    installed.chmod(0o755)
            bundle = output_dir / f"{expected}.tar.gz"
            with provider_endpoint(endpoint):
                beta_ops.diagnostics(layout, bundle)
            payload, leaks = inspect_bundle(
                bundle,
                (*SECRET_CANARIES, str(home), str(ROOT)),
            )
            actual = diagnose(payload)
            receipts.append({
                "scenario": expected,
                "diagnosis": actual,
                "status": "pass" if actual == expected and not leaks else "fail",
                "bundle": bundle.name,
                "bundle_bytes": bundle.stat().st_size,
                "bundle_sha256": sha256(bundle),
                "redaction_leaks": leaks,
                "readiness_gate": payload["readiness"]["gate_status"],
            })
    passed = all(item["status"] == "pass" for item in receipts)
    return {
        "contract": CONTRACT,
        "status": "pass" if passed else "fail",
        "diagnosis_input": "redacted archive contents only",
        "scenarios": receipts,
        "privacy": {
            "crash_reporting_default": "disabled",
            "external_transmission": "none",
            "operator_review_required_before_sharing": True,
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    try:
        receipt = execute(args.output_dir)
        output = args.output_dir / "support-exercise.json"
        output.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        print(json.dumps({"status": receipt["status"], "output": str(output)}, sort_keys=True))
        return 0 if receipt["status"] == "pass" else 1
    except (OSError, RuntimeError, beta_ops.BetaOpsError) as error:
        print(json.dumps({"status": "fail", "error": str(error)}, sort_keys=True), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
