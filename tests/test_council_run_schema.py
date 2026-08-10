import copy
import json
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator


ROOT = Path(__file__).resolve().parents[1]
SPEC = ROOT / "spec" / "council-run" / "v1"


class CouncilRunSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = json.loads((SPEC / "council-run.schema.json").read_text())
        Draft202012Validator.check_schema(cls.schema)
        cls.validator = Draft202012Validator(cls.schema)
        cls.fixture = json.loads(
            (SPEC / "fixtures" / "valid-independent-disagreement.json").read_text()
        )

    def test_independent_disagreement_fixture_is_valid(self) -> None:
        self.assertEqual(list(self.validator.iter_errors(self.fixture)), [])
        self.assertTrue(self.fixture["non_approval"])
        self.assertGreater(len(self.fixture["material_tensions"]), 0)

    def test_final_approval_claim_and_missing_route_provenance_are_rejected(self) -> None:
        payload = copy.deepcopy(self.fixture)
        payload["non_approval"] = False
        self.assertTrue(list(self.validator.iter_errors(payload)))

        payload = copy.deepcopy(self.fixture)
        del payload["participants"][1]["provider_id"]
        self.assertTrue(list(self.validator.iter_errors(payload)))

    def test_unknown_fields_and_versions_are_rejected(self) -> None:
        payload = copy.deepcopy(self.fixture)
        payload["schema_version"] = "arda.council-run.v2"
        payload["fabricated_consensus"] = True
        self.assertTrue(list(self.validator.iter_errors(payload)))


if __name__ == "__main__":
    unittest.main()
