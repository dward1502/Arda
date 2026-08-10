"""Regression tests for the durable Hermes-to-Arda handoff."""

from __future__ import annotations

import asyncio
import importlib.util
import io
import os
import tempfile
import unittest
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


if __name__ == "__main__":
    unittest.main()
