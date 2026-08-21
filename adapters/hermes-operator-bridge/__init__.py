"""Authenticated Hermes Discord handoff to Arda's loopback harness."""

from __future__ import annotations

import asyncio
import hashlib
import json
import logging
import os
import re
import uuid
from datetime import datetime, timedelta, timezone
from pathlib import Path
from zoneinfo import ZoneInfo
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

_LOG = logging.getLogger(__name__)
_ENDPOINT = "http://127.0.0.1:7878/v1/operator/messages"
_CONTINUITY_ENDPOINT = "http://127.0.0.1:7878/v1/continuity/events"
_PREFIX = "arda "
_RETRY_DELAYS = (1.0, 2.0, 5.0, 10.0, 30.0)
_RETRY_TASKS: dict[str, asyncio.Task] = {}
_CONTINUITY_RETRY_TASKS: dict[str, asyncio.Task] = {}
_REMINDER_TASK: asyncio.Task | None = None
_REMINDER_POLL_SECONDS = 30.0
_REMINDER_NAMESPACE = uuid.UUID("a6f05f68-81dc-4a14-9f1e-4dd8d5efc721")

_PROJECT_OBJECTIVE = re.compile(
    r"^for\s+project\s+([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})\s*,?\s*(?:objective\s+)?(.+)$",
    re.IGNORECASE,
)
_CAPTURE = re.compile(
    r"^(?:remember\s+that|save\s+this\s+thought\s*:|note\s+that|capture\s+this\s*:?)\s+(.+)$",
    re.IGNORECASE,
)
_REMINDER = re.compile(r"^remind\s+me\s+to\s+(.+)$", re.IGNORECASE)
_RESEARCH = re.compile(
    r"^(?:research|look\s+into|find\s+out\s+about)\s+(.+)$", re.IGNORECASE
)
_CONSEQUENTIAL = re.compile(
    r"^(?:deploy|publish|send|buy|purchase|pay|transfer|trade|delete|remove|message|email)\b",
    re.IGNORECASE,
)
_CONTEXT_REQUESTS = {
    "what should i work on next",
    "what should i do next",
    "what's next",
    "whats next",
    "what was i doing",
    "where did i leave off",
    "resume my context",
}


def _value(value):
    return getattr(value, "value", value)


def _iso_timestamp(value) -> str:
    if isinstance(value, datetime):
        if value.tzinfo is None:
            value = value.replace(tzinfo=timezone.utc)
        return value.astimezone(timezone.utc).isoformat()
    return str(value)


def _utcnow() -> datetime:
    return datetime.now(timezone.utc)


def _canonical_operator_id() -> str:
    operator_id = os.environ.get("ARDA_OPERATOR_ID", "operator:mythos").strip()
    if not operator_id:
        raise ValueError("ARDA_OPERATOR_ID cannot be empty")
    return operator_id


def _state_root() -> Path:
    hermes_home = Path(os.environ.get("HERMES_HOME", "~/.hermes")).expanduser()
    return hermes_home / "state" / "arda-operator-bridge"


def _request_json(path, *, method="GET", payload=None, idempotency_key=None):
    body = None if payload is None else json.dumps(payload).encode("utf-8")
    headers = {
        "Accept": "application/json",
        "x-arda-operator-id": _canonical_operator_id(),
    }
    if body is not None:
        headers["Content-Type"] = "application/json"
    if idempotency_key:
        headers["Idempotency-Key"] = idempotency_key
    request = Request(
        f"http://127.0.0.1:7878{path}",
        data=body,
        headers=headers,
        method=method,
    )
    with urlopen(request, timeout=5.0) as response:
        return json.loads(response.read().decode("utf-8"))


def _reminder_state_path() -> Path:
    return _state_root() / "reminder-attempts.json"


def _load_reminder_attempt_times() -> dict[str, str]:
    try:
        value = json.loads(_reminder_state_path().read_text(encoding="utf-8"))
        return value if isinstance(value, dict) else {}
    except (OSError, ValueError):
        return {}


def _store_reminder_attempt_times(value: dict[str, str]) -> None:
    path = _reminder_state_path()
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(".tmp")
    tmp.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")
    os.chmod(tmp, 0o600)
    tmp.replace(path)


def _parse_time(value) -> datetime | None:
    if not value:
        return None
    try:
        parsed = datetime.fromisoformat(str(value).replace("Z", "+00:00"))
        return parsed if parsed.tzinfo else parsed.replace(tzinfo=timezone.utc)
    except ValueError:
        return None


def _quiet_window_active(window, now: datetime) -> bool:
    if not isinstance(window, dict):
        return False
    try:
        local = now.astimezone(ZoneInfo(window["timezone"]))
        start_h, start_m = map(int, window["start"].split(":"))
        end_h, end_m = map(int, window["end"].split(":"))
    except (KeyError, TypeError, ValueError):
        return True
    current = local.hour * 60 + local.minute
    start = start_h * 60 + start_m
    end = end_h * 60 + end_m
    return start <= current < end if start < end else current >= start or current < end


def _due_reminder(item, reminder_config, now: datetime):
    scheduled = _parse_time(item.get("scheduled_at") or item.get("due_at"))
    if scheduled is None or scheduled > now:
        return None
    state = item.get("reminder_state") or {}
    if str(state.get("delivery_state", "")).lower() in {
        "delivered",
        "acknowledged",
        "deferred",
        "dismissed",
    }:
        return None
    attempts = int(item.get("reminder_attempts") or 0)
    if attempts >= int(reminder_config.get("max_attempts") or 3):
        return None
    if _quiet_window_active(reminder_config.get("quiet_window"), now):
        return None
    reminder_id = item.get("reminder_id") or str(
        uuid.uuid5(_REMINDER_NAMESPACE, str(item["item_id"]))
    )
    last = _parse_time(_load_reminder_attempt_times().get(reminder_id))
    interval = int(reminder_config.get("minimum_interval_minutes") or 15)
    if last is not None and now - last < timedelta(minutes=max(1, interval)):
        return None
    return reminder_id


async def _deliver_due_reminders_once(gateway, source) -> None:
    if str(getattr(source, "chat_type", "") or "").lower() not in {"dm", "private"}:
        return
    capabilities, brief = await asyncio.gather(
        asyncio.to_thread(_request_json, "/v1/personal/capabilities"),
        asyncio.to_thread(_request_json, "/v1/personal/briefs/today"),
    )
    reminder_config = capabilities.get("reminders") or {}
    if reminder_config.get("state") != "configured" or reminder_config.get(
        "adapter"
    ) != "discord_dm":
        return
    adapter = gateway._adapter_for_source(source)
    if adapter is None:
        return
    now = _utcnow()
    for item in (brief.get("brief") or {}).get("today") or []:
        reminder_id = _due_reminder(item, reminder_config, now)
        if reminder_id is None:
            continue
        result = None
        try:
            result = await adapter.send(
                source.chat_id,
                f"Arda reminder: {item.get('content', '').strip()}\n\nOpen Personal Operations to acknowledge, defer, dismiss, or complete it.",
                metadata=gateway._thread_metadata_for_source(source),
            )
        except Exception:
            _LOG.exception("Arda reminder delivery failed")
        provider_message_id = (
            str(result.message_id)
            if result is not None
            and getattr(result, "success", False)
            and getattr(result, "message_id", None)
            else None
        )
        attempt_times = _load_reminder_attempt_times()
        attempt_times[reminder_id] = now.isoformat()
        _store_reminder_attempt_times(attempt_times)
        payload = {
            "operator_id": _canonical_operator_id(),
            "reminder_id": reminder_id,
            "item_id": str(item["item_id"]),
            "state": "delivered" if provider_message_id else "attempted",
            "provider_message_id": provider_message_id,
        }
        await asyncio.to_thread(
            _request_json,
            "/v1/personal/reminders/attempt",
            method="POST",
            payload=payload,
            idempotency_key=f"discord-reminder:{reminder_id}:{int(item.get('reminder_attempts') or 0) + 1}",
        )


async def _reminder_loop(gateway, source) -> None:
    while True:
        try:
            await _deliver_due_reminders_once(gateway, source)
        except asyncio.CancelledError:
            raise
        except Exception:
            _LOG.exception("Arda reminder poll failed")
        await asyncio.sleep(_REMINDER_POLL_SECONDS)


def _ensure_reminder_loop(gateway, source) -> None:
    global _REMINDER_TASK
    if os.environ.get("ARDA_REMINDER_TRANSPORT", "").strip().lower() != "discord_dm":
        return
    if _REMINDER_TASK is None or _REMINDER_TASK.done():
        _REMINDER_TASK = asyncio.create_task(_reminder_loop(gateway, source))


def _natural_intent(text: str, source) -> dict | None:
    """Classify only narrow private-language forms with deterministic authority."""
    if str(getattr(source, "chat_type", "") or "").lower() not in {"dm", "private"}:
        return None
    normalized = " ".join(text.strip().split())
    if not normalized:
        return None
    context_key = normalized.rstrip("?.!").lower()
    if context_key in _CONTEXT_REQUESTS:
        return {"kind": "forward", "command": "arda context"}
    if match := _PROJECT_OBJECTIVE.fullmatch(normalized):
        objective = match.group(2).strip()
        if objective:
            return {
                "kind": "forward",
                "command": f"arda objective {match.group(1).lower()} {objective}",
            }
    if match := _CAPTURE.fullmatch(normalized):
        return {"kind": "forward", "command": f"arda capture {match.group(1).strip()}"}
    if match := _REMINDER.fullmatch(normalized):
        return {
            "kind": "forward",
            "command": f"arda capture Reminder: {match.group(1).strip()}",
        }
    if match := _RESEARCH.fullmatch(normalized):
        return {"kind": "forward", "command": f"arda research {match.group(1).strip()}"}
    if _CONSEQUENTIAL.match(normalized):
        return {
            "kind": "clarify",
            "summary": (
                "That request may be consequential. Name the target or attached project and "
                "confirm whether you want a review-only proposal. Nothing was saved or executed."
            ),
        }
    return None


def _payload(event) -> dict:
    source = event.source
    operator_id = _canonical_operator_id()
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
    return _state_root() / "pending"


def _continuity_pending_root() -> Path:
    return _state_root() / "continuity-pending"


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
    canonical_operator = _canonical_operator_id()
    message_id = str(event.message_id or source.message_id or "")
    if not canonical_operator or not operator_id or not message_id or not source.chat_id:
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
            "operator_id": canonical_operator,
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
            "source_user_ref": operator_id,
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
    explicit_command = platform == "discord" and text.strip().lower().startswith(_PREFIX)
    natural_intent = (
        _natural_intent(text, source) if platform == "discord" and not explicit_command else None
    )
    is_arda_command = explicit_command or natural_intent is not None
    try:
        if not gateway._is_user_authorized(source):
            return None
    except Exception:
        _LOG.exception("Hermes authorization check failed for Arda command")
        return None

    if platform == "discord" and str(getattr(source, "chat_type", "") or "").lower() in {
        "dm",
        "private",
    }:
        _ensure_reminder_loop(gateway, source)

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
    if natural_intent and natural_intent["kind"] == "clarify":
        _reply(gateway, source, natural_intent["summary"])
        return {"action": "skip"}
    if event.media_urls or event.media_types:
        _reply(gateway, source, "Arda operator commands do not accept attachments.")
        return {"action": "skip"}

    payload = _payload(event)
    if natural_intent:
        payload["event"]["text"] = natural_intent["command"]
    _reply(gateway, source, _submit(payload, gateway, source))
    _schedule_backlog(gateway, source)
    return {"action": "skip"}


def register(ctx):
    ctx.register_hook("pre_gateway_dispatch", _pre_gateway_dispatch)
