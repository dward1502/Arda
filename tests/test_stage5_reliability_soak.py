import importlib.util
import sys
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


if __name__ == "__main__":
    unittest.main()
