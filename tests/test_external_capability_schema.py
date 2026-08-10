import json
from pathlib import Path

import jsonschema


ROOT = Path(__file__).resolve().parents[1]
SPEC = ROOT / "spec" / "external-capability" / "v1"


def test_external_capability_schema_and_valid_fixture():
    schema = json.loads((SPEC / "external-capability.schema.json").read_text())
    jsonschema.Draft202012Validator.check_schema(schema)
    validator = jsonschema.Draft202012Validator(
        schema, format_checker=jsonschema.FormatChecker()
    )
    valid = json.loads((SPEC / "fixtures" / "valid-hermes-workbench.json").read_text())
    validator.validate(valid)
    for contract in sorted((ROOT / "config" / "adapters").glob("*.external-capability.json")):
        validator.validate(json.loads(contract.read_text()))


def test_duplicate_authority_fixture_is_schema_invalid():
    schema = json.loads((SPEC / "external-capability.schema.json").read_text())
    invalid = json.loads(
        (SPEC / "fixtures" / "invalid-duplicate-authority.json").read_text()
    )
    errors = list(jsonschema.Draft202012Validator(schema).iter_errors(invalid))
    assert errors
    assert any(list(error.path) == ["authority", "task_authority"] for error in errors)