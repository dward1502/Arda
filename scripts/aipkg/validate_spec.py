#!/usr/bin/env python3
"""Read-only AIPKG spec validator.

Usage:
    python scripts/aipkg/validate_spec.py

Exit status:
    0 - spec bundle is valid
    nonzero - spec bundle is invalid or incomplete
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SPEC_ROOT = REPO_ROOT / "spec" / "aipkg" / "v0.1"
EXPECTED_FILES = {
    "AIPKG-CONTAINER-v0.1.md",
    "execution-request.schema.json",
    "manifest.example.json",
    "manifest.schema.json",
    "receipt.schema.md",
}
REQUIRED_CONTAINER_HINTS = [
    "arda-core",
    "preflight",
    "receipt",
    "manifest",
]
REQUIRED_CONTAINER_SCRIPT_HINT = "scripts/aipkg/validate_spec.py"


def load_json(path: Path) -> dict[str, object]:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise ValueError(f"invalid JSON: {path}: {exc}") from exc


def validate_manifest_example_matches_schema() -> str | None:
    manifest_schema = load_json(SPEC_ROOT / "manifest.schema.json")
    manifest_example = load_json(SPEC_ROOT / "manifest.example.json")

    if not isinstance(manifest_schema, dict):
        return "manifest.schema.json is not a JSON object"
    if not isinstance(manifest_example, dict):
        return "manifest.example.json is not a JSON object"

    required_fields = [
        "manifest_version",
        "package_id",
        "version",
        "package_digest",
        "runtime_profile",
        "preflight",
        "governance",
        "receipts",
    ]
    missing = [field for field in required_fields if field not in manifest_example]
    if missing:
        return f"manifest.example.json missing fields: {missing}"

    preflight = manifest_example["preflight"]
    governance = manifest_example["governance"]
    receipts = manifest_example["receipts"]
    if not isinstance(preflight, dict):
        return "manifest.example.json preflight must be an object"
    if not isinstance(governance, dict):
        return "manifest.example.json governance must be an object"
    if not isinstance(receipts, dict):
        return "manifest.example.json receipts must be an object"
    return None


def validate_container_doc() -> str | None:
    container_path = SPEC_ROOT / "AIPKG-CONTAINER-v0.1.md"
    if not container_path.exists() or container_path.stat().st_size == 0:
        return "AIPKG-CONTAINER-v0.1.md is missing or empty"

    text = container_path.read_text(encoding="utf-8")
    missing = [hint for hint in REQUIRED_CONTAINER_HINTS if hint not in text]
    if missing:
        return f"AIPKG-CONTAINER-v0.1.md missing conceptual hints: {missing}"

    if REQUIRED_CONTAINER_SCRIPT_HINT not in text:
        return (
            "AIPKG-CONTAINER-v0.1.md does not reference the spec validator script"
        )
    return None


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate AIPKG spec bundle")
    parser.add_argument(
        "--spec-root",
        type=Path,
        default=SPEC_ROOT,
        help="Path to the AIPKG spec root",
    )
    args = parser.parse_args()
    spec_root = args.spec_root.resolve()

    errors = []
    files = {p.name for p in spec_root.iterdir() if p.is_file()}
    missing_files = EXPECTED_FILES - files
    if missing_files:
        errors.append(f"missing spec files: {sorted(missing_files)}")

    for name in ("manifest.schema.json", "execution-request.schema.json"):
        path = spec_root / name
        if path.exists():
            try:
                load_json(path)
            except ValueError as exc:
                errors.append(str(exc))

    manifest_example_error = validate_manifest_example_matches_schema()
    if manifest_example_error:
        errors.append(manifest_example_error)

    container_error = validate_container_doc()
    if container_error:
        errors.append(container_error)

    receipt_path = spec_root / "receipt.schema.md"
    if receipt_path.exists() and receipt_path.stat().st_size == 0:
        errors.append("receipt.schema.md is empty")

    if errors:
        print("AIPKG spec validation failed:")
        for error in errors:
            print(f"- {error}")
        return 1
    print("AIPKG spec validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
