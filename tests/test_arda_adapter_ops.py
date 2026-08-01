import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "arda_adapter_ops.py"
SPEC = importlib.util.spec_from_file_location("arda_adapter_ops", SCRIPT)
assert SPEC and SPEC.loader
adapter_ops = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = adapter_ops
SPEC.loader.exec_module(adapter_ops)


class ArdaAdapterOpsTests(unittest.TestCase):
    def setUp(self):
        schema = json.loads(
            (ROOT / "spec/project-contract/v1/project-contract.schema.json").read_text(encoding="utf-8")
        )
        self.validator = Draft202012Validator(schema, format_checker=FormatChecker())

    def test_all_language_templates_validate_and_default_deny_network_and_secrets(self):
        expected_programs = {"rust": "cargo", "python": "python3", "javascript": "pnpm"}
        for index, (kind, program) in enumerate(expected_programs.items(), start=1):
            contract = adapter_ops.project_contract(
                kind,
                f"sample-{kind}",
                f"550e8400-e29b-41d4-a716-44665544000{index}",
                "2026-07-31T00:00:00Z",
            )
            self.assertEqual(list(self.validator.iter_errors(contract)), [])
            self.assertEqual(contract["commands"][0]["program"], program)
            self.assertFalse(contract["permissions"]["network"]["allow"])
            self.assertEqual(contract["permissions"]["secrets"]["env_names"], [])

    def test_template_rejects_unsafe_name_and_invalid_uuid(self):
        with self.assertRaises(ValueError):
            adapter_ops.project_contract("python", "../escape")
        with self.assertRaises(ValueError):
            adapter_ops.project_contract("python", "valid", "not-a-uuid")

    def test_write_json_refuses_overwrite_without_force(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "arda-project.json"
            adapter_ops.write_json(output, {"first": True})
            with self.assertRaises(FileExistsError):
                adapter_ops.write_json(output, {"second": True})
            self.assertEqual(json.loads(output.read_text(encoding="utf-8")), {"first": True})

    def test_schema_receipt_requires_all_three_examples(self):
        receipt = adapter_ops.schema_receipt(ROOT)
        self.assertEqual(receipt["status"], "pass")
        self.assertEqual(len(receipt["validated_examples"]), 3)


if __name__ == "__main__":
    unittest.main()
