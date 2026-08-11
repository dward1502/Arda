"""Authenticated Hermes Discord handoff to Arda's loopback harness."""

from __future__ import annotations

import asyncio
import hashlib
import json
import logging
import os
from datetime import datetime, timezone
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

_LOG = logging.getLogger(__name__)
_ENDPOINT = "http://127.0.0.1:7878/v1/operator/messages"
_PREFIX = "arda "
_RETRY_DELAYS = (1.0, 2.0, 5.0, 10.0, 30.0)
_RETRY_TASKS: dict[str, asyncio.Task] = {}


def _value(value):
    return getattr(value, "value", value)


def _iso_timestamp(value) -> str:
    if isinstance(value, datetime):
        if value.tzinfo is None:
            value = value.replace(tzinfo=timezone.utc)
        return value.astimezone(timezone.utc).isoformat()
    return str(value)


def _payload(event) -> dict:
    source = event.source
    operator_id = str(event.user_id or source.user_id or "")
    message_id = str(event.message_id or source.message_id or "")
    timestamp = _iso_timestamp(event.timestamp)
    return {
        "operator": {
            "operator_id": operator_id,
            "authenticated": True,
            "authentication_method": "gateway_identity",
            "authenticated_at": datetime.now(timezone.utc).isoformat(),
        },
        "adapter_id": "hermes-discord-default",
        "event": {
            "text": event.text,
            "message_type": str(_value(event.message_type)),
            "user_id": operator_id,
            "user_name": event.user_name or source.user_name,
            "source": {
                "platform": str(_value(source.platform)),
                "chat_id": str(source.chat_id),
                "chat_type": str(source.chat_type or "unknown"),
                "thread_id": str(source.thread_id) if source.thread_id else None,
                "message_id": message_id,
            },
            "message_id": message_id,
            "media_urls": list(event.media_urls or []),
            "media_types": list(event.media_types or []),
            "timestamp": timestamp,
            "prompt_response": event.prompt_response,
        },
    }


def _post(payload: dict) -> tuple[bool, str]:
    request = Request(
        _ENDPOINT,
        data=json.dumps(payload, separators=(",", ":")).encode("utf-8"),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urlopen(request, timeout=3.0) as response:
            body = json.loads(response.read().decode("utf-8"))
            return True, str(body.get("summary") or "Arda command completed.")
    except HTTPError as error:
        try:
            body = json.loads(error.read().decode("utf-8"))
            detail = body.get("error") or body.get("message")
        except Exception:
            detail = None
        if error.code >= 500:
            _LOG.warning("Arda operator handoff returned %s", error.code)
            return False, "Arda is unavailable; command queued for retry."
        if error.code == 409 and detail and "duplicate transport event" in detail:
            return True, "Arda already accepted this command."
        return True, f"Arda rejected the command: {detail or error.reason}."
    except URLError as error:
        _LOG.warning("Arda operator handoff unavailable: %s", error)
        return False, "Arda is unavailable; command queued for retry."
    except Exception as error:
        _LOG.exception("Arda operator handoff failed")
        return False, f"Arda command queued after {type(error).__name__}."


def _pending_root() -> Path:
    hermes_home = Path(os.environ.get("HERMES_HOME", "~/.hermes")).expanduser()
    return hermes_home / "state" / "arda-operator-bridge" / "pending"


def _pending_path(payload: dict) -> Path:
    message_id = str(payload["event"]["message_id"])
    digest = hashlib.sha256(message_id.encode("utf-8")).hexdigest()
    return _pending_root() / f"{digest}.json"


def _persist_pending(payload: dict) -> Path:
    path = _pending_path(payload)
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(payload, separators=(",", ":")), encoding="utf-8")
    temporary.chmod(0o600)
    temporary.replace(path)
    return path


def _load_pending(path: Path) -> dict | None:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        _LOG.exception("Discarding unreadable Arda pending command at %s", path)
        path.unlink(missing_ok=True)
        return None


async def _retry_pending(path: Path, gateway, source) -> None:
    for delay in _RETRY_DELAYS:
        await asyncio.sleep(delay)
        payload = _load_pending(path)
        if payload is None:
            return
        terminal, summary = await asyncio.to_thread(_post, payload)
        if terminal:
            path.unlink(missing_ok=True)
            _reply(gateway, source, summary)
            return
    _LOG.warning("Arda command remains queued after bounded retries: %s", path.name)


def _schedule_retry(path: Path, gateway, source) -> None:
    key = str(path)
    existing = _RETRY_TASKS.get(key)
    if existing is not None and not existing.done():
        return
    task = asyncio.get_running_loop().create_task(
        _retry_pending(path, gateway, source),
        name=f"arda-operator-retry-{path.stem[:12]}",
    )
    _RETRY_TASKS[key] = task
    task.add_done_callback(lambda _task: _RETRY_TASKS.pop(key, None))


def _same_destination(payload: dict, source) -> bool:
    pending = payload.get("event", {}).get("source", {})
    return (
        str(pending.get("platform")) == str(_value(source.platform))
        and str(pending.get("chat_id")) == str(source.chat_id)
        and str(pending.get("thread_id") or "") == str(source.thread_id or "")
    )


def _schedule_backlog(gateway, source) -> None:
    root = _pending_root()
    if not root.is_dir():
        return
    for path in root.glob("*.json"):
        payload = _load_pending(path)
        if payload is not None and _same_destination(payload, source):
            _schedule_retry(path, gateway, source)


def _submit(payload: dict, gateway, source) -> str:
    path = _persist_pending(payload)
    terminal, summary = _post(payload)
    if terminal:
        path.unlink(missing_ok=True)
    else:
        _schedule_retry(path, gateway, source)
    return summary


def _reply(gateway, source, text: str) -> None:
    asyncio.get_running_loop().create_task(
        gateway._deliver_platform_notice(source, text),
        name="arda-operator-reply",
    )


def _pre_gateway_dispatch(event, gateway, **_hook_context):
    # Hook dispatch adds shared observer metadata (for example
    # ``telemetry_schema_version``) in addition to hook-specific fields such as
    # ``session_store``. Accept the forward-compatible context so an added
    # observer field cannot fail open into the normal agent path.
    source = getattr(event, "source", None)
    text = str(getattr(event, "text", "") or "")
    if source is None or str(_value(source.platform)) != "discord":
        return None
    if not text.strip().lower().startswith(_PREFIX):
        return None
    try:
        if not gateway._is_user_authorized(source):
            return None
    except Exception:
        _LOG.exception("Hermes authorization check failed for Arda command")
        return None

    if not (event.user_id or source.user_id) or not (event.message_id or source.message_id):
        _reply(gateway, source, "Arda rejected the command: missing authenticated message identity.")
        return {"action": "skip"}
    if event.media_urls or event.media_types:
        _reply(gateway, source, "Arda operator commands do not accept attachments.")
        return {"action": "skip"}

    _reply(gateway, source, _submit(_payload(event), gateway, source))
    _schedule_backlog(gateway, source)
    return {"action": "skip"}


def register(ctx):
    ctx.register_hook("pre_gateway_dispatch", _pre_gateway_dispatch)
