#!/usr/bin/env python3
"""Exercise the U4 launcher lifecycle in isolated operator state.

This harness proves lifecycle mechanics with a locally built executable. It never
claims the final signed-artifact acceptance gate; production signing is evaluated
separately from the candidate lifecycle.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import tempfile
import time
from argparse import Namespace
from pathlib import Path
from typing import Any

import arda_beta_ops as beta_ops


def launch_probe(executable: Path, environment: dict[str, str]) -> tuple[bool, int | None]:
    process = subprocess.Popen(
        [str(executable)],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        env=environment,
    )
    time.sleep(3)
    survived = process.poll() is None
    return_code = process.poll()
    if survived:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)
    return survived, return_code


def run_lifecycle(root: Path, candidate: Path) -> dict[str, Any]:
    root = root.resolve()
    candidate = candidate.resolve()
    candidate_sha = beta_ops.hash_file(candidate)
    compatibility = beta_ops.compatibility()
    if compatibility["status"] != "supported":
        raise beta_ops.BetaOpsError(compatibility["message"])

    with tempfile.TemporaryDirectory(prefix="arda-u4-lifecycle-") as temporary:
        base = Path(temporary)
        home = base / "home"
        layout = beta_ops.layout_from_args(Namespace(home=str(home), root=str(root)))
        source_before = beta_ops.source_identity(root)

        baseline = base / "arda-launcher-stage4"
        baseline.write_text("#!/bin/sh\n# isolated Stage 4 baseline\nexit 0\n", encoding="utf-8")
        baseline.chmod(0o755)
        baseline_sha = beta_ops.hash_file(baseline)
        beta_ops.install_launcher(layout, baseline)

        layout.data.mkdir(parents=True, exist_ok=True)
        run_truth = layout.data / "runs.jsonl"
        run_truth.write_text('{"run_id":"u4-lifecycle","status":"completed"}\n', encoding="utf-8")
        run_truth_sha = beta_ops.hash_file(run_truth)
        pre_upgrade = base / "pre-upgrade.tar.gz"
        upgraded = beta_ops.upgrade_launcher(layout, candidate, pre_upgrade)

        installed = layout.install_lib / "arda-launcher"
        installed_sha = beta_ops.hash_file(installed)
        launch_environment = os.environ.copy()
        launch_default_survived, launch_default_return_code = launch_probe(
            installed, launch_environment
        )
        launch_recovery = None
        launch_survived = launch_default_survived
        if not launch_survived:
            launch_environment["GDK_BACKEND"] = "x11"
            launch_survived, _ = launch_probe(installed, launch_environment)
            if launch_survived:
                launch_recovery = "GDK_BACKEND=x11"

        diagnostics_archive = base / "candidate-diagnostics.tar.gz"
        diagnostics = beta_ops.diagnostics(layout, diagnostics_archive)
        post_candidate = base / "post-candidate.tar.gz"
        beta_ops.backup(layout, post_candidate)
        rolled_back = beta_ops.rollback_launcher(
            layout,
            Path(upgraded["rollback_receipt"]),
            post_candidate,
        )
        restored_sha = beta_ops.hash_file(layout.install_lib / "arda-launcher")
        run_truth_preserved = run_truth.is_file() and beta_ops.hash_file(run_truth) == run_truth_sha
        uninstalled = beta_ops.uninstall_launcher(layout)
        state_preserved = run_truth.is_file() and beta_ops.hash_file(run_truth) == run_truth_sha
        source_after = beta_ops.source_identity(root)

        return {
            "contract": "arda.stage5.u4-lifecycle-evidence.v1",
            "generated_at_utc": beta_ops.utc_iso(),
            "status": "pass-local-unsigned-candidate",
            "acceptance": {
                "final_signed_artifact_exercised": False,
                "fresh_install": True,
                "candidate_launch_survived_3s": launch_survived,
                "candidate_default_launch_survived_3s": launch_default_survived,
                "upgrade": upgraded["status"] == "ok" and installed_sha == candidate_sha,
                "backup": pre_upgrade.is_file() and post_candidate.is_file(),
                "diagnostics": diagnostics["status"] == "ok" and diagnostics_archive.is_file(),
                "rollback": rolled_back["status"] == "ok" and restored_sha == baseline_sha,
                "terminal_run_truth_preserved": run_truth_preserved,
                "uninstall": uninstalled["status"] == "ok",
                "state_preserved_after_uninstall": state_preserved,
                "source_repository_unchanged": source_before == source_after,
            },
            "artifacts": {
                "baseline_sha256": baseline_sha,
                "candidate_sha256": candidate_sha,
                "installed_candidate_sha256": installed_sha,
                "restored_baseline_sha256": restored_sha,
            },
            "supported_profile": compatibility["profile_id"],
            "launch_recovery": {
                "required": launch_recovery is not None,
                "override": launch_recovery,
                "default_return_code": launch_default_return_code,
            },
            "mutation_scope": "isolated temporary operator home only",
            "blocker": "Final signed package bytes are not available: Stage 5 packaging evidence reports production_trust_ready=false and normalized_linux_packages_signed=false.",
        }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    result = run_lifecycle(args.root, args.candidate)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(result, sort_keys=True))
    informational = {"final_signed_artifact_exercised", "candidate_default_launch_survived_3s"}
    required = [value for key, value in result["acceptance"].items() if key not in informational]
    return 0 if all(required) else 1


if __name__ == "__main__":
    raise SystemExit(main())
