import importlib.util
import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory


SCRIPT_PATH = Path(__file__).resolve().parents[1] / "scripts" / "audit" / "portability_audit.py"
SPEC = importlib.util.spec_from_file_location("portability_audit", SCRIPT_PATH)
assert SPEC is not None
portability = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = portability
SPEC.loader.exec_module(portability)


class PortabilityAuditTests(unittest.TestCase):
    def test_pattern_detection_classifies_local_paths_and_endpoints(self):
        text = "root=/var/home/mythos/Arda\nurl=http://127.0.0.1:5110/v1\n"
        matches = portability.detect_patterns(text)
        kinds = {match.pattern_id for match in matches}

        self.assertIn("hardcoded_var_home_mythos", kinds)
        self.assertIn("loopback_endpoint", kinds)

    def test_safe_environment_forms_are_not_reported(self):
        text = "root=${ARDA_ROOT:-$HOME/Arda}\ncache=${XDG_CACHE_HOME:-$HOME/.cache}/annunimas-build\n"
        matches = portability.detect_patterns(text)

        self.assertEqual(matches, [])

    def test_classification_separates_active_script_docs_archive_generated_and_tests(self):
        cases = {
            "scripts/ingest_human_notes.py": "active_script_must_parameterize",
            "config/charon.toml": "active_config_must_parameterize",
            "crates/arda-cli/src/main.rs": "active_source_must_fix",
            "docs/operations/runbook.md": "documentation_example_review",
            "archive/old/run.sh": "archive_historical_ok",
            "data/state.json": "generated_runtime_state_ignore_or_regenerate",
            "tests/fixtures/example.sh": "test_fixture_ok",
        }

        for rel, expected in cases.items():
            with self.subTest(rel=rel):
                self.assertEqual(portability.classify_path(Path(rel)), expected)

    def test_audit_writes_json_jsonl_and_markdown_receipts(self):
        with TemporaryDirectory() as raw:
            root = Path(raw)
            (root / "scripts").mkdir()
            (root / "docs").mkdir()
            (root / "tests/fixtures").mkdir(parents=True)
            (root / "scripts/run.sh").write_text("cd /var/home/mythos/Arda\n", encoding="utf-8")
            (root / "docs/example.md").write_text("example http://localhost:9119\n", encoding="utf-8")
            (root / "tests/fixtures/path.txt").write_text("/home/mythos/demo\n", encoding="utf-8")
            out = root / "audit/PORTABILITY_AUDIT_TEST"

            report = portability.run_audit(root=root, out_dir=out, use_git=False)

            self.assertEqual(report["contract"], "arda.portability_config_hygiene_audit.v1")
            self.assertTrue((out / "summary.json").exists())
            self.assertTrue((out / "findings.jsonl").exists())
            self.assertTrue((out / "summary.md").exists())
            self.assertGreaterEqual(report["summary"]["findings_total"], 3)
            lines = (out / "findings.jsonl").read_text(encoding="utf-8").splitlines()
            parsed = [json.loads(line) for line in lines]
            classifications = {item["classification"] for item in parsed}
            self.assertIn("active_script_must_parameterize", classifications)
            self.assertIn("documentation_example_review", classifications)
            self.assertIn("test_fixture_ok", classifications)


if __name__ == "__main__":
    unittest.main()

