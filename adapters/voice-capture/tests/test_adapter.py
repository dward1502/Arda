from __future__ import annotations

import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ADAPTER_PATH = Path(__file__).resolve().parents[1] / "arda_adapter.py"
SPEC = importlib.util.spec_from_file_location("voice_capture_adapter", ADAPTER_PATH)
assert SPEC and SPEC.loader
adapter = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = adapter
SPEC.loader.exec_module(adapter)


class VoiceCaptureAdapterTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name).resolve()
        self.audio = self.root / "capture.wav"
        self.audio.write_bytes(b"RIFF-local-audio")
        self.config = adapter.AdapterConfig(
            executable="local-stt",
            arguments=("--model", "{model}", "--audio", "{audio_path}"),
            model="local-model.bin",
            audio_root=self.root,
            allowed_extensions=frozenset({".wav", ".flac"}),
            timeout_seconds=3.0,
            max_audio_bytes=1024,
            max_output_bytes=64,
            default_audio_retention="ephemeral",
            default_transcript_retention="ephemeral",
        )

    def tearDown(self) -> None:
        self.temp.cleanup()

    def request(self, **extra: object) -> dict[str, object]:
        value: dict[str, object] = {
            "schema_version": adapter.REQUEST_SCHEMA,
            "audio_path": str(self.audio),
        }
        value.update(extra)
        return value

    @staticmethod
    def success(command, timeout, maximum):
        return adapter.ProcessResult(0, b"editable local transcript")

    def test_success_is_ephemeral_pending_review_and_never_action_authorized(self) -> None:
        result = adapter.process_request(self.request(), self.config, self.success)
        self.assertEqual(result["status"], "transcript_pending_review")
        self.assertEqual(result["audio_retention"], "ephemeral")
        self.assertEqual(result["transcript_retention"], "ephemeral")
        self.assertTrue(result["editable"])
        self.assertFalse(result["external_send_authorized"])
        self.assertFalse(result["governed_action_authorized"])

    def test_explicit_transcript_retention_is_reported_but_not_authorized(self) -> None:
        result = adapter.process_request(self.request(transcript_retention="retain"), self.config, self.success)
        self.assertEqual(result["transcript_retention"], "retain")
        self.assertFalse(result["external_send_authorized"])

    def test_unavailable_backend_returns_recoverable_audio_reference(self) -> None:
        def unavailable(command, timeout, maximum):
            raise FileNotFoundError
        result = adapter.process_request(self.request(), self.config, unavailable)
        self.assertEqual(result["status"], "recoverable_inbox")
        self.assertEqual(result["audio_reference"], str(self.audio))
        self.assertEqual(result["audio_retention"], "preserve_until_recovered")
        self.assertEqual(result["error_class"], "backend_unavailable")

    def test_timeout_returns_recoverable_inbox(self) -> None:
        def timeout(command, seconds, maximum):
            raise subprocess.TimeoutExpired(command, seconds)
        result = adapter.process_request(self.request(), self.config, timeout)
        self.assertEqual(result["status"], "recoverable_inbox")
        self.assertEqual(result["error_class"], "backend_timeout")

    def test_backend_failure_returns_redacted_class_without_output(self) -> None:
        result = adapter.process_request(
            self.request(), self.config, lambda command, timeout, maximum: adapter.ProcessResult(7, b"secret diagnostic")
        )
        self.assertEqual(result["error_class"], "backend_failed")
        self.assertNotIn("secret diagnostic", str(result))

    def test_oversized_output_is_recoverable(self) -> None:
        result = adapter.process_request(
            self.request(), self.config, lambda command, timeout, maximum: adapter.ProcessResult(0, b"x" * (maximum + 1))
        )
        self.assertEqual(result["error_class"], "output_limit_exceeded")
        self.assertIsNone(result["transcript"])

    def test_rejects_audio_outside_root_and_unsupported_extension(self) -> None:
        outside = Path(self.temp.name).parent / "outside.wav"
        outside.write_bytes(b"audio")
        try:
            escaped = adapter.process_request(self.request(audio_path=str(outside)), self.config, self.success)
            self.assertEqual(escaped["error_class"], "rejected_audio")
        finally:
            outside.unlink(missing_ok=True)
        text = self.root / "capture.txt"
        text.write_text("not audio", encoding="utf-8")
        rejected = adapter.process_request(self.request(audio_path=str(text)), self.config, self.success)
        self.assertEqual(rejected["error_class"], "rejected_audio")

    def test_rejects_oversized_audio_before_starting_backend(self) -> None:
        self.audio.write_bytes(b"x" * 1025)
        called = False
        def runner(command, timeout, maximum):
            nonlocal called
            called = True
            return adapter.ProcessResult(0, b"not used")
        result = adapter.process_request(self.request(), self.config, runner)
        self.assertEqual(result["error_class"], "rejected_audio")
        self.assertFalse(called)


if __name__ == "__main__":
    unittest.main()
