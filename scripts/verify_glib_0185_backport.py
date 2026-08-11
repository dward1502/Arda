#!/usr/bin/env python3
"""Verify Arda's glib 0.18.5 backport against the pinned crates.io archive."""

from __future__ import annotations

import argparse
import hashlib
import io
from pathlib import Path
import tarfile
import tempfile
import urllib.request


CRATE_URL = "https://static.crates.io/crates/glib/glib-0.18.5.crate"
CRATE_SHA256 = "233daaf6e83ae6a12a52055f568f9d7cf4671dabb78ff9560ab6da230ce00ee5"
OLD_DECL = b"let p: *mut libc::c_char = std::ptr::null_mut();"
NEW_DECL = b"let mut p: *mut libc::c_char = std::ptr::null_mut();"
OLD_ARG = b"                &p,\n"
NEW_ARG = b"                &mut p,\n"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--crate", type=Path, help="Use an existing .crate archive")
    return parser.parse_args()


def load_archive(path: Path | None) -> bytes:
    if path is not None:
        return path.read_bytes()
    with urllib.request.urlopen(CRATE_URL, timeout=30) as response:
        return response.read()


def regular_files(root: Path) -> dict[Path, bytes]:
    return {
        path.relative_to(root): path.read_bytes()
        for path in root.rglob("*")
        if path.is_file()
    }


def main() -> int:
    args = parse_args()
    archive = load_archive(args.crate)
    actual_hash = hashlib.sha256(archive).hexdigest()
    if actual_hash != CRATE_SHA256:
        raise SystemExit(f"crate checksum mismatch: {actual_hash}")

    with tempfile.TemporaryDirectory(prefix="arda-glib-verify-") as directory:
        unpack_root = Path(directory)
        with tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz") as tar:
            tar.extractall(unpack_root, filter="data")
        upstream_root = unpack_root / "glib-0.18.5"
        upstream = regular_files(upstream_root)

    vendor_root = args.root / "vendor" / "glib-0.18.5"
    vendor = regular_files(vendor_root)
    patch_note = Path("ARDA_PATCH.md")
    if patch_note not in vendor:
        raise SystemExit("missing ARDA_PATCH.md")
    del vendor[patch_note]

    variant_path = Path("src/variant_iter.rs")
    original = upstream[variant_path]
    if original.count(OLD_DECL) != 1 or original.count(OLD_ARG) != 1:
        raise SystemExit("upstream VariantStrIter source no longer matches expected baseline")
    expected = original.replace(OLD_DECL, NEW_DECL).replace(OLD_ARG, NEW_ARG)

    unexpected: list[str] = []
    for path in sorted(set(upstream) | set(vendor)):
        expected_bytes = expected if path == variant_path else upstream.get(path)
        if vendor.get(path) != expected_bytes:
            unexpected.append(str(path))
    if unexpected:
        raise SystemExit("unexpected vendored delta: " + ", ".join(unexpected))

    print(
        "glib_backport_verified "
        f"crate_sha256={CRATE_SHA256} files={len(upstream)} "
        "source_delta=src/variant_iter.rs:&p->&mut_p"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
