#!/usr/bin/env python3
"""Deterministic release-bundle identity, checksums, and dependency inventory."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any

CONTRACT = "arda.release-bundle.v1"
SBOM_CONTRACT = "arda.release-sbom.v1"
SUPPORTED_PROFILE = "bluefin-lts-10-x86_64"


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def canonical_json(payload: dict[str, Any]) -> bytes:
    return (json.dumps(payload, indent=2, sort_keys=True) + "\n").encode("utf-8")


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.tmp-{os.getpid()}")
    temporary.write_bytes(content)
    os.chmod(temporary, 0o644)
    temporary.replace(path)


def command_line(command: list[str], cwd: Path) -> str:
    result = subprocess.run(command, cwd=cwd, text=True, capture_output=True, check=True)
    return result.stdout.strip().splitlines()[0] if result.stdout.strip() else ""


def source_tree_sha256(root: Path) -> str:
    """Hash release-relevant tracked and untracked source, independent of mtimes."""
    result = subprocess.run(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
        cwd=root,
        capture_output=True,
        check=True,
    )
    included_roots = ("apps/arda-launcher/", "crates/", "src/")
    included_files = {"Cargo.toml", "Cargo.lock", "package.json", "pnpm-lock.yaml", "rust-toolchain.toml"}
    excluded_parts = {"node_modules", "dist", "target", "__pycache__"}
    digest = hashlib.sha256()
    for raw_path in sorted(filter(None, result.stdout.split(b"\0"))):
        relative = raw_path.decode("utf-8")
        path = Path(relative)
        if path.name.endswith(".tsbuildinfo") or any(part in excluded_parts for part in path.parts):
            continue
        if relative not in included_files and not relative.startswith(included_roots):
            continue
        absolute = root / path
        if not absolute.is_file() or absolute.is_symlink():
            continue
        digest.update(relative.encode("utf-8"))
        digest.update(b"\0")
        digest.update(sha256_file(absolute).encode("ascii"))
        digest.update(b"\n")
    return digest.hexdigest()


def default_build_inputs(root: Path) -> dict[str, Any]:
    source_epoch = os.environ.get("SOURCE_DATE_EPOCH")
    if source_epoch is None:
        source_epoch = command_line(["git", "show", "-s", "--format=%ct", "HEAD"], root)
    status = subprocess.run(
        ["git", "status", "--porcelain", "--untracked-files=no"],
        cwd=root,
        text=True,
        capture_output=True,
        check=True,
    ).stdout
    locks: dict[str, str] = {}
    for relative in ("Cargo.lock", "apps/arda-launcher/pnpm-lock.yaml"):
        path = root / relative
        if path.is_file():
            locks[relative] = sha256_file(path)
    appimagetool = Path(os.environ.get("APPIMAGETOOL") or shutil.which("appimagetool") or "appimagetool")
    if not appimagetool.is_file():
        raise FileNotFoundError(f"appimagetool not found: {appimagetool}")
    return {
        "source_commit": command_line(["git", "rev-parse", "HEAD"], root),
        "source_tree_sha256": source_tree_sha256(root),
        "source_date_epoch": int(source_epoch),
        "tracked_worktree_status_sha256": hashlib.sha256(status.encode("utf-8")).hexdigest(),
        "tracked_worktree_clean": not bool(status),
        "lockfiles": locks,
        "toolchain": {
            "cargo": command_line(["cargo", "--version"], root),
            "rustc": command_line(["rustc", "--version"], root),
            "pnpm": command_line(["pnpm", "--version"], root),
            "node": command_line(["node", "--version"], root),
            "appimagetool": command_line([str(appimagetool), "--version"], root),
            "appimagetool_sha256": sha256_file(appimagetool),
        },
    }


def artifact_kind(path: Path) -> str:
    if path.name.endswith(".AppImage"):
        return "appimage"
    if path.suffix == ".deb":
        return "deb"
    if path.suffix == ".rpm":
        return "rpm"
    return path.suffix.lstrip(".") or "binary"


def generate_bundle_manifest(
    artifacts: list[Path],
    output: Path,
    checksums_output: Path,
    version: str,
    build_inputs: dict[str, Any],
) -> dict[str, Any]:
    if not artifacts:
        raise ValueError("at least one artifact is required")
    names = [path.name for path in artifacts]
    if len(names) != len(set(names)):
        raise ValueError("artifact basenames must be unique")
    entries = []
    for artifact in sorted(artifacts, key=lambda item: item.name):
        if not artifact.is_file():
            raise FileNotFoundError(f"artifact not found: {artifact}")
        entries.append(
            {
                "kind": artifact_kind(artifact),
                "name": artifact.name,
                "sha256": sha256_file(artifact),
                "size_bytes": artifact.stat().st_size,
            }
        )
    payload: dict[str, Any] = {
        "contract": CONTRACT,
        "version": version,
        "supported_profile": SUPPORTED_PROFILE,
        "artifacts": entries,
        "build_inputs": build_inputs,
        "rollback_compatibility": {
            "from": "stage-4-private-beta",
            "config_schema": "arda.hermes-adapter.v1",
            "project_schema": "arda.project-contract.v1",
            "migration_required": False,
        },
    }
    payload["release_id"] = "sha256:" + hashlib.sha256(canonical_json(payload)).hexdigest()
    manifest_bytes = canonical_json(payload)
    write_atomic(output, manifest_bytes)
    checksum_lines = [f"{entry['sha256']}  {entry['name']}" for entry in entries]
    checksum_lines.append(f"{hashlib.sha256(manifest_bytes).hexdigest()}  {output.name}")
    write_atomic(checksums_output, ("\n".join(checksum_lines) + "\n").encode("utf-8"))
    return payload


def verify_checksums(checksums_path: Path, artifact_dir: Path) -> bool:
    seen: set[str] = set()
    for raw in checksums_path.read_text(encoding="utf-8").splitlines():
        if not raw.strip():
            continue
        expected, separator, name = raw.partition("  ")
        if not separator or not name or Path(name).name != name or name in seen:
            return False
        seen.add(name)
        target = artifact_dir / name
        if not target.is_file() or sha256_file(target) != expected:
            return False
    return bool(seen)


def reachable_cargo_ids(cargo_metadata: dict[str, Any], root_package: str) -> set[str]:
    packages = cargo_metadata.get("packages", [])
    roots = [item["id"] for item in packages if item.get("name") == root_package]
    if len(roots) != 1:
        raise ValueError(f"expected one Cargo package named {root_package}, found {len(roots)}")
    graph = {
        node["id"]: [dependency["pkg"] for dependency in node.get("deps", [])]
        for node in cargo_metadata["resolve"]["nodes"]
    }
    reachable: set[str] = set()
    pending = [roots[0]]
    while pending:
        package_id = pending.pop()
        if package_id in reachable:
            continue
        reachable.add(package_id)
        pending.extend(graph.get(package_id, []))
    return reachable


def generate_sbom(
    cargo_metadata: dict[str, Any],
    pnpm_licenses: dict[str, Any],
    output: Path,
    *,
    root_package: str,
) -> dict[str, Any]:
    reachable = reachable_cargo_ids(cargo_metadata, root_package)
    components: list[dict[str, Any]] = []
    for package in cargo_metadata["packages"]:
        if package["id"] not in reachable:
            continue
        components.append(
            {
                "ecosystem": "cargo",
                "name": package["name"],
                "version": package["version"],
                "license": package.get("license"),
                "source": package.get("source") or "workspace",
            }
        )
    for license_name, packages in pnpm_licenses.items():
        for package in packages:
            for version in package.get("versions", []):
                components.append(
                    {
                        "ecosystem": "pnpm",
                        "name": package["name"],
                        "version": version,
                        "license": package.get("license") or license_name or None,
                        "source": "npm",
                    }
                )
    components.sort(key=lambda item: (item["ecosystem"], item["name"], item["version"]))
    deduplicated: list[dict[str, Any]] = []
    seen: set[tuple[str, str, str]] = set()
    for component in components:
        identity = (component["ecosystem"], component["name"], component["version"])
        if identity not in seen:
            deduplicated.append(component)
            seen.add(identity)
    missing = [
        f"{item['ecosystem']}:{item['name']}@{item['version']}"
        for item in deduplicated
        if not item.get("license")
    ]
    payload = {
        "contract": SBOM_CONTRACT,
        "status": "pass" if not missing else "blocked",
        "root_package": root_package,
        "component_count": len(deduplicated),
        "components": deduplicated,
        "missing_license": missing,
    }
    write_atomic(output, canonical_json(payload))
    return payload


def run_json(command: list[str], cwd: Path) -> dict[str, Any]:
    result = subprocess.run(command, cwd=cwd, text=True, capture_output=True, check=True)
    return json.loads(result.stdout)


def parser() -> argparse.ArgumentParser:
    top = argparse.ArgumentParser(description=__doc__)
    sub = top.add_subparsers(dest="command", required=True)
    bundle = sub.add_parser("bundle-manifest")
    bundle.add_argument("--root", type=Path, default=Path.cwd())
    bundle.add_argument("--version", required=True)
    bundle.add_argument("--artifact", action="append", type=Path, required=True)
    bundle.add_argument("--output", type=Path, required=True)
    bundle.add_argument("--checksums-output", type=Path, required=True)
    verify = sub.add_parser("verify-checksums")
    verify.add_argument("--checksums", type=Path, required=True)
    verify.add_argument("--artifact-dir", type=Path, required=True)
    sbom = sub.add_parser("sbom")
    sbom.add_argument("--root", type=Path, default=Path.cwd())
    sbom.add_argument("--launcher-dir", type=Path, default=Path("apps/arda-launcher"))
    sbom.add_argument("--output", type=Path, required=True)
    return top


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    try:
        if args.command == "bundle-manifest":
            root = args.root.resolve()
            payload = generate_bundle_manifest(
                [path.resolve() for path in args.artifact],
                args.output.resolve(),
                args.checksums_output.resolve(),
                args.version,
                default_build_inputs(root),
            )
            result = {"contract": CONTRACT, "status": "ok", "release_id": payload["release_id"]}
        elif args.command == "verify-checksums":
            passed = verify_checksums(args.checksums.resolve(), args.artifact_dir.resolve())
            result = {"contract": CONTRACT, "status": "pass" if passed else "fail"}
            print(json.dumps(result, sort_keys=True))
            return 0 if passed else 1
        else:
            root = args.root.resolve()
            cargo = run_json(["cargo", "metadata", "--locked", "--format-version", "1"], root)
            pnpm = run_json(["pnpm", "licenses", "list", "--prod", "--json"], (root / args.launcher_dir).resolve())
            payload = generate_sbom(cargo, pnpm, args.output.resolve(), root_package="arda-launcher")
            result = {
                "contract": SBOM_CONTRACT,
                "status": payload["status"],
                "component_count": payload["component_count"],
                "missing_license_count": len(payload["missing_license"]),
            }
        print(json.dumps(result, sort_keys=True))
        return 0
    except (OSError, ValueError, subprocess.SubprocessError, json.JSONDecodeError) as error:
        print(json.dumps({"contract": CONTRACT, "status": "error", "error": str(error)}, sort_keys=True))
        return 2


if __name__ == "__main__":
    sys.exit(main())
