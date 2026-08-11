from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
import importlib.util
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
SCRIPT = ROOT / "scripts" / "stage5_u4_lifecycle.py"
SPEC = importlib.util.spec_from_file_location("stage5_u4_lifecycle", SCRIPT)
assert SPEC and SPEC.loader
lifecycle = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = lifecycle
SPEC.loader.exec_module(lifecycle)
beta_ops = lifecycle.beta_ops


class Stage5U4LifecycleSigningTests(unittest.TestCase):
    def test_requires_complete_sigstore_identity_arguments(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            candidate = Path(raw) / "candidate.AppImage"
            candidate.write_bytes(b"candidate")
            with self.assertRaisesRegex(beta_ops.BetaOpsError, "must be supplied together"):
                lifecycle.verify_sigstore_bundle(candidate, Path(raw) / "bundle.json", None, None)

    def test_records_verified_identity_bound_bundle(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            candidate = Path(raw) / "candidate.AppImage"
            bundle = Path(raw) / "candidate.AppImage.sigstore.json"
            candidate.write_bytes(b"candidate")
            bundle.write_text("{}\n", encoding="utf-8")
            completed = subprocess.CompletedProcess([], 0, "Verified OK\n", "")
            with patch.object(lifecycle.subprocess, "run", return_value=completed) as run:
                result = lifecycle.verify_sigstore_bundle(
                    candidate,
                    bundle,
                    "https://github.com/example/repo/.github/workflows/release.yml@refs/tags/v1",
                    "https://token.actions.githubusercontent.com",
                )

            self.assertTrue(result["verified"])
            self.assertEqual(result["status"], "pass")
            self.assertEqual(result["bundle_sha256"], beta_ops.hash_file(bundle))
            command = run.call_args.args[0]
            self.assertEqual(command[0:2], ["cosign", "verify-blob"])
            self.assertIn(str(candidate.resolve()), command)

    def test_fails_closed_when_cosign_rejects_bundle(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            candidate = Path(raw) / "candidate.AppImage"
            bundle = Path(raw) / "candidate.AppImage.sigstore.json"
            candidate.write_bytes(b"candidate")
            bundle.write_text("{}\n", encoding="utf-8")
            completed = subprocess.CompletedProcess([], 1, "", "signature mismatch")
            with patch.object(lifecycle.subprocess, "run", return_value=completed):
                with self.assertRaisesRegex(beta_ops.BetaOpsError, "signature mismatch"):
                    lifecycle.verify_sigstore_bundle(
                        candidate,
                        bundle,
                        "identity",
                        "issuer",
                    )


if __name__ == "__main__":
    unittest.main()
