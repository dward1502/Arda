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
from unittest.mock import AsyncMock, patch
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

    def _adapter_for_source(self, _source):
        return self

    def _thread_metadata_for_source(self, _source):
        return {}

    async def send(self, _chat_id, text, metadata=None):
        self.notices.append(text)
        return SimpleNamespace(success=True, message_id="receipt-1")

    async def _deliver_platform_notice(self, _source, text: str):
        self.notices.append(text)

    def _is_user_authorized(self, _source):
        return True


class _SendingAdapter:
    def __init__(self):
        self.sent: list[str] = []

    async def send(self, _chat_id, text: str, metadata=None):
        self.sent.append(text)
        return SimpleNamespace(success=True, message_id="discord-message-1")


class _ReminderGateway(_Gateway):
    def __init__(self):
        super().__init__()
        self.adapter = _SendingAdapter()

    def _adapter_for_source(self, _source):
        return self.adapter

    def _thread_metadata_for_source(self, _source):
        return {}


_SOURCE = SimpleNamespace(platform="discord", chat_id="private-chat", thread_id=None, chat_type="private")


class CapabilityAuthenticationTests(unittest.TestCase):
    def test_research_result_survives_a_real_three_second_http_delay(self):
        from http.server import BaseHTTPRequestHandler, HTTPServer
        import threading
        import time

        class Handler(BaseHTTPRequestHandler):
            def do_POST(self):
                self.rfile.read(int(self.headers["Content-Length"]))
                time.sleep(3.2)
                body = b'{"summary":"Delayed advisory.","evidence_refs":["arda://research/briefs/delayed"]}'
                self.send_response(200)
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)

            def log_message(self, *_args):
                pass

        server = HTTPServer(("127.0.0.1", 0), Handler)
        worker = threading.Thread(target=server.serve_forever)
        worker.start()
        try:
            payload = _payload()
            payload["event"]["text"] = "arda research bounded public question"
            with (
                patch.object(plugin, "_ENDPOINT", f"http://127.0.0.1:{server.server_port}/v1/operator/messages"),
                patch.object(plugin, "_gateway_capability", return_value="fixture-only"),
            ):
                accepted, summary = plugin._post(payload)
            self.assertTrue(accepted)
            self.assertIn("Delayed advisory.", summary)
            self.assertIn("arda://research/briefs/delayed", summary)
        finally:
            server.shutdown()
            worker.join(timeout=5)
            server.server_close()
            self.assertFalse(worker.is_alive())

    def test_research_waits_for_bounded_result_and_delivers_evidence(self):
        payload = _payload()
        payload["event"]["text"] = "arda research public reference question"
        with (
            patch.dict(os.environ, {"ARDA_HERMES_GATEWAY_CAPABILITY": "test-only"}),
            patch.object(plugin, "urlopen", return_value=_Response(
                '{"summary":"Advisory only.","evidence_refs":["arda://research/briefs/test-brief"]}'
            )) as mocked,
        ):
            accepted, summary = plugin._post(payload)
        self.assertTrue(accepted)
        self.assertGreater(mocked.call_args.kwargs["timeout"], 60)
        self.assertIn("Advisory only.", summary)
        self.assertIn("arda://research/briefs/test-brief", summary)

    def test_operator_post_sends_gateway_capability_header(self):
        with (
            patch.dict(
                os.environ,
                {"ARDA_HERMES_GATEWAY_CAPABILITY": "test-gateway-capability"},
            ),
            patch.object(
                plugin,
                "urlopen",
                return_value=_Response('{"summary":"Accepted."}'),
            ) as mocked,
        ):
            accepted, summary = plugin._post(_payload())

        self.assertTrue(accepted)
        self.assertEqual(summary, "Accepted.")
        request = mocked.call_args.args[0]
        self.assertEqual(
            request.get_header("X-arda-gateway-capability"),
            "test-gateway-capability",
        )


class ReminderDeliveryTests(unittest.IsolatedAsyncioTestCase):
    async def asyncSetUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.env = patch.dict(os.environ, {"HERMES_HOME": self.temp.name})
        self.env.start()

    async def asyncTearDown(self):
        self.env.stop()
        self.temp.cleanup()

    async def test_due_reminder_records_delivered_only_with_provider_receipt(self):
        gateway = _ReminderGateway()
        source = SimpleNamespace(
            platform="discord",
            chat_id="private-chat",
            chat_type="dm",
            thread_id=None,
            user_id="operator-1",
        )
        brief = {
            "brief": {
                "today": [
                    {
                        "item_id": "550e8400-e29b-41d4-a716-446655440000",
                        "content": "Operator-authored test reminder",
                        "scheduled_at": "2026-08-20T19:00:00Z",
                        "due_at": None,
                        "reminder_id": None,
                        "reminder_state": None,
                        "reminder_attempts": 0,
                    }
                ]
            }
        }
        capabilities = {
            "reminders": {
                "state": "configured",
                "adapter": "discord_dm",
                "max_attempts": 3,
                "minimum_interval_minutes": 15,
                "quiet_window": None,
            }
        }
        calls: list[tuple[str, str, dict | None]] = []

        def request_json(path, *, method="GET", payload=None, idempotency_key=None):
            calls.append((path, method, payload))
            if path == "/v1/personal/capabilities":
                return capabilities
            if path == "/v1/personal/briefs/today":
                return brief
            if path == "/v1/personal/reminders/attempt":
                return {"event_id": "event-1"}
            raise AssertionError(path)

        with (
            patch.object(plugin, "_request_json", side_effect=request_json),
            patch.object(
                plugin,
                "_utcnow",
                return_value=datetime(2026, 8, 20, 20, 0, tzinfo=timezone.utc),
            ),
        ):
            await plugin._deliver_due_reminders_once(gateway, source)

        self.assertEqual(len(gateway.adapter.sent), 1)
        self.assertIn("Operator-authored test reminder", gateway.adapter.sent[0])
        attempt = next(call for call in calls if call[0].endswith("/attempt"))
        self.assertIsNotNone(attempt[2])
        self.assertEqual(attempt[2]["state"], "delivered")
        self.assertEqual(attempt[2]["provider_message_id"], "discord-message-1")


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
    async def test_completed_callback_cannot_remove_replacement(self):
        class Task:
            completed = False
            def __init__(self):
                self.callbacks = []
            def done(self):
                return self.completed
            def add_done_callback(self, callback):
                self.callbacks.append(callback)
        def create_task(coroutine, **_kwargs):
            coroutine.close()
            return Task()
        path = plugin._pending_path(_payload("race"))
        gateway = _Gateway()
        cases = [
            (plugin._RETRY_TASKS, lambda: plugin._submit(_payload("race"), gateway, _SOURCE)),
            (plugin._RETRY_TASKS, lambda: plugin._schedule_retry(path, gateway, _SOURCE)),
            (plugin._CONTINUITY_RETRY_TASKS, lambda: plugin._submit_continuity({}, gateway, _SOURCE)),
            (plugin._CONTINUITY_RETRY_TASKS, lambda: plugin._schedule_continuity_retry(path)),
        ]
        with patch.object(plugin.asyncio, "get_running_loop", return_value=SimpleNamespace(create_task=create_task)), \
             patch.object(plugin, "_persist_continuity", return_value=path):
            for tasks, submit in cases:
                with self.subTest(submit=submit):
                    tasks.clear()
                    try:
                        submit()
                        previous = tasks[str(path)]
                        previous.completed = True
                        submit()
                        replacement = tasks[str(path)]
                        self.assertIsNot(previous, replacement)
                        for callback in previous.callbacks:
                            callback(previous)
                        self.assertIs(tasks.get(str(path)), replacement)
                        for callback in replacement.callbacks:
                            callback(replacement)
                        self.assertNotIn(str(path), tasks)
                    finally:
                        tasks.clear()

    async def test_initial_research_hook_keeps_gateway_loop_responsive(self):
        import threading
        release = threading.Event()
        finished = threading.Event()
        gateway = _Gateway()
        source = SimpleNamespace(platform="discord", chat_id="private-chat", thread_id=None,
                                 chat_type="dm", user_id="operator", message_id="hook-research")
        event = SimpleNamespace(source=source, text="arda research Example Domain",
                                user_id="operator", message_id="hook-research", media_urls=[], media_types=[])
        def delayed_post(_payload):
            release.wait(1)
            finished.set()
            return True, "Research delivered with evidence."
        with patch.object(plugin, "_post", side_effect=delayed_post), \
             patch.object(plugin, "_payload", return_value=_payload("hook-research")), \
             patch.object(plugin, "_continuity_payload", return_value={}), \
             patch.object(plugin, "_submit_continuity"), \
             patch.object(plugin, "_schedule_continuity_backlog"), \
             patch.object(plugin, "_ensure_reminder_loop"):
            try:
                self.assertEqual(plugin._pre_gateway_dispatch(event, gateway), {"action":"skip"})
                self.assertFalse(finished.is_set(), "hook waited for blocking HTTP submission")
                await asyncio.sleep(0.01)
                self.assertFalse(finished.is_set())
                self.assertEqual(len(plugin._RETRY_TASKS), 1)
                self.assertTrue(plugin._pending_path(_payload("hook-research")).exists())
            finally:
                release.set()
                await asyncio.gather(*list(plugin._RETRY_TASKS.values()))
        self.assertIn("Research delivered with evidence.", gateway.notices)

    async def asyncSetUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.env = patch.dict(
            os.environ,
            {
                "HERMES_HOME": self.temp.name,
                "ARDA_HERMES_GATEWAY_CAPABILITY": "test-gateway-capability",
                "ARDA_OPERATOR_ID": "operator-1",
            },
        )
        self.env.start()
        plugin._RETRY_DELAYS = (0.0,)
        plugin._RETRY_TASKS.clear()
        plugin._COMMAND_LOCK = None

    async def asyncTearDown(self):
        if plugin._RETRY_TASKS:
            await asyncio.gather(*plugin._RETRY_TASKS.values(), return_exceptions=True)
        self.env.stop()
        self.temp.cleanup()


    async def test_outbox_identity_includes_every_delivery_scope_component(self):
        import copy
        original = _payload("same-id")
        paths = [plugin._pending_path(original)]
        for keys, value in [
            (("operator", "operator_id"), "other-operator"),
            (("adapter_id",), "other-adapter"),
            (("event", "source", "platform"), "telegram"),
            (("event", "source", "chat_id"), "other-room"),
            (("event", "source", "thread_id"), "other-thread"),
            (("event", "source", "chat_type"), "group"),
            (("event", "message_id"), "other-message"),
        ]:
            payload = copy.deepcopy(original)
            target = payload
            for key in keys[:-1]:
                target = target[key]
            target[keys[-1]] = value
            paths.append(plugin._pending_path(payload))
        self.assertEqual(len(set(paths)), len(paths))

    async def test_retained_private_response_cannot_follow_colliding_shared_message(self):
        import copy
        import hashlib
        private = _payload("same-id")
        private["_bridge_result"] = "SYNTHETIC PRIVATE CONTEXT"
        legacy = plugin._pending_root() / (hashlib.sha256(b"same-id").hexdigest() + ".json")
        plugin._write_pending(legacy, private)
        shared = copy.deepcopy(private)
        shared.pop("_bridge_result")
        shared["event"]["source"].update(chat_id="shared-chat", chat_type="group")
        source = SimpleNamespace(platform="discord", chat_id="shared-chat", thread_id=None, chat_type="group")
        gateway = _Gateway()
        with patch.object(plugin, "_post", return_value=(True, "Shared status")) as post:
            # The destination check also protects entries written before scoped keys.
            self.assertFalse(await plugin._attempt_pending(legacy, private, gateway, source))
            post.assert_not_called()
            self.assertEqual(gateway.notices, [])
            self.assertTrue(legacy.exists())
            plugin._submit(shared, gateway, source)
            await asyncio.gather(*list(plugin._RETRY_TASKS.values()))
        self.assertEqual(gateway.notices, ["Shared status"])
        self.assertTrue(legacy.exists())
        self.assertEqual(plugin._persist_pending(_payload("same-id")), legacy)
        with patch.object(plugin, "_post") as post:
            self.assertTrue(await plugin._attempt_pending(legacy, private, gateway, _SOURCE))
            post.assert_not_called()
        self.assertEqual(gateway.notices[-1], "SYNTHETIC PRIVATE CONTEXT")

    async def test_pending_destination_is_rechecked_after_backend_await(self):
        source = SimpleNamespace(**vars(_SOURCE))
        gateway = _Gateway()
        payload = _payload("source-changed")
        path = plugin._persist_pending(payload)
        def post(_payload):
            source.chat_id = "shared-chat"
            source.chat_type = "group"
            return True, "SYNTHETIC PRIVATE CONTEXT"
        with patch.object(plugin, "_post", side_effect=post):
            self.assertFalse(await plugin._attempt_pending(path, payload, gateway, source))
        self.assertEqual(gateway.notices, [])
        self.assertTrue(path.exists())
        with patch.object(plugin, "_post") as repost:
            self.assertTrue(await plugin._attempt_pending(path, payload, gateway, _SOURCE))
            repost.assert_not_called()
        self.assertEqual(gateway.notices, ["SYNTHETIC PRIVATE CONTEXT"])

    async def test_response_survives_delivery_failure_and_restart_without_repost(self):
        for failure in (None, SimpleNamespace(success=False, message_id=None), RuntimeError("offline")):
            with self.subTest(failure=failure):
                gateway = _Gateway()
                gateway.send = AsyncMock(side_effect=failure) if isinstance(failure, Exception) else AsyncMock(return_value=failure)
                payload = _payload("failed-delivery")
                summary = "Audit result\nEvidence:\narda://evidence/original"
                with patch.object(plugin, "_post", return_value=(True, summary)) as post:
                    plugin._submit(payload, gateway, _SOURCE)
                    await asyncio.gather(*list(plugin._RETRY_TASKS.values()))
                    await asyncio.sleep(0)
                    path = plugin._pending_path(payload)
                    self.assertEqual(plugin._load_pending(path)["_bridge_result"], summary)
                    # Duplicate dispatch must not clobber the result on disk.
                    plugin._persist_pending(payload)
                    self.assertEqual(plugin._load_pending(path)["_bridge_result"], summary)
                    plugin._RETRY_TASKS.clear()
                    recovered = _Gateway()
                    plugin._schedule_backlog(recovered, _SOURCE)
                    await asyncio.gather(*list(plugin._RETRY_TASKS.values()))
                    await asyncio.sleep(0)
                    post.assert_called_once()
                    self.assertEqual(recovered.notices, [summary])
                    self.assertFalse(path.exists())

    async def test_missing_adapter_retains_response(self):
        gateway = _Gateway()
        gateway._adapter_for_source = lambda _source: None
        with patch.object(plugin, "_post", return_value=(True, "saved")) as post:
            plugin._submit(_payload(), gateway, _SOURCE)
            await asyncio.gather(*list(plugin._RETRY_TASKS.values()))
        post.assert_called_once()
        self.assertEqual(plugin._load_pending(plugin._pending_path(_payload()))["_bridge_result"], "saved")

    async def test_private_delivery_never_falls_back_to_public(self):
        gateway = _Gateway()
        gateway.config = SimpleNamespace(get_notice_delivery=lambda _: "private")
        gateway.send_private_notice = AsyncMock(return_value=SimpleNamespace(success=False))
        source = SimpleNamespace(platform="discord", chat_id="channel", thread_id=None, user_id="operator")
        self.assertFalse(await plugin._deliver_result(gateway, source, "private result"))
        self.assertEqual(gateway.notices, [])

    async def test_private_policy_with_public_fallback_adapter_fails_closed(self):
        gateway = _Gateway()
        gateway.config = SimpleNamespace(get_notice_delivery=lambda _: "public")
        # The adapter-level setting takes precedence, including normalization.
        adapter = SimpleNamespace(config=SimpleNamespace(extra={"notice_delivery": " private "}))
        adapter.send = AsyncMock(return_value=SimpleNamespace(success=True, message_id="receipt"))
        async def inherited_private_notice(chat_id, user_id, text, metadata=None):
            return await adapter.send(chat_id, text, metadata=metadata)
        adapter.send_private_notice = inherited_private_notice
        gateway._adapter_for_source = lambda _: adapter
        gateway._thread_metadata_for_source = lambda _: {"thread_id": "public-thread"}
        source = SimpleNamespace(platform="discord", chat_id="public-channel", chat_type="group", user_id="operator")
        self.assertFalse(await plugin._deliver_result(gateway, source, "sensitive result"))
        adapter.send.assert_not_called()
        source.chat_type = "dm"
        source.chat_id = "private-chat"
        self.assertTrue(await plugin._deliver_result(gateway, source, "sensitive result"))
        adapter.send.assert_awaited_once_with("private-chat", "sensitive result", metadata={})

    async def test_distinct_initial_requests_are_serialized_without_blocking_hook(self):
        import threading
        release = threading.Event()
        entered = threading.Event()
        order = []
        def post(payload):
            identity = payload["event"]["message_id"]
            order.append(identity)
            if identity == "first":
                entered.set()
                release.wait(2)
            return True, identity
        with patch.object(plugin, "_post", side_effect=post):
            plugin._submit(_payload("first"), _Gateway(), _SOURCE)
            self.assertTrue(await asyncio.to_thread(entered.wait, 1))
            plugin._submit(_payload("second"), _Gateway(), _SOURCE)
            await asyncio.sleep(0.02)
            try:
                self.assertEqual(order, ["first"])
            finally:
                release.set()
                await asyncio.gather(*list(plugin._RETRY_TASKS.values()))
        self.assertEqual(order, ["first", "second"])

    async def test_gateway_unavailable_is_persisted_then_retried(self):
        # Initial submission is owned by the same task table as retries.
        gateway = _Gateway()
        with patch.object(
            plugin,
            "urlopen",
            side_effect=[URLError("offline"), _Response('{"summary":"Run status recovered."}')],
        ) as mocked:
            summary = plugin._submit(_payload(), gateway, _SOURCE)
            self.assertIn("Hermes queued this request locally", summary)
            self.assertIn("Arda admission and response delivery are not yet confirmed", summary)
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
            await asyncio.gather(*list(plugin._RETRY_TASKS.values()))
            await asyncio.sleep(0)

        self.assertIn("Arda admission and response delivery are not yet confirmed", summary)
        self.assertEqual(gateway.notices, ["Arda already accepted this command."])
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

    async def test_project_intake_crosses_authenticated_gateway_surfaces(self):
        project_id = "550e8400-e29b-41d4-a716-446655440000"
        for platform in ("discord", "telegram"):
            for chat_type in ("dm", "group", "thread"):
                with self.subTest(platform=platform, chat_type=chat_type):
                    event = self.event(chat_type=chat_type,
                        text=f"For project {project_id}, objective inspect routing")
                    event.source.platform = platform
                    with (
                        patch.object(plugin, "_submit", return_value="Recorded.") as submit,
                        patch.object(plugin, "_submit_continuity"),
                    ):
                        result = plugin._pre_gateway_dispatch(event, _Gateway(), session_store=self.session_store())
                    self.assertEqual(result, {"action": "skip"})
                    payload = submit.call_args.args[0]
                    self.assertEqual(payload["event"]["source"]["chat_type"], chat_type)
                    self.assertEqual(payload["adapter_id"], f"hermes-{platform}-default")

    async def test_shared_project_intake_requires_authentication_and_known_audience(self):
        for chat_type in ("group", "guild", "channel", "thread"):
            event = self.event(chat_type=chat_type, text="Show work in this conversation")
            with patch.object(plugin, "_submit", return_value="Conversation status") as submit, patch.object(plugin, "_submit_continuity"):
                result = plugin._pre_gateway_dispatch(event, _Gateway(), session_store=self.session_store())
            self.assertEqual(result, {"action": "skip"})
            self.assertEqual(submit.call_args.args[0]["event"]["text"], "arda objectives")
        text = "For project 550e8400-e29b-41d4-a716-446655440000, objective inspect routing"
        for chat_type, authorized in (("group", False), ("unknown", True)):
            gateway = _Gateway()
            with (
                patch.object(gateway, "_is_user_authorized", return_value=authorized),
                patch.object(plugin, "_submit") as submit,
                patch.object(plugin, "_submit_continuity"),
            ):
                result = plugin._pre_gateway_dispatch(self.event(chat_type=chat_type, text=text), gateway, session_store=self.session_store())
            self.assertIsNone(result)
            submit.assert_not_called()

    async def test_named_project_intake_does_not_require_a_uuid(self):
        event = self.event(chat_type="group", text="For project Routing review, objective inspect routing")
        with (
            patch.object(plugin, "_submit", return_value="Recorded.") as submit,
            patch.object(plugin, "_submit_continuity"),
        ):
            result = plugin._pre_gateway_dispatch(event, _Gateway(), session_store=self.session_store())
        self.assertEqual(result, {"action": "skip"})
        self.assertEqual(submit.call_args.args[0]["event"]["text"], 'arda objective "Routing review" inspect routing')

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
