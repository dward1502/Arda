import importlib.util
import json
import sys
import unittest
from datetime import datetime, timezone
from pathlib import Path
from tempfile import TemporaryDirectory


SCRIPT_PATH = Path(__file__).resolve().parents[1] / "scripts" / "audit" / "system_audit.py"
SPEC = importlib.util.spec_from_file_location("system_audit", SCRIPT_PATH)
assert SPEC is not None
system_audit = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = system_audit
SPEC.loader.exec_module(system_audit)


class SystemAuditTests(unittest.TestCase):
    def test_default_output_dir_is_date_first_then_run_id(self):
        root = Path("/tmp/arda")
        now = datetime(2026, 5, 27, 1, 2, 3, tzinfo=timezone.utc)
        run_id = system_audit.default_run_id(now)

        self.assertEqual(run_id, "system-audit-20260527T010203Z")
        self.assertEqual(
            system_audit.default_output_dir(root, run_id, now),
            root / "audit/system-audit-runs/2026-05-27/system-audit-20260527T010203Z",
        )

    def test_validate_report_requires_score_sum_and_rubric_bounds(self):
        report = {
            "contract": system_audit.CONTRACT,
            "run_id": "test-run",
            "target": "HADES",
            "target_type": "agent_crate",
            "score": 100,
            "score_breakdown": dict(system_audit.RUBRIC_MAX),
            "overview": "overview",
            "duties": ["duty"],
            "good": [],
            "bad": [],
            "ugly": [],
            "potential_changes": [],
            "needs_removed": [],
            "evidence": [],
            "candidate_tasks": [],
        }

        system_audit.validate_report(report)
        report["score"] = 99
        with self.assertRaises(ValueError):
            system_audit.validate_report(report)

    def test_run_audit_emits_schema_checked_receipts_for_first_batch(self):
        with TemporaryDirectory() as raw:
            root = Path(raw)
            for target in ("HADES", "PROMETHEUS", "MANWE"):
                src = root / system_audit.TARGETS[target].root / "src"
                src.mkdir(parents=True, exist_ok=True)
                (src / "lib.rs").write_text(
                    "pub fn status() { tracing::info!(\"status receipt audit metric\"); }\n",
                    encoding="utf-8",
                )
                (src / "tests.rs").write_text("#[test] fn smoke() {}\n", encoding="utf-8")
            (root / "config").mkdir()
            (root / "config/manwe.providers.toml").write_text("provider = 'local'\n", encoding="utf-8")
            (root / "scripts").mkdir(exist_ok=True)
            (root / "scripts/hades_organization_maintenance.sh").write_text("#!/usr/bin/env bash\n", encoding="utf-8")
            (root / "data/hades").mkdir(parents=True)
            (root / "data/hades/action_queue.jsonl").write_text("", encoding="utf-8")
            (root / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")

            out = root / "audit/system-audit-runs/TEST"
            summary = system_audit.run_audit(root, out, ["HADES", "PROMETHEUS", "MANWE"], "test-run")

            self.assertEqual(summary["contract"], system_audit.RUN_CONTRACT)
            self.assertEqual(summary["target_count"], 3)
            self.assertEqual(summary["layout"]["default_shape"], "audit/system-audit-runs/YYYY-MM-DD/<run-id>")
            self.assertTrue((out / "summary.json").exists())
            self.assertTrue((out / "agent-scores.json").exists())
            self.assertTrue((out / "findings.jsonl").exists())
            self.assertTrue((out / "tasks-candidate.jsonl").exists())
            self.assertTrue((out / "targets/HADES.json").exists())
            for target in ("HADES", "PROMETHEUS", "MANWE"):
                report = json.loads((out / f"targets/{target}.json").read_text(encoding="utf-8"))
                system_audit.validate_report(report)
                self.assertEqual(report["contract"], system_audit.CONTRACT)
                self.assertIn("score_breakdown", report)

    def test_phase5_target_set_covers_workspace_crates_and_folders(self):
        workspace_crates = {
            "crates/arda-barrow-wight",
            "crates/spine/foundation/arda-ule",
            "crates/spine/interface/arda-orome",
            "crates/spine/runtime/manwe",
            "crates/spine/observability/arda-aule",
            "crates/spine/arms/arda-lorien",
            "crates/spine/arms/arda-mandos",
            "crates/spine/executors/arda-varda",
            "crates/spine/executors/arda-core",
            "crates/spine/executors/arda-hadhafang",
        }
        phase5_roots = {system_audit.TARGETS[name].root for name in system_audit.PHASE5_TARGETS}
        self.assertTrue(workspace_crates.issubset(phase5_roots))
        for folder in ("apps", "config", "core", "data", "docs", "human", "audit", "scripts", "tests", "archive", "archived_scripts"):
            self.assertIn(folder, phase5_roots)

    def test_crash_tokens_in_non_rust_audit_evidence_do_not_reduce_reliability(self):
        with TemporaryDirectory() as raw:
            root = Path(raw)
            (root / "audit/reports").mkdir(parents=True)
            (root / "audit/reports/example.md").write_text(
                "Audit note: Rust production code must avoid .unwrap() and .expect(\"msg\").\n",
                encoding="utf-8",
            )
            tracked = [root / "audit/reports/example.md"]
            snapshot = system_audit.collect_target_snapshot(
                root,
                system_audit.TARGETS["FOLDER-AUDIT"],
                tracked,
                set(),
            )

            self.assertEqual(sum(snapshot["crash_counts"].values()), 0)
            score = system_audit.score_snapshot(snapshot, [])
            self.assertEqual(score["reliability_safety"], system_audit.RUBRIC_MAX["reliability_safety"])

    def test_crash_tokens_in_rust_source_still_reduce_reliability(self):
        with TemporaryDirectory() as raw:
            root = Path(raw)
            (root / "crates/spine/executors/arda-core/src").mkdir(parents=True)
            source = root / "crates/spine/executors/arda-core/src/lib.rs"
            source.write_text(
                "pub fn unsafe_path(value: Option<u8>) -> u8 { value.unwrap() }\n",
                encoding="utf-8",
            )
            tracked = [source]
            snapshot = system_audit.collect_target_snapshot(
                root,
                system_audit.TARGETS["ARDA-CORE"],
                tracked,
                {"crates/spine/executors/arda-core"},
            )

            self.assertEqual(snapshot["crash_counts"][".unwrap()"], 1)
            score = system_audit.score_snapshot(snapshot, [])
            self.assertEqual(score["reliability_safety"], system_audit.RUBRIC_MAX["reliability_safety"] - 1)

    def test_crash_tokens_in_rust_tests_do_not_reduce_reliability(self):
        with TemporaryDirectory() as raw:
            root = Path(raw)
            (root / "crates/spine/executors/arda-core/src").mkdir(parents=True)
            inline = root / "crates/spine/executors/arda-core/src/lib.rs"
            inline.write_text(
                "pub fn safe_path(value: Option<u8>) -> u8 { value.unwrap_or(0) }\n"
                "#[cfg(test)]\n"
                "mod tests {\n"
                "    #[test]\n"
                "    fn allows_assertive_test_setup() {\n"
                "        let value = Some(1).unwrap();\n"
                "        assert_eq!(value, 1);\n"
                "    }\n"
                "}\n",
                encoding="utf-8",
            )
            test_file = root / "crates/spine/executors/arda-core/src/tests.rs"
            test_file.write_text("#[test] fn smoke() { Some(1).expect(\"fixture\"); }\n", encoding="utf-8")
            inline_test_file = root / "crates/spine/executors/arda-core/src/route_policy_tests.rs"
            inline_test_file.write_text("#[test] fn route() { Some(1).expect(\"fixture\"); }\n", encoding="utf-8")
            bench = root / "crates/spine/executors/arda-core/benches/bench.rs"
            bench.parent.mkdir(parents=True)
            bench.write_text("fn main() { Some(1).expect(\"bench fixture\"); }\n", encoding="utf-8")
            tracked = [inline, test_file, inline_test_file, bench]
            snapshot = system_audit.collect_target_snapshot(
                root,
                system_audit.TARGETS["ARDA-CORE"],
                tracked,
                {"crates/spine/executors/arda-core"},
            )

            self.assertEqual(sum(snapshot["crash_counts"].values()), 0)
            score = system_audit.score_snapshot(snapshot, [])
            self.assertEqual(score["reliability_safety"], system_audit.RUBRIC_MAX["reliability_safety"])

    def test_phase5_reports_are_written_to_crates_and_folders_subdirs(self):
        with TemporaryDirectory() as raw:
            root = Path(raw)
            (root / "crates/spine/executors/arda-core/src").mkdir(parents=True)
            (root / "crates/spine/executors/arda-core/src/lib.rs").write_text(
                "pub fn contract_status() { println!(\"status receipt audit\"); }\n",
                encoding="utf-8",
            )
            (root / "scripts").mkdir()
            (root / "scripts/example.sh").write_text("#!/usr/bin/env bash\necho audit status\n", encoding="utf-8")
            (root / "Cargo.toml").write_text(
                "[workspace]\nmembers = [\"crates/spine/executors/arda-core\"]\n",
                encoding="utf-8",
            )

            out = root / "audit/system-audit-runs/PHASE5"
            summary = system_audit.run_audit(root, out, ["ARDA-CORE", "FOLDER-SCRIPTS"], "phase5-test")

            self.assertEqual(summary["target_count"], 2)
            self.assertTrue((out / "crates/ARDA-CORE.json").exists())
            self.assertTrue((out / "folders/FOLDER-SCRIPTS.md").exists())
            crate_report = json.loads((out / "crates/ARDA-CORE.json").read_text(encoding="utf-8"))
            self.assertTrue(crate_report["snapshot"]["workspace_member"])
            folder_report = json.loads((out / "folders/FOLDER-SCRIPTS.json").read_text(encoding="utf-8"))
            self.assertEqual(folder_report["target_type"], "folder")
            self.assertEqual(folder_report["candidate_tasks"], [])
            self.assertIn("do not add tests mechanically", "\n".join(folder_report["potential_changes"]))


if __name__ == "__main__":
    unittest.main()
