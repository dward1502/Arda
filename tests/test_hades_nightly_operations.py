import importlib.util
import json
import sys
import unittest
from datetime import datetime, timezone
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch


SCRIPT_PATH = Path(__file__).resolve().parents[1] / "scripts" / "hades_nightly_operations.py"
SPEC = importlib.util.spec_from_file_location("hades_nightly_operations", SCRIPT_PATH)
assert SPEC is not None
hades_nightly = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = hades_nightly
SPEC.loader.exec_module(hades_nightly)


class HadesNightlyOperationsTests(unittest.TestCase):
    def test_date_run_dir_is_family_date_run_id(self):
        root = Path("/tmp/arda")
        now = datetime(2026, 5, 27, 1, 2, 3, tzinfo=timezone.utc)
        run_id = hades_nightly.default_run_id(now)

        self.assertEqual(run_id, "hades-nightly-20260527T010203Z")
        self.assertEqual(
            hades_nightly.date_run_dir(root, "hades-nightly-runs", run_id, now),
            root / "audit/hades-nightly-runs/2026-05-27/hades-nightly-20260527T010203Z",
        )

    def test_run_nightly_writes_summary_state_and_history_without_mutation_commands(self):
        with TemporaryDirectory() as raw:
            root = Path(raw)
            out = root / "audit/hades-nightly-runs/2026-05-27/hades-nightly-test"
            now = datetime(2026, 5, 27, 1, 2, 3, tzinfo=timezone.utc)
            (root / "data/hades").mkdir(parents=True)
            (root / "data/hades/markdown_link_check_last.md").write_text(
                "Local links checked: 1\nBroken local links: 0\n",
                encoding="utf-8",
            )
            (root / "data/hades/storage_hygiene_last.json").write_text("{}\n", encoding="utf-8")

            def fake_run(command, command_root, timeout=900):
                return {
                    "command": command,
                    "started_at_utc": "2026-05-27T01:02:03Z",
                    "finished_at_utc": "2026-05-27T01:02:04Z",
                    "exit_code": 0,
                    "timed_out": False,
                    "stdout_tail": "{}",
                    "stderr_tail": "",
                }

            with patch.object(hades_nightly, "run_command", side_effect=fake_run):
                summary = hades_nightly.run_nightly(root, "hades-nightly-test", out, now)

            self.assertEqual(summary["contract"], hades_nightly.CONTRACT)
            self.assertEqual(summary["status"], "pass")
            self.assertEqual(summary["mutation_policy"], "audit_receipts_only_no_source_config_service_or_queue_mutation")
            self.assertEqual(summary["layout"]["default_shape"], "audit/<family>/YYYY-MM-DD/<run-id>")
            self.assertTrue((out / "summary.json").exists())
            self.assertTrue((root / "core/state/hades_nightly_operations.json").exists())
            self.assertTrue((root / "data/hades/nightly_operations_history.jsonl").exists())
            self.assertTrue((out / "organization/storage_hygiene_last.json").exists())
            self.assertIn("audit/system-audit-runs/2026-05-27/system-audit-test/summary.json", summary["artifacts"]["system_audit_summary"])
            portability_command = summary["commands"]["portability_audit"]["command"]
            self.assertEqual(portability_command[:2], ["python3", "scripts/audit/portability_audit.py"])
            portability_summary = summary["artifacts"]["portability_summary"]
            setup_command = summary["commands"]["setup_console_readiness"]["command"]
            self.assertEqual(
                setup_command[setup_command.index("--portability-receipt") + 1],
                portability_summary,
            )
            repeated_command = summary["commands"]["repeated_audit"]["command"]
            self.assertEqual(
                repeated_command[repeated_command.index("--portability-summary") + 1],
                portability_summary,
            )
            self.assertEqual(
                repeated_command[repeated_command.index("--setup-receipt") + 1],
                summary["artifacts"]["setup_console_receipt"],
            )


if __name__ == "__main__":
    unittest.main()
