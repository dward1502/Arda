#!/usr/bin/env python3
"""Portability regression tests for the Arda-owned active script tranche."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
ACTIVE = [
    ROOT / "scripts/runtime_build_env.sh",
    ROOT / "scripts/runtime/nanoclaw_runtime.sh",
    ROOT / "scripts/check_task_queue_append_only.sh",
    ROOT / "scripts/package_arda_hud.sh",
    ROOT / "scripts/launch_arda_hud.sh",
    ROOT / "scripts/refresh_provider_intelligence.py",
    ROOT / "scripts/hades_nightly_operations.py",
    ROOT / "scripts/rumil_organization_maintenance.sh",
    ROOT / "scripts/rumil_markdown_link_check.py",
    ROOT / "scripts/rumil_storage_hygiene_audit.py",
    ROOT / "scripts/audit/repeated_audit.py",
    ROOT / "scripts/audit/system_audit.py",
    ROOT / "scripts/audit/setup_console_audit.py",
    ROOT / "scripts/monitor_queue_hygiene.sh",
]

class PortabilityParameterizationTests(unittest.TestCase):
    def test_active_tranche_is_present(self):
        self.assertEqual([], [str(path.relative_to(ROOT)) for path in ACTIVE if not path.is_file()])

    def test_active_tranche_has_no_annunimas_root_or_namespace(self):
        forbidden = ("/var/home/mythos/Annunimas", "ANNUNIMAS_", "annunimas-cli", "annunimas-")
        failures = []
        for path in ACTIVE:
            text = path.read_text(encoding="utf-8")
            for token in forbidden:
                if token in text:
                    failures.append(f"{path.relative_to(ROOT)}: {token}")
        self.assertEqual([], failures)

    def test_hud_is_in_repository(self):
        self.assertTrue((ROOT / "apps/arda-hud/package.json").is_file())
        self.assertFalse((ROOT.parent / "arda-hud").exists())

if __name__ == "__main__":
    unittest.main()
