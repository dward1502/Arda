#!/usr/bin/env python3
"""Reusable Arda target audit runner.

This Phase 4 runner creates schema-checked Good / Bad / Ugly audit reports for
bounded Arda targets. It is intentionally read-only: it inspects repository
files and existing portability receipts, then emits JSON/JSONL/Markdown audit
artifacts without rewriting source, config, or runtime state.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import tomllib
from collections import Counter
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable

CONTRACT = "arda.audit.target_report.v1"
RUN_CONTRACT = "arda.audit.run.v1"
DEFAULT_RUN_ID_PREFIX = "system-audit"
RUBRIC_MAX = {
    "mission_fit": 20,
    "implementation_completeness": 20,
    "reliability_safety": 20,
    "observability_auditability": 15,
    "portability_config_hygiene": 15,
    "ux_operator_experience": 10,
}
REQUIRED_REPORT_KEYS = {
    "contract",
    "run_id",
    "target",
    "target_type",
    "score",
    "score_breakdown",
    "overview",
    "duties",
    "good",
    "bad",
    "ugly",
    "potential_changes",
    "needs_removed",
    "evidence",
    "candidate_tasks",
}


@dataclass(frozen=True)
class TargetSpec:
    target: str
    target_type: str
    root: str
    duties: tuple[str, ...]
    support_paths: tuple[str, ...] = ()
    keywords: tuple[str, ...] = ()
    rollup_target: str | None = None


TARGETS: dict[str, TargetSpec] = {
    "RUMIL": TargetSpec("RUMIL", "subsystem", "crates/spine/runtime/arda-rumil", ("Project organization, audit evidence, lifecycle review, and no-delete hygiene.",), support_paths=("scripts/rumil_organization_maintenance.sh", "data/rumil"), keywords=("rumil", "lifecycle", "archive", "review", "cleanup")),
    "PROMETHEUS": TargetSpec("PROMETHEUS", "subsystem", "crates/spine/observability/arda-aule", ("Autopilot orchestration, governance gates, and queue posture reporting.",), support_paths=("data/hades/action_queue.jsonl",), keywords=("autopilot", "objective", "governance", "queue", "promote")),
    "MANWE": TargetSpec("MANWE", "agent_crate", "crates/spine/runtime/manwe", ("Delegation, inference routing, provider health, and fallback behavior.",), support_paths=("config/manwe.providers.toml", "scripts/refresh_provider_intelligence.py"), keywords=("provider", "route", "model", "health", "delegation")),
}

CRATE_TARGETS: dict[str, TargetSpec] = {
    "ARDA-BARROW-WIGHT": TargetSpec("ARDA-BARROW-WIGHT", "workspace_crate", "crates/arda-barrow-wight", ("Policy and receipt validation.",), keywords=("policy", "receipt", "validation")),
    "ARDA-ULE": TargetSpec("ARDA-ULE", "workspace_crate", "crates/spine/foundation/arda-ule", ("Foundation types and contracts.",), keywords=("contract", "task", "agent")),
    "ARDA-OROME": TargetSpec("ARDA-OROME", "workspace_crate", "crates/spine/interface/arda-orome", ("Interface and Hermes integration.",), keywords=("interface", "hermes", "service")),
    "ARDA-MANWE": TargetSpec("ARDA-MANWE", "workspace_crate", "crates/spine/runtime/manwe", ("Delegation and provider runtime.",), keywords=("delegation", "provider", "route"), rollup_target="MANWE"),
    "ARDA-AULE": TargetSpec("ARDA-AULE", "workspace_crate", "crates/spine/observability/arda-aule", ("Observability, governance, HADES, and Prometheus surfaces.",), keywords=("audit", "metric", "hades", "prometheus"), rollup_target="PROMETHEUS"),
    "ARDA-LORIEN": TargetSpec("ARDA-LORIEN", "workspace_crate", "crates/spine/arms/arda-lorien", ("Lorien arm.",), keywords=("lorien", "arm")),
    "ARDA-MANDOS": TargetSpec("ARDA-MANDOS", "workspace_crate", "crates/spine/arms/arda-mandos", ("Mandos arm.",), keywords=("mandos", "arm")),
    "ARDA-VARDA": TargetSpec("ARDA-VARDA", "workspace_crate", "crates/spine/executors/arda-varda", ("Athena/Varda ingestion and provenance.",), keywords=("athena", "varda", "ingest", "provenance")),
    "ARDA-CORE": TargetSpec("ARDA-CORE", "workspace_crate", "crates/spine/executors/arda-core", ("Core executor.",), keywords=("core", "execute")),
    "ARDA-HADHAFANG": TargetSpec("ARDA-HADHAFANG", "workspace_crate", "crates/spine/executors/arda-hadhafang", ("Hadhafang executor.",), keywords=("hadhafang", "execute")),
}

FOLDER_TARGETS: dict[str, TargetSpec] = {
    "FOLDER-CRATES": TargetSpec("FOLDER-CRATES", "folder", "crates", ("Rust workspace and agent crate surface.",), support_paths=("Cargo.toml",), keywords=("arda", "Cargo.toml", "src", "README")),
    "FOLDER-APPS": TargetSpec("FOLDER-APPS", "folder", "apps", ("Frontend and device applications including ARDA HUD and CITADEL avatar.",), keywords=("tauri", "vite", "arda", "citadel", "package")),
    "FOLDER-CONFIG": TargetSpec("FOLDER-CONFIG", "folder", "config", ("Operator-managed TOML/YAML/JSON configuration and generated runtime env examples.",), keywords=("toml", "yaml", "provider", "endpoint", "env")),
    "FOLDER-CORE": TargetSpec("FOLDER-CORE", "folder", "core", ("Realm authority, runtime state, and project queues.",), keywords=("realm", "state", "queue", "task", "governance")),
    "FOLDER-DATA": TargetSpec("FOLDER-DATA", "folder", "data", ("Runtime outputs, receipts, ledgers, snapshots, and telemetry.",), keywords=("receipt", "ledger", "telemetry", "snapshot", "state")),
    "FOLDER-DOCS": TargetSpec("FOLDER-DOCS", "folder", "docs", ("Human-facing architecture, safety, operations, contracts, and plans.",), keywords=("architecture", "operations", "plan", "safety", "contract")),
    "FOLDER-HUMAN": TargetSpec("FOLDER-HUMAN", "folder", "human", ("Obsidian-connected human knowledge and ingestion input surface.",), keywords=("knowledge", "inbox", "review", "human", "vault")),
    "FOLDER-AUDIT": TargetSpec("FOLDER-AUDIT", "folder", "audit", ("Audit reports, receipts, follow-up findings, and evidence bundles.",), keywords=("audit", "summary", "findings", "receipt", "score")),
    "FOLDER-SCRIPTS": TargetSpec("FOLDER-SCRIPTS", "folder", "scripts", ("Operator scripts, bootstrap flows, system utilities, and unit sources.",), keywords=("bash", "systemd", "runtime", "verify", "audit")),
    "FOLDER-TESTS": TargetSpec("FOLDER-TESTS", "folder", "tests", ("Cross-crate integration tests and Python audit tests.",), keywords=("test", "unittest", "fixture", "assert", "integration")),
    "FOLDER-ARCHIVE": TargetSpec("FOLDER-ARCHIVE", "folder", "archive", ("Historical snapshots and retired evidence; avoid as active source of truth.",), keywords=("archive", "historical", "retired", "snapshot", "superseded")),
    "FOLDER-ARCHIVED-SCRIPTS": TargetSpec("FOLDER-ARCHIVED-SCRIPTS", "folder", "archived_scripts", ("Retired automation kept for historical reference.",), keywords=("archive", "retired", "script", "historical", "superseded")),
}

TARGETS.update(CRATE_TARGETS)
TARGETS.update(FOLDER_TARGETS)
PHASE5_TARGETS = ("RUMIL", "PROMETHEUS", "MANWE") + tuple(CRATE_TARGETS) + tuple(FOLDER_TARGETS)
ALL_TARGETS = tuple(TARGETS)


TEXT_SUFFIXES = {".rs", ".toml", ".yaml", ".yml", ".json", ".md", ".sh", ".py", ".service", ".txt"}
CRASH_TOKENS = (".unwrap()", ".expect(", "panic!(", "todo!(", "unimplemented!(")
# Crash tokens are Rust panic-path syntax. Keep reliability scoring scoped to active
# production Rust source so markdown, JSON receipts, historical audit evidence,
# test modules, and benchmark fixtures can mention panic-path syntax without being
# treated as production crash paths.
ACTIVE_CRASH_SUFFIXES = {".rs"}
OBSERVABILITY_TOKENS = ("receipt", "audit", "metric", "telemetry", "status", "tracing", "log::", "println!")
RUST_TEST_PATH_PARTS = {"tests", "benches"}
RUST_TEST_FILE_NAMES = {"tests.rs"}
RUST_TEST_FILE_SUFFIXES = ("_test.rs", "_tests.rs")


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def default_run_id(now: datetime | None = None) -> str:
    now = now or datetime.now(timezone.utc)
    return f"{DEFAULT_RUN_ID_PREFIX}-{now.strftime('%Y%m%dT%H%M%SZ')}"


def default_output_dir(root: Path, run_id: str, now: datetime | None = None) -> Path:
    now = now or datetime.now(timezone.utc)
    day = now.strftime("%Y-%m-%d")
    return root / "audit" / "system-audit-runs" / day / run_id


def rel_path(path: Path, root: Path) -> str:
    try:
        return path.relative_to(root).as_posix()
    except ValueError:
        return path.as_posix()


def git_tracked_files(root: Path) -> list[Path]:
    try:
        result = subprocess.run(
            ["git", "ls-files"],
            cwd=root,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except (OSError, subprocess.CalledProcessError):
        return [p for p in root.rglob("*") if p.is_file() and ".git" not in p.parts]
    return [root / line for line in result.stdout.splitlines() if line.strip()]


def workspace_members(root: Path) -> set[str]:
    manifest = root / "Cargo.toml"
    if not manifest.exists():
        return set()
    try:
        data = tomllib.loads(manifest.read_text(encoding="utf-8"))
    except (tomllib.TOMLDecodeError, UnicodeDecodeError):
        return set()
    members = data.get("workspace", {}).get("members", [])
    return {str(member).rstrip("/") for member in members}


def is_text_candidate(path: Path) -> bool:
    return path.suffix.lower() in TEXT_SUFFIXES or path.name in {"Cargo.toml", "README.md", "AGENTS.md"}


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        return path.read_text(encoding="utf-8", errors="ignore")


def is_rust_test_path(path: Path, root: Path) -> bool:
    rel = path.relative_to(root) if path.is_absolute() and path.is_relative_to(root) else path
    return (
        path.name in RUST_TEST_FILE_NAMES
        or path.name.endswith(RUST_TEST_FILE_SUFFIXES)
        or any(part in RUST_TEST_PATH_PARTS for part in rel.parts)
    )


def strip_cfg_test_modules(text: str) -> str:
    """Remove inline #[cfg(test)] mod blocks before production crash-token scoring."""
    output: list[str] = []
    cursor = 0
    marker = "#[cfg(test)]"
    while True:
        start = text.find(marker, cursor)
        if start == -1:
            output.append(text[cursor:])
            break
        mod_start = text.find("mod", start + len(marker))
        brace_start = text.find("{", start + len(marker))
        if mod_start == -1 or brace_start == -1 or mod_start > brace_start:
            output.append(text[cursor : start + len(marker)])
            cursor = start + len(marker)
            continue
        output.append(text[cursor:start])
        depth = 0
        end = brace_start
        while end < len(text):
            char = text[end]
            if char == "{":
                depth += 1
            elif char == "}":
                depth -= 1
                if depth == 0:
                    end += 1
                    break
            end += 1
        cursor = end
    return "".join(output)


def production_rust_text(path: Path, root: Path, text: str) -> str:
    if path.suffix not in ACTIVE_CRASH_SUFFIXES or is_rust_test_path(path, root):
        return ""
    return strip_cfg_test_modules(text)


def files_under(root: Path, rel_root: str, tracked: Iterable[Path]) -> list[Path]:
    base = root / rel_root
    return [p for p in tracked if p.is_file() and is_text_candidate(p) and (p == base or base in p.parents)]


def load_portability_blockers(root: Path) -> list[dict[str, Any]]:
    candidates = sorted((root / "audit").glob("PORTABILITY_AUDIT_*/active-blockers.jsonl"))
    if not candidates:
        return []
    latest = candidates[-1]
    blockers: list[dict[str, Any]] = []
    for line in latest.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        try:
            item = json.loads(line)
        except json.JSONDecodeError:
            continue
        item["receipt_path"] = rel_path(latest, root)
        blockers.append(item)
    return blockers


def target_portability_blockers(spec: TargetSpec, blockers: list[dict[str, Any]]) -> list[dict[str, Any]]:
    prefixes = [spec.root, *spec.support_paths]
    matched = []
    for item in blockers:
        path = item.get("path", "")
        if any(path == prefix or path.startswith(prefix.rstrip("/") + "/") for prefix in prefixes):
            matched.append(item)
    return matched


def collect_target_snapshot(root: Path, spec: TargetSpec, tracked: list[Path], workspace: set[str]) -> dict[str, Any]:
    primary_files = files_under(root, spec.root, tracked)
    support_status = []
    support_files: list[Path] = []
    for support in spec.support_paths:
        path = root / support
        support_status.append({"path": support, "exists": path.exists()})
        if path.exists():
            if path.is_file() and is_text_candidate(path):
                support_files.append(path)
            elif path.is_dir():
                support_files.extend(files_under(root, support, tracked))
    scanned = primary_files + [p for p in support_files if p not in primary_files]
    text_chunks: list[tuple[Path, str]] = []
    for path in scanned[:250]:
        text_chunks.append((path, read_text(path)))
    joined_lower = "\n".join(text.lower() for _, text in text_chunks)
    keyword_hits = {keyword: joined_lower.count(keyword.lower()) for keyword in spec.keywords}
    crash_counts = Counter()
    observability_count = 0
    for path, text in text_chunks:
        production_text = production_rust_text(path, root, text)
        for token in CRASH_TOKENS:
            crash_counts[token] += production_text.count(token)
        lowered = text.lower()
        observability_count += sum(lowered.count(token.lower()) for token in OBSERVABILITY_TOKENS)
    test_files = [p for p in primary_files if "test" in p.name.lower() or "/tests/" in rel_path(p, root)]
    return {
        "primary_exists": (root / spec.root).exists(),
        "primary_file_count": len(primary_files),
        "workspace_member": spec.root in workspace,
        "rollup_target": spec.rollup_target,
        "rust_file_count": sum(1 for p in primary_files if p.suffix == ".rs"),
        "test_file_count": len(test_files),
        "support_status": support_status,
        "keyword_hits": keyword_hits,
        "crash_counts": dict(crash_counts),
        "observability_hits": observability_count,
        "sample_files": [rel_path(p, root) for p in primary_files[:8]],
    }


def score_snapshot(snapshot: dict[str, Any], blockers: list[dict[str, Any]]) -> dict[str, int]:
    keyword_total = sum(snapshot["keyword_hits"].values())
    crash_total = sum(snapshot["crash_counts"].values())
    support_total = len(snapshot["support_status"])
    support_present = sum(1 for item in snapshot["support_status"] if item["exists"])
    score = {
        "mission_fit": 14 + min(6, keyword_total),
        "implementation_completeness": 0,
        "reliability_safety": max(8, 20 - min(12, crash_total)),
        "observability_auditability": min(15, 6 + snapshot["observability_hits"] // 3 + snapshot["test_file_count"]),
        "portability_config_hygiene": max(4, 15 - min(11, len(blockers))),
        "ux_operator_experience": 6 + (2 if support_present else 0) + (1 if snapshot["sample_files"] else 0),
    }
    if snapshot["primary_exists"]:
        score["implementation_completeness"] = 10 + min(8, snapshot["rust_file_count"]) + (2 if snapshot["test_file_count"] else 0)
    if support_total:
        score["ux_operator_experience"] += min(1, support_present // max(1, support_total))
    return {key: min(RUBRIC_MAX[key], max(0, value)) for key, value in score.items()}


def build_report(root: Path, run_id: str, spec: TargetSpec, tracked: list[Path], portability_blockers: list[dict[str, Any]], workspace: set[str] | None = None) -> dict[str, Any]:
    workspace = workspace or set()
    snapshot = collect_target_snapshot(root, spec, tracked, workspace)
    blockers = target_portability_blockers(spec, portability_blockers)
    score_breakdown = score_snapshot(snapshot, blockers)
    score = sum(score_breakdown.values())
    crash_total = sum(snapshot["crash_counts"].values())
    support_missing = [item["path"] for item in snapshot["support_status"] if not item["exists"]]

    good = []
    bad = []
    ugly = []
    potential_changes = []
    needs_removed = []

    if snapshot["primary_exists"]:
        if spec.target_type == "folder":
            good.append(f"Folder surface `{spec.root}` exists with {snapshot['primary_file_count']} tracked text files sampled for this audit.")
        else:
            good.append(f"Primary implementation surface exists at `{spec.root}` with {snapshot['rust_file_count']} tracked Rust files.")
    else:
        ugly.append(f"Primary implementation surface `{spec.root}` is missing.")
    if spec.target_type == "workspace_crate":
        if snapshot["workspace_member"]:
            good.append("Crate is listed in the live Cargo workspace manifest.")
        else:
            ugly.append("Crate directory exists but is not listed in the live Cargo workspace manifest.")
    if spec.target == "ARDA-CHRONOS" and snapshot["workspace_member"]:
        good.append("Live evidence supersedes the older plan note: arda-chronos is currently a workspace member.")
    if snapshot["rollup_target"]:
        good.append(f"Findings can be rolled up into primary target `{snapshot['rollup_target']}` while keeping crate-level evidence traceable.")
    if snapshot["test_file_count"]:
        good.append(f"Found {snapshot['test_file_count']} test-like files under the primary target surface.")
    elif spec.target_type == "folder":
        potential_changes.append("Review whether this folder needs explicit validation or only upstream crate/script tests; do not add tests mechanically.")
    else:
        bad.append("No target-local test-like file was detected in the primary surface.")
    if snapshot["observability_hits"]:
        good.append(f"Observed {snapshot['observability_hits']} audit/status/telemetry/logging token hits in scoped files.")
    elif spec.target_type == "folder":
        bad.append("Folder has no obvious audit/status/telemetry/logging token hits in sampled tracked files; decide whether that is expected for this surface.")
    else:
        bad.append("No obvious audit/status/telemetry/logging token hits were found in scoped files.")
    if support_missing:
        bad.append("Expected support paths missing or renamed: " + ", ".join(f"`{p}`" for p in support_missing) + ".")
    if crash_total:
        bad.append(f"Scoped files contain {crash_total} crash-path tokens (`unwrap`, `expect`, `panic`, `todo`, or `unimplemented`) requiring manual review.")
        potential_changes.append("Classify crash-path tokens as production, test, or impossible-state assertions and remove production `unwrap()` usages.")
    if spec.target_type == "folder":
        potential_changes.append("Keep current useful operational surface, parameterize portability blockers, and remove only stale/generated/archive drift after a separate evidence pass.")
    if blockers:
        ugly.append(f"Latest portability receipt reports {len(blockers)} active blockers on this target/support surface.")
        potential_changes.append("Parameterize active portability blockers in a bounded follow-up slice with tests and regenerated receipts.")
    else:
        good.append("Latest portability receipt has no active blockers for this target/support surface.")
    if not needs_removed:
        needs_removed.append("No automatic removal recommendation; removal candidates require a follow-up evidence pass.")

    evidence = [
        {"path": spec.root, "line": None, "note": "primary target root", "exists": snapshot["primary_exists"]},
        {"path": "Cargo.toml", "line": None, "note": f"workspace membership checked from repository manifest: {snapshot['workspace_member']}", "exists": (root / "Cargo.toml").exists()},
    ]
    for path in snapshot["sample_files"][:5]:
        evidence.append({"path": path, "line": None, "note": "sample tracked target file", "exists": True})
    for item in snapshot["support_status"]:
        evidence.append({"path": item["path"], "line": None, "note": "support path", "exists": item["exists"]})
    for blocker in blockers[:5]:
        evidence.append({
            "path": blocker.get("path"),
            "line": blocker.get("line"),
            "note": f"portability blocker {blocker.get('pattern_id')} from {blocker.get('receipt_path')}",
            "exists": True,
        })

    candidate_tasks = []
    if blockers:
        candidate_tasks.append({
            "title": f"Parameterize {spec.target} active portability blockers",
            "owner": spec.target.lower().replace("_", "-"),
            "priority": "high",
            "risk_class": "non_destructive",
            "acceptance": "Focused tests pass; regenerated portability audit no longer lists the remediated files as active blockers.",
        })
    if crash_total:
        candidate_tasks.append({
            "title": f"Classify and remediate {spec.target} crash-path tokens",
            "owner": spec.target.lower().replace("_", "-"),
            "priority": "medium",
            "risk_class": "source_change_review_required",
            "acceptance": "Production `unwrap()` usages are removed or proven unreachable; tests cover changed paths.",
        })
    if spec.target_type != "folder" and not snapshot["test_file_count"]:
        candidate_tasks.append({
            "title": f"Add target-local tests for {spec.target}",
            "owner": spec.target.lower().replace("_", "-"),
            "priority": "medium",
            "risk_class": "source_change_review_required",
            "acceptance": "Target-local test coverage exists for the primary duty surface and passes in focused cargo test runs.",
        })

    report = {
        "contract": CONTRACT,
        "run_id": run_id,
        "generated_at": utc_now(),
        "target": spec.target,
        "target_type": spec.target_type,
        "score": score,
        "score_breakdown": score_breakdown,
        "overview": f"Read-only Phase 4/5 Good / Bad / Ugly audit for {spec.target}.",
        "duties": list(spec.duties),
        "good": good,
        "bad": bad,
        "ugly": ugly,
        "potential_changes": potential_changes or ["No immediate source change proposed by this read-only audit."],
        "needs_removed": needs_removed,
        "evidence": evidence,
        "candidate_tasks": candidate_tasks,
        "snapshot": snapshot,
    }
    validate_report(report)
    return report


def validate_report(report: dict[str, Any]) -> None:
    missing = REQUIRED_REPORT_KEYS - report.keys()
    if missing:
        raise ValueError(f"report missing required keys: {sorted(missing)}")
    if report["contract"] != CONTRACT:
        raise ValueError(f"unexpected contract: {report['contract']}")
    breakdown = report["score_breakdown"]
    if set(breakdown) != set(RUBRIC_MAX):
        raise ValueError(f"score breakdown keys mismatch: {sorted(breakdown)}")
    for key, max_value in RUBRIC_MAX.items():
        value = breakdown[key]
        if not isinstance(value, int) or value < 0 or value > max_value:
            raise ValueError(f"invalid score for {key}: {value}")
    if report["score"] != sum(breakdown.values()):
        raise ValueError("score does not equal score_breakdown sum")
    for key in ("good", "bad", "ugly", "potential_changes", "needs_removed", "evidence", "candidate_tasks"):
        if not isinstance(report[key], list):
            raise ValueError(f"{key} must be a list")


def report_markdown(report: dict[str, Any]) -> str:
    def bullets(items: list[Any]) -> str:
        if not items:
            return "- None\n"
        lines = []
        for item in items:
            if isinstance(item, dict):
                note = item.get("note") or item.get("title") or json.dumps(item, sort_keys=True)
                path = item.get("path")
                line = item.get("line")
                if path and line:
                    suffix = f" (`{path}`:{line})"
                elif path:
                    suffix = f" (`{path}`)"
                else:
                    suffix = ""
                lines.append(f"- {note}{suffix}")
            else:
                lines.append(f"- {item}")
        return "\n".join(lines) + "\n"

    return f"""# {report['target']} Good / Bad / Ugly Audit

Generated: {report['generated_at']}
Run ID: `{report['run_id']}`
Contract: `{report['contract']}`
Score: **{report['score']}/100**

## Duties
{bullets(report['duties'])}
## Score Breakdown
{bullets([f"{key}: {value}/{RUBRIC_MAX[key]}" for key, value in report['score_breakdown'].items()])}
## Good
{bullets(report['good'])}
## Bad
{bullets(report['bad'])}
## Ugly
{bullets(report['ugly'])}
## Potential Changes
{bullets(report['potential_changes'])}
## What Needs Removed
{bullets(report['needs_removed'])}
## Evidence
{bullets(report['evidence'])}
## Candidate Tasks
{bullets(report['candidate_tasks'])}
""".strip() + "\n"


def report_subdir(spec: TargetSpec) -> str:
    if spec.target_type == "folder":
        return "folders"
    if spec.target_type == "workspace_crate":
        return "crates"
    return "targets"


def run_audit(root: Path, out_dir: Path, targets: Iterable[str], run_id: str | None = None) -> dict[str, Any]:
    root = root.resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    run_id = run_id or default_run_id()
    tracked = git_tracked_files(root)
    workspace = workspace_members(root)
    blockers = load_portability_blockers(root)
    target_names = [name.upper() for name in targets]
    reports = []

    for name in target_names:
        if name not in TARGETS:
            raise ValueError(f"unknown target {name}; known targets: {', '.join(sorted(TARGETS))}")
        spec = TARGETS[name]
        report = build_report(root, run_id, spec, tracked, blockers, workspace)
        subdir = report_subdir(spec)
        report_dir = out_dir / subdir
        report_dir.mkdir(parents=True, exist_ok=True)
        report["report_json_path"] = f"{subdir}/{name}.json"
        report["report_markdown_path"] = f"{subdir}/{name}.md"
        reports.append(report)
        (report_dir / f"{name}.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        (report_dir / f"{name}.md").write_text(report_markdown(report), encoding="utf-8")

    findings = []
    tasks = []
    for report in reports:
        for category in ("bad", "ugly"):
            for item in report[category]:
                findings.append({
                    "run_id": run_id,
                    "target": report["target"],
                    "category": category,
                    "finding": item,
                    "score": report["score"],
                })
        for task in report["candidate_tasks"]:
            tasks.append({"run_id": run_id, "target": report["target"], **task})

    (out_dir / "findings.jsonl").write_text("".join(json.dumps(item, sort_keys=True) + "\n" for item in findings), encoding="utf-8")
    (out_dir / "tasks-candidate.jsonl").write_text("".join(json.dumps(item, sort_keys=True) + "\n" for item in tasks), encoding="utf-8")
    scores = {report["target"]: {"score": report["score"], "score_breakdown": report["score_breakdown"]} for report in reports}
    (out_dir / "agent-scores.json").write_text(json.dumps(scores, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    summary = {
        "contract": RUN_CONTRACT,
        "run_id": run_id,
        "generated_at": utc_now(),
        "layout": {
            "contract": "arda.audit.run_layout.v1",
            "scope": "system-audit-runs",
            "date_first": True,
            "default_shape": "audit/system-audit-runs/YYYY-MM-DD/<run-id>",
        },
        "targets": target_names,
        "target_count": len(reports),
        "findings_count": len(findings),
        "candidate_task_count": len(tasks),
        "scores": scores,
        "output_dir": out_dir.as_posix(),
    }
    (out_dir / "summary.json").write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    (out_dir / "system-summary.md").write_text(system_summary_markdown(summary, reports), encoding="utf-8")
    (out_dir / "INDEX.md").write_text(index_markdown(summary, reports), encoding="utf-8")
    return summary


def system_summary_markdown(summary: dict[str, Any], reports: list[dict[str, Any]]) -> str:
    rows = ["| Target | Score | Candidate tasks |", "|---|---:|---:|"]
    for report in reports:
        rows.append(f"| {report['target']} | {report['score']} | {len(report['candidate_tasks'])} |")
    return f"""# Arda System Audit Summary

Run ID: `{summary['run_id']}`
Generated: {summary['generated_at']}
Contract: `{summary['contract']}`

{chr(10).join(rows)}

Findings JSONL: `findings.jsonl`
Candidate tasks JSONL: `tasks-candidate.jsonl`
Scores JSON: `agent-scores.json`
"""


def index_markdown(summary: dict[str, Any], reports: list[dict[str, Any]]) -> str:
    lines = ["# System Audit Run Index", "", f"Run ID: `{summary['run_id']}`", "", "## Targets"]
    for report in reports:
        default_report_path = f"targets/{report['target']}.md"
        report_path = report.get("report_markdown_path", default_report_path)
        lines.append(f"- [{report['target']}]({report_path}) — {report['score']}/100")
    lines.extend(["", "## Receipts", "- `summary.json`", "- `agent-scores.json`", "- `findings.jsonl`", "- `tasks-candidate.jsonl`"])
    return "\n".join(lines) + "\n"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--out", type=Path, default=None)
    parser.add_argument("--run-id", default=None)
    parser.add_argument("--targets", nargs="+", default=["RUMIL", "PROMETHEUS", "MANWE"])
    parser.add_argument("--target-set", choices=["explicit", "first-batch", "phase5", "all"], default="explicit")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    now = datetime.now(timezone.utc)
    run_id = args.run_id or default_run_id(now)
    out = args.out or default_output_dir(args.root, run_id, now)
    targets = args.targets
    if args.target_set == "first-batch":
        targets = ["RUMIL", "PROMETHEUS", "MANWE"]
    elif args.target_set == "phase5":
        targets = list(PHASE5_TARGETS)
    elif args.target_set == "all":
        targets = list(ALL_TARGETS)
    summary = run_audit(args.root, out, targets, run_id)
    print(json.dumps(summary, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
