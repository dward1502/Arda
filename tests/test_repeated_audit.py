import importlib.util
import json
import os
import subprocess
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory


SCRIPT_PATH = Path(__file__).resolve().parents[1] / "scripts" / "audit" / "repeated_audit.py"
SPEC = importlib.util.spec_from_file_location("repeated_audit", SCRIPT_PATH)
assert SPEC is not None
repeated_audit = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = repeated_audit
SPEC.loader.exec_module(repeated_audit)


def write_json(path: Path, payload: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


class RepeatedAuditTests(unittest.TestCase):
    def test_repeated_audit_emits_receipts_state_and_candidate_tasks(self):
        with TemporaryDirectory() as raw:
            root = Path(raw)
            portability = root / "audit/PORTABILITY_AUDIT_2026-05-24/summary.json"
            setup = root / "audit/SETUP_CONSOLE_READINESS_2026-05-25/setup_console_readiness_receipt.json"
            system = root / "audit/system-audit-runs/current/summary.json"
            state = root / "core/state/repeated_audit_status.json"
            out = root / "audit/repeated-audit-runs/current"

            write_json(
                portability,
                {
                    "summary": {
                        "active_blocker_findings": 3,
                        "findings_total": 10,
                        "classification_counts": {"active_source_must_fix": 2},
                        "pattern_counts": {"loopback_endpoint": 1},
                        "top_active_blockers": [{"path": "crates/example/src/lib.rs", "findings": 2}],
                    }
                },
            )
            write_json(
                setup,
                {
                    "gate_status": "warn",
                    "summary": {"pass": 1, "warn": 1},
                    "portability_status": {
                        "status": "warn",
                        "active_blocker_findings": 3,
                        "findings_total": 10,
                        "label": "active portability blockers present",
                        "source": "audit/PORTABILITY_AUDIT_2026-05-24/summary.json",
                    },
                },
            )
            write_json(
                system,
                {
                    "contract": "arda.audit.run.v1",
                    "run_id": "system-current",
                    "target_count": 2,
                    "findings_count": 5,
                    "candidate_task_count": 2,
                    "scores": {
                        "ARDA-CORE": {"score": 79, "score_breakdown": {}},
                        "ARDA-CLI": {"score": 82, "score_breakdown": {}},
                    },
                },
            )

            summary = repeated_audit.run_repeated_audit(
                root,
                out,
                state,
                "repeat-current",
                portability,
                setup,
                system,
                Path("audit/repeated-audit-runs"),
            )

            self.assertEqual(summary["contract"], repeated_audit.CONTRACT)
            self.assertEqual(summary["run_id"], "repeat-current")
            self.assertEqual(summary["trends"]["baseline"], "first_repeated_run")
            self.assertGreaterEqual(summary["candidate_task_count"], 2)
            self.assertTrue((out / "summary.json").exists())
            self.assertTrue((out / "SUMMARY.md").exists())
            self.assertTrue((out / "regressions.jsonl").exists())
            self.assertTrue((out / "tasks-candidate.jsonl").exists())
            self.assertTrue(state.exists())

            state_payload = json.loads(state.read_text(encoding="utf-8"))
            self.assertEqual(state_payload["run_id"], "repeat-current")
            self.assertEqual(state_payload["visibility"]["portability_zero_active_blockers"], False)
            self.assertEqual(state_payload["visibility"]["portability_active_blocker_findings"], 3)
            self.assertEqual(state_payload["snapshot"]["setup_console"]["portability_status"]["label"], "active portability blockers present")
            self.assertEqual(state_payload["visibility"]["setup_console_portability_status_label"], "active portability blockers present")
            self.assertEqual(state_payload["visibility"]["setup_console_portability_active_blocker_findings"], 3)
            tasks = [json.loads(line) for line in (out / "tasks-candidate.jsonl").read_text(encoding="utf-8").splitlines()]
            task_titles = "\n".join(task["title"] for task in tasks)
            self.assertIn("ARDA-CORE", task_titles)
            self.assertIn("crates/example/src/lib.rs", task_titles)

    def test_trend_comparison_detects_score_and_aggregate_regressions(self):
        with TemporaryDirectory() as raw:
            root = Path(raw)
            previous_out = root / "audit/repeated-audit-runs/previous"
            current_out = root / "audit/repeated-audit-runs/current"
            state = root / "core/state/repeated_audit_status.json"
            previous_summary = {
                "contract": repeated_audit.CONTRACT,
                "run_id": "repeat-previous",
                "snapshot": {
                    "system_audit": {
                        "findings_count": 2,
                        "candidate_task_count": 1,
                        "scores": {"ARDA-CLI": {"score": 90}},
                    },
                    "portability": {"active_blocker_findings": 1, "findings_total": 5},
                    "setup_console": {"gate_status": "pass"},
                },
            }
            write_json(previous_out / "summary.json", previous_summary)
            write_json(root / "audit/portability/summary.json", {"summary": {"active_blocker_findings": 4, "findings_total": 7}})
            write_json(root / "audit/setup/receipt.json", {"gate_status": "warn", "summary": {"warn": 1}})
            write_json(
                root / "audit/system/summary.json",
                {
                    "run_id": "system-current",
                    "target_count": 1,
                    "findings_count": 6,
                    "candidate_task_count": 3,
                    "scores": {"ARDA-CLI": {"score": 83}},
                },
            )

            summary = repeated_audit.run_repeated_audit(
                root,
                current_out,
                state,
                "repeat-current",
                root / "audit/portability/summary.json",
                root / "audit/setup/receipt.json",
                root / "audit/system/summary.json",
                Path("audit/repeated-audit-runs"),
            )

            self.assertEqual(summary["trends"]["previous_run_id"], "repeat-previous")
            self.assertEqual(summary["trends"]["score_deltas"]["ARDA-CLI"], -7)
            messages = "\n".join(item["message"] for item in summary["regressions"])
            self.assertIn("ARDA-CLI score decreased", messages)
            self.assertIn("portability_active_blocker_findings increased", messages)
            self.assertIn("Setup console gate moved from pass to warn", messages)
            self.assertEqual(summary["gate_status"], "warn")

    def test_cli_discovers_latest_system_summary(self):
        with TemporaryDirectory() as raw:
            root = Path(raw)
            write_json(root / "audit/PORTABILITY_AUDIT_2026-05-24/summary.json", {"summary": {"active_blocker_findings": 0, "findings_total": 0}})
            write_json(
                root / "audit/SETUP_CONSOLE_READINESS_2026-05-25/setup_console_readiness_receipt.json",
                {
                    "gate_status": "pass",
                    "summary": {"pass": 2},
                    "portability_status": {
                        "status": "pass",
                        "active_blocker_findings": 0,
                        "findings_total": 0,
                        "label": "zero active portability blockers",
                        "source": "audit/PORTABILITY_AUDIT_2026-05-24/summary.json",
                    },
                },
            )
            flat_summary = root / "audit/system-audit-runs/discovered/summary.json"
            nested_summary = root / "audit/system-audit-runs/2026-05-27/system-discovered-nested/summary.json"
            write_json(flat_summary, {"run_id": "system-discovered-flat", "target_count": 1, "findings_count": 0, "candidate_task_count": 0, "scores": {"HADES": {"score": 80}}})
            write_json(nested_summary, {"run_id": "system-discovered-nested", "target_count": 1, "findings_count": 0, "candidate_task_count": 0, "scores": {"HADES": {"score": 90}}})
            os.utime(flat_summary, (100, 100))
            os.utime(nested_summary, (200, 200))
            result = subprocess.run(
                [
                    "python3",
                    str(SCRIPT_PATH),
                    "--root",
                    str(root),
                    "--run-id",
                    "repeat-cli",
                ],
                check=True,
                text=True,
                capture_output=True,
            )
            stdout = json.loads(result.stdout)
            self.assertEqual(stdout["run_id"], "repeat-cli")
            summary = json.loads((root / stdout["summary"]).read_text(encoding="utf-8"))
            self.assertEqual(summary["snapshot"]["system_audit"]["run_id"], "system-discovered-nested")
            self.assertEqual(summary["visibility"]["portability_zero_active_blockers"], True)
            self.assertEqual(summary["visibility"]["portability_status_label"], "zero active portability blockers")
            self.assertEqual(summary["visibility"]["setup_console_portability_status_label"], "zero active portability blockers")
            self.assertEqual(summary["visibility"]["setup_console_portability_active_blocker_findings"], 0)


if __name__ == "__main__":
    unittest.main()
