from __future__ import annotations

import json
import os
import subprocess
import sys
import time
import unittest
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[3]
PYTHON_SDK = REPO_ROOT / "sdk" / "python"
SCHEMA_PATH = REPO_ROOT / "spec" / "project-adapter" / "v1" / "messages.schema.json"

try:
    from jsonschema import Draft202012Validator, FormatChecker
except ModuleNotFoundError:  # The reference suite remains runnable with the stdlib alone.
    MESSAGE_VALIDATOR = None
else:
    _schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(_schema)
    MESSAGE_VALIDATOR = Draft202012Validator(_schema, format_checker=FormatChecker())


class AdapterProcess:
    def __init__(self) -> None:
        environment = os.environ.copy()
        environment["PYTHONPATH"] = str(PYTHON_SDK)
        self.process = subprocess.Popen(
            [sys.executable, "-u", "-m", "arda_project_adapter.server"],
            cwd=REPO_ROOT,
            env=environment,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )

    def send(self, message: dict[str, Any]) -> None:
        self.validate(message)
        assert self.process.stdin is not None
        self.process.stdin.write(json.dumps(message, separators=(",", ":")) + "\n")
        self.process.stdin.flush()

    def send_raw(self, line: str) -> None:
        assert self.process.stdin is not None
        self.process.stdin.write(line + "\n")
        self.process.stdin.flush()

    def receive(self) -> dict[str, Any]:
        assert self.process.stdout is not None
        line = self.process.stdout.readline()
        self.assert_alive(line)
        message = json.loads(line)
        self.validate(message)
        return message

    @staticmethod
    def validate(message: dict[str, Any]) -> None:
        if MESSAGE_VALIDATOR is not None:
            errors = sorted(
                MESSAGE_VALIDATOR.iter_errors(message), key=lambda error: list(error.path)
            )
            if errors:
                raise AssertionError("invalid protocol frame: " + "; ".join(map(str, errors)))

    def assert_alive(self, line: str) -> None:
        if not line:
            stderr = self.process.stderr.read() if self.process.stderr else ""
            raise AssertionError(f"adapter exited before response: {stderr}")

    def close(self) -> None:
        if self.process.stdin:
            self.process.stdin.close()
        try:
            self.process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait(timeout=2)
        if self.process.stdout:
            self.process.stdout.close()
        if self.process.stderr:
            self.process.stderr.close()


class ProjectAdapterConformanceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.adapter = AdapterProcess()

    def tearDown(self) -> None:
        self.adapter.close()

    @staticmethod
    def message(identifier: str, kind: str, **fields: Any) -> dict[str, Any]:
        return {
            "schema_version": "arda.project-adapter.v1",
            "id": identifier,
            "type": kind,
            **fields,
        }

    def initialize(self, capabilities: list[str] | None = None) -> dict[str, Any]:
        capabilities = capabilities or ["echo", "progress", "sleep", "inspect"]
        self.adapter.send(
            self.message(
                "init-1",
                "initialize",
                protocol_version="1",
                project_root=str(REPO_ROOT),
                allowed_capabilities=capabilities,
            )
        )
        response = self.adapter.receive()
        self.assertEqual(response["type"], "initialized")
        self.assertEqual(response["request_id"], "init-1")
        self.assertEqual(response["capabilities"], capabilities)
        return response

    def test_schema_is_draft_2020_12_and_covers_all_protocol_messages(self) -> None:
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        self.assertEqual(schema["$schema"], "https://json-schema.org/draft/2020-12/schema")
        self.assertEqual(
            set(schema["$defs"]),
            {
                "id",
                "capability",
                "capabilities",
                "common",
                "provenance",
                "initialize",
                "initialized",
                "health",
                "health_status",
                "request",
                "progress",
                "result",
                "cancel",
                "cancelled",
                "denied_capability",
                "error",
            },
        )

    def test_initialize_health_and_result_include_capabilities_and_provenance(self) -> None:
        initialized = self.initialize()
        self.assertTrue(initialized["recovery_supported"])

        self.adapter.send(self.message("health-1", "health"))
        health = self.adapter.receive()
        self.assertEqual((health["type"], health["status"]), ("health_status", "ready"))
        self.assertEqual(health["request_id"], "health-1")

        self.adapter.send(
            self.message(
                "request-1",
                "request",
                operation="echo",
                arguments={"value": 42},
                timeout_ms=1000,
                required_capabilities=["echo"],
                idempotency_key="echo-1",
            )
        )
        result = self.adapter.receive()
        self.assertEqual((result["type"], result["status"]), ("result", "succeeded"))
        self.assertEqual(result["output"], {"value": 42})
        self.assertRegex(result["provenance"]["request_digest"], r"^sha256:[0-9a-f]{64}$")
        self.assertEqual(result["request_id"], "request-1")

    def test_progress_and_recovery_token_are_correlated(self) -> None:
        self.initialize()
        self.adapter.send(
            self.message(
                "request-progress",
                "request",
                operation="progress",
                arguments={"steps": 2},
                timeout_ms=1000,
                required_capabilities=["progress"],
                idempotency_key="progress-1",
                recovery_token="resume-from-7",
            )
        )
        first = self.adapter.receive()
        second = self.adapter.receive()
        result = self.adapter.receive()
        self.assertEqual([first["sequence"], second["sequence"]], [1, 2])
        self.assertTrue(all(item["request_id"] == "request-progress" for item in (first, second)))
        self.assertEqual(result["recovery_token"], "resume-from-7")

    def test_unapproved_capability_is_denied_without_execution(self) -> None:
        self.initialize(["echo"])
        self.adapter.send(
            self.message(
                "request-denied",
                "request",
                operation="sleep",
                arguments={"seconds": 1},
                timeout_ms=1000,
                required_capabilities=["sleep"],
                idempotency_key="denied-1",
            )
        )
        denied = self.adapter.receive()
        self.assertEqual(denied["type"], "denied_capability")
        self.assertEqual(denied["capability"], "sleep")
        self.assertEqual(denied["request_id"], "request-denied")

    def test_cooperative_cancellation_is_acknowledged_and_terminal(self) -> None:
        self.initialize()
        self.adapter.send(
            self.message(
                "request-sleep",
                "request",
                operation="sleep",
                arguments={"seconds": 5},
                timeout_ms=6000,
                required_capabilities=["sleep"],
                idempotency_key="sleep-1",
            )
        )
        time.sleep(0.05)
        self.adapter.send(
            self.message("cancel-1", "cancel", request_id="request-sleep")
        )
        frames = [self.adapter.receive(), self.adapter.receive()]
        self.assertEqual({frame["type"] for frame in frames}, {"cancelled", "result"})
        result = next(frame for frame in frames if frame["type"] == "result")
        self.assertEqual(result["status"], "cancelled")

    def test_malformed_json_fails_closed_with_protocol_error(self) -> None:
        self.adapter.send_raw("{not-json")
        error = self.adapter.receive()
        self.assertEqual(error["type"], "error")
        self.assertEqual(error["code"], "invalid_json")
        self.assertFalse(error["retryable"])


if __name__ == "__main__":
    unittest.main()
