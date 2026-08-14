#!/usr/bin/env python3
"""Check Markdown links and optional active-plan completion language."""

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
COMPLETION_CLAIM_RE = re.compile(
    r"\b(?:is|are|was|were)\s+(?:now\s+)?(?:complete|operational)\b"
    r"|\bfully integrated\b",
    re.IGNORECASE,
)
MATURITY_RE = re.compile(
    r"\b(?:specified|implemented|compile[_-]active|root[_-]composed|operator[_-]reachable|"
    r"workflow[_-]proven|failure[_-]proven|operator[_-]accepted|release[_-]supported)\b",
    re.IGNORECASE,
)
HISTORICAL_QUALIFIER_RE = re.compile(
    r"\b(?:historical|historically|quoted|quotation|previously reported|"
    r"at the time|archived record)\b",
    re.IGNORECASE,
)
CONDITIONAL_CLAIM_RE = re.compile(r"\b(?:gate|when|only when|until)\b", re.IGNORECASE)
QUOTED_COMPLETION_TERM_RE = re.compile(
    r"(?:“[^”]*(?:complete|operational|fully integrated)[^”]*”|"
    r"\"[^\"]*(?:complete|operational|fully integrated)[^\"]*\")",
    re.IGNORECASE,
)
EVIDENCE_DECLARATION_RE = re.compile(
    r"^\s*(?:[-*]\s*)?(?:\*\*)?(?:maturity|evidence(?: tags?)?|status)(?:\*\*)?\s*:",
    re.IGNORECASE,
)
HIGH_EVIDENCE_TAG_RE = re.compile(
    r"\b(?:operator_accepted|release_supported|workflow_proven|failure_proven|"
    r"[a-z0-9_]+_verified)\b",
    re.IGNORECASE,
)
PLACEHOLDER_TARGETS = {
    "URL",
    "path/to/document.md",
    "path/to/plan.md",
}
RUSTDOC_LINK_PREFIXES = (
    "struct@",
    "enum@",
    "trait@",
    "type@",
    "fn@",
    "macro@",
    "derive@",
    "mod@",
)
RUSTDOC_PATH_RE = re.compile(
    r"^[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)+$"
)


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
    if target.startswith(RUSTDOC_LINK_PREFIXES) or RUSTDOC_PATH_RE.fullmatch(target):
        return True
    if target.startswith("/"):
        candidate = Path(target)
    else:
        candidate = source.parent / target
    return candidate.exists()


def completion_language_issue(line: str) -> str | None:
    """Return a stable issue code for an unsafe active-plan claim line."""
    if HISTORICAL_QUALIFIER_RE.search(line):
        return None

    if (
        EVIDENCE_DECLARATION_RE.search(line)
        and HIGH_EVIDENCE_TAG_RE.search(line)
        and not LINK_RE.search(line)
    ):
        return "missing_evidence_link"

    if (
        COMPLETION_CLAIM_RE.search(line)
        and not MATURITY_RE.search(line)
        and not LINK_RE.search(line)
        and not CONDITIONAL_CLAIM_RE.search(line)
        and not QUOTED_COMPLETION_TERM_RE.search(line)
    ):
        return "unqualified_completion_claim"

    return None


def completion_plan_root(root: Path) -> Path | None:
    if root.name == "plans" and root.parent.name == "docs":
        return root
    candidate = root / "docs" / "plans"
    return candidate if candidate.is_dir() else None


def scan_completion_language(root: Path) -> list[tuple[Path, int, str, str]]:
    plans_root = completion_plan_root(root)
    if plans_root is None:
        return []

    issues: list[tuple[Path, int, str, str]] = []
    for path in sorted(plans_root.glob("*.md")):
        for line_number, line in enumerate(
            path.read_text(encoding="utf-8", errors="replace").splitlines(),
            start=1,
        ):
            issue = completion_language_issue(line)
            if issue:
                issues.append((path.relative_to(root), line_number, issue, line.strip()))
    return issues


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default=".")
    parser.add_argument("--out", default="data/hades/markdown_link_check_last.md")
    parser.add_argument(
        "--check-completion-language",
        action="store_true",
        help="check active docs/plans claims and high-evidence maturity declarations",
    )
    args = parser.parse_args()

    root = Path(args.root).resolve()
    broken: list[tuple[Path, str]] = []
    completion_issues = scan_completion_language(root) if args.check_completion_language else []
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
        f"- Completion-language check enabled: {str(args.check_completion_language).lower()}",
        f"- Completion-language issues: {len(completion_issues)}",
        "",
    ]
    if broken:
        lines.append("## Broken Links")
        lines.append("")
        for source, target in broken:
            lines.append(f"- `{source}` -> `{target}`")
    else:
        lines.append("No broken local Markdown links found.")
    if args.check_completion_language:
        lines.extend(["", "## Completion Language"])
        if completion_issues:
            lines.append("")
            for source, line_number, issue, line in completion_issues:
                lines.append(f"- `{source}:{line_number}` `{issue}` — {line}")
        else:
            lines.extend(["", "No completion-language issues found."])
    out_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"HADES markdown link check written: {out_path}")
    print(f"  local_links_checked={checked} broken_local_links={len(broken)}")
    return 1 if broken or completion_issues else 0


if __name__ == "__main__":
    raise SystemExit(main())
