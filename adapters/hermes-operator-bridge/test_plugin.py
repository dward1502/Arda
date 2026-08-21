"""Regression tests for the durable Hermes-to-Arda handoff."""

from __future__ import annotations

import asyncio
import importlib.util
import io
import os
import tempfile
import unittest
from datetime import datetime, timezone
from email.message import Message
from pathlib import Path
from types import SimpleNamespace
from typing import Any
from unittest.mock import patch
from urllib.error import HTTPError, URLError

_PLUGIN_PATH = Path(__file__).with_name("__init__.py")
_SPEC = importlib.util.spec_from_file_location("arda_operator_bridge_plugin", _PLUGIN_PATH)
assert _SPEC is not None and _SPEC.loader is not None
plugin: Any = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(plugin)


class _Response:
    def __init__(self, body: str):
        self._body = body.encode("utf-8")

    def __enter__(self):
        return self

    def __exit__(self, *_args):
        return False

    def read(self):
        return self._body


class _Gateway:
    def __init__(self):
        self.notices: list[str] = []

    async def _deliver_platform_notice(self, _source, text: str):
        self.notices.append(text)

    def _is_user_authorized(self, _source):
        return True


_SOURCE = SimpleNamespace(platform="discord", chat_id="private-chat", thread_id=None)


def _payload(message_id: str = "message-1") -> dict:
    return {
        "operator": {
            "operator_id": "operator-1",
            "authenticated": True,
            "authentication_method": "gateway_identity",
            "authenticated_at": "2026-08-09T00:00:00Z",
        },
        "adapter_id": "hermes-discord-default",
        "event": {
            "text": "arda status",
            "message_type": "text",
            "user_id": "operator-1",
            "source": {
                "platform": "discord",
                "chat_id": "private-chat",
                "chat_type": "private",
                "thread_id": None,
                "message_id": message_id,
            },
            "message_id": message_id,
            "media_urls": [],
            "media_types": [],
            "timestamp": "2026-08-09T00:00:00Z",
            "prompt_response": None,
        },
    }


class DurableRetryTests(unittest.IsolatedAsyncioTestCase):
    async def asyncSetUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.env = patch.dict(os.environ, {"HERMES_HOME": self.temp.name})
        self.env.start()
        plugin._RETRY_DELAYS = (0.0,)
        plugin._RETRY_TASKS.clear()

    async def asyncTearDown(self):
        if plugin._RETRY_TASKS:
            await asyncio.gather(*plugin._RETRY_TASKS.values(), return_exceptions=True)
        self.env.stop()
        self.temp.cleanup()

    async def test_gateway_unavailable_is_persisted_then_retried(self):
        gateway = _Gateway()
        with patch.object(
            plugin,
            "urlopen",
            side_effect=[URLError("offline"), _Response('{"summary":"Run status recovered."}')],
        ) as mocked:
            summary = plugin._submit(_payload(), gateway, _SOURCE)
            self.assertEqual(summary, "Arda is unavailable; command queued for retry.")
            pending = list(plugin._pending_root().glob("*.json"))
            self.assertEqual(len(pending), 1)
            self.assertEqual(pending[0].stat().st_mode & 0o777, 0o600)
            await asyncio.gather(*list(plugin._RETRY_TASKS.values()))
            await asyncio.sleep(0)

        self.assertEqual(mocked.call_count, 2)
        self.assertFalse(list(plugin._pending_root().glob("*.json")))
        self.assertEqual(gateway.notices, ["Run status recovered."])

    async def test_completed_command_replay_is_terminal(self):
        gateway = _Gateway()
        error = HTTPError(
            plugin._ENDPOINT,
            409,
            "Conflict",
            Message(),
            io.BytesIO(b'{"error":"duplicate transport event"}'),
        )
        with patch.object(plugin, "urlopen", side_effect=error):
            summary = plugin._submit(_payload("replayed-message"), gateway, _SOURCE)

        self.assertEqual(summary, "Arda already accepted this command.")
        self.assertFalse(list(plugin._pending_root().glob("*.json")))
        self.assertFalse(plugin._RETRY_TASKS)

    async def test_gateway_restart_reactivates_pending_delivery(self):
        gateway = _Gateway()
        path = plugin._persist_pending(_payload("pending-across-restart"))
        self.assertTrue(path.exists())

        with patch.object(
            plugin,
            "urlopen",
            return_value=_Response('{"summary":"Pending command delivered."}'),
        ) as mocked:
            plugin._schedule_backlog(gateway, _SOURCE)
            await asyncio.gather(*list(plugin._RETRY_TASKS.values()))
            await asyncio.sleep(0)

        mocked.assert_called_once()
        self.assertFalse(path.exists())
        self.assertEqual(gateway.notices, ["Pending command delivered."])


class ContinuityEventTests(unittest.IsolatedAsyncioTestCase):
    async def asyncSetUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.env = patch.dict(os.environ, {"HERMES_HOME": self.temp.name})
        self.env.start()
        plugin._RETRY_DELAYS = (0.0,)
        plugin._CONTINUITY_RETRY_TASKS.clear()

    async def asyncTearDown(self):
        if plugin._CONTINUITY_RETRY_TASKS:
            await asyncio.gather(
                *plugin._CONTINUITY_RETRY_TASKS.values(), return_exceptions=True
            )
        self.env.stop()
        self.temp.cleanup()

    def event(
        self,
        *,
        chat_type="dm",
        thread_id=None,
        message_id="message-1",
        text="ordinary private conversation text that must not be copied",
    ):
        source = SimpleNamespace(
            platform="discord",
            chat_id="private-chat" if thread_id is None else "thread-chat",
            chat_type=chat_type,
            thread_id=thread_id,
            user_id="operator-1",
            user_name="Operator",
            message_id=message_id,
        )
        return SimpleNamespace(
            source=source,
            user_id="operator-1",
            user_name="Operator",
            message_id=message_id,
            timestamp=datetime(2026, 8, 17, tzinfo=timezone.utc),
            text=text,
            message_type="text",
            media_urls=[],
            media_types=[],
            prompt_response=None,
        )

    def session_store(self):
        return SimpleNamespace(
            get_or_create_session=lambda _source, touch_activity=False: SimpleNamespace(
                session_id="hermes-session-1", session_key="main:discord:dm:private-chat"
            )
        )

    async def test_ordinary_message_continues_while_metadata_is_emitted(self):
        gateway = _Gateway()
        with patch.object(plugin, "_submit_continuity") as submit:
            result = plugin._pre_gateway_dispatch(
                self.event(), gateway, session_store=self.session_store()
            )
        self.assertIsNone(result)
        submit.assert_called_once()
        payload = submit.call_args.args[0]
        self.assertNotIn("text", payload["event"])
        self.assertEqual(payload["operator"]["operator_id"], "operator:mythos")
        self.assertEqual(payload["event"]["source_user_ref"], "operator-1")
        self.assertEqual(payload["event"]["current_session_id"], "hermes-session-1")
        self.assertEqual(payload["event"]["privacy_class"], "personal_device")

    async def test_private_natural_capture_is_forwarded_as_explicit_fallback_command(self):
        gateway = _Gateway()
        event = self.event(text="Remember that I need to renew the prescription")
        with (
            patch.object(plugin, "_submit", return_value="Captured.") as submit,
            patch.object(plugin, "_submit_continuity"),
        ):
            result = plugin._pre_gateway_dispatch(
                event, gateway, session_store=self.session_store()
            )
            await asyncio.sleep(0)

        self.assertEqual(result, {"action": "skip"})
        payload = submit.call_args.args[0]
        self.assertEqual(
            payload["event"]["text"],
            "arda capture I need to renew the prescription",
        )
        self.assertEqual(payload["operator"]["operator_id"], "operator:mythos")
        self.assertEqual(payload["event"]["user_id"], "operator:mythos")
        self.assertEqual(gateway.notices, ["Captured."])

    async def test_private_context_and_research_intents_are_bounded(self):
        cases = [
            ("What should I work on next?", "arda context"),
            ("Research practical x402 earning opportunities", "arda research practical x402 earning opportunities"),
        ]
        for index, (text, expected) in enumerate(cases):
            with self.subTest(text=text):
                event = self.event(message_id=f"intent-{index}", text=text)
                with (
                    patch.object(plugin, "_submit", return_value="Handled.") as submit,
                    patch.object(plugin, "_submit_continuity"),
                ):
                    result = plugin._pre_gateway_dispatch(
                        event, _Gateway(), session_store=self.session_store()
                    )
                self.assertEqual(result, {"action": "skip"})
                self.assertEqual(submit.call_args.args[0]["event"]["text"], expected)

    async def test_explicit_arda_command_remains_the_deterministic_fallback(self):
        event = self.event(text="arda context")
        with (
            patch.object(plugin, "_submit", return_value="Handled.") as submit,
            patch.object(plugin, "_submit_continuity"),
        ):
            result = plugin._pre_gateway_dispatch(
                event, _Gateway(), session_store=self.session_store()
            )
        self.assertEqual(result, {"action": "skip"})
        self.assertEqual(submit.call_args.args[0]["event"]["text"], "arda context")

    async def test_attached_project_objective_requires_explicit_project_identity(self):
        project_id = "550e8400-e29b-41d4-a716-446655440000"
        event = self.event(
            text=f"For project {project_id}, objective finish the release checklist"
        )
        with (
            patch.object(plugin, "_submit", return_value="Objective recorded.") as submit,
            patch.object(plugin, "_submit_continuity"),
        ):
            result = plugin._pre_gateway_dispatch(
                event, _Gateway(), session_store=self.session_store()
            )

        self.assertEqual(result, {"action": "skip"})
        self.assertEqual(
            submit.call_args.args[0]["event"]["text"],
            f"arda objective {project_id} finish the release checklist",
        )

    async def test_consequential_or_ambiguous_intent_requests_clarification_without_mutation(self):
        gateway = _Gateway()
        event = self.event(text="Deploy this to production")
        with (
            patch.object(plugin, "_submit") as submit,
            patch.object(plugin, "_submit_continuity"),
        ):
            result = plugin._pre_gateway_dispatch(
                event, gateway, session_store=self.session_store()
            )
            await asyncio.sleep(0)

        self.assertEqual(result, {"action": "skip"})
        submit.assert_not_called()
        self.assertEqual(len(gateway.notices), 1)
        self.assertIn("consequential", gateway.notices[0].lower())
        self.assertIn("nothing was saved", gateway.notices[0].lower())

    async def test_natural_intent_does_not_intercept_shared_rooms(self):
        event = self.event(chat_type="group", text="Remember that this is casual")
        with (
            patch.object(plugin, "_submit") as submit,
            patch.object(plugin, "_submit_continuity"),
        ):
            result = plugin._pre_gateway_dispatch(
                event, _Gateway(), session_store=self.session_store()
            )
        self.assertIsNone(result)
        submit.assert_not_called()

    async def test_thread_and_shared_destination_are_explicit(self):
        payload = plugin._continuity_payload(
            self.event(chat_type="thread", thread_id="topic-7"),
            self.session_store(),
        )
        self.assertEqual(payload["event"]["thread_id"], "topic-7")
        self.assertEqual(payload["event"]["privacy_class"], "shared_room")
        self.assertIn("topic-7", payload["event"]["surface_id"])

    async def test_malformed_source_identity_does_not_emit_or_intercept(self):
        event = self.event(message_id="")
        event.source.message_id = ""
        with patch.object(plugin, "_submit_continuity") as submit:
            result = plugin._pre_gateway_dispatch(
                event, _Gateway(), session_store=self.session_store()
            )
        self.assertIsNone(result)
        submit.assert_not_called()

    async def test_unavailable_arda_persists_continuity_for_restart_retry(self):
        payload = plugin._continuity_payload(self.event(), self.session_store())
        with patch.object(plugin, "urlopen", side_effect=URLError("offline")):
            plugin._submit_continuity(payload, _Gateway(), self.event().source)
            await asyncio.gather(*list(plugin._CONTINUITY_RETRY_TASKS.values()))
        pending = list(plugin._continuity_pending_root().glob("*.json"))
        self.assertEqual(len(pending), 1)
        self.assertEqual(pending[0].stat().st_mode & 0o777, 0o600)

    async def test_gateway_restart_reactivates_continuity_backlog(self):
        payload = plugin._continuity_payload(self.event(), self.session_store())
        path = plugin._persist_continuity(payload)
        with patch.object(plugin, "_post_continuity", return_value=True):
            plugin._schedule_continuity_backlog()
            await asyncio.gather(*list(plugin._CONTINUITY_RETRY_TASKS.values()))
        self.assertFalse(path.exists())

    async def test_duplicate_continuity_response_is_terminal(self):
        error = HTTPError(
            plugin._CONTINUITY_ENDPOINT,
            409,
            "Conflict",
            Message(),
            io.BytesIO(b'{"code":"conflict"}'),
        )
        with patch.object(plugin, "urlopen", side_effect=error):
            self.assertTrue(
                plugin._post_continuity(
                    plugin._continuity_payload(self.event(), self.session_store())
                )
            )


if __name__ == "__main__":
    unittest.main()
