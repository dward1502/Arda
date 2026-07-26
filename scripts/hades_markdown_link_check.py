#!/usr/bin/env python3
"""Check local Markdown links for HADES organization maintenance."""

from __future__ import annotations

import argparse
import os
import re
from pathlib import Path


EXCLUDED_DIRS = {
    ".git",
    ".cache",
    ".target-local",
    ".tmp",
    ".agents",
    ".claude",
    ".hermes",
    ".opencode",
    ".venv",
    "archive",
    "build",
    "data",
    "dist",
    "node_modules",
    "target",
    "__pycache__",
}

LINK_RE = re.compile(r"(?<!!)\[[^\]]+\]\(([^)]+)\)")
PLACEHOLDER_TARGETS = {
    "URL",
    "path/to/document.md",
    "path/to/plan.md",
}


def is_external(target: str) -> bool:
    lowered = target.lower()
    return (
        "://" in lowered
        or lowered.startswith("#")
        or lowered.startswith("mailto:")
        or lowered.startswith("tel:")
    )


def iter_markdown(root: Path):
    for current, dirs, files in os.walk(root):
        dirs[:] = [name for name in dirs if name not in EXCLUDED_DIRS]
        for name in files:
            if name.endswith(".md"):
                yield Path(current) / name


def link_target_exists(source: Path, raw_target: str, root: Path) -> bool:
    target = raw_target.split("#", 1)[0].strip()
    target = re.sub(r":\d+$", "", target)
    if not target or target in PLACEHOLDER_TARGETS:
        return True
    if target.startswith("/"):
        candidate = Path(target)
    else:
        candidate = source.parent / target
    return candidate.exists()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default=".")
    parser.add_argument("--out", default="data/hades/markdown_link_check_last.md")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    broken: list[tuple[Path, str]] = []
    checked = 0

    for path in iter_markdown(root):
        text = path.read_text(encoding="utf-8", errors="replace")
        for match in LINK_RE.finditer(text):
            raw_target = match.group(1).strip()
            if is_external(raw_target):
                continue
            checked += 1
            if not link_target_exists(path, raw_target, root):
                broken.append((path.relative_to(root), raw_target))

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    lines = [
        "# HADES Markdown Link Check",
        "",
        f"- Root: `{root}`",
        f"- Local links checked: {checked}",
        f"- Broken local links: {len(broken)}",
        "",
    ]
    if broken:
        lines.append("## Broken Links")
        lines.append("")
        for source, target in broken:
            lines.append(f"- `{source}` -> `{target}`")
    else:
        lines.append("No broken local Markdown links found.")
    out_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"HADES markdown link check written: {out_path}")
    print(f"  local_links_checked={checked} broken_local_links={len(broken)}")
    return 1 if broken else 0


if __name__ == "__main__":
    raise SystemExit(main())
