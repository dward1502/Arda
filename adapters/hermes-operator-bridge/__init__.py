"""Authenticated Hermes Discord handoff to Arda's loopback harness."""

from __future__ import annotations

import asyncio
import hashlib
import json
import logging
import os
from datetime import datetime, timedelta, timezone
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

_LOG = logging.getLogger(__name__)
_ENDPOINT = "http://127.0.0.1:7878/v1/operator/messages"
_CONTINUITY_ENDPOINT = "http://127.0.0.1:7878/v1/continuity/events"
_PREFIX = "arda "
_RETRY_DELAYS = (1.0, 2.0, 5.0, 10.0, 30.0)
_RETRY_TASKS: dict[str, asyncio.Task] = {}
_CONTINUITY_RETRY_TASKS: dict[str, asyncio.Task] = {}


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


def _continuity_pending_root() -> Path:
    hermes_home = Path(os.environ.get("HERMES_HOME", "~/.hermes")).expanduser()
    return hermes_home / "state" / "arda-operator-bridge" / "continuity-pending"


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


def _continuity_payload(event, session_store) -> dict:
    if session_store is None:
        raise ValueError("Hermes session store is unavailable")
    source = event.source
    operator_id = str(event.user_id or source.user_id or "")
    message_id = str(event.message_id or source.message_id or "")
    if not operator_id or not message_id or not source.chat_id:
        raise ValueError("continuity source identity is incomplete")
    entry = session_store.get_or_create_session(source, touch_activity=False)
    session_id = str(getattr(entry, "session_id", "") or "")
    if not session_id:
        raise ValueError("Hermes session identity is unavailable")
    platform = str(_value(source.platform))
    thread_id = str(source.thread_id) if source.thread_id else None
    surface_parts = [platform, str(source.chat_id)]
    if thread_id:
        surface_parts.append(thread_id)
    surface_id = ":".join(surface_parts)
    chat_type = str(source.chat_type or "unknown").lower()
    privacy_class = "personal_device" if chat_type in {"dm", "private"} else "shared_room"
    observed_at = datetime.now(timezone.utc)
    replay_material = f"{message_id}\0{session_id}\0{surface_id}"
    replay_digest = hashlib.sha256(replay_material.encode("utf-8")).hexdigest()
    return {
        "operator": {
            "operator_id": operator_id,
            "authenticated": True,
            "authentication_method": "gateway_identity",
            "authenticated_at": observed_at.isoformat(),
        },
        "event": {
            "schema_version": "arda.continuity-event.v1",
            "event_id": f"hermes:{message_id}",
            "session_lineage_id": session_id,
            "current_session_id": session_id,
            "surface_id": surface_id,
            "platform": platform,
            "chat_id": str(source.chat_id),
            "thread_id": thread_id,
            "privacy_class": privacy_class,
            "authorized_domains": ["system"],
            "requested_domains": ["system"],
            "topic_refs": [],
            "commitment_refs": [],
            "memory_scope_refs": ["vaire:scope:system-continuity"],
            "observed_at": observed_at.isoformat(),
            "expires_at": (observed_at + timedelta(minutes=15)).isoformat(),
            "idempotency_key": f"sha256:{replay_digest}",
        },
    }


def _continuity_pending_path(payload: dict) -> Path:
    key = str(payload["event"]["idempotency_key"])
    digest = hashlib.sha256(key.encode("utf-8")).hexdigest()
    return _continuity_pending_root() / f"{digest}.json"


def _persist_continuity(payload: dict) -> Path:
    path = _continuity_pending_path(payload)
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(payload, separators=(",", ":")), encoding="utf-8")
    temporary.chmod(0o600)
    temporary.replace(path)
    return path


def _post_continuity(payload: dict) -> bool:
    request = Request(
        _CONTINUITY_ENDPOINT,
        data=json.dumps(payload, separators=(",", ":")).encode("utf-8"),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urlopen(request, timeout=3.0):
            return True
    except HTTPError as error:
        if error.code == 409:
            return True
        if error.code >= 500:
            _LOG.warning("Arda continuity handoff returned %s", error.code)
            return False
        _LOG.warning("Arda rejected continuity metadata with %s", error.code)
        return True
    except (URLError, OSError) as error:
        _LOG.warning("Arda continuity handoff unavailable: %s", error)
        return False


async def _retry_continuity(path: Path) -> None:
    for delay in _RETRY_DELAYS:
        await asyncio.sleep(delay)
        payload = _load_pending(path)
        if payload is None:
            return
        if await asyncio.to_thread(_post_continuity, payload):
            path.unlink(missing_ok=True)
            return
    _LOG.warning("Arda continuity event remains queued after bounded retries: %s", path.name)


async def _deliver_continuity(path: Path, payload: dict) -> None:
    if await asyncio.to_thread(_post_continuity, payload):
        path.unlink(missing_ok=True)
        return
    await _retry_continuity(path)


def _schedule_continuity_retry(path: Path) -> None:
    key = str(path)
    existing = _CONTINUITY_RETRY_TASKS.get(key)
    if existing is not None and not existing.done():
        return
    task = asyncio.get_running_loop().create_task(
        _retry_continuity(path), name=f"arda-continuity-retry-{path.stem[:12]}"
    )
    _CONTINUITY_RETRY_TASKS[key] = task
    task.add_done_callback(lambda _task: _CONTINUITY_RETRY_TASKS.pop(key, None))


def _schedule_continuity_backlog() -> None:
    root = _continuity_pending_root()
    if not root.is_dir():
        return
    for path in root.glob("*.json"):
        _schedule_continuity_retry(path)


def _submit_continuity(payload: dict, _gateway, _source) -> None:
    path = _persist_continuity(payload)
    key = str(path)
    existing = _CONTINUITY_RETRY_TASKS.get(key)
    if existing is not None and not existing.done():
        return
    task = asyncio.get_running_loop().create_task(
        _deliver_continuity(path, payload),
        name=f"arda-continuity-delivery-{path.stem[:12]}",
    )
    _CONTINUITY_RETRY_TASKS[key] = task
    task.add_done_callback(lambda _task: _CONTINUITY_RETRY_TASKS.pop(key, None))


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
    if source is None:
        return None
    platform = str(_value(source.platform))
    is_arda_command = platform == "discord" and text.strip().lower().startswith(_PREFIX)
    try:
        if not gateway._is_user_authorized(source):
            return None
    except Exception:
        _LOG.exception("Hermes authorization check failed for Arda command")
        return None

    if not (event.user_id or source.user_id) or not (event.message_id or source.message_id):
        if not is_arda_command:
            return None
        _reply(gateway, source, "Arda rejected the command: missing authenticated message identity.")
        return {"action": "skip"}
    try:
        continuity = _continuity_payload(event, _hook_context.get("session_store"))
        _submit_continuity(continuity, gateway, source)
        _schedule_continuity_backlog()
    except Exception:
        _LOG.exception("Hermes continuity metadata emission failed open")
    if not is_arda_command:
        return None
    if event.media_urls or event.media_types:
        _reply(gateway, source, "Arda operator commands do not accept attachments.")
        return {"action": "skip"}

    _reply(gateway, source, _submit(_payload(event), gateway, source))
    _schedule_backlog(gateway, source)
    return {"action": "skip"}


def register(ctx):
    ctx.register_hook("pre_gateway_dispatch", _pre_gateway_dispatch)
