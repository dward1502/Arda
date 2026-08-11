import copy
import json
import unittest
from datetime import datetime
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker


ROOT = Path(__file__).resolve().parents[1]
SPEC = ROOT / "spec" / "operator-session" / "v1"
APPROVAL_OPERATIONS = {"approve", "reject", "revise"}


def parse_timestamp(value: str) -> datetime:
    return datetime.fromisoformat(value.replace("Z", "+00:00"))


def semantic_errors(payload: dict) -> list[str]:
    errors: list[str] = []
    approval = payload["approval"]
    operation = payload["operation"]

    if operation in APPROVAL_OPERATIONS:
        if approval["single_use_state"] != "available":
            errors.append("approval is not available for single use")
        if approval["consumed_by_event_id"] is not None:
            errors.append("approval already records a consuming event")
        if parse_timestamp(approval["expires_at"]) <= parse_timestamp(
            payload["incoming"]["received_at"]
        ):
            errors.append("approval expired before receipt")

    incoming_event = payload["incoming"]["event_id"]
    for attachment in payload["content"]["attachments"]:
        if attachment["provenance"]["transport_event_id"] != incoming_event:
            errors.append("attachment provenance does not match incoming event")

    projection = payload["projection"]
    content = payload["content"]
    if projection["audience"] in {"group", "public"} and content[
        "sensitivity"
    ] in {"private", "health", "financial"}:
        if content["text"] not in {None, "[REDACTED]"} or content["attachments"]:
            errors.append("sensitive content is exposed to a non-private audience")

    delivery = payload["delivery"]
    if delivery["state"] == "acknowledged" and delivery["acknowledgement_id"] is None:
        errors.append("acknowledged delivery lacks acknowledgement id")
    if delivery["state"] == "retry_scheduled" and delivery["next_retry_at"] is None:
        errors.append("scheduled retry lacks retry timestamp")

    return errors


class OperatorSessionSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = json.loads((SPEC / "operator-session.schema.json").read_text())
        Draft202012Validator.check_schema(cls.schema)
        cls.validator = Draft202012Validator(
            cls.schema, format_checker=FormatChecker()
        )

    def fixture(self, name: str) -> dict:
        return json.loads((SPEC / "fixtures" / name).read_text())

    def assert_schema_valid(self, payload: dict) -> None:
        errors = sorted(self.validator.iter_errors(payload), key=lambda item: list(item.path))
        self.assertEqual(errors, [], [error.message for error in errors])

    def test_valid_phone_capture_and_approval_response(self) -> None:
        for name in ("valid-phone-capture.json", "valid-approval-response.json"):
            payload = self.fixture(name)
            self.assert_schema_valid(payload)
            self.assertEqual(semantic_errors(payload), [], name)

    def test_replayed_approval_is_structurally_preserved_but_semantically_rejected(self) -> None:
        payload = self.fixture("invalid-replayed-approval.json")
        self.assert_schema_valid(payload)
        errors = semantic_errors(payload)
        self.assertIn("approval is not available for single use", errors)
        self.assertIn("approval already records a consuming event", errors)

    def test_schema_fails_closed_for_version_authentication_and_approval_shape(self) -> None:
        base = self.fixture("valid-phone-capture.json")

        unknown = copy.deepcopy(base)
        unknown["unknown_field"] = True
        self.assertTrue(list(self.validator.iter_errors(unknown)))

        version = copy.deepcopy(base)
        version["schema_version"] = "arda.operator-session.v2"
        self.assertTrue(list(self.validator.iter_errors(version)))

        unauthenticated = copy.deepcopy(base)
        unauthenticated["operator"]["authenticated"] = False
        self.assertTrue(list(self.validator.iter_errors(unauthenticated)))

        approval_without_scope = self.fixture("valid-approval-response.json")
        approval_without_scope["approval"]["scope"] = []
        self.assertTrue(list(self.validator.iter_errors(approval_without_scope)))

    def test_private_content_cannot_project_to_group_or_public_audience(self) -> None:
        payload = self.fixture("valid-phone-capture.json")
        payload["projection"]["audience"] = "group"
        self.assertIn(
            "sensitive content is exposed to a non-private audience",
            semantic_errors(payload),
        )

    def test_expired_approval_is_semantically_rejected(self) -> None:
        payload = self.fixture("valid-approval-response.json")
        payload["approval"]["expires_at"] = "2026-08-08T10:09:59Z"
        self.assertIn("approval expired before receipt", semantic_errors(payload))

    def test_attachment_provenance_and_delivery_state_are_correlated(self) -> None:
        payload = self.fixture("valid-phone-capture.json")
        payload["content"]["attachments"][0]["provenance"][
            "transport_event_id"
        ] = "telegram:update:other"
        self.assertIn(
            "attachment provenance does not match incoming event",
            semantic_errors(payload),
        )

        delivery = self.fixture("valid-approval-response.json")
        delivery["delivery"]["acknowledgement_id"] = None
        self.assertIn(
            "acknowledged delivery lacks acknowledgement id",
            semantic_errors(delivery),
        )


if __name__ == "__main__":
    unittest.main()
