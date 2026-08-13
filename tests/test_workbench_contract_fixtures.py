#!/usr/bin/env python3
"""Cross-language S4-C1 contract fixture validation against JSON Schema."""

from __future__ import annotations

import json
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker

REPO_ROOT = Path(__file__).resolve().parents[1]


class WorkbenchContractFixtureTests(unittest.TestCase):
    def validator(self, schema_path: str) -> Draft202012Validator:
        schema = json.loads((REPO_ROOT / schema_path).read_text(encoding="utf-8"))
        Draft202012Validator.check_schema(schema)
        return Draft202012Validator(schema, format_checker=FormatChecker())

    def assert_valid(self, validator: Draft202012Validator, fixture_path: str) -> None:
        fixture = json.loads((REPO_ROOT / fixture_path).read_text(encoding="utf-8"))
        errors = sorted(validator.iter_errors(fixture), key=lambda error: list(error.path))
        self.assertEqual(errors, [], "\n".join(error.message for error in errors))

    def assert_invalid(self, validator: Draft202012Validator, fixture_path: str) -> None:
        fixture = json.loads((REPO_ROOT / fixture_path).read_text(encoding="utf-8"))
        self.assertTrue(list(validator.iter_errors(fixture)), f"{fixture_path} must fail closed")

    def test_project_contract_fixed_fixtures(self) -> None:
        validator = self.validator("spec/project-contract/v1/project-contract.schema.json")
        self.assert_valid(
            validator,
            "spec/project-contract/v1/fixtures/valid-project-contract.json",
        )
        self.assert_invalid(
            validator,
            "spec/project-contract/v1/fixtures/invalid-schema-version.json",
        )
        self.assert_invalid(
            validator,
            "spec/project-contract/v1/fixtures/invalid-project-contract.json",
        )

    def test_run_graph_fixed_fixtures(self) -> None:
        validator = self.validator("spec/run-graph/v1/run-graph.schema.json")
        self.assert_valid(
            validator,
            "spec/run-graph/v1/fixtures/valid-run-graph.json",
        )
        self.assert_invalid(
            validator,
            "spec/run-graph/v1/fixtures/invalid-schema-version.json",
        )
        self.assert_invalid(
            validator,
            "spec/run-graph/v1/fixtures/invalid-run-graph.json",
        )

    def test_hud_convergence_shared_fixture(self) -> None:
        validator = self.validator(
            "spec/hud-convergence/v1/hud-frontend-backend-contract.schema.json"
        )
        fixture_path = "spec/hud-convergence/v1/fixtures/valid-shared-contract.json"
        self.assert_valid(validator, fixture_path)
        self.assert_invalid(
            validator,
            "spec/hud-convergence/v1/fixtures/invalid-client-authority.json",
        )

        fixture = json.loads((REPO_ROOT / fixture_path).read_text(encoding="utf-8"))
        self.assertEqual(
            fixture["mutation"]["intent"]["intent_id"],
            fixture["mutation"]["receipt"]["intent_id"],
        )
        self.assertEqual(
            fixture["mutation"]["intent"]["operator_id"],
            fixture["mutation"]["receipt"]["operator_id"],
        )
        self.assertEqual(
            fixture["mutation"]["intent"]["action"],
            fixture["mutation"]["receipt"]["action"],
        )
        self.assertEqual(fixture["error_envelope"]["schema_version"], "arda.hud.error.v1")
        self.assertEqual(fixture["error_envelope"]["status"], "failed")
        self.assertTrue(fixture["error_envelope"]["code"])
        self.assertTrue(fixture["error_envelope"]["message"])
        self.assertTrue(fixture["error_envelope"]["recovery_action"])
        self.assertEqual(
            {state["status"] for state in fixture["load_states"]},
            {"loading", "healthy", "stale", "partial", "degraded", "unavailable", "failed"},
        )
        expected_slots = {
            "monitor_1",
            "monitor_2",
            "monitor_3",
            "monitor_4",
            "monitor_5",
        }
        sessions = fixture["monitor_sessions"]
        self.assertEqual(set(sessions), expected_slots)
        self.assertEqual({session["slot_id"] for session in sessions.values()}, expected_slots)
        self.assertEqual(len({session["owner"] for session in sessions.values()}), 5)
        self.assertEqual(
            {session["content"]["kind"] for session in sessions.values()},
            {"web", "media", "document", "terminal", "custom"},
        )
        for session in sessions.values():
            self.assertEqual(
                session["session_id"],
                session["workstation_handoff"]["session_id"],
            )


if __name__ == "__main__":
    unittest.main()
