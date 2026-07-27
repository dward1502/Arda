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
import re
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
SPEC_ROOT = REPO_ROOT / "spec" / "aipkg" / "v0.1"
RUST_SURFACE = REPO_ROOT / "crates" / "spine" / "governance" / "arda-core" / "src" / "aipkg.rs"
EXPECTED_FILES = {
    "AIPKG-CONTAINER-v0.1.md",
    "execution-request.schema.json",
    "manifest.example.json",
    "manifest.schema.json",
    "receipt.schema.json",
    "receipt.schema.md",
}
REQUIRED_CONTAINER_HINTS = [
    "arda-core",
    "preflight",
    "receipt",
    "manifest",
]
REQUIRED_CONTAINER_SCRIPT_HINT = "scripts/aipkg/validate_spec.py"
REQUIRED_RUST_MARKERS = (
    "pub struct AipkgManifest",
    "pub struct AipkgPreflightReceipt",
    "pub struct AipkgExecutionReceipt",
    "pub struct AipkgValidationReceipt",
    "pub struct AipkgGovernanceEvidence",
    "pub struct AipkgReceiptChain",
    "preflight_check_with_signature",
)


def load_json(path: Path) -> dict[str, object]:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise ValueError(f"invalid JSON: {path}: {exc}") from exc


def _matches_type(value: Any, expected: str) -> bool:
    return {
        "object": isinstance(value, dict),
        "array": isinstance(value, list),
        "string": isinstance(value, str),
        "integer": isinstance(value, int) and not isinstance(value, bool),
        "boolean": isinstance(value, bool),
        "null": value is None,
    }.get(expected, True)


def _validate_schema_subset(
    value: Any, schema: dict[str, Any], path: str, errors: list[str]
) -> None:
    expected_type = schema.get("type")
    if expected_type:
        accepted = expected_type if isinstance(expected_type, list) else [expected_type]
        if not any(_matches_type(value, item) for item in accepted):
            errors.append(f"{path}: expected type {expected_type!r}")
            return

    if "const" in schema and value != schema["const"]:
        errors.append(f"{path}: value does not match schema const")
    if "enum" in schema and value not in schema["enum"]:
        errors.append(f"{path}: value is outside schema enum")
    if isinstance(value, str):
        if len(value) < schema.get("minLength", 0):
            errors.append(f"{path}: string is shorter than minLength")
        pattern = schema.get("pattern")
        if pattern and re.fullmatch(pattern, value) is None:
            errors.append(f"{path}: string does not match schema pattern")
    if isinstance(value, list):
        if len(value) < schema.get("minItems", 0):
            errors.append(f"{path}: array has too few items")
        item_schema = schema.get("items")
        if isinstance(item_schema, dict):
            for index, item in enumerate(value):
                _validate_schema_subset(item, item_schema, f"{path}[{index}]", errors)
    if isinstance(value, dict):
        properties = schema.get("properties", {})
        for key in schema.get("required", []):
            if key not in value:
                errors.append(f"{path}: missing required key {key!r}")
        if schema.get("additionalProperties") is False:
            for key in value:
                if key not in properties:
                    errors.append(f"{path}: unexpected key {key!r}")
        for key, child in value.items():
            child_schema = properties.get(key)
            if isinstance(child_schema, dict):
                _validate_schema_subset(child, child_schema, f"{path}.{key}", errors)


def validate_manifest_example_matches_schema(spec_root: Path) -> list[str]:
    manifest_schema = load_json(spec_root / "manifest.schema.json")
    manifest_example = load_json(spec_root / "manifest.example.json")

    if not isinstance(manifest_schema, dict):
        return ["manifest.schema.json is not a JSON object"]
    if not isinstance(manifest_example, dict):
        return ["manifest.example.json is not a JSON object"]

    errors: list[str] = []
    _validate_schema_subset(
        manifest_example, manifest_schema, "manifest.example.json", errors
    )
    return errors


def validate_container_doc(spec_root: Path) -> str | None:
    container_path = spec_root / "AIPKG-CONTAINER-v0.1.md"
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


def validate_rust_surface() -> list[str]:
    if not RUST_SURFACE.is_file():
        return [f"missing Rust AIPKG implementation: {RUST_SURFACE.relative_to(REPO_ROOT)}"]
    source = RUST_SURFACE.read_text(encoding="utf-8")
    return [
        f"Rust AIPKG implementation missing marker: {marker}"
        for marker in REQUIRED_RUST_MARKERS
        if marker not in source
    ]


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

    for name in (
        "manifest.schema.json",
        "execution-request.schema.json",
        "receipt.schema.json",
    ):
        path = spec_root / name
        if path.exists():
            try:
                load_json(path)
            except ValueError as exc:
                errors.append(str(exc))

    try:
        errors.extend(validate_manifest_example_matches_schema(spec_root))
    except (OSError, ValueError) as exc:
        errors.append(str(exc))

    container_error = validate_container_doc(spec_root)
    if container_error:
        errors.append(container_error)

    receipt_path = spec_root / "receipt.schema.md"
    if receipt_path.exists() and receipt_path.stat().st_size == 0:
        errors.append("receipt.schema.md is empty")

    errors.extend(validate_rust_surface())

    if errors:
        print("AIPKG spec validation failed:")
        for error in errors:
            print(f"- {error}")
        return 1
    print("AIPKG spec validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
