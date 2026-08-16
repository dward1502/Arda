#!/usr/bin/env bash
set -euo pipefail

app_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

python3 - "$app_root" <<'PY'
from __future__ import annotations

import re
import sys
from pathlib import Path

root = Path(sys.argv[1])
excluded_parts = {"dist", "node_modules", "target", ".git"}
text_suffixes = {
    ".css",
    ".html",
    ".js",
    ".json",
    ".md",
    ".mjs",
    ".rs",
    ".sh",
    ".toml",
    ".ts",
    ".tsx",
    ".yaml",
    ".yml",
}
failures: list[str] = []
checked = 0
frontmatter_checked = 0

for path in sorted(root.rglob("*")):
    if not path.is_file() or path.suffix.lower() not in text_suffixes:
        continue
    if excluded_parts.intersection(path.relative_to(root).parts):
        continue

    checked += 1
    relative = path.relative_to(root)
    try:
        text = path.read_text(encoding="utf-8", errors="strict")
    except UnicodeDecodeError as error:
        failures.append(f"{relative}: invalid UTF-8 ({error})")
        continue

    if "\ufffd" in text:
        failures.append(f"{relative}: contains Unicode replacement character U+FFFD")

    if path.suffix.lower() != ".md" or not text.startswith("---\n"):
        continue

    frontmatter = text.split("---", 2)[1]
    if not re.search(r"^soterion:\s*$", frontmatter, re.MULTILINE):
        continue

    glyph_match = re.search(r'^\s+glyph:\s*["\']?(.+?)["\']?\s*$', frontmatter, re.MULTILINE)
    code_point_match = re.search(
        r'^\s+code_point:\s*["\']?(U\+[0-9A-Fa-f]+)["\']?\s*$',
        frontmatter,
        re.MULTILINE,
    )
    if glyph_match is None or code_point_match is None:
        continue

    frontmatter_checked += 1
    glyph = glyph_match.group(1).strip().strip('"\'')
    expected = code_point_match.group(1).upper()
    actual = f"U+{ord(glyph[0]):04X}" if glyph else "missing"
    if actual != expected:
        failures.append(
            f"{relative}: glyph {glyph!r} begins with {actual}, declared {expected}"
        )

if failures:
    print("Soterion Unicode guard failed:", file=sys.stderr)
    for failure in failures:
        print(f"- {failure}", file=sys.stderr)
    raise SystemExit(1)

print(
    "Soterion Unicode guard passed: "
    f"{checked} UTF-8 text files; {frontmatter_checked} glyph/code-point declarations"
)
PY
