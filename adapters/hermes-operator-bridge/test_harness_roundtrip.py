"""Called by the Rust harness test; only the platform delivery is a fixture."""

import asyncio
import json
import re
import sys
from urllib.parse import urlparse

from test_plugin import ContinuityEventTests, _Gateway, plugin


async def main():
    base = sys.argv[1]
    assert urlparse(base).hostname == "127.0.0.1"
    plugin._ENDPOINT = base + "/v1/operator/messages"
    plugin._CONTINUITY_ENDPOINT = base + "/v1/continuity/events"
    fixture = ContinuityEventTests()
    gateway = _Gateway()

    async def send(text, message_id):
        gateway.notices.clear()
        event = fixture.event(chat_type="thread", thread_id="roundtrip-thread",
                              message_id=message_id, text=text)
        event.source.platform = "telegram"
        result = plugin._pre_gateway_dispatch(
            event, gateway, session_store=fixture.session_store())
        assert result == {"action": "skip"}, result
        await asyncio.gather(*list(plugin._RETRY_TASKS.values()),
                             *list(plugin._CONTINUITY_RETRY_TASKS.values()))
        await asyncio.sleep(0)
        return "\n".join(gateway.notices)

    admitted = await send("For project Routing review, objective Roundtrip inspection", "roundtrip-intake")
    assert "execution still requires review" in admitted, admitted
    assert "queued" in admitted and "not yet confirmed" in admitted, admitted
    status = await send("Show work in this conversation", "roundtrip-status")
    assert "Objectives in this conversation: 1" in status, status
    assert "pending_approval" in status, status
    assert "Roundtrip inspection" not in status, status
    controls = re.search(r"Controls: (.+)", status)
    assert controls, status
    for index, command in enumerate(controls.group(1).split("; ")):
        result = await send(command.replace("<reason>", "roundtrip operator request"), f"roundtrip-control-{index}")
        assert ("Paused", "Resumed", "Cancelled")[index] + " resident objective" in result, result
    assert not list((plugin._state_root() / "pending").glob("*.json"))
    print(json.dumps({"admitted": True, "scoped_status": True, "controls": 3,
                      "delivery": "fixture", "http_and_storage": "real"}))


if __name__ == "__main__":
    asyncio.run(asyncio.wait_for(main(), timeout=15))
