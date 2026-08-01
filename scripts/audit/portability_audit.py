#!/usr/bin/env python3
"""Read-only Arda portability and config hygiene audit.

The runner scans tracked text files when Git metadata is available, classifies
hardcoded local paths and endpoint assumptions by repository surface, and emits
machine-readable receipts plus a Markdown summary. It does not rewrite matched
files or mutate source/config state.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
from collections import Counter
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable

CONTRACT = "arda.portability_config_hygiene_audit.v1"
DEFAULT_SCAN_LIMIT_BYTES = 1_000_000

SKIP_DIR_NAMES = {
    ".git",
    ".cache",
    ".hermes",
    ".pytest_cache",
    ".target-local",
    "__pycache__",
    "debug",
    "dist",
    "dist_build",
    "logs",
    "node_modules",
    "target",
    "target-check",
    "tmp",
}

GENERATED_PREFIXES = (
    "audit/",
    "core/knowledge/",
    "core/projects/tasks/",
    "core/state/",
    "data/",
    "logs/",
    "tmp/",
    "target/",
    "target-check/",
    ".cache/",
    ".target-local/",
    "apps/arda-hud/src/assets/scene/world/",
    "apps/arda-hud/data/",
    "core/metrics/history/",
    "core/metrics/by_crate/",
)
ARCHIVE_PREFIXES = ("archive/", "archived_scripts/")
DOC_PREFIXES = ("docs/", "human/", "project/", "references/")
TEST_PREFIXES = ("tests/",)
CONFIG_PREFIXES = ("config/", ".config/", "systemd/", "core/realm/")
SCRIPT_PREFIXES = ("scripts/",)
SOURCE_PREFIXES = ("crates/", "apps/", "core/", "src/")

TEXT_SUFFIXES = {
    "",
    ".bash",
    ".cfg",
    ".css",
    ".csv",
    ".env",
    ".example",
    ".html",
    ".ini",
    ".js",
    ".json",
    ".jsonl",
    ".lock",
    ".md",
    ".py",
    ".rs",
    ".service",
    ".sh",
    ".sql",
    ".svg",
    ".toml",
    ".tsx",
    ".ts",
    ".txt",
    ".unit",
    ".yaml",
    ".yml",
}


@dataclass(frozen=True)
class PatternSpec:
    pattern_id: str
    regex: re.Pattern[str]
    severity: str
    replacement_hint: str


@dataclass(frozen=True)
class PatternMatch:
    pattern_id: str
    line: int
    column: int
    match: str
    severity: str
    replacement_hint: str


PATTERNS: tuple[PatternSpec, ...] = (
    PatternSpec(
        "hardcoded_var_home_mythos",
        re.compile(r"/var/home/mythos(?:/|\b)"),
        "high",
        "Use $HOME for the operator home or $ARDA_ROOT for the repository root.",
    ),
    PatternSpec(
        "hardcoded_home_mythos",
        re.compile(r"/home/mythos(?:/|\b)"),
        "high",
        "Use $HOME rather than a named user's home directory.",
    ),
    PatternSpec(
        "hardcoded_mythos_path_segment",
        re.compile(r"(?<![A-Za-z0-9_\-$])/mythos/"),
        "medium",
        "Parameterize machine/user-specific path segments.",
    ),
    PatternSpec(
        "user_home_constructed_from_user",
        re.compile(r"/(?:var/)?home/\$\{?USER\}?\b"),
        "medium",
        "Use $HOME for home-directory construction; reserve $USER for identity only.",
    ),
    PatternSpec(
        "loopback_endpoint",
        re.compile(r"(?:(?:https?|ws)://)?(?:127\.0\.0\.1|localhost):\d{2,5}(?:/[A-Za-z0-9_./?=&%-]*)?"),
        "medium",
        "Move endpoint defaults behind config/env such as CHARON_BASE_URL or HERMES_BASE_URL.",
    ),
    PatternSpec(
        "private_lan_ip_endpoint",
        re.compile(r"(?:(?:https?|ws)://)?(?:10\.\d{1,3}\.\d{1,3}\.\d{1,3}|192\.168\.\d{1,3}\.\d{1,3}|172\.(?:1[6-9]|2\d|3[0-1])\.\d{1,3}\.\d{1,3})(?::\d{2,5})?"),
        "medium",
        "Parameterize machine-local or LAN endpoints in config.",
    ),
    PatternSpec(
        "tailscale_hostname",
        re.compile(r"\b[A-Za-z0-9-]+\.(?:ts\.net|tailnet-[A-Za-z0-9-]+\.ts\.net)\b"),
        "medium",
        "Move tailnet-specific hostnames to fleet/config contracts.",
    ),
)


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def rel_path(path: Path, root: Path) -> str:
    try:
        return path.relative_to(root).as_posix()
    except ValueError:
        return path.as_posix()


def classify_path(path: Path) -> str:
    rel = path.as_posix().lstrip("./")
    parts = set(path.parts)
    name = path.name.lower()

    if rel.startswith(ARCHIVE_PREFIXES) or "/archive/" in f"/{rel}" or "/archived/" in f"/{rel}":
        return "archive_historical_ok"
    if rel.startswith(GENERATED_PREFIXES):
        return "generated_runtime_state_ignore_or_regenerate"
    if (
        rel.startswith(TEST_PREFIXES)
        or "fixtures" in parts
        or name.startswith("fixture")
        or name in {"tests.rs", "test.rs"}
        or name.endswith("_tests.rs")
    ):
        return "test_fixture_ok"
    if rel.startswith(DOC_PREFIXES) or path.suffix.lower() in {".md", ".rst", ".txt"} or name in {"readme.md", "agents.md", "codemap.md", "arda_system_status_report.md", "arda_root_protocol.md"}:
        return "documentation_example_review"
    if rel.startswith(CONFIG_PREFIXES) or path.suffix.lower() in {".toml", ".yaml", ".yml", ".service", ".env"} and "src" not in parts:
        return "active_config_must_parameterize"
    if rel.startswith(SCRIPT_PREFIXES) or path.suffix.lower() in {".sh", ".bash", ".py"}:
        return "active_script_must_parameterize"
    if rel.startswith(SOURCE_PREFIXES) or path.suffix.lower() in {".rs", ".ts", ".tsx", ".js"}:
        return "active_source_must_fix"
    return "ambiguous_review_required"


def line_offsets(text: str) -> list[int]:
    offsets = [0]
    for index, char in enumerate(text):
        if char == "\n":
            offsets.append(index + 1)
    return offsets


def position_for(offsets: list[int], index: int) -> tuple[int, int]:
    # Small files and few matches: linear search keeps implementation simple and deterministic.
    line = 1
    for current, start in enumerate(offsets, start=1):
        if start > index:
            break
        line = current
    column = index - offsets[line - 1] + 1
    return line, column


def detect_patterns(text: str) -> list[PatternMatch]:
    offsets = line_offsets(text)
    matches: list[PatternMatch] = []
    for spec in PATTERNS:
        for found in spec.regex.finditer(text):
            line, column = position_for(offsets, found.start())
            matches.append(
                PatternMatch(
                    pattern_id=spec.pattern_id,
                    line=line,
                    column=column,
                    match=found.group(0),
                    severity=spec.severity,
                    replacement_hint=spec.replacement_hint,
                )
            )
    return sorted(matches, key=lambda item: (item.line, item.column, item.pattern_id))


def is_probably_text(path: Path) -> bool:
    if path.suffix.lower() in TEXT_SUFFIXES:
        return True
    return "." not in path.name


def read_text(path: Path, limit: int = DEFAULT_SCAN_LIMIT_BYTES) -> str | None:
    try:
        data = path.read_bytes()
    except OSError:
        return None
    if len(data) > limit:
        data = data[:limit]
    if b"\x00" in data:
        return None
    try:
        return data.decode("utf-8", errors="replace")
    except UnicodeDecodeError:
        return None


def git_files(root: Path) -> list[Path]:
    try:
        completed = subprocess.run(
            ["git", "ls-files", "-z"],
            cwd=str(root),
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except OSError:
        return []
    if completed.returncode != 0:
        return []
    rels = [item.decode("utf-8", errors="replace") for item in completed.stdout.split(b"\x00") if item]
    return [root / rel for rel in rels]


def walk_files(root: Path) -> list[Path]:
    paths: list[Path] = []
    for current, dirnames, filenames in os.walk(root):
        dirnames[:] = [name for name in dirnames if name not in SKIP_DIR_NAMES]
        for filename in filenames:
            paths.append(Path(current) / filename)
    return sorted(paths)


def iter_candidate_files(root: Path, use_git: bool = True) -> tuple[list[Path], str]:
    if use_git:
        tracked = git_files(root)
        if tracked:
            return sorted(tracked), "git_ls_files"
    return walk_files(root), "filesystem_walk"


def redacted_snippet(line_text: str) -> str:
    snippet = line_text.strip()[:240]
    snippet = re.sub(r"(?i)(token|secret|password|api[_-]?key)\s*[:=]\s*[^\s'\"]+", r"\1=[REDACTED]", snippet)
    return snippet


def finding_from_match(root: Path, path: Path, match: PatternMatch, text: str, index: int) -> dict[str, Any]:
    lines = text.splitlines()
    line_text = lines[match.line - 1] if match.line - 1 < len(lines) else ""
    rel = rel_path(path, root)
    classification = classify_path(Path(rel))
    return {
        "id": f"portability-{index:05d}",
        "contract": "arda.portability_finding.v1",
        "path": rel,
        "line": match.line,
        "column": match.column,
        "pattern_id": match.pattern_id,
        "match": match.match,
        "classification": classification,
        "severity": match.severity,
        "replacement_hint": match.replacement_hint,
        "snippet": redacted_snippet(line_text),
    }


def active_blocker(classification: str) -> bool:
    return classification in {
        "active_source_must_fix",
        "active_config_must_parameterize",
        "active_script_must_parameterize",
        "ambiguous_review_required",
    }


def write_jsonl(path: Path, records: Iterable[dict[str, Any]]) -> int:
    count = 0
    with path.open("w", encoding="utf-8") as handle:
        for record in records:
            handle.write(json.dumps(record, sort_keys=True) + "\n")
            count += 1
    return count


def build_summary(root: Path, out_dir: Path, findings: list[dict[str, Any]], scanned_files: int, skipped_files: int, scan_source: str) -> dict[str, Any]:
    by_classification = Counter(item["classification"] for item in findings)
    by_pattern = Counter(item["pattern_id"] for item in findings)
    by_path = Counter(item["path"] for item in findings if active_blocker(item["classification"]))
    active_findings = [item for item in findings if active_blocker(item["classification"])]
    top_active_blockers = [
        {"path": path, "findings": count}
        for path, count in by_path.most_common(20)
    ]
    return {
        "contract": CONTRACT,
        "generated_at": utc_now(),
        "root": str(root),
        "output_dir": str(out_dir),
        "scan_source": scan_source,
        "summary": {
            "files_scanned": scanned_files,
            "files_skipped": skipped_files,
            "findings_total": len(findings),
            "active_blocker_findings": len(active_findings),
            "classification_counts": dict(sorted(by_classification.items())),
            "pattern_counts": dict(sorted(by_pattern.items())),
            "top_active_blockers": top_active_blockers,
        },
        "outputs": {
            "summary_json": str(out_dir / "summary.json"),
            "findings_jsonl": str(out_dir / "findings.jsonl"),
            "summary_md": str(out_dir / "summary.md"),
            "ignored_jsonl": str(out_dir / "ignored.jsonl"),
            "active_blockers_jsonl": str(out_dir / "active-blockers.jsonl"),
        },
    }


def markdown_summary(report: dict[str, Any], findings: list[dict[str, Any]]) -> str:
    summary = report["summary"]
    lines = [
        "# Arda Portability and Config Hygiene Audit",
        "",
        f"Generated: `{report['generated_at']}`",
        f"Contract: `{report['contract']}`",
        f"Root: `{report['root']}`",
        f"Scan source: `{report['scan_source']}`",
        "",
        "## Summary",
        "",
        f"- Files scanned: {summary['files_scanned']}",
        f"- Files skipped: {summary['files_skipped']}",
        f"- Total findings: {summary['findings_total']}",
        f"- Active blocker findings: {summary['active_blocker_findings']}",
        "",
        "## Classification Counts",
        "",
    ]
    for classification, count in summary["classification_counts"].items():
        lines.append(f"- `{classification}`: {count}")
    lines.extend(["", "## Pattern Counts", ""])
    for pattern, count in summary["pattern_counts"].items():
        lines.append(f"- `{pattern}`: {count}")
    lines.extend(["", "## Top Active Blockers", ""])
    if summary["top_active_blockers"]:
        for item in summary["top_active_blockers"]:
            lines.append(f"- `{item['path']}`: {item['findings']} findings")
    else:
        lines.append("- None")
    lines.extend(["", "## Sample Active Findings", ""])
    active = [item for item in findings if active_blocker(item["classification"])]
    for item in active[:50]:
        lines.append(
            f"- `{item['path']}:{item['line']}` `{item['classification']}` "
            f"`{item['pattern_id']}` — {item['replacement_hint']}"
        )
    if not active:
        lines.append("- None")
    lines.extend([
        "",
        "## Read-Only Guarantee",
        "",
        "This Phase 1 runner only scans text files and writes audit receipts under the requested output directory. It does not rewrite matched source/config/script files.",
        "",
    ])
    return "\n".join(lines)


def run_audit(root: Path, out_dir: Path, use_git: bool = True) -> dict[str, Any]:
    root = root.resolve()
    out_dir = out_dir.resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    candidates, scan_source = iter_candidate_files(root, use_git=use_git)
    findings: list[dict[str, Any]] = []
    ignored: list[dict[str, Any]] = []
    scanned_files = 0
    skipped_files = 0

    for path in candidates:
        if not path.exists() or not path.is_file():
            skipped_files += 1
            continue
        rel = rel_path(path, root)
        if any(part in SKIP_DIR_NAMES for part in Path(rel).parts):
            skipped_files += 1
            continue
        if not is_probably_text(path):
            skipped_files += 1
            continue
        text = read_text(path)
        if text is None:
            skipped_files += 1
            continue
        scanned_files += 1
        matches = detect_patterns(text)
        for match in matches:
            finding = finding_from_match(root, path, match, text, len(findings) + 1)
            findings.append(finding)
            if not active_blocker(finding["classification"]):
                ignored.append(finding)

    active_findings = [item for item in findings if active_blocker(item["classification"])]
    report = build_summary(root, out_dir, findings, scanned_files, skipped_files, scan_source)

    (out_dir / "summary.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_jsonl(out_dir / "findings.jsonl", findings)
    write_jsonl(out_dir / "ignored.jsonl", ignored)
    write_jsonl(out_dir / "active-blockers.jsonl", active_findings)
    (out_dir / "summary.md").write_text(markdown_summary(report, findings), encoding="utf-8")
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run read-only Arda portability/config hygiene audit.")
    parser.add_argument("--root", default=".", help="Repository root to scan. Defaults to current directory.")
    parser.add_argument("--out", default=None, help="Output directory. Defaults to audit/PORTABILITY_AUDIT_YYYY-MM-DD.")
    parser.add_argument("--no-git", action="store_true", help="Do not use git ls-files; walk filesystem instead.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root = Path(args.root)
    out_dir = Path(args.out) if args.out else root / "audit" / f"PORTABILITY_AUDIT_{datetime.now(timezone.utc).date().isoformat()}"
    report = run_audit(root=root, out_dir=out_dir, use_git=not args.no_git)
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

