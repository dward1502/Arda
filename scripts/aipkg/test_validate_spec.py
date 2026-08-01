#!/usr/bin/env python3
"""Regression tests for the read-only AIPKG spec validator."""

from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
VALIDATOR = REPO_ROOT / "scripts" / "aipkg" / "validate_spec.py"
SPEC_ROOT = REPO_ROOT / "spec" / "aipkg" / "v0.1"


class AipkgSpecValidatorTests(unittest.TestCase):
    def run_validator(self, spec_root: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["python3", str(VALIDATOR), "--spec-root", str(spec_root)],
            cwd=REPO_ROOT,
            check=False,
            capture_output=True,
            text=True,
        )

    def test_repository_spec_bundle_passes(self) -> None:
        result = self.run_validator(SPEC_ROOT)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_schema_incompatible_manifest_example_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            spec_root = Path(directory) / "v0.1"
            shutil.copytree(SPEC_ROOT, spec_root)
            example_path = spec_root / "manifest.example.json"
            example = json.loads(example_path.read_text(encoding="utf-8"))
            example["package_digest"] = "sha256:abc"
            example_path.write_text(json.dumps(example), encoding="utf-8")

            result = self.run_validator(spec_root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("package_digest", result.stdout)


if __name__ == "__main__":
    unittest.main()
