#!/usr/bin/env python3
"""Static provenance gate for tag-bound Arda releases."""

from __future__ import annotations

import json
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REGISTRY = ROOT / "docs/contracts/PROVENANCE_AND_ATTRIBUTION.md"
REQUIRED_ROOT_FILES = (ROOT / "LICENSE", ROOT / "NOTICE", REGISTRY)
APP_MANIFESTS = (
    ROOT / "apps/arda-hud/package.json",
    ROOT / "apps/arda-launcher/package.json",
)


def fail(message: str) -> None:
    print(f"provenance gate: {message}", file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    missing = [str(path.relative_to(ROOT)) for path in REQUIRED_ROOT_FILES if not path.is_file()]
    if missing:
        fail(f"missing required file(s): {', '.join(missing)}")

    license_text = (ROOT / "LICENSE").read_text(encoding="utf-8")
    if not license_text.startswith("MIT License\n"):
        fail("root LICENSE is not the reviewed MIT license")

    notice_text = (ROOT / "NOTICE").read_text(encoding="utf-8")
    if "third-party license inventory" not in notice_text:
        fail("NOTICE does not preserve the third-party distribution boundary")

    registry = REGISTRY.read_text(encoding="utf-8")
    if "owner: arda-rumil" not in registry:
        fail("registry has no provenance owner")
    if "no repository-root `LICENSE`" in registry:
        fail("registry contains the stale missing-LICENSE claim")

    cargo = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    members = cargo["workspace"]["members"]
    for member in members:
        manifest = ROOT / member / "Cargo.toml" if member != "." else ROOT / "Cargo.toml"
        if not manifest.is_file():
            fail(f"workspace member manifest is missing: {manifest.relative_to(ROOT)}")
        package = tomllib.loads(manifest.read_text(encoding="utf-8"))["package"]
        manifest_path = str(manifest.relative_to(ROOT))
        if f"`{manifest_path}`" not in registry:
            fail(f"workspace package lacks registry ownership entry: {package['name']}")

    for manifest in APP_MANIFESTS:
        if not manifest.is_file():
            fail(f"application manifest is missing: {manifest.relative_to(ROOT)}")
        name = json.loads(manifest.read_text(encoding="utf-8"))["name"]
        manifest_path = str(manifest.relative_to(ROOT))
        if f"`{manifest_path}`" not in registry:
            fail(f"application lacks registry ownership entry: {name}")

    print(
        "provenance gate: ok "
        f"({len(members)} workspace packages, {len(APP_MANIFESTS)} application manifests)"
    )


if __name__ == "__main__":
    main()
