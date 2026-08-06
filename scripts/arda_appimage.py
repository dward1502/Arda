#!/usr/bin/env python3
"""Fetch, verify, and run Arda's pinned AppImage assembly toolchain."""

from __future__ import annotations

import argparse
import hashlib
import os
import shutil
import subprocess
import sys
import urllib.request
from pathlib import Path

APPIMAGETOOL_URL = (
    "https://github.com/AppImage/appimagetool/releases/download/1.9.1/"
    "appimagetool-x86_64.AppImage"
)
APPIMAGETOOL_SHA256 = "ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0"
RUNTIME_URL = "https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-x86_64"
RUNTIME_SHA256 = "1cc49bcf1e2ccd593c379adb17c9f85a36d619088296504de95b1d06215aebbf"


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def require_sha256(path: Path, expected: str, label: str) -> None:
    if not path.is_file():
        raise FileNotFoundError(f"{label} not found: {path}")
    actual = sha256_file(path)
    if actual != expected:
        raise ValueError(f"{label} SHA-256 mismatch: expected {expected}, got {actual}")


def fetch_verified(url: str, output: Path, expected_sha256: str) -> Path:
    if output.is_file() and sha256_file(output) == expected_sha256:
        return output
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = output.with_name(f".{output.name}.tmp-{os.getpid()}")
    try:
        with urllib.request.urlopen(url, timeout=120) as response, temporary.open("wb") as handle:
            shutil.copyfileobj(response, handle)
        require_sha256(temporary, expected_sha256, output.name)
        os.chmod(temporary, 0o755)
        temporary.replace(output)
    finally:
        temporary.unlink(missing_ok=True)
    return output


def validate_appdir(appdir: Path) -> None:
    if not appdir.is_dir():
        raise FileNotFoundError(f"AppDir not found: {appdir}")
    if not (appdir / "AppRun").is_file():
        raise ValueError(f"AppDir has no AppRun: {appdir}")
    if not any(appdir.glob("*.desktop")) and not any(appdir.glob("usr/share/applications/*.desktop")):
        raise ValueError(f"AppDir has no desktop file: {appdir}")


def package_appimage(
    appdir: Path,
    output: Path,
    appimagetool: Path,
    runtime: Path,
    source_date_epoch: int,
    *,
    tool_sha256: str = APPIMAGETOOL_SHA256,
    runtime_sha256: str = RUNTIME_SHA256,
) -> Path:
    validate_appdir(appdir)
    require_sha256(appimagetool, tool_sha256, "appimagetool")
    require_sha256(runtime, runtime_sha256, "AppImage runtime")
    output.parent.mkdir(parents=True, exist_ok=True)
    output.unlink(missing_ok=True)
    environment = os.environ.copy()
    environment.update(
        {
            "APPIMAGE_EXTRACT_AND_RUN": "1",
            "ARCH": "x86_64",
            "SOURCE_DATE_EPOCH": str(source_date_epoch),
        }
    )
    subprocess.run(
        [
            str(appimagetool),
            "--no-appstream",
            "--runtime-file",
            str(runtime),
            str(appdir),
            str(output),
        ],
        env=environment,
        check=True,
    )
    if not output.is_file() or output.stat().st_size <= runtime.stat().st_size:
        raise RuntimeError(f"appimagetool did not produce a complete AppImage: {output}")
    with output.open("rb") as handle:
        handle.seek(8)
        if handle.read(3) != b"AI\x02":
            raise RuntimeError(f"output is not a type-2 AppImage: {output}")
    os.chmod(output, 0o755)
    return output


def parser() -> argparse.ArgumentParser:
    top = argparse.ArgumentParser(description=__doc__)
    sub = top.add_subparsers(dest="command", required=True)

    fetch = sub.add_parser("fetch")
    fetch.add_argument("--cache-dir", type=Path, default=Path.home() / ".cache/arda/appimage-tools")

    package = sub.add_parser("package")
    package.add_argument("--appdir", type=Path, required=True)
    package.add_argument("--output", type=Path, required=True)
    package.add_argument("--appimagetool", type=Path, required=True)
    package.add_argument("--runtime", type=Path, required=True)
    package.add_argument("--source-date-epoch", type=int, required=True)
    return top


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    try:
        if args.command == "fetch":
            tool = fetch_verified(APPIMAGETOOL_URL, args.cache_dir / "appimagetool-1.9.1-x86_64.AppImage", APPIMAGETOOL_SHA256)
            runtime = fetch_verified(RUNTIME_URL, args.cache_dir / f"runtime-x86_64-{RUNTIME_SHA256[:12]}", RUNTIME_SHA256)
            print(f"APPIMAGETOOL={tool}")
            print(f"APPIMAGE_RUNTIME={runtime}")
        else:
            result = package_appimage(
                args.appdir.resolve(),
                args.output.resolve(),
                args.appimagetool.resolve(),
                args.runtime.resolve(),
                args.source_date_epoch,
            )
            print(result)
    except (FileNotFoundError, RuntimeError, ValueError, subprocess.CalledProcessError) as error:
        print(f"appimage: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
