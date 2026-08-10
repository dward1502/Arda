import json
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator


ROOT = Path(__file__).resolve().parents[1]
SPEC = ROOT / "spec" / "capability-composition" / "v1"


class CapabilityCompositionSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = json.loads((SPEC / "capability-composition.schema.json").read_text())
        Draft202012Validator.check_schema(cls.schema)
        cls.validator = Draft202012Validator(cls.schema)

    def fixture(self, name: str) -> dict:
        return json.loads((SPEC / "fixtures" / name).read_text())

    def test_all_fixed_fixtures_are_structurally_schema_valid(self) -> None:
        for name in (
            "valid-personal-objective.json",
            "valid-software-project.json",
            "valid-council-assisted-project.json",
            "invalid-authority-escalation.json",
            "invalid-sensitive-egress.json",
        ):
            errors = list(self.validator.iter_errors(self.fixture(name)))
            self.assertEqual(errors, [], f"{name}: {errors}")

    def test_schema_rejects_unknown_fields_and_versions(self) -> None:
        payload = self.fixture("valid-personal-objective.json")
        payload["unknown_field"] = True
        self.assertTrue(list(self.validator.iter_errors(payload)))

        payload = self.fixture("valid-personal-objective.json")
        payload["schema_version"] = "arda.capability-composition.v2"
        self.assertTrue(list(self.validator.iter_errors(payload)))


if __name__ == "__main__":
    unittest.main()
