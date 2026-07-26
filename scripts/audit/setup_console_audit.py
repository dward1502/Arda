#!/usr/bin/env python3
"""Read-only setup console audit runner for Arda portability/onboarding.

This runner intentionally does not rewrite source/config files or touch services.
It inventories onboarding prerequisites and consumes the portability audit output
as evidence for a future setup console / ARDA projection.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import socket
import subprocess
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


DEFAULT_OUT_DIR = Path("audit/SETUP_CONSOLE_READINESS_2026-05-25")
DEFAULT_STATE_PATH = Path("core/state/setup_console_readiness.json")
PORTABILITY_RECEIPT = Path("audit/PORTABILITY_AUDIT_2026-05-24/summary.json")


@dataclass(frozen=True)
class Check:
    check_id: str
    title: str
    status: str
    severity: str
    evidence: list[str]
    recommendation: str

    def as_dict(self) -> dict[str, Any]:
        return {
            "check_id": self.check_id,
            "title": self.title,
            "status": self.status,
            "severity": self.severity,
            "evidence": self.evidence,
            "recommendation": self.recommendation,
        }


def utc_now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        json.dump(payload, handle, indent=2, sort_keys=True)
        handle.write("\n")


def git_root(start: Path) -> Path:
    try:
        result = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            cwd=start,
            check=True,
            text=True,
            capture_output=True,
        )
        return Path(result.stdout.strip())
    except (OSError, subprocess.CalledProcessError):
        return start.resolve()


def repo_relative(path: Path, root: Path) -> str:
    try:
        return str(path.resolve().relative_to(root.resolve()))
    except ValueError:
        return str(path)


def file_check(root: Path, rel_path: str, title: str, severity: str, recommendation: str) -> Check:
    path = root / rel_path
    if path.exists():
        status = "pass"
        evidence = [f"present: {rel_path}"]
    else:
        status = "warn"
        evidence = [f"missing: {rel_path}"]
    return Check(rel_path.replace("/", "."), title, status, severity, evidence, recommendation)


def portability_check(root: Path, receipt_rel: Path) -> Check:
    path = root / receipt_rel
    if not path.exists():
        return Check(
            "portability.receipt",
            "Portability/config hygiene receipt available",
            "warn",
            "medium",
            [f"missing: {receipt_rel}"],
            "Run scripts/audit/portability_audit.py before setup-console readiness publication.",
        )

    receipt = load_json(path)
    summary = receipt.get("summary", {}) if isinstance(receipt, dict) else {}
    total = int(summary.get("total_findings", summary.get("findings_total", 0)) or 0) if isinstance(summary, dict) else 0
    active_blockers = int(summary.get("active_blocker_findings", 0) or 0) if isinstance(summary, dict) else 0
    by_severity = summary.get("by_severity", {}) if isinstance(summary, dict) else {}
    high = int(by_severity.get("high", 0) or 0) if isinstance(by_severity, dict) else 0
    medium = int(by_severity.get("medium", 0) or 0) if isinstance(by_severity, dict) else 0
    classification_counts = summary.get("classification_counts", {}) if isinstance(summary, dict) else {}
    if isinstance(classification_counts, dict):
        high += int(classification_counts.get("active_source_must_fix", 0) or 0)
        medium += int(classification_counts.get("active_config_must_parameterize", 0) or 0)
        medium += int(classification_counts.get("active_script_must_parameterize", 0) or 0)
    status = "pass" if active_blockers == 0 and high == 0 else "warn"
    evidence = [
        f"receipt: {receipt_rel}",
        f"total_findings={total}",
        f"active_blocker_findings={active_blockers}",
        f"high={high}",
        f"medium={medium}",
    ]
    recommendation = "Parameterize high/medium portability findings before enabling automated setup actions."
    if active_blockers == 0 and high == 0 and medium == 0:
        recommendation = "No high/medium portability blockers recorded in the latest receipt."
    return Check(
        "portability.receipt",
        "Portability/config hygiene findings classified",
        status,
        "high" if active_blockers or high else "medium",
        evidence,
        recommendation,
    )


def env_surface_check(root: Path) -> Check:
    candidates = [
        "config/arda.toml",
        "config/manwe.providers.toml",
        "config/.env.example",
        ".env.example",
        "core/state/environment_profile.schema.json",
    ]
    present = [candidate for candidate in candidates if (root / candidate).exists()]
    missing = [candidate for candidate in candidates if not (root / candidate).exists()]
    status = "pass" if "core/state/environment_profile.schema.json" in present else "warn"
    evidence = [f"present: {item}" for item in present] + [f"missing: {item}" for item in missing]
    return Check(
        "environment.surface",
        "Environment profile/template surface discoverable",
        status,
        "medium",
        evidence,
        "Expose a single setup-console path from environment profile schema to local override templates.",
    )


def endpoint_assumption_check(root: Path, portability_receipt_rel: Path) -> Check:
    path = root / portability_receipt_rel
    if not path.exists():
        return Check(
            "endpoint.assumptions",
            "Endpoint assumptions inventoried",
            "warn",
            "medium",
            ["portability receipt unavailable"],
            "Generate portability receipt to classify hardcoded endpoint assumptions.",
        )
    receipt = load_json(path)
    if not isinstance(receipt, dict):
        return Check(
            "endpoint.assumptions",
            "Endpoint assumptions inventoried",
            "warn",
            "medium",
            [f"invalid portability receipt shape: {portability_receipt_rel}"],
            "Regenerate portability audit summary before setup-console publication.",
        )

    summary = receipt.get("summary")
    if isinstance(summary, dict):
        active_blockers = int(summary.get("active_blocker_findings", 0) or 0)
        classification_counts = summary.get("classification_counts", {})
        pattern_counts = summary.get("pattern_counts", {})
        top_active_blockers = summary.get("top_active_blockers", [])
        evidence = [
            f"portability_summary={portability_receipt_rel}",
            f"active_blocker_findings={active_blockers}",
        ]
        if isinstance(classification_counts, dict):
            for key in (
                "active_config_must_parameterize",
                "active_script_must_parameterize",
                "active_source_must_fix",
            ):
                evidence.append(f"{key}={classification_counts.get(key, 0)}")
        if isinstance(pattern_counts, dict):
            for key in (
                "loopback_endpoint",
                "private_lan_ip_endpoint",
                "hardcoded_home_mythos",
                "hardcoded_var_home_mythos",
            ):
                evidence.append(f"{key}={pattern_counts.get(key, 0)}")
        if isinstance(top_active_blockers, list):
            for blocker in top_active_blockers[:5]:
                if isinstance(blocker, dict):
                    evidence.append(f"top_blocker={blocker.get('path', 'unknown')} ({blocker.get('findings', '?')})")
        status = "pass" if active_blockers == 0 else "warn"
        return Check(
            "endpoint.assumptions",
            "Hardcoded endpoint/local-path assumptions inventoried",
            status,
            "medium" if active_blockers else "low",
            evidence,
            "Keep setup console read-only until assumptions are parameterized behind environment profiles.",
        )

    findings = receipt.get("findings", [])
    endpoint_count = 0
    path_count = 0
    examples: list[str] = []
    if isinstance(findings, list):
        for finding in findings:
            if not isinstance(finding, dict):
                continue
            classification = str(finding.get("classification", ""))
            if "endpoint" in classification:
                endpoint_count += 1
            if "local_path" in classification:
                path_count += 1
            if len(examples) < 5 and ("endpoint" in classification or "local_path" in classification):
                examples.append(
                    f"{finding.get('severity', 'unknown')} {classification}: {finding.get('path', 'unknown')}:{finding.get('line', '?')}"
                )
    status = "pass" if endpoint_count == 0 and path_count == 0 else "warn"
    evidence = [f"endpoint_assumptions={endpoint_count}", f"local_path_assumptions={path_count}"] + examples
    return Check(
        "endpoint.assumptions",
        "Hardcoded endpoint/local-path assumptions inventoried",
        status,
        "medium" if endpoint_count or path_count else "low",
        evidence,
        "Keep setup console read-only until assumptions are parameterized behind environment profiles.",
    )


def portability_status_projection(root: Path, receipt_rel: Path) -> dict[str, Any]:
    path = root / receipt_rel
    if not path.exists():
        return {
            "status": "missing",
            "active_blocker_findings": None,
            "findings_total": None,
            "label": "portability receipt missing",
            "source": str(receipt_rel),
        }

    receipt = load_json(path)
    summary = receipt.get("summary", {}) if isinstance(receipt, dict) else {}
    if not isinstance(summary, dict):
        return {
            "status": "warn",
            "active_blocker_findings": None,
            "findings_total": None,
            "label": "portability summary unavailable",
            "source": str(receipt_rel),
        }

    active_blockers = int(summary.get("active_blocker_findings", 0) or 0)
    findings_total = int(summary.get("findings_total", summary.get("total_findings", 0)) or 0)
    return {
        "status": "pass" if active_blockers == 0 else "warn",
        "active_blocker_findings": active_blockers,
        "findings_total": findings_total,
        "label": "zero active portability blockers" if active_blockers == 0 else "active portability blockers present",
        "source": str(receipt_rel),
    }


def runtime_context(root: Path) -> dict[str, Any]:
    return {
        "host": socket.gethostname(),
        "platform": platform.platform(),
        "python": platform.python_version(),
        "repo_root": str(root),
        "cwd": os.getcwd(),
    }


def build_receipt(root: Path, portability_receipt_rel: Path) -> dict[str, Any]:
    checks = [
        file_check(
            root,
            "AGENTS.md",
            "Agent/project operating instructions available",
            "high",
            "Keep AGENTS.md current as setup-console operator context.",
        ),
        file_check(
            root,
            "ARDA_ROOT_PROTOCOL.md",
            "Root protocol available",
            "high",
            "Preserve root protocol pointer for new-machine onboarding.",
        ),
        file_check(
            root,
            "docs/CODEMAP.md",
            "Low-token codemap available",
            "medium",
            "Regenerate CODEMAP when repository structure materially changes.",
        ),
        file_check(
            root,
            "scripts/runtime_build_env.sh",
            "Runtime build environment script available",
            "medium",
            "Keep build output/cache paths centralized in runtime_build_env.sh.",
        ),
        file_check(
            root,
            "config/manwe.providers.toml",
            "Manwe provider registry available",
            "medium",
            "Use provider registry values rather than hardcoded endpoints in setup flows.",
        ),
        env_surface_check(root),
        portability_check(root, portability_receipt_rel),
        endpoint_assumption_check(root, portability_receipt_rel),
    ]

    counts: dict[str, int] = {}
    for check in checks:
        counts[check.status] = counts.get(check.status, 0) + 1

    gate_status = "pass" if counts.get("fail", 0) == 0 and counts.get("warn", 0) == 0 else "warn"
    return {
        "schema_version": 1,
        "runner": "scripts/audit/setup_console_audit.py",
        "generated_at_utc": utc_now(),
        "mode": "read_only",
        "mutation_policy": "receipts_only_no_source_config_or_service_rewrites",
        "gate_status": gate_status,
        "summary": counts,
        "portability_status": portability_status_projection(root, portability_receipt_rel),
        "runtime": runtime_context(root),
        "checks": [check.as_dict() for check in checks],
    }


def markdown_summary(receipt: dict[str, Any], receipt_path: Path, state_path: Path) -> str:
    lines = [
        "# Setup Console Readiness Audit",
        "",
        f"Generated: {receipt['generated_at_utc']}",
        f"Mode: {receipt['mode']}",
        f"Gate status: {receipt['gate_status']}",
        f"Receipt: `{receipt_path}`",
        f"ARDA projection state: `{state_path}`",
        "",
        "## Summary",
        "",
    ]
    for status, count in sorted(receipt.get("summary", {}).items()):
        lines.append(f"- {status}: {count}")
    portability_status = receipt.get("portability_status", {})
    if isinstance(portability_status, dict):
        lines.extend(
            [
                "",
                "## Portability projection",
                "",
                f"- Status: {portability_status.get('status')}",
                f"- Active blockers: {portability_status.get('active_blocker_findings')}",
                f"- Label: {portability_status.get('label')}",
                f"- Source: `{portability_status.get('source')}`",
            ]
        )
    lines.extend(["", "## Checks", ""])
    for check in receipt["checks"]:
        evidence = "; ".join(check["evidence"])
        lines.extend(
            [
                f"### {check['check_id']} — {check['status'].upper()}",
                f"- Title: {check['title']}",
                f"- Severity: {check['severity']}",
                f"- Evidence: {evidence}",
                f"- Recommendation: {check['recommendation']}",
                "",
            ]
        )
    lines.extend(
        [
            "## Scope guard",
            "",
            "This audit is read-only except for generated receipt/state/Markdown artifacts. It does not rewrite source files, configs, systemd units, secrets, or runtime services.",
            "",
        ]
    )
    return "\n".join(lines)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default=".", help="Repository root, defaults to current git root")
    parser.add_argument("--out-dir", default=str(DEFAULT_OUT_DIR), help="Receipt output directory")
    parser.add_argument("--state-path", default=str(DEFAULT_STATE_PATH), help="State JSON for ARDA projection")
    parser.add_argument(
        "--portability-receipt",
        default=str(PORTABILITY_RECEIPT),
        help="Existing portability/config hygiene receipt to summarize",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root_arg = Path(args.root).resolve()
    root = git_root(root_arg) if args.root == "." else root_arg
    out_dir = (root / args.out_dir).resolve() if not Path(args.out_dir).is_absolute() else Path(args.out_dir)
    state_path = (root / args.state_path).resolve() if not Path(args.state_path).is_absolute() else Path(args.state_path)
    portability_receipt = Path(args.portability_receipt)
    portability_rel = portability_receipt if not portability_receipt.is_absolute() else portability_receipt.relative_to(root)

    receipt = build_receipt(root, portability_rel)
    receipt_path = out_dir / "setup_console_readiness_receipt.json"
    summary_path = out_dir / "SUMMARY.md"

    write_json(receipt_path, receipt)
    write_json(state_path, receipt)
    summary_path.parent.mkdir(parents=True, exist_ok=True)
    summary_path.write_text(markdown_summary(receipt, receipt_path, state_path), encoding="utf-8")

    print(json.dumps({
        "gate_status": receipt["gate_status"],
        "receipt": repo_relative(receipt_path, root),
        "summary": repo_relative(summary_path, root),
        "state": repo_relative(state_path, root),
    }, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
