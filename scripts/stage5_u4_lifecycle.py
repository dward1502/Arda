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


def verify_sigstore_bundle(
    candidate: Path,
    bundle: Path | None,
    certificate_identity: str | None,
    certificate_oidc_issuer: str | None,
) -> dict[str, Any]:
    supplied = (bundle is not None, certificate_identity is not None, certificate_oidc_issuer is not None)
    if any(supplied) and not all(supplied):
        raise beta_ops.BetaOpsError(
            "--sigstore-bundle, --certificate-identity, and --certificate-oidc-issuer must be supplied together"
        )
    if bundle is None:
        return {"verified": False, "status": "not-provided"}
    bundle = bundle.resolve()
    if not bundle.is_file():
        raise beta_ops.BetaOpsError(f"Sigstore bundle not found: {bundle}")
    try:
        result = subprocess.run(
            [
                "cosign", "verify-blob", str(candidate.resolve()),
                "--bundle", str(bundle),
                "--certificate-identity", certificate_identity or "",
                "--certificate-oidc-issuer", certificate_oidc_issuer or "",
            ],
            check=False,
            capture_output=True,
            text=True,
            timeout=60,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise beta_ops.BetaOpsError(f"Sigstore verification could not run: {error}") from error
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or f"exit {result.returncode}"
        raise beta_ops.BetaOpsError(f"Sigstore verification failed: {detail}")
    return {
        "verified": True,
        "status": "pass",
        "bundle": bundle.name,
        "bundle_sha256": beta_ops.hash_file(bundle),
        "certificate_identity": certificate_identity,
        "certificate_oidc_issuer": certificate_oidc_issuer,
    }


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


def run_lifecycle(root: Path, candidate: Path, signing: dict[str, Any] | None = None) -> dict[str, Any]:
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

        signed = bool(signing and signing.get("verified"))
        if signed:
            status = "pass-signed-candidate-lifecycle" if launch_default_survived else "fail-signed-candidate-lifecycle"
            blocker = (
                None
                if launch_default_survived
                else "The signed candidate did not launch on the supported profile without a compatibility override."
            )
        else:
            status = "pass-local-candidate-lifecycle"
            blocker = "No identity-bound Sigstore bundle was supplied for this candidate."
        return {
            "contract": "arda.stage5.u4-lifecycle-evidence.v1",
            "generated_at_utc": beta_ops.utc_iso(),
            "status": status,
            "acceptance": {
                "final_signed_artifact_exercised": signed,
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
            "signing": signing or {"verified": False, "status": "not-provided"},
            "blocker": blocker,
        }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--sigstore-bundle", type=Path)
    parser.add_argument("--certificate-identity")
    parser.add_argument("--certificate-oidc-issuer")
    args = parser.parse_args()

    try:
        signing = verify_sigstore_bundle(
            args.candidate,
            args.sigstore_bundle,
            args.certificate_identity,
            args.certificate_oidc_issuer,
        )
        result = run_lifecycle(args.root, args.candidate, signing)
    except beta_ops.BetaOpsError as error:
        print(json.dumps({"status": "fail", "error": str(error)}, sort_keys=True))
        return 2
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(result, sort_keys=True))
    informational = {"final_signed_artifact_exercised", "candidate_default_launch_survived_3s"}
    required = [value for key, value in result["acceptance"].items() if key not in informational]
    if result["acceptance"]["final_signed_artifact_exercised"]:
        required.append(result["acceptance"]["candidate_default_launch_survived_3s"])
    return 0 if all(required) else 1


if __name__ == "__main__":
    raise SystemExit(main())
