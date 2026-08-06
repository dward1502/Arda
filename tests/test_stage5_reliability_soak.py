import importlib.util
import os
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "stage5_reliability_soak.py"
SPEC = importlib.util.spec_from_file_location("stage5_reliability_soak", SCRIPT)
assert SPEC and SPEC.loader
soak = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = soak
SPEC.loader.exec_module(soak)


class Stage5ReliabilitySoakTests(unittest.TestCase):
    def test_percentile_is_deterministic_and_bounded(self):
        self.assertEqual(soak.percentile([40.0, 10.0, 30.0, 20.0], 0.50), 20.0)
        self.assertEqual(soak.percentile([40.0, 10.0, 30.0, 20.0], 0.95), 30.0)
        self.assertIsNone(soak.percentile([], 0.95))

    def test_report_fails_on_state_growth_or_failed_scenario(self):
        states = {"fixture": soak.ScenarioState(runs=2, passed=1, durations_ms=[10.0, 20.0])}
        report = soak.render_report(
            started_at="2026-01-01T00:00:00Z",
            finished_at="2026-01-01T00:01:00Z",
            requested_seconds=60,
            elapsed_seconds=60,
            states=states,
            failures=[{"scenario": "fixture"}] * (soak.MAX_FAILURES + 3),
            protected_before={"files": 1, "bytes": 10},
            protected_after={"files": 2, "bytes": 11},
        )
        self.assertEqual(report["status"], "fail")
        self.assertEqual(report["summary"], {"runs": 2, "passed": 1, "failed": 1})
        self.assertEqual(report["protected_state"]["growth"], {"files": 1, "bytes": 1})
        self.assertEqual(len(report["failures"]), soak.MAX_FAILURES)

    def test_report_passes_only_with_runs_and_zero_growth(self):
        states = {"fixture": soak.ScenarioState(runs=1, passed=1, durations_ms=[12.5])}
        report = soak.render_report(
            started_at="2026-01-01T00:00:00Z",
            finished_at="2026-01-01T00:00:01Z",
            requested_seconds=1,
            elapsed_seconds=1,
            states=states,
            failures=[],
            protected_before={"files": 1, "bytes": 10},
            protected_after={"files": 1, "bytes": 10},
        )
        self.assertEqual(report["status"], "pass")
        self.assertEqual(report["scenarios"]["fixture"]["latency_ms"]["p95"], 12.5)

    def test_report_passes_growth_within_explicit_finite_budget(self):
        states = {"fixture": soak.ScenarioState(runs=1, passed=1, durations_ms=[12.5])}
        report = soak.render_report(
            started_at="2026-01-01T00:00:00Z",
            finished_at="2026-01-01T00:00:01Z",
            requested_seconds=1,
            elapsed_seconds=1,
            states=states,
            failures=[],
            protected_before={"files": 1, "bytes": 10},
            protected_after={"files": 2, "bytes": 11},
            max_growth_files=1,
            max_growth_bytes=1,
        )
        self.assertEqual(report["status"], "pass")
        self.assertEqual(report["protected_state"]["maximum_growth"], {"files": 1, "bytes": 1})

    def test_diagnostic_tail_is_bounded_and_redacts_local_roots(self):
        root = Path("/srv/private/arda")
        output = (
            b"prefix\n"
            + str(root).encode()
            + b"/crates/engine/error.rs\n"
            + b"token=private-token prompt=private-objective\n"
            + b"x" * 80
        )

        diagnostic = soak.bounded_diagnostic(output, root=root, max_bytes=256)

        self.assertLessEqual(diagnostic["captured_bytes"], 256)
        self.assertEqual(diagnostic["total_bytes"], len(output))
        self.assertNotIn(str(root), diagnostic["tail"])
        self.assertNotIn("private-token", diagnostic["tail"])
        self.assertNotIn("private-objective", diagnostic["tail"])

    def test_failure_classifier_assigns_an_explicit_root_cause(self):
        self.assertEqual(soak.classify_failure(124, b""), "timeout")
        self.assertEqual(soak.classify_failure(101, b"test result: FAILED"), "test_failure")
        self.assertEqual(soak.classify_failure(-9, b""), "process_signal")
        self.assertEqual(soak.classify_failure(1, b"opaque failure"), "unknown_failure")

    def test_scenario_matrix_covers_every_u3_failure_class(self):
        names = {name for name, _ in soak.SCENARIOS}
        self.assertTrue({
            "provider-loss",
            "network-loss",
            "process-kill",
            "disk-pressure",
            "corrupted-tail",
            "model-timeout",
            "adapter-crash",
            "large-noisy-output",
            "repeated-cancellation",
            "operator-rejection",
            "checkpoint-restart",
        }.issubset(names))

    def test_source_fingerprint_changes_when_an_input_changes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")
            before = soak.source_fingerprint(root, inputs=(Path("Cargo.toml"),))
            (root / "Cargo.toml").write_text("[workspace]\nmembers = []\n", encoding="utf-8")
            after = soak.source_fingerprint(root, inputs=(Path("Cargo.toml"),))

        self.assertNotEqual(before["sha256"], after["sha256"])
        self.assertEqual(before["files"], 1)
        self.assertEqual(after["files"], 1)

    def test_source_fingerprint_excludes_environment_secret_files(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            config = root / "config"
            config.mkdir()
            (config / "runtime.toml").write_text("enabled = true\n", encoding="utf-8")
            secret = config / ".env"
            secret.write_text("TOKEN=first\n", encoding="utf-8")
            before = soak.source_fingerprint(root, inputs=(Path("config"),))
            secret.write_text("TOKEN=second\n", encoding="utf-8")
            after = soak.source_fingerprint(root, inputs=(Path("config"),))

        self.assertEqual(before, after)
        self.assertEqual(before["files"], 1)

    def test_soak_environment_uses_an_isolated_cargo_target(self):
        target = Path("/tmp/arda-stage5-target")

        environment = soak.soak_environment(target, base={"PATH": os.environ["PATH"]})

        self.assertEqual(environment["CARGO_TARGET_DIR"], str(target.resolve()))

    def test_report_fails_when_source_drifts_or_disk_floor_is_breached(self):
        states = {"fixture": soak.ScenarioState(runs=1, passed=1, durations_ms=[12.5])}
        report = soak.render_report(
            started_at="2026-01-01T00:00:00Z",
            finished_at="2026-01-01T00:00:01Z",
            requested_seconds=1,
            elapsed_seconds=1,
            states=states,
            failures=[],
            protected_before={"files": 1, "bytes": 10},
            protected_after={"files": 1, "bytes": 10},
            source_before={"sha256": "before", "files": 1, "bytes": 10},
            source_after={"sha256": "after", "files": 1, "bytes": 11},
            storage={"minimum_free_bytes": 100, "minimum_observed_free_bytes": 99},
            invalid_reason="source_changed",
        )

        self.assertEqual(report["status"], "fail")
        self.assertEqual(report["validity"], "invalid")
        self.assertEqual(report["invalid_reason"], "source_changed")
        self.assertFalse(report["source_integrity"]["unchanged"])
        self.assertFalse(report["storage"]["floor_preserved"])


if __name__ == "__main__":
    unittest.main()
