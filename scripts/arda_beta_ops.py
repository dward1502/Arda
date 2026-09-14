#!/usr/bin/env python3
"""Private-beta install, recovery, and redacted diagnostics operations.

All mutable operations are constrained to operator-owned XDG state roots. The
Arda source repository is used only for read-only readiness and identity checks.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path, PurePosixPath
from typing import Any, Iterable

CONTRACT = "arda.private_beta_ops.v1"
COMPATIBILITY_CONTRACT = "arda.release-compatibility.v1"
RELEASE_MANIFEST_CONTRACT = "arda.release-manifest.v1"
UPGRADE_CONTRACT = "arda.release-upgrade.v1"
ARCHIVE_MANIFEST = "manifest.json"
STATE_KINDS = ("config", "data")
RESET_KINDS = ("config", "data", "launcher_data", "cache", "runtime")
SECRET_NAME_RE = re.compile(
    r"(^|[._-])(secrets?|credentials?|tokens?|passwords?|passwd|api[._-]?keys?|private[._-]?keys?)([._-]|$)|^\.env$|\.env$",
    re.IGNORECASE,
)
SECRET_VALUE_RE = re.compile(
    r"(?im)^(\s*(?:export\s+)?[A-Za-z0-9_.-]*(?:KEY|TOKEN|SECRET|PASSWORD|PASSWD|CREDENTIAL)[A-Za-z0-9_.-]*\s*[:=]\s*)([^\r\n#]+)"
)
BEARER_RE = re.compile(r"(?i)\b(Bearer\s+)[A-Za-z0-9._~+/=-]+")
URL_CREDENTIAL_RE = re.compile(r"(?i)(https?://)([^/@\s:]+):([^/@\s]+)@")
SUPPORTED_PROFILE = {
    "profile_id": "bluefin-lts-10-x86_64",
    "system": "Linux",
    "machine": "x86_64",
    "os_id": "centos",
    "version_id": "10",
    "pretty_name_prefix": "Bluefin LTS",
}
RELEASE_SCHEMAS = {
    "project_contract": "arda.project-contract.v1",
    "run_event": "arda.run-event.v1",
    "execution_receipt": "arda.execution-receipt.v1",
    "private_beta_operations": CONTRACT,
}


class BetaOpsError(RuntimeError):
    """Expected operator-facing failure."""


@dataclass(frozen=True)
class Layout:
    home: Path
    root: Path
    config: Path
    data: Path
    launcher_data: Path
    cache: Path
    runtime: Path
    install_lib: Path
    install_bin: Path
    desktop_file: Path
    operation_state: Path


def utc_stamp() -> str:
    return datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")


def utc_iso() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def env_path(name: str, fallback: Path) -> Path:
    value = os.environ.get(name)
    return Path(value).expanduser() if value else fallback


def layout_from_args(args: argparse.Namespace) -> Layout:
    home = Path(args.home or os.environ.get("HOME", "~")).expanduser().resolve()
    root = Path(args.root or os.environ.get("ARDA_ROOT", Path.cwd())).expanduser().resolve()
    if args.home:
        # An explicit HOME denotes an isolated profile. Do not leak the caller's
        # inherited XDG roots (especially XDG_RUNTIME_DIR) into that profile.
        xdg_config = home / ".config"
        xdg_data = home / ".local" / "share"
        xdg_cache = home / ".cache"
        xdg_state = home / ".local" / "state"
        xdg_runtime = home / ".local" / "run"
    else:
        xdg_config = env_path("XDG_CONFIG_HOME", home / ".config")
        xdg_data = env_path("XDG_DATA_HOME", home / ".local" / "share")
        xdg_cache = env_path("XDG_CACHE_HOME", home / ".cache")
        xdg_state = env_path("XDG_STATE_HOME", home / ".local" / "state")
        xdg_runtime_raw = os.environ.get("XDG_RUNTIME_DIR")
        xdg_runtime = (
            Path(xdg_runtime_raw).expanduser() if xdg_runtime_raw else home / ".local" / "run"
        )
    return Layout(
        home=home,
        root=root,
        config=env_path("ARDA_CONFIG_DIR", xdg_config / "arda").resolve(),
        data=env_path("ARDA_DATA_DIR", xdg_data / "arda").resolve(),
        launcher_data=env_path("ARDA_LAUNCHER_DATA_DIR", xdg_data / "arda.launcher").resolve(),
        cache=env_path("ARDA_CACHE_DIR", xdg_cache / "arda").resolve(),
        runtime=env_path("ARDA_RUNTIME_DIR", xdg_runtime / "arda").resolve(),
        install_lib=(home / ".local" / "lib" / "arda").resolve(),
        # Keep managed link paths lexical. Resolving an existing launcher symlink
        # aliases install_bin to its target and leaves a broken link on uninstall.
        install_bin=home / ".local" / "bin" / "arda-launcher",
        desktop_file=xdg_data / "applications" / "arda-launcher.desktop",
        operation_state=(xdg_state / "arda").resolve(),
    )


def is_relative_to(path: Path, parent: Path) -> bool:
    try:
        path.relative_to(parent)
        return True
    except ValueError:
        return False


def validate_state_path(path: Path, layout: Layout) -> None:
    resolved = path.resolve()
    forbidden = (Path("/"), layout.home, layout.root)
    if resolved in forbidden:
        raise BetaOpsError(f"refusing unsafe state root: {resolved}")
    if is_relative_to(layout.root, resolved):
        raise BetaOpsError(f"state root would contain the source repository: {resolved}")
    if is_relative_to(resolved, layout.root):
        raise BetaOpsError(f"state root is inside the source repository: {resolved}")
    if not is_relative_to(resolved, layout.home):
        raise BetaOpsError(f"state root must remain under the selected HOME: {resolved}")


def validate_layout(layout: Layout) -> None:
    for kind in RESET_KINDS:
        validate_state_path(getattr(layout, kind), layout)
    validate_state_path(layout.operation_state, layout)
    validate_state_path(layout.install_lib, layout)
    validate_state_path(layout.install_bin, layout)
    validate_state_path(layout.desktop_file, layout)


def rel_display(path: Path, layout: Layout) -> str:
    if is_relative_to(path, layout.root):
        relative = path.relative_to(layout.root).as_posix()
        return "$ARDA_ROOT" if relative == "." else "$ARDA_ROOT/" + relative
    if is_relative_to(path, layout.home):
        relative = path.relative_to(layout.home).as_posix()
        return "$HOME" if relative == "." else "$HOME/" + relative
    return str(path)


def json_print(payload: dict[str, Any]) -> None:
    print(json.dumps(payload, indent=2, sort_keys=True))


def hash_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def parse_os_release(path: Path = Path("/etc/os-release")) -> dict[str, str]:
    values: dict[str, str] = {}
    try:
        raw = path.read_text(encoding="utf-8")
    except OSError:
        return values
    for line in raw.splitlines():
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        values[key] = value.strip().strip('"').strip("'")
    return values


def compatibility(
    os_release: dict[str, str] | None = None,
    machine: str | None = None,
) -> dict[str, Any]:
    observed = os_release if os_release is not None else parse_os_release()
    observed_machine = machine or platform.machine()
    supported = (
        platform.system() == SUPPORTED_PROFILE["system"]
        and observed_machine == SUPPORTED_PROFILE["machine"]
        and observed.get("ID") == SUPPORTED_PROFILE["os_id"]
        and observed.get("VERSION_ID") == SUPPORTED_PROFILE["version_id"]
        and observed.get("PRETTY_NAME", "").startswith(SUPPORTED_PROFILE["pretty_name_prefix"])
    )
    status = "supported" if supported else "unsupported"
    return {
        "contract": COMPATIBILITY_CONTRACT,
        "status": status,
        "profile_id": SUPPORTED_PROFILE["profile_id"] if supported else None,
        "supported_profile": SUPPORTED_PROFILE,
        "observed": {
            "system": platform.system(),
            "machine": observed_machine,
            "os_id": observed.get("ID", "unknown"),
            "version_id": observed.get("VERSION_ID", "unknown"),
            "pretty_name": observed.get("PRETTY_NAME", "unknown"),
        },
        "message": (
            f"supported release profile: {SUPPORTED_PROFILE['profile_id']}"
            if supported
            else "unsupported profile; refusing before installation or state mutation"
        ),
    }


def require_supported_profile() -> dict[str, Any]:
    result = compatibility()
    if result["status"] != "supported":
        observed = result.get("observed", {})
        detail = " ".join(
            value
            for value in (observed.get("pretty_name"), observed.get("machine"))
            if value
        )
        suffix = f": {detail}" if detail else ""
        raise BetaOpsError(f"{result['message']}{suffix}")
    return result


def command_line(command: list[str], cwd: Path | None = None) -> str:
    result = command_version(command, cwd)
    if not result["available"] or result.get("exit_code") != 0:
        return "unavailable"
    return str(result["detail"])


def declared_build_inputs(layout: Layout) -> dict[str, Any]:
    root = layout.root
    epoch = os.environ.get("SOURCE_DATE_EPOCH")
    if not epoch:
        epoch = command_line(["git", "show", "-s", "--format=%ct", "HEAD"], root)
    rustc_verbose = (
        subprocess.run(
            ["rustc", "-vV"],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        if shutil.which("rustc")
        else None
    )
    target = "unavailable"
    if rustc_verbose and rustc_verbose.returncode == 0:
        target = next(
            (
                line.split(":", 1)[1].strip()
                for line in rustc_verbose.stdout.splitlines()
                if line.startswith("host:")
            ),
            "unavailable",
        )
    return {
        "source_commit": command_line(["git", "rev-parse", "HEAD"], root),
        "source_tree_identity": source_identity(root),
        "source_date_epoch": int(epoch) if str(epoch).isdigit() else epoch,
        "rustc": command_line(["rustc", "--version"]),
        "cargo": command_line(["cargo", "--version"]),
        "node": command_line(["node", "--version"]),
        "pnpm": command_line(["pnpm", "--version"]),
        "target": target,
        "cargo_lock_sha256": hash_file(root / "Cargo.lock")
        if (root / "Cargo.lock").is_file()
        else None,
        "launcher_pnpm_lock_sha256": hash_file(root / "apps/arda-launcher/pnpm-lock.yaml")
        if (root / "apps/arda-launcher/pnpm-lock.yaml").is_file()
        else None,
    }


def generate_release_manifest(
    layout: Layout,
    artifact: Path,
    output: Path,
    version: str,
    build_inputs: dict[str, Any] | None = None,
    checksums_output: Path | None = None,
) -> dict[str, Any]:
    validate_layout(layout)
    artifact = artifact.expanduser().resolve()
    output = output.expanduser().resolve()
    if not artifact.is_file():
        raise BetaOpsError(f"launcher artifact not found: {artifact}")
    if not version.strip():
        raise BetaOpsError("release version cannot be empty")
    payload = {
        "contract": RELEASE_MANIFEST_CONTRACT,
        "artifact": {
            "name": "arda-launcher",
            "version": version,
            "sha256": hash_file(artifact),
            "size": artifact.stat().st_size,
        },
        "build_inputs": build_inputs if build_inputs is not None else declared_build_inputs(layout),
        "supported_profiles": [SUPPORTED_PROFILE],
        "schemas": RELEASE_SCHEMAS,
        "rollback_compatibility": {
            "state_preserving": True,
            "compatible_private_beta_contract": CONTRACT,
            "requires_schema_migration": False,
            "strategy": "restore prior launcher bytes; preserve or restore verified config/data archive",
        },
    }
    canonical = json.dumps(payload, indent=2, sort_keys=True) + "\n"
    release_id = "sha256:" + hashlib.sha256(canonical.encode("utf-8")).hexdigest()
    payload["release_id"] = release_id
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = output.with_name(output.name + ".tmp")
    temporary.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    os.chmod(temporary, stat.S_IRUSR | stat.S_IWUSR)
    os.replace(temporary, output)
    checksum_path: Path | None = None
    if checksums_output is not None:
        checksum_path = checksums_output.expanduser().resolve()
        checksum_path.parent.mkdir(parents=True, exist_ok=True)
        checksum_path.write_text(
            f"{hash_file(artifact)}  {artifact.name}\n{hash_file(output)}  {output.name}\n",
            encoding="utf-8",
        )
        os.chmod(checksum_path, stat.S_IRUSR | stat.S_IWUSR)
    return {
        "contract": RELEASE_MANIFEST_CONTRACT,
        "operation": "release-manifest",
        "status": "ok",
        "release_id": release_id,
        "manifest": str(output),
        "manifest_sha256": hash_file(output),
        "artifact_sha256": payload["artifact"]["sha256"],
        "checksums": str(checksum_path) if checksum_path else None,
    }


def secret_named(path: Path) -> bool:
    return any(SECRET_NAME_RE.search(part) for part in path.parts)


def iter_regular_files(root: Path) -> Iterable[Path]:
    if not root.exists():
        return
    for path in sorted(root.rglob("*")):
        if path.is_symlink():
            raise BetaOpsError(f"refusing symlink in state tree: {path}")
        if path.is_file():
            yield path


def backup(layout: Layout, output: Path, include_secrets: bool = False) -> dict[str, Any]:
    validate_layout(layout)
    output = output.expanduser().resolve()
    for kind in RESET_KINDS:
        if is_relative_to(output, getattr(layout, kind)):
            raise BetaOpsError("backup output cannot be inside an Arda state root")
    output.parent.mkdir(parents=True, exist_ok=True)

    files: list[dict[str, Any]] = []
    excluded: list[str] = []
    with tempfile.TemporaryDirectory(prefix="arda-backup-") as raw:
        manifest_path = Path(raw) / ARCHIVE_MANIFEST
        candidates: list[tuple[str, Path, Path]] = []
        for kind in STATE_KINDS:
            state_root = getattr(layout, kind)
            for path in iter_regular_files(state_root):
                relative = path.relative_to(state_root)
                archive_name = f"{kind}/{relative.as_posix()}"
                if not include_secrets and secret_named(relative):
                    excluded.append(archive_name)
                    continue
                candidates.append((archive_name, path, relative))
                files.append(
                    {
                        "path": archive_name,
                        "sha256": hash_file(path),
                        "size": path.stat().st_size,
                    }
                )

        manifest = {
            "contract": CONTRACT,
            "operation": "backup",
            "created_at_utc": utc_iso(),
            "state_roots": list(STATE_KINDS),
            "includes_secrets": include_secrets,
            "excluded_secret_paths": excluded,
            "files": files,
        }
        manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        temporary = output.with_name(output.name + ".tmp")
        try:
            with tarfile.open(temporary, "w:gz", format=tarfile.PAX_FORMAT) as archive:
                archive.add(manifest_path, arcname=ARCHIVE_MANIFEST, recursive=False)
                for archive_name, path, _ in candidates:
                    archive.add(path, arcname=archive_name, recursive=False)
            os.chmod(temporary, stat.S_IRUSR | stat.S_IWUSR)
            os.replace(temporary, output)
        finally:
            temporary.unlink(missing_ok=True)

    return {
        "contract": CONTRACT,
        "operation": "backup",
        "status": "ok",
        "archive": str(output),
        "file_count": len(files),
        "excluded_secret_count": len(excluded),
        "includes_secrets": include_secrets,
    }


def safe_member_name(name: str) -> PurePosixPath:
    member = PurePosixPath(name)
    if member.is_absolute() or ".." in member.parts:
        raise BetaOpsError(f"unsafe archive member: {name}")
    if name == ARCHIVE_MANIFEST:
        return member
    if not member.parts or member.parts[0] not in STATE_KINDS or len(member.parts) < 2:
        raise BetaOpsError(f"unexpected archive member: {name}")
    return member


def load_archive(archive_path: Path) -> tuple[dict[str, Any], list[tarfile.TarInfo]]:
    try:
        with tarfile.open(archive_path, "r:gz") as archive:
            members = archive.getmembers()
            for member in members:
                safe_member_name(member.name)
                if not (member.isfile() or member.name == ARCHIVE_MANIFEST):
                    raise BetaOpsError(f"archive contains non-file member: {member.name}")
                if member.issym() or member.islnk():
                    raise BetaOpsError(f"archive contains link member: {member.name}")
            manifest_member = archive.getmember(ARCHIVE_MANIFEST)
            handle = archive.extractfile(manifest_member)
            if handle is None:
                raise BetaOpsError("backup manifest is unreadable")
            manifest = json.load(handle)
    except (tarfile.TarError, KeyError, json.JSONDecodeError) as error:
        raise BetaOpsError(f"invalid backup archive: {error}") from error
    if manifest.get("contract") != CONTRACT or manifest.get("operation") != "backup":
        raise BetaOpsError("backup manifest contract mismatch")
    manifest_files = manifest.get("files")
    if not isinstance(manifest_files, list) or not all(
        isinstance(entry, dict) and isinstance(entry.get("path"), str) for entry in manifest_files
    ):
        raise BetaOpsError("backup manifest files are malformed")
    expected_names = [entry["path"] for entry in manifest_files]
    archive_names = [member.name for member in members if member.name != ARCHIVE_MANIFEST]
    if (
        sum(member.name == ARCHIVE_MANIFEST for member in members) != 1
        or len(expected_names) != len(set(expected_names))
        or len(archive_names) != len(set(archive_names))
        or sorted(expected_names) != sorted(archive_names)
    ):
        raise BetaOpsError("archive members do not match backup manifest")
    return manifest, members


def restore(layout: Layout, archive_path: Path, force: bool = False) -> dict[str, Any]:
    validate_layout(layout)
    archive_path = archive_path.expanduser().resolve()
    manifest, _ = load_archive(archive_path)
    expected = {entry["path"]: entry["sha256"] for entry in manifest.get("files", [])}
    if not expected and manifest.get("files") != []:
        raise BetaOpsError("backup manifest files are malformed")

    for kind in STATE_KINDS:
        target = getattr(layout, kind)
        if target.exists() and any(target.iterdir()) and not force:
            raise BetaOpsError(f"restore target is not empty: {target}; use --force to displace it")

    staging_parent = layout.operation_state / "restore-staging"
    staging_parent.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix="restore-", dir=staging_parent))
    displaced_root = layout.operation_state / "displaced" / utc_stamp()
    displaced: dict[str, str] = {}
    installed: list[tuple[Path, Path | None]] = []
    try:
        with tarfile.open(archive_path, "r:gz") as archive:
            for member in archive.getmembers():
                if member.name == ARCHIVE_MANIFEST:
                    continue
                pure = safe_member_name(member.name)
                destination = staging.joinpath(*pure.parts)
                destination.parent.mkdir(parents=True, exist_ok=True)
                handle = archive.extractfile(member)
                if handle is None:
                    raise BetaOpsError(f"cannot read archive member: {member.name}")
                with destination.open("wb") as output:
                    shutil.copyfileobj(handle, output)
                os.chmod(destination, stat.S_IRUSR | stat.S_IWUSR)

        for archive_name, expected_hash in expected.items():
            pure = safe_member_name(archive_name)
            restored_file = staging.joinpath(*pure.parts)
            if not restored_file.is_file() or hash_file(restored_file) != expected_hash:
                raise BetaOpsError(f"backup hash verification failed: {archive_name}")

        for kind in STATE_KINDS:
            source = staging / kind
            source.mkdir(parents=True, exist_ok=True)
            target = getattr(layout, kind)
            previous: Path | None = None
            if target.exists():
                if any(target.iterdir()):
                    displaced_root.mkdir(parents=True, exist_ok=True)
                    previous = displaced_root / kind
                    os.replace(target, previous)
                    displaced[kind] = str(previous)
                else:
                    target.rmdir()
            target.parent.mkdir(parents=True, exist_ok=True)
            os.replace(source, target)
            installed.append((target, previous))
    except Exception:
        for target, previous in reversed(installed):
            if target.exists():
                shutil.rmtree(target)
            if previous and previous.exists():
                os.replace(previous, target)
        raise
    finally:
        shutil.rmtree(staging, ignore_errors=True)

    return {
        "contract": CONTRACT,
        "operation": "restore",
        "status": "ok",
        "archive": str(archive_path),
        "file_count": len(expected),
        "displaced": displaced,
        "secrets_were_included": bool(manifest.get("includes_secrets")),
    }


def reset(layout: Layout, backup_output: Path) -> dict[str, Any]:
    validate_layout(layout)
    backup_receipt = backup(layout, backup_output, include_secrets=False)
    quarantine = layout.operation_state / "resets" / utc_stamp()
    moved: dict[str, str] = {}
    for kind in RESET_KINDS:
        source = getattr(layout, kind)
        if not source.exists():
            continue
        destination = quarantine / kind
        destination.parent.mkdir(parents=True, exist_ok=True)
        os.replace(source, destination)
        moved[kind] = str(destination)
    return {
        "contract": CONTRACT,
        "operation": "reset",
        "status": "ok",
        "backup": backup_receipt,
        "quarantine": str(quarantine),
        "moved": moved,
        "source_repository_touched": False,
    }


def redact_text(text: str, layout: Layout) -> str:
    text = SECRET_VALUE_RE.sub(r"\1<redacted>", text)
    text = BEARER_RE.sub(r"\1<redacted>", text)
    text = URL_CREDENTIAL_RE.sub(r"\1<redacted>:<redacted>@", text)
    text = text.replace(str(layout.root), "$ARDA_ROOT")
    text = text.replace(str(layout.home), "$HOME")
    return text


def command_version(command: list[str], cwd: Path | None = None) -> dict[str, Any]:
    try:
        result = subprocess.run(
            command,
            cwd=cwd,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=10,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return {"available": False, "detail": type(error).__name__}
    first_line = result.stdout.strip().splitlines()[0] if result.stdout.strip() else ""
    return {"available": True, "exit_code": result.returncode, "detail": first_line[:240]}


def tree_summary(path: Path) -> dict[str, Any]:
    files = 0
    bytes_total = 0
    if path.exists():
        for item in path.rglob("*"):
            if item.is_file() and not item.is_symlink():
                files += 1
                bytes_total += item.stat().st_size
    return {"exists": path.exists(), "files": files, "bytes": bytes_total}


def tree_digest(path: Path) -> str:
    digest = hashlib.sha256()
    if not path.exists():
        return digest.hexdigest()
    for item in iter_regular_files(path):
        relative = item.relative_to(path).as_posix().encode("utf-8")
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        digest.update(bytes.fromhex(hash_file(item)))
    return digest.hexdigest()


def source_identity(root: Path) -> dict[str, Any]:
    if not (root / ".git").exists():
        return {"kind": "tree", "sha256": tree_digest(root)}
    status = subprocess.run(
        ["git", "status", "--short"],
        cwd=root,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    status_text = status.stdout if status.returncode == 0 else "unavailable"
    return {
        "kind": "git",
        "head": command_line(["git", "rev-parse", "HEAD"], root),
        "status_sha256": hashlib.sha256(status_text.encode("utf-8")).hexdigest(),
    }


def launcher_dependency_status(launcher: Path) -> tuple[bool, str]:
    if not launcher.is_file() or not os.access(launcher, os.X_OK):
        return False, "launcher is absent or not executable"
    ldd = shutil.which("ldd")
    if ldd is None:
        return False, "ldd is unavailable; native launcher dependencies could not be checked"
    try:
        result = subprocess.run(
            [ldd, str(launcher)],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=10,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return False, f"native dependency check failed: {type(error).__name__}"
    missing = sorted(
        line.split("=>", 1)[0].strip()
        for line in result.stdout.splitlines()
        if "=> not found" in line
    )
    if missing:
        return False, "missing shared libraries: " + ", ".join(missing)
    if result.returncode != 0:
        detail = result.stdout.strip().splitlines()[0] if result.stdout.strip() else f"ldd exited {result.returncode}"
        return False, f"native dependency check failed: {detail[:240]}"
    return True, "all native shared libraries resolved"


def readiness(layout: Layout) -> dict[str, Any]:
    validate_layout(layout)
    checks: list[dict[str, Any]] = []

    def add(check_id: str, passed: bool, severity: str, evidence: str, recovery: str) -> None:
        checks.append(
            {
                "check_id": check_id,
                "status": "pass" if passed else ("fail" if severity == "high" else "warn"),
                "severity": severity,
                "evidence": evidence,
                "recovery": recovery,
            }
        )

    profile = compatibility()
    add(
        "release.supported_profile",
        profile["status"] == "supported",
        "high",
        profile["message"],
        f"Use the declared {SUPPORTED_PROFILE['profile_id']} profile or a separately qualified release.",
    )
    add(
        "source.contract_registry",
        (layout.root / "core/state/contract_registry.json").is_file(),
        "high",
        rel_display(layout.root / "core/state/contract_registry.json", layout),
        "Set ARDA_ROOT to a complete Arda runtime root.",
    )
    add(
        "source.operator_instructions",
        (layout.root / "AGENTS.md").is_file(),
        "medium",
        rel_display(layout.root / "AGENTS.md", layout),
        "Restore the release's operator instruction file.",
    )
    add(
        "recovery.tool",
        (layout.root / "scripts/arda_beta_ops.py").is_file(),
        "high",
        rel_display(layout.root / "scripts/arda_beta_ops.py", layout),
        "Reinstall the matching Arda private-beta release.",
    )
    add(
        "recovery.documentation",
        (layout.root / "docs/operator/private-beta-install-recovery.md").is_file(),
        "medium",
        rel_display(layout.root / "docs/operator/private-beta-install-recovery.md", layout),
        "Reinstall the matching Arda private-beta documentation.",
    )
    launcher = layout.install_lib / "arda-launcher"
    add(
        "launcher.installed",
        launcher.is_file() and os.access(launcher, os.X_OK),
        "high",
        rel_display(launcher, layout),
        "Run arda_beta_ops.py install-launcher with the verified release binary.",
    )
    dependencies_ok, dependencies_evidence = launcher_dependency_status(launcher)
    add(
        "launcher.dynamic_libraries",
        dependencies_ok,
        "high",
        dependencies_evidence,
        "Install the documented GTK3 and WebKitGTK 4.1 runtime packages, then rerun readiness.",
    )
    for command in ("git", "cargo", "node", "pnpm"):
        available = shutil.which(command) is not None
        severity = "high" if command in {"git", "cargo"} else "medium"
        add(
            f"tool.{command}",
            available,
            severity,
            shutil.which(command) or "not found on PATH",
            f"Install {command} and rerun readiness.",
        )
    for endpoint in ("MANWE_BASE_URL", "ARDA_MANWE_BASE_URL"):
        if os.environ.get(endpoint):
            add(
                "provider.manwe_endpoint",
                True,
                "high",
                f"configured via {endpoint}",
                "none",
            )
            break
    else:
        add(
            "provider.manwe_endpoint",
            False,
            "high",
            "MANWE_BASE_URL and ARDA_MANWE_BASE_URL are unset",
            "Configure a tested Manwe endpoint before a live-provider run.",
        )

    counts = {status: sum(1 for check in checks if check["status"] == status) for status in ("pass", "warn", "fail")}
    gate = "fail" if counts["fail"] else ("warn" if counts["warn"] else "pass")
    return {
        "contract": CONTRACT,
        "operation": "readiness",
        "generated_at_utc": utc_iso(),
        "gate_status": gate,
        "summary": counts,
        "checks": checks,
        "state": {kind: tree_summary(getattr(layout, kind)) for kind in RESET_KINDS},
        "source_repository": rel_display(layout.root, layout),
    }


def diagnostics(layout: Layout, output: Path) -> dict[str, Any]:
    validate_layout(layout)
    output = output.expanduser().resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "contract": CONTRACT,
        "operation": "diagnostics",
        "generated_at_utc": utc_iso(),
        "platform": {
            "system": platform.system(),
            "release": platform.release(),
            "machine": platform.machine(),
            "python": platform.python_version(),
        },
        "tools": {
            "git": command_version(["git", "--version"]),
            "cargo": command_version(["cargo", "--version"]),
            "rustc": command_version(["rustc", "--version"]),
            "node": command_version(["node", "--version"]),
            "pnpm": command_version(["pnpm", "--version"]),
        },
        "readiness": readiness(layout),
    }
    if (layout.root / ".git").exists():
        payload["source_identity"] = {
            "head": command_version(["git", "rev-parse", "--short", "HEAD"], layout.root),
            "branch": command_version(["git", "branch", "--show-current"], layout.root),
            "status": command_version(["git", "status", "--short"], layout.root),
        }
    installed_manifest = layout.install_lib / "install-manifest.json"
    if installed_manifest.is_file():
        try:
            installed = json.loads(installed_manifest.read_text(encoding="utf-8"))
            payload["installed_release"] = {
                "artifact_sha256": installed.get("artifact_sha256"),
                "version": installed.get("release_version"),
                "release_id": installed.get("release_id"),
                "schemas": installed.get("schemas"),
                "rollback_result": installed.get("rollback_result"),
                "backup_sha256": installed.get("upgrade", {}).get("backup_sha256")
                or installed.get("backup_sha256"),
            }
        except (OSError, json.JSONDecodeError):
            payload["installed_release"] = {"status": "unreadable"}

    with tempfile.TemporaryDirectory(prefix="arda-diagnostics-") as raw:
        staging = Path(raw)
        (staging / "diagnostics.json").write_text(
            redact_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", layout),
            encoding="utf-8",
        )
        config_out = staging / "config-redacted"
        if layout.config.exists():
            for source in iter_regular_files(layout.config):
                relative = source.relative_to(layout.config)
                if secret_named(relative) or source.stat().st_size > 256 * 1024:
                    continue
                try:
                    text = source.read_text(encoding="utf-8")
                except UnicodeDecodeError:
                    continue
                destination = config_out / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_text(redact_text(text, layout), encoding="utf-8")
        temporary = output.with_name(output.name + ".tmp")
        try:
            with tarfile.open(temporary, "w:gz", format=tarfile.PAX_FORMAT) as archive:
                for source in sorted(staging.rglob("*")):
                    if source.is_file():
                        archive.add(source, arcname=source.relative_to(staging).as_posix(), recursive=False)
            os.chmod(temporary, stat.S_IRUSR | stat.S_IWUSR)
            os.replace(temporary, output)
        finally:
            temporary.unlink(missing_ok=True)

    return {
        "contract": CONTRACT,
        "operation": "diagnostics",
        "status": "ok",
        "bundle": str(output),
        "redaction": "secret-named files omitted; secret-like values and absolute operator paths redacted",
    }


def install_launcher(
    layout: Layout,
    artifact: Path,
    release_manifest: Path | None = None,
) -> dict[str, Any]:
    validate_layout(layout)
    artifact = artifact.expanduser().resolve()
    if not artifact.is_file():
        raise BetaOpsError(f"launcher artifact not found: {artifact}")
    if not os.access(artifact, os.X_OK):
        raise BetaOpsError(f"launcher artifact is not executable: {artifact}")
    release: dict[str, Any] | None = None
    if release_manifest is not None:
        release_manifest = release_manifest.expanduser().resolve()
        try:
            parsed_release = json.loads(release_manifest.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            raise BetaOpsError(f"invalid release manifest: {error}") from error
        if parsed_release.get("contract") != RELEASE_MANIFEST_CONTRACT:
            raise BetaOpsError("release manifest contract mismatch")
        if parsed_release.get("artifact", {}).get("sha256") != hash_file(artifact):
            raise BetaOpsError("release manifest artifact checksum mismatch")
        release = parsed_release
    layout.install_lib.mkdir(parents=True, exist_ok=True)
    destination = layout.install_lib / "arda-launcher"
    temporary = destination.with_name(destination.name + ".tmp")
    shutil.copy2(artifact, temporary)
    os.chmod(temporary, stat.S_IRUSR | stat.S_IWUSR | stat.S_IXUSR)
    os.replace(temporary, destination)
    layout.install_bin.parent.mkdir(parents=True, exist_ok=True)
    layout.install_bin.unlink(missing_ok=True)
    layout.install_bin.symlink_to(destination)
    desktop_template = (
        layout.root / "apps/arda-launcher/packaging/linux/io.arda.Launcher.desktop"
    )
    if not desktop_template.is_file():
        raise BetaOpsError(f"launcher desktop template missing: {desktop_template}")
    desktop_text = desktop_template.read_text(encoding="utf-8").replace(
        "@ARDA_LAUNCHER_EXEC@", str(destination)
    )
    layout.desktop_file.parent.mkdir(parents=True, exist_ok=True)
    desktop_temporary = layout.desktop_file.with_suffix(".desktop.tmp")
    desktop_temporary.write_text(desktop_text, encoding="utf-8")
    os.replace(desktop_temporary, layout.desktop_file)
    icon_source = layout.root / "apps/arda-launcher/src-tauri/icons/128x128.png"
    icon_destination = (
        layout.desktop_file.parent.parent
        / "icons/hicolor/128x128/apps/arda-launcher.png"
    )
    if not icon_source.is_file():
        raise BetaOpsError(f"launcher icon missing: {icon_source}")
    icon_destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(icon_source, icon_destination)
    if release_manifest is not None:
        shutil.copy2(release_manifest, layout.install_lib / "release-manifest.json")
    manifest = layout.install_lib / "install-manifest.json"
    install_payload = {
        "contract": CONTRACT,
        "installed_at_utc": utc_iso(),
        "artifact_sha256": hash_file(destination),
        "managed_paths": [
            str(destination),
            str(layout.install_bin),
            str(layout.desktop_file),
            str(icon_destination),
        ],
    }
    if release is not None:
        install_payload.update(
            {
                "release_version": release["artifact"]["version"],
                "release_id": release["release_id"],
                "schemas": release["schemas"],
            }
        )
    manifest.write_text(
        json.dumps(install_payload, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return {
        "contract": CONTRACT,
        "operation": "install-launcher",
        "status": "ok",
        "launcher": str(destination),
        "command": str(layout.install_bin),
        "desktop_file": str(layout.desktop_file),
        "sha256": hash_file(destination),
        "release_id": release.get("release_id") if release else None,
    }


def upgrade_launcher(
    layout: Layout,
    candidate: Path,
    backup_output: Path,
    release_manifest: Path | None = None,
) -> dict[str, Any]:
    validate_layout(layout)
    installed = layout.install_lib / "arda-launcher"
    if not installed.is_file():
        raise BetaOpsError("cannot upgrade: no Stage 4 launcher is installed")
    source_before = source_identity(layout.root)
    prior_sha = hash_file(installed)
    backup_output = backup_output.expanduser().resolve()
    backup(layout, backup_output)
    rollback_dir = layout.operation_state / "upgrades" / f"{utc_stamp()}-{prior_sha[:12]}"
    rollback_dir.mkdir(parents=True, exist_ok=False)
    prior_artifact = rollback_dir / "arda-launcher"
    shutil.copy2(installed, prior_artifact)
    prior_manifest = layout.install_lib / "install-manifest.json"
    if prior_manifest.is_file():
        shutil.copy2(prior_manifest, rollback_dir / "install-manifest.json")
    try:
        installed_receipt = install_launcher(layout, candidate, release_manifest=release_manifest)
    except Exception:
        shutil.copy2(prior_artifact, installed)
        raise
    installed_manifest_path = layout.install_lib / "install-manifest.json"
    installed_manifest = json.loads(installed_manifest_path.read_text(encoding="utf-8"))
    installed_manifest["upgrade"] = {
        "backup_sha256": hash_file(backup_output),
        "prior_artifact_sha256": prior_sha,
        "rollback_available": True,
    }
    installed_manifest_path.write_text(
        json.dumps(installed_manifest, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    receipt_path = rollback_dir / "upgrade-receipt.json"
    receipt = {
        "contract": UPGRADE_CONTRACT,
        "operation": "upgrade",
        "status": "ok",
        "created_at_utc": utc_iso(),
        "source_identity_before": source_before,
        "prior_artifact": str(prior_artifact),
        "prior_artifact_sha256": prior_sha,
        "candidate_artifact_sha256": installed_receipt["sha256"],
        "pre_upgrade_backup": str(backup_output),
        "pre_upgrade_backup_sha256": hash_file(backup_output),
    }
    receipt_path.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return {**receipt, "rollback_receipt": str(receipt_path)}


def rollback_launcher(
    layout: Layout,
    upgrade_receipt: Path,
    state_archive: Path,
) -> dict[str, Any]:
    validate_layout(layout)
    try:
        receipt = json.loads(upgrade_receipt.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise BetaOpsError(f"invalid upgrade receipt: {error}") from error
    if receipt.get("contract") != UPGRADE_CONTRACT:
        raise BetaOpsError("upgrade receipt contract mismatch")
    prior_artifact = Path(receipt["prior_artifact"])
    if not prior_artifact.is_file() or hash_file(prior_artifact) != receipt["prior_artifact_sha256"]:
        raise BetaOpsError("rollback artifact checksum mismatch")
    source_before = receipt["source_identity_before"]
    run_truth_before = tree_digest(layout.data)
    secret_state: list[tuple[str, Path, bytes, int]] = []
    for kind in STATE_KINDS:
        state_root = getattr(layout, kind)
        for path in iter_regular_files(state_root):
            relative = path.relative_to(state_root)
            if secret_named(relative):
                secret_state.append((kind, relative, path.read_bytes(), stat.S_IMODE(path.stat().st_mode)))
    install_launcher(layout, prior_artifact)
    prior_manifest = upgrade_receipt.parent / "install-manifest.json"
    if prior_manifest.is_file():
        shutil.copy2(prior_manifest, layout.install_lib / "install-manifest.json")
    restored = restore(layout, state_archive, force=True)
    for kind, relative, content, mode in secret_state:
        destination = getattr(layout, kind) / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(content)
        os.chmod(destination, mode)
    run_truth_after = tree_digest(layout.data)
    source_after = source_identity(layout.root)
    rollback_result = {
        "status": "ok",
        "state_archive_sha256": hash_file(state_archive),
        "terminal_run_truth_preserved": run_truth_before == run_truth_after,
        "source_repository_touched": source_before != source_after,
    }
    install_manifest = layout.install_lib / "install-manifest.json"
    if install_manifest.is_file():
        installed_payload = json.loads(install_manifest.read_text(encoding="utf-8"))
        installed_payload["rollback_result"] = rollback_result
        installed_payload["backup_sha256"] = hash_file(state_archive)
        install_manifest.write_text(
            json.dumps(installed_payload, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    return {
        "contract": UPGRADE_CONTRACT,
        "operation": "rollback",
        "status": "ok",
        "restored_artifact_sha256": hash_file(layout.install_lib / "arda-launcher"),
        "restored_state_file_count": restored["file_count"],
        "terminal_run_truth_preserved": rollback_result["terminal_run_truth_preserved"],
        "source_repository_touched": rollback_result["source_repository_touched"],
    }


def uninstall_launcher(layout: Layout) -> dict[str, Any]:
    validate_layout(layout)
    removed: list[str] = []
    icon_file = layout.desktop_file.parent.parent / "icons/hicolor/128x128/apps/arda-launcher.png"
    for path in (layout.install_bin, layout.desktop_file, icon_file):
        if path.exists() or path.is_symlink():
            path.unlink()
            removed.append(str(path))
    if layout.install_lib.exists():
        shutil.rmtree(layout.install_lib)
        removed.append(str(layout.install_lib))
    return {
        "contract": CONTRACT,
        "operation": "uninstall-launcher",
        "status": "ok",
        "removed": removed,
        "state_preserved": [str(getattr(layout, kind)) for kind in STATE_KINDS],
        "source_repository_touched": False,
    }


def parser() -> argparse.ArgumentParser:
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--home", help="isolated operator HOME (defaults to $HOME)")
    common.add_argument("--root", help="read-only Arda runtime/source root (defaults to $ARDA_ROOT or cwd)")
    root_parser = argparse.ArgumentParser(description=__doc__)
    subparsers = root_parser.add_subparsers(dest="command", required=True)

    subparsers.add_parser("compatibility", parents=[common], help="check the declared Stage 5 Linux profile")
    subparsers.add_parser("readiness", parents=[common], help="emit honest private-beta readiness JSON")

    release_parser = subparsers.add_parser(
        "release-manifest", parents=[common], help="write a deterministic release manifest"
    )
    release_parser.add_argument("--artifact", required=True, type=Path)
    release_parser.add_argument("--output", required=True, type=Path)
    release_parser.add_argument("--version", required=True)
    release_parser.add_argument("--checksums-output", required=True, type=Path)

    backup_parser = subparsers.add_parser("backup", parents=[common], help="archive operator config/data state")
    backup_parser.add_argument("--output", required=True, type=Path)
    backup_parser.add_argument("--include-secrets", action="store_true")

    restore_parser = subparsers.add_parser("restore", parents=[common], help="restore a verified state archive")
    restore_parser.add_argument("--archive", required=True, type=Path)
    restore_parser.add_argument("--force", action="store_true", help="displace non-empty targets before restore")

    reset_parser = subparsers.add_parser("reset", parents=[common], help="backup then quarantine operator state")
    reset_parser.add_argument("--backup-output", required=True, type=Path)

    diagnostics_parser = subparsers.add_parser("diagnostics", parents=[common], help="write a redacted diagnostics archive")
    diagnostics_parser.add_argument("--output", required=True, type=Path)

    install_parser = subparsers.add_parser("install-launcher", parents=[common], help="install a verified launcher binary for one user")
    install_parser.add_argument("--artifact", required=True, type=Path)
    install_parser.add_argument("--release-manifest", type=Path)

    upgrade_parser = subparsers.add_parser(
        "upgrade-launcher", parents=[common], help="back up state and install a release candidate"
    )
    upgrade_parser.add_argument("--artifact", required=True, type=Path)
    upgrade_parser.add_argument("--backup-output", required=True, type=Path)
    upgrade_parser.add_argument("--release-manifest", type=Path)

    rollback_parser = subparsers.add_parser(
        "rollback-launcher", parents=[common], help="restore the prior launcher and verified run state"
    )
    rollback_parser.add_argument("--upgrade-receipt", required=True, type=Path)
    rollback_parser.add_argument("--state-archive", required=True, type=Path)

    subparsers.add_parser("uninstall-launcher", parents=[common], help="remove managed launcher files and preserve state")
    return root_parser


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    try:
        layout = layout_from_args(args)
        if args.command == "compatibility":
            result = compatibility()
            if result["status"] != "supported":
                json_print(result)
                return 2
        elif args.command == "readiness":
            result = readiness(layout)
        elif args.command == "release-manifest":
            require_supported_profile()
            result = generate_release_manifest(
                layout,
                args.artifact,
                args.output,
                args.version,
                checksums_output=args.checksums_output,
            )
        elif args.command == "backup":
            result = backup(layout, args.output, args.include_secrets)
        elif args.command == "restore":
            result = restore(layout, args.archive, args.force)
        elif args.command == "reset":
            result = reset(layout, args.backup_output)
        elif args.command == "diagnostics":
            result = diagnostics(layout, args.output)
        elif args.command == "install-launcher":
            require_supported_profile()
            result = install_launcher(layout, args.artifact, args.release_manifest)
        elif args.command == "upgrade-launcher":
            require_supported_profile()
            result = upgrade_launcher(layout, args.artifact, args.backup_output, args.release_manifest)
        elif args.command == "rollback-launcher":
            result = rollback_launcher(layout, args.upgrade_receipt, args.state_archive)
        elif args.command == "uninstall-launcher":
            result = uninstall_launcher(layout)
        else:
            raise BetaOpsError(f"unsupported operation: {args.command}")
        json_print(result)
        return 0
    except BetaOpsError as error:
        print(json.dumps({"contract": CONTRACT, "status": "error", "error": str(error)}), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
