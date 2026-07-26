#!/usr/bin/env python3
"""Read-only repeated Arda audit automation runner.

Phase 7 turns existing audit receipts into a cyclic status surface. It does not
rewrite source/config/service state; it only reads prior audit artifacts and
emits repeated-run receipts, trend comparisons, regression findings, candidate
tasks, and ARDA/Hermes projection state.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable

CONTRACT = "arda.audit.repeated_run.v1"
DEFAULT_OUT_ROOT = Path("audit/repeated-audit-runs")
DEFAULT_STATE_PATH = Path("core/state/repeated_audit_status.json")
DEFAULT_PORTABILITY_SUMMARY = Path("audit/PORTABILITY_AUDIT_2026-05-24/summary.json")
DEFAULT_SETUP_RECEIPT = Path("audit/SETUP_CONSOLE_READINESS_2026-05-25/setup_console_readiness_receipt.json")
DEFAULT_SYSTEM_RUNS_ROOT = Path("audit/system-audit-runs")


def utc_now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def default_run_id() -> str:
    return f"repeated-audit-{datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')}"


def read_json(path: Path) -> dict[str, Any] | None:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError, UnicodeDecodeError):
        return None
    return data if isinstance(data, dict) else None


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_jsonl(path: Path, rows: Iterable[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("".join(json.dumps(row, sort_keys=True) + "\n" for row in rows), encoding="utf-8")


def repo_relative(path: Path, root: Path) -> str:
    try:
        return path.resolve().relative_to(root.resolve()).as_posix()
    except ValueError:
        return path.as_posix()


def resolve_under_root(root: Path, path: Path) -> Path:
    return path if path.is_absolute() else root / path


def discover_latest_summary(root: Path, runs_root: Path) -> Path | None:
    base = resolve_under_root(root, runs_root)
    candidates = [path for path in base.glob("**/summary.json") if path.is_file()]
    if not candidates:
        return None
    return max(candidates, key=lambda path: path.stat().st_mtime)


def load_latest_previous_repeated(root: Path, out_root: Path, current_out: Path) -> dict[str, Any] | None:
    base = resolve_under_root(root, out_root)
    candidates = []
    for path in base.glob("*/summary.json"):
        if path.resolve() == (current_out / "summary.json").resolve():
            continue
        data = read_json(path)
        if data and data.get("contract") == CONTRACT:
            candidates.append((path.stat().st_mtime, data))
    if not candidates:
        return None
    return max(candidates, key=lambda item: item[0])[1]


def portability_snapshot(path: Path | None, root: Path) -> dict[str, Any]:
    if path is None:
        return {"status": "missing", "path": None, "active_blocker_findings": None, "findings_total": None, "classification_counts": {}, "pattern_counts": {}, "top_active_blockers": []}
    data = read_json(path)
    if not data:
        return {"status": "missing", "path": repo_relative(path, root), "active_blocker_findings": None, "findings_total": None, "classification_counts": {}, "pattern_counts": {}, "top_active_blockers": []}
    summary = data.get("summary", {}) if isinstance(data.get("summary"), dict) else {}
    return {
        "status": "present",
        "path": repo_relative(path, root),
        "active_blocker_findings": int(summary.get("active_blocker_findings", 0) or 0),
        "findings_total": int(summary.get("findings_total", summary.get("total_findings", 0)) or 0),
        "classification_counts": summary.get("classification_counts", {}) if isinstance(summary.get("classification_counts"), dict) else {},
        "pattern_counts": summary.get("pattern_counts", {}) if isinstance(summary.get("pattern_counts"), dict) else {},
        "top_active_blockers": summary.get("top_active_blockers", []) if isinstance(summary.get("top_active_blockers"), list) else [],
    }


def setup_snapshot(path: Path | None, root: Path) -> dict[str, Any]:
    if path is None:
        return {"status": "missing", "path": None, "gate_status": "missing", "summary": {}, "portability_status": {}}
    data = read_json(path)
    if not data:
        return {"status": "missing", "path": repo_relative(path, root), "gate_status": "missing", "summary": {}, "portability_status": {}}
    portability_status = data.get("portability_status", {})
    return {
        "status": "present",
        "path": repo_relative(path, root),
        "gate_status": str(data.get("gate_status", "unknown")),
        "summary": data.get("summary", {}) if isinstance(data.get("summary"), dict) else {},
        "portability_status": portability_status if isinstance(portability_status, dict) else {},
    }


def system_snapshot(path: Path | None, root: Path) -> dict[str, Any]:
    if path is None:
        return {"status": "missing", "path": None, "run_id": None, "target_count": 0, "findings_count": None, "candidate_task_count": None, "scores": {}}
    data = read_json(path)
    if not data:
        return {"status": "missing", "path": repo_relative(path, root), "run_id": None, "target_count": 0, "findings_count": None, "candidate_task_count": None, "scores": {}}
    scores = data.get("scores", {}) if isinstance(data.get("scores"), dict) else {}
    return {
        "status": "present",
        "path": repo_relative(path, root),
        "run_id": data.get("run_id"),
        "target_count": int(data.get("target_count", len(scores)) or 0),
        "findings_count": int(data.get("findings_count", 0) or 0),
        "candidate_task_count": int(data.get("candidate_task_count", 0) or 0),
        "scores": scores,
    }


def score_value(score_payload: Any) -> int | None:
    if isinstance(score_payload, dict):
        value = score_payload.get("score")
        return int(value) if isinstance(value, int) else None
    return None


def compare_trends(current: dict[str, Any], previous: dict[str, Any] | None) -> dict[str, Any]:
    if previous is None:
        return {"baseline": "first_repeated_run", "score_deltas": {}, "aggregate_deltas": {}}
    prior_snapshot = previous.get("snapshot", {}) if isinstance(previous.get("snapshot"), dict) else {}
    prior_system = prior_snapshot.get("system_audit", {}) if isinstance(prior_snapshot.get("system_audit"), dict) else {}
    prior_portability = prior_snapshot.get("portability", {}) if isinstance(prior_snapshot.get("portability"), dict) else {}
    prior_setup = prior_snapshot.get("setup_console", {}) if isinstance(prior_snapshot.get("setup_console"), dict) else {}

    current_system = current.get("system_audit", {}) if isinstance(current.get("system_audit"), dict) else {}
    current_portability = current.get("portability", {}) if isinstance(current.get("portability"), dict) else {}
    current_setup = current.get("setup_console", {}) if isinstance(current.get("setup_console"), dict) else {}

    score_deltas: dict[str, int] = {}
    current_scores = current_system.get("scores", {}) if isinstance(current_system.get("scores"), dict) else {}
    prior_scores = prior_system.get("scores", {}) if isinstance(prior_system.get("scores"), dict) else {}
    for target, score_payload in current_scores.items():
        current_score = score_value(score_payload)
        prior_score = score_value(prior_scores.get(target))
        if current_score is not None and prior_score is not None:
            score_deltas[target] = current_score - prior_score

    return {
        "baseline": "compared_to_previous_repeated_run",
        "previous_run_id": previous.get("run_id"),
        "score_deltas": score_deltas,
        "aggregate_deltas": {
            "system_findings_count": delta(current_system.get("findings_count"), prior_system.get("findings_count")),
            "system_candidate_task_count": delta(current_system.get("candidate_task_count"), prior_system.get("candidate_task_count")),
            "portability_active_blocker_findings": delta(current_portability.get("active_blocker_findings"), prior_portability.get("active_blocker_findings")),
            "portability_findings_total": delta(current_portability.get("findings_total"), prior_portability.get("findings_total")),
        },
        "setup_gate_transition": {
            "previous": prior_setup.get("gate_status"),
            "current": current_setup.get("gate_status"),
        },
    }


def delta(current: Any, previous: Any) -> int | None:
    if isinstance(current, int) and isinstance(previous, int):
        return current - previous
    return None


def detect_regressions(snapshot: dict[str, Any], trends: dict[str, Any]) -> list[dict[str, Any]]:
    regressions: list[dict[str, Any]] = []
    if snapshot.get("system_audit", {}).get("status") == "missing":
        regressions.append({"kind": "missing_receipt", "severity": "high", "message": "System audit summary is unavailable for cyclic comparison."})
    if snapshot.get("portability", {}).get("status") == "missing":
        regressions.append({"kind": "missing_receipt", "severity": "high", "message": "Portability summary is unavailable for cyclic comparison."})
    if snapshot.get("setup_console", {}).get("status") == "missing":
        regressions.append({"kind": "missing_receipt", "severity": "medium", "message": "Setup console readiness receipt is unavailable for ARDA/Hermes visibility."})

    for target, change in trends.get("score_deltas", {}).items():
        if change < 0:
            regressions.append({"kind": "score_regression", "severity": "high" if change <= -5 else "medium", "target": target, "delta": change, "message": f"{target} score decreased by {abs(change)} points."})

    for key, change in trends.get("aggregate_deltas", {}).items():
        if isinstance(change, int) and change > 0:
            severity = "high" if "portability_active_blocker" in key else "medium"
            regressions.append({"kind": "aggregate_regression", "severity": severity, "metric": key, "delta": change, "message": f"{key} increased by {change}."})

    transition = trends.get("setup_gate_transition", {}) if isinstance(trends.get("setup_gate_transition"), dict) else {}
    if transition.get("previous") == "pass" and transition.get("current") != "pass":
        regressions.append({"kind": "gate_regression", "severity": "high", "message": f"Setup console gate moved from pass to {transition.get('current')}."})

    return regressions


def generate_candidate_tasks(snapshot: dict[str, Any], regressions: list[dict[str, Any]]) -> list[dict[str, Any]]:
    tasks: list[dict[str, Any]] = []
    for index, regression in enumerate(regressions, start=1):
        tasks.append({
            "task_id": f"phase7_regression_{index:03d}",
            "priority": "high" if regression.get("severity") == "high" else "medium",
            "risk_class": "read_only_or_bounded_fix",
            "source": "repeated_audit_regression",
            "title": regression.get("message", "Investigate repeated audit regression"),
            "recommended_owner": "hades" if regression.get("kind") != "score_regression" else "prometheus",
        })

    system = snapshot.get("system_audit", {}) if isinstance(snapshot.get("system_audit"), dict) else {}
    scores = system.get("scores", {}) if isinstance(system.get("scores"), dict) else {}
    for target, payload in scores.items():
        score = score_value(payload)
        if score is not None and score < 80:
            tasks.append({
                "task_id": f"phase7_low_score_{target.lower().replace('-', '_')}",
                "priority": "medium",
                "risk_class": "read_only_then_bounded_fix",
                "source": "repeated_audit_score_threshold",
                "title": f"Review {target} audit score below 80 ({score}/100) and create bounded remediation slice.",
                "recommended_owner": "prometheus",
            })

    portability = snapshot.get("portability", {}) if isinstance(snapshot.get("portability"), dict) else {}
    blockers = portability.get("top_active_blockers", []) if isinstance(portability.get("top_active_blockers"), list) else []
    for blocker in blockers[:5]:
        if isinstance(blocker, dict) and blocker.get("path"):
            tasks.append({
                "task_id": f"phase7_portability_{len(tasks) + 1:03d}",
                "priority": "high",
                "risk_class": "bounded_parameterization",
                "source": "portability_top_active_blocker",
                "title": f"Parameterize portability blocker {blocker.get('path')} ({blocker.get('findings', '?')} findings) after focused test coverage.",
                "recommended_owner": "hades",
            })
    return tasks


def gate_status(regressions: list[dict[str, Any]]) -> str:
    if any(item.get("severity") == "high" for item in regressions):
        return "warn"
    return "pass" if not regressions else "warn"


def visibility_projection(snapshot: dict[str, Any]) -> dict[str, Any]:
    portability = snapshot.get("portability", {}) if isinstance(snapshot.get("portability"), dict) else {}
    setup_console = snapshot.get("setup_console", {}) if isinstance(snapshot.get("setup_console"), dict) else {}
    setup_portability = setup_console.get("portability_status", {}) if isinstance(setup_console.get("portability_status"), dict) else {}
    active_blockers = portability.get("active_blocker_findings")
    zero_active = active_blockers == 0
    return {
        "portability_active_blocker_findings": active_blockers,
        "portability_zero_active_blockers": zero_active,
        "portability_status_label": "zero active portability blockers" if zero_active else "active portability blockers present",
        "portability_summary_path": portability.get("path"),
        "setup_console_portability_status": setup_portability.get("status"),
        "setup_console_portability_status_label": setup_portability.get("label"),
        "setup_console_portability_active_blocker_findings": setup_portability.get("active_blocker_findings"),
        "setup_console_portability_source": setup_portability.get("source"),
    }


def markdown_summary(summary: dict[str, Any]) -> str:
    severity_counts = Counter(item.get("severity", "unknown") for item in summary.get("regressions", []))
    lines = [
        "# Repeated Audit Automation Summary",
        "",
        f"Run ID: `{summary['run_id']}`",
        f"Generated: {summary['generated_at_utc']}",
        f"Contract: `{summary['contract']}`",
        f"Gate status: **{summary['gate_status']}**",
        "",
        "## Source receipts",
        "",
    ]
    for key, value in summary.get("snapshot", {}).items():
        if isinstance(value, dict):
            lines.append(f"- {key}: {value.get('status')} — `{value.get('path')}`")
    lines.extend(["", "## Trend comparison", "", f"- Baseline: {summary.get('trends', {}).get('baseline')}"])
    previous_run = summary.get("trends", {}).get("previous_run_id")
    if previous_run:
        lines.append(f"- Previous run: `{previous_run}`")
    for metric, change in summary.get("trends", {}).get("aggregate_deltas", {}).items():
        lines.append(f"- {metric}: {change}")
    lines.extend(["", "## Regressions", ""])
    if summary.get("regressions"):
        for item in summary["regressions"]:
            lines.append(f"- {item.get('severity', 'unknown').upper()}: {item.get('message')}")
    else:
        lines.append("- None detected")
    lines.extend(["", "## Candidate tasks", ""])
    if summary.get("candidate_tasks"):
        for task in summary["candidate_tasks"]:
            lines.append(f"- [{task['priority']}] {task['title']} ({task['risk_class']})")
    else:
        lines.append("- None")
    lines.extend([
        "",
        "## Visibility",
        "",
        f"- Portability status: {summary.get('visibility', {}).get('portability_status_label')} ({summary.get('visibility', {}).get('portability_active_blocker_findings')} active blockers)",
        f"- Setup-console portability status: {summary.get('visibility', {}).get('setup_console_portability_status_label')} ({summary.get('visibility', {}).get('setup_console_portability_active_blocker_findings')} active blockers)",
        f"- Portability zero-active-blocker projection: {summary.get('visibility', {}).get('portability_zero_active_blockers')}",
        f"- ARDA/Hermes state: `{summary.get('outputs', {}).get('state_json')}`",
        f"- Regression JSONL: `{summary.get('outputs', {}).get('regressions_jsonl')}`",
        f"- Candidate task JSONL: `{summary.get('outputs', {}).get('tasks_candidate_jsonl')}`",
        "",
        "## Scope guard",
        "",
        "This Phase 7 runner is read-only except for generated receipt/state/Markdown artifacts. It does not perform autonomous destructive cleanup, source rewrites, config rewrites, service restarts, or queue mutation.",
        "",
        "## Severity counts",
        "",
    ])
    if severity_counts:
        for severity, count in sorted(severity_counts.items()):
            lines.append(f"- {severity}: {count}")
    else:
        lines.append("- none: 0")
    return "\n".join(lines) + "\n"


def run_repeated_audit(
    root: Path,
    out_dir: Path,
    state_path: Path,
    run_id: str,
    portability_summary_path: Path | None,
    setup_receipt_path: Path | None,
    system_summary_path: Path | None,
    out_root: Path,
) -> dict[str, Any]:
    root = root.resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    snapshot = {
        "portability": portability_snapshot(portability_summary_path, root),
        "setup_console": setup_snapshot(setup_receipt_path, root),
        "system_audit": system_snapshot(system_summary_path, root),
    }
    previous = load_latest_previous_repeated(root, out_root, out_dir)
    trends = compare_trends(snapshot, previous)
    regressions = detect_regressions(snapshot, trends)
    candidate_tasks = generate_candidate_tasks(snapshot, regressions)

    summary_path = out_dir / "summary.json"
    regressions_path = out_dir / "regressions.jsonl"
    tasks_path = out_dir / "tasks-candidate.jsonl"
    md_path = out_dir / "SUMMARY.md"

    summary = {
        "contract": CONTRACT,
        "run_id": run_id,
        "generated_at_utc": utc_now(),
        "mode": "read_only",
        "mutation_policy": "receipts_and_projection_state_only_no_source_config_service_or_queue_mutation",
        "gate_status": gate_status(regressions),
        "snapshot": snapshot,
        "trends": trends,
        "visibility": visibility_projection(snapshot),
        "regressions": regressions,
        "regression_count": len(regressions),
        "candidate_tasks": candidate_tasks,
        "candidate_task_count": len(candidate_tasks),
        "outputs": {
            "summary_json": repo_relative(summary_path, root),
            "summary_md": repo_relative(md_path, root),
            "regressions_jsonl": repo_relative(regressions_path, root),
            "tasks_candidate_jsonl": repo_relative(tasks_path, root),
            "state_json": repo_relative(state_path, root),
        },
    }

    write_json(summary_path, summary)
    write_jsonl(regressions_path, regressions)
    write_jsonl(tasks_path, candidate_tasks)
    md_path.write_text(markdown_summary(summary), encoding="utf-8")
    write_json(state_path, summary)
    return summary


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--out-root", type=Path, default=DEFAULT_OUT_ROOT)
    parser.add_argument("--out", type=Path, default=None)
    parser.add_argument("--state-path", type=Path, default=DEFAULT_STATE_PATH)
    parser.add_argument("--run-id", default=None)
    parser.add_argument("--portability-summary", type=Path, default=DEFAULT_PORTABILITY_SUMMARY)
    parser.add_argument("--setup-receipt", type=Path, default=DEFAULT_SETUP_RECEIPT)
    parser.add_argument("--system-summary", type=Path, default=None)
    parser.add_argument("--system-runs-root", type=Path, default=DEFAULT_SYSTEM_RUNS_ROOT)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root = args.root.resolve()
    run_id = args.run_id or default_run_id()
    out_root = resolve_under_root(root, args.out_root)
    out_dir = resolve_under_root(root, args.out) if args.out else out_root / run_id
    state_path = resolve_under_root(root, args.state_path)
    portability_summary_path = resolve_under_root(root, args.portability_summary) if args.portability_summary else None
    setup_receipt_path = resolve_under_root(root, args.setup_receipt) if args.setup_receipt else None
    system_summary_path = resolve_under_root(root, args.system_summary) if args.system_summary else discover_latest_summary(root, args.system_runs_root)
    summary = run_repeated_audit(
        root,
        out_dir,
        state_path,
        run_id,
        portability_summary_path,
        setup_receipt_path,
        system_summary_path,
        args.out_root,
    )
    print(json.dumps({
        "gate_status": summary["gate_status"],
        "run_id": summary["run_id"],
        "summary": summary["outputs"]["summary_json"],
        "state": summary["outputs"]["state_json"],
        "regression_count": summary["regression_count"],
        "candidate_task_count": summary["candidate_task_count"],
    }, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
