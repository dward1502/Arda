#!/usr/bin/env python3
"""Verify ARDA README structure across local sibling repositories.

This is a lightweight documentation gate. It validates common headings,
Mermaid fence shape, and the central repository map/reading-order links. It does
not replace repo-local build, test, or Markdown rendering checks.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import re
import sys


ROOT = Path(__file__).resolve().parents[1]
RELEASE_ROOT = ROOT.parent


@dataclass(frozen=True)
class RepoSpec:
    local_name: str
    display_name: str
    url: str
    required_headings: tuple[str, ...]
    expected_terms: tuple[str, ...]


CHILD_REQUIRED_HEADINGS = (
    "## Vision",
    "## Getting Started",
    "## Architecture Overview",
    "## Relationship to ARDA",
    "## Status",
)

REPOS = (
    RepoSpec(
        "Arda-Agent-Loop-Contract",
        "Arda-Agent-Loop-Contract",
        "https://github.com/dward1502/Arda-Agent-Loop-Contract",
        CHILD_REQUIRED_HEADINGS,
        ("inspect-act-verify", "receipt"),
    ),
    RepoSpec(
        "Arda-Tool-Gate",
        "Arda-tool-gate",
        "https://github.com/dward1502/Arda-tool-gate",
        CHILD_REQUIRED_HEADINGS,
        ("policy", "receipt"),
    ),
    RepoSpec(
        "Arda-Service-Registry",
        "Arda-Service-Registry",
        "https://github.com/dward1502/Arda-Service-Registry",
        CHILD_REQUIRED_HEADINGS,
        ("service", "registry"),
    ),
    RepoSpec(
        "Arda-Signal-Grid",
        "Arda-Signal-Grid",
        "https://github.com/dward1502/Arda-Signal-Grid",
        CHILD_REQUIRED_HEADINGS,
        ("signal", "route"),
    ),
    RepoSpec(
        "Arda-Council",
        "Arda-Council",
        "https://github.com/dward1502/Arda-Council",
        CHILD_REQUIRED_HEADINGS,
        ("council", "governance"),
    ),
    RepoSpec(
        "Arda-HUD",
        "Arda-HUD",
        "https://github.com/dward1502/Arda-HUD",
        CHILD_REQUIRED_HEADINGS,
        ("operator", "HUD"),
    ),
)

CENTRAL_REQUIRED_HEADINGS = (
    "## Vision",
    "## Repository Map",
    "## Recommended Reading Order",
    "## Architecture Overview",
    "## Getting Started",
    "## Design Principles",
    "## Status",
)


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError:
        raise AssertionError(f"missing file: {path}") from None


def assert_heading_order(text: str, headings: tuple[str, ...], label: str) -> None:
    positions: list[int] = []
    for heading in headings:
        match = re.search(rf"^{re.escape(heading)}$", text, re.MULTILINE)
        if not match:
            raise AssertionError(f"{label}: missing heading {heading!r}")
        positions.append(match.start())
    if positions != sorted(positions):
        raise AssertionError(f"{label}: headings are present but out of expected order")


def assert_mermaid(text: str, label: str) -> None:
    blocks = re.findall(r"```mermaid\n(.*?)\n```", text, flags=re.DOTALL)
    if not blocks:
        raise AssertionError(f"{label}: missing Mermaid fenced block")
    for block in blocks:
        if "flowchart TB" not in block:
            raise AssertionError(f"{label}: Mermaid block missing 'flowchart TB'")

    mermaid_fences = len(re.findall(r"^```mermaid$", text, flags=re.MULTILINE))
    all_fences = len(re.findall(r"^```$", text, flags=re.MULTILINE))
    if all_fences < mermaid_fences:
        raise AssertionError(f"{label}: Mermaid fence count is not well formed")


def assert_central_readme() -> None:
    text = read(ROOT / "README.md")
    assert_heading_order(text, CENTRAL_REQUIRED_HEADINGS, "Arda/README.md")
    assert_mermaid(text, "Arda/README.md")

    for spec in REPOS:
        if spec.url not in text:
            raise AssertionError(f"Arda/README.md: missing link for {spec.display_name}: {spec.url}")
        repo_map_row = f"[{spec.display_name}]({spec.url})"
        if repo_map_row not in text:
            raise AssertionError(f"Arda/README.md: missing repository map entry for {spec.display_name}")

    reading_order = text.split("## Recommended Reading Order", 1)[1].split("## Architecture Overview", 1)[0]
    last_position = -1
    for spec in REPOS:
        position = reading_order.find(spec.url)
        if position == -1:
            raise AssertionError(f"Arda/README.md: reading order missing {spec.display_name}")
        if position <= last_position:
            raise AssertionError("Arda/README.md: reading-order links are out of expected order")
        last_position = position


def assert_child_readme(spec: RepoSpec) -> None:
    path = RELEASE_ROOT / spec.local_name / "README.md"
    text = read(path)
    label = f"{spec.local_name}/README.md"
    assert_heading_order(text, spec.required_headings, label)
    assert_mermaid(text, label)
    lower_text = text.lower()
    for term in spec.expected_terms:
        if term.lower() not in lower_text:
            raise AssertionError(f"{label}: expected role term {term!r} not found")


def main() -> int:
    failures: list[str] = []

    checks = [assert_central_readme]
    checks.extend(lambda spec=spec: assert_child_readme(spec) for spec in REPOS)

    for check in checks:
        try:
            check()
        except AssertionError as exc:
            failures.append(str(exc))

    if failures:
        print("ARDA README verification: FAIL")
        for failure in failures:
            print(f"- {failure}")
        return 1

    print("ARDA README verification: PASS")
    print(f"Checked central README plus {len(REPOS)} sibling repo READMEs.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
