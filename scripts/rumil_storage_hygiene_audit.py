#!/usr/bin/env python3
"""Generate a no-delete Rúmil storage hygiene audit.

The audit classifies high-churn and bulky repository surfaces so cleanup can be
approved from evidence instead of guessed from directory names.
"""

from __future__ import annotations

import argparse
import json
import os
import re
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

CONTRACT = "arda.rumil.storage_hygiene_audit.v1"
DEFAULT_ROOTS = ("audit", "data", "logs", "tmp", ".tmp", "core/state")
MODEL_SUFFIXES = {
    ".bin",
    ".gguf",
    ".onnx",
    ".pt",
    ".pth",
    ".safetensors",
}
BACKUP_RE = re.compile(r"\.bak(?:\.|$)|~$|\.orig$")
TICK_RE = re.compile(r"^tick_output_\d+\.txt$")


def now_utc() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def repo_relative(path: Path, root: Path) -> str:
    try:
        return path.resolve().relative_to(root.resolve()).as_posix()
    except ValueError:
        return path.as_posix()


def stat_mtime_utc(path: Path) -> str:
    return datetime.fromtimestamp(path.stat().st_mtime, timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def iter_files(root: Path, roots: list[str]) -> list[Path]:
    files: list[Path] = []
    for rel in roots:
        base = root / rel
        if not base.exists():
            continue
        if base.is_file():
            files.append(base)
            continue
        for dirpath, dirnames, filenames in os.walk(base):
            dirnames[:] = [name for name in dirnames if name not in {".git", "target", "node_modules"}]
            for name in filenames:
                path = Path(dirpath) / name
                if path.is_file():
                    files.append(path)
    return files


def top_level_for(path: Path, root: Path) -> str:
    rel = repo_relative(path, root)
    if rel.startswith("core/state/"):
        return "core/state"
    return rel.split("/", 1)[0]


def classify(path: Path, root: Path) -> tuple[str, str]:
    rel = repo_relative(path, root)
    name = path.name
    suffix = path.suffix.lower()

    if suffix in MODEL_SUFFIXES:
        return ("misplaced_model_artifact", "move_to_models_root")
    if rel.startswith(".tmp/") or rel.startswith("tmp/"):
        return ("rebuildable_temp", "delete_after_active_process_check")
    if rel.startswith("logs/"):
        return ("runtime_log", "rotate_or_delete_after_service_check")
    if BACKUP_RE.search(name):
        return ("runtime_backup", "retain_latest_n_then_archive_or_delete")
    if rel.startswith("core/state/") and TICK_RE.match(name):
        return ("generated_tick_output", "remove_or_move_to_runtime_logs")
    if rel.startswith("audit/"):
        parts = rel.split("/")
        if len(parts) >= 3 and re.match(r"^\d{4}-\d{2}-\d{2}$", parts[2]):
            return ("date_partitioned_audit_receipt", "retain_latest_then_archive_by_family")
        return ("audit_evidence", "retain_index_and_current_receipts")
    if rel.startswith("data/"):
        if suffix in {".jsonl", ".log"}:
            return ("runtime_ledger", "compact_by_policy")
        if "/archive/" in rel:
            return ("runtime_archive", "review_retention_window")
        return ("runtime_data", "classify_owner_policy")
    if rel.startswith("core/state/"):
        if suffix in {".json", ".jsonl"}:
            return ("core_state_projection", "suppress_noop_or_promote_receipt")
        return ("core_state_misc", "review_owner_policy")
    return ("other", "manual_review")


def summarize(root: Path, roots: list[str], largest_limit: int, candidate_limit: int) -> dict[str, Any]:
    files = iter_files(root, roots)
    generated_at = now_utc()
    by_top: dict[str, dict[str, Any]] = defaultdict(lambda: {"bytes": 0, "files": 0})
    by_class: dict[str, dict[str, Any]] = defaultdict(lambda: {"bytes": 0, "files": 0, "recommended_action": ""})
    largest: list[dict[str, Any]] = []
    candidates: list[dict[str, Any]] = []
    misplaced_models: list[dict[str, Any]] = []

    total_bytes = 0
    for path in files:
        try:
            size = path.stat().st_size
        except OSError:
            continue
        total_bytes += size
        rel = repo_relative(path, root)
        category, action = classify(path, root)

        top = top_level_for(path, root)
        by_top[top]["bytes"] += size
        by_top[top]["files"] += 1

        by_class[category]["bytes"] += size
        by_class[category]["files"] += 1
        by_class[category]["recommended_action"] = action

        item = {
            "path": rel,
            "bytes": size,
            "category": category,
            "recommended_action": action,
            "mtime_utc": stat_mtime_utc(path),
        }
        largest.append(item)
        if category in {
            "misplaced_model_artifact",
            "rebuildable_temp",
            "runtime_backup",
            "generated_tick_output",
            "runtime_log",
        }:
            candidates.append(item)
        if category == "misplaced_model_artifact":
            misplaced_models.append(item)

    largest.sort(key=lambda item: item["bytes"], reverse=True)
    candidates.sort(key=lambda item: (item["category"], -item["bytes"], item["path"]))

    classes = [
        {
            "category": category,
            "bytes": payload["bytes"],
            "files": payload["files"],
            "recommended_action": payload["recommended_action"],
        }
        for category, payload in sorted(by_class.items(), key=lambda kv: kv[1]["bytes"], reverse=True)
    ]
    top_roots = [
        {"path": path, "bytes": payload["bytes"], "files": payload["files"]}
        for path, payload in sorted(by_top.items(), key=lambda kv: kv[1]["bytes"], reverse=True)
    ]

    status = "warn" if candidates or misplaced_models else "ok"
    return {
        "contract": CONTRACT,
        "generated_at_utc": generated_at,
        "mode": "dry_run_no_delete",
        "status": status,
        "roots_scanned": roots,
        "summary": {
            "total_bytes": total_bytes,
            "total_files": len(files),
            "cleanup_candidate_count": len(candidates),
            "misplaced_model_artifact_count": len(misplaced_models),
        },
        "top_roots": top_roots,
        "classes": classes,
        "largest_files": largest[:largest_limit],
        "cleanup_candidates": candidates[:candidate_limit],
        "misplaced_model_artifacts": misplaced_models[:candidate_limit],
        "policy": {
            "destructive_actions_performed": False,
            "archive_or_delete_requires": [
                "fresh operator-approved Rúmil review packet",
                "operator-selected scope",
                "rollback or archive note",
                "post-change verification receipt",
            ],
            "model_artifact_boundary": "model weights and staging artifacts belong under ~/models, not Arda data/",
        },
    }


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def render_md(payload: dict[str, Any]) -> str:
    lines = [
        "# Rúmil Storage Hygiene Audit",
        "",
        f"- Generated: `{payload['generated_at_utc']}`",
        f"- Contract: `{payload['contract']}`",
        f"- Mode: `{payload['mode']}`",
        f"- Status: `{payload['status']}`",
        f"- Total observed: `{payload['summary']['total_bytes']}` bytes across `{payload['summary']['total_files']}` files",
        f"- Cleanup candidates: `{payload['summary']['cleanup_candidate_count']}`",
        f"- Misplaced model artifacts: `{payload['summary']['misplaced_model_artifact_count']}`",
        "",
        "## Top Roots",
        "",
        "| Path | Bytes | Files |",
        "| --- | ---: | ---: |",
    ]
    for item in payload["top_roots"][:20]:
        lines.append(f"| `{item['path']}` | {item['bytes']} | {item['files']} |")

    lines.extend(["", "## Classes", "", "| Category | Bytes | Files | Action |", "| --- | ---: | ---: | --- |"])
    for item in payload["classes"]:
        lines.append(
            f"| `{item['category']}` | {item['bytes']} | {item['files']} | `{item['recommended_action']}` |"
        )

    lines.extend(["", "## Largest Files", "", "| Path | Bytes | Category |", "| --- | ---: | --- |"])
    for item in payload["largest_files"][:25]:
        lines.append(f"| `{item['path']}` | {item['bytes']} | `{item['category']}` |")

    lines.extend(["", "## Cleanup Candidate Preview", "", "| Path | Bytes | Category | Action |", "| --- | ---: | --- | --- |"])
    for item in payload["cleanup_candidates"][:50]:
        lines.append(
            f"| `{item['path']}` | {item['bytes']} | `{item['category']}` | `{item['recommended_action']}` |"
        )

    lines.extend(
        [
            "",
            "## Boundary",
            "",
            "No files were deleted, moved, archived, or rewritten by this audit.",
            "Any archive/delete pass requires a fresh operator-approved Rúmil review packet, an explicit operator-selected scope, and a post-change receipt.",
            "Model weights and staged model artifacts belong outside the repository under `~/models`.",
            "",
        ]
    )
    return "\n".join(lines)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--out-dir", type=Path, default=Path("audit/storage-hygiene-2026-06-05"))
    parser.add_argument("--state-path", type=Path, default=Path("core/state/storage_hygiene.json"))
    parser.add_argument("--root-scope", action="append", dest="roots", default=None)
    parser.add_argument("--largest-limit", type=int, default=50)
    parser.add_argument("--candidate-limit", type=int, default=200)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root = args.root.resolve()
    roots = args.roots or list(DEFAULT_ROOTS)
    payload = summarize(root, roots, args.largest_limit, args.candidate_limit)

    out_dir = (root / args.out_dir).resolve()
    state_path = (root / args.state_path).resolve()
    write_json(out_dir / "summary.json", payload)
    (out_dir / "summary.md").write_text(render_md(payload), encoding="utf-8")
    write_json(state_path, payload)
    print(json.dumps({"status": payload["status"], "summary": repo_relative(out_dir / "summary.json", root)}, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
