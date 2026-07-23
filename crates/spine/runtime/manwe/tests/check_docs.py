#!/usr/bin/env python3
"""Validate local links and the generated Manwe source index."""

from __future__ import annotations

import re
import sys
from pathlib import Path
from urllib.parse import unquote

MANWE_ROOT = Path(__file__).resolve().parents[1]
SOURCE_ROOT = MANWE_ROOT / "src"
SOURCE_INDEX = SOURCE_ROOT / "INDEX.md"
LINK_RE = re.compile(r"(?<!!)\[[^]]*\]\(([^)]+)\)")
INDEX_ENTRY_RE = re.compile(r"^- \[[^]]+\]\(([^)]+)\)$", re.MULTILINE)


def local_target(raw_target: str) -> str | None:
    target = raw_target.strip().strip("<>")
    if not target or target.startswith(("#", "http://", "https://", "mailto:")):
        return None
    return unquote(target.split("#", 1)[0].split("?", 1)[0])


def validate_links(markdown_files: list[Path]) -> tuple[list[str], int]:
    failures: list[str] = []
    checked = 0
    for document in markdown_files:
        for raw_target in LINK_RE.findall(document.read_text(encoding="utf-8")):
            target = local_target(raw_target)
            if target is None:
                continue
            checked += 1
            resolved = (document.parent / target).resolve()
            if not resolved.exists():
                failures.append(
                    f"{document.relative_to(MANWE_ROOT)}: missing local link target `{target}`"
                )
    return failures, checked


def validate_source_index() -> tuple[list[str], int]:
    contents = SOURCE_INDEX.read_text(encoding="utf-8")
    indexed = {target.rstrip("/") for target in INDEX_ENTRY_RE.findall(contents)}
    expected = {
        child.name
        for child in SOURCE_ROOT.iterdir()
        if child.name != SOURCE_INDEX.name and not child.name.startswith(".")
    }
    failures: list[str] = []
    for missing in sorted(expected - indexed):
        failures.append(f"src/INDEX.md: missing direct child `{missing}`")
    for stale in sorted(indexed - expected):
        failures.append(f"src/INDEX.md: stale direct child `{stale}`")
    return failures, len(indexed)


def validate_source_readme() -> list[str]:
    contents = (SOURCE_ROOT / "README.md").read_text(encoding="utf-8")
    canonical = "crates/spine/runtime/manwe/src"
    legacy = "crates/annunimas-charon/src"
    failures: list[str] = []
    if canonical not in contents:
        failures.append(f"src/README.md: missing canonical source path `{canonical}`")
    if legacy in contents:
        failures.append(f"src/README.md: contains legacy source path `{legacy}`")
    return failures


def main() -> int:
    markdown_files = sorted(MANWE_ROOT.rglob("*.md"))
    failures, link_count = validate_links(markdown_files)
    index_failures, index_count = validate_source_index()
    failures.extend(index_failures)
    failures.extend(validate_source_readme())

    if failures:
        print("Manwe documentation validation failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1

    print(
        "Manwe documentation validation passed: "
        f"{len(markdown_files)} Markdown files, {link_count} local links, "
        f"{index_count} source-index entries"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
