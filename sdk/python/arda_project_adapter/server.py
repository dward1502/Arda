"""Offline JSON Lines over stdio reference server for project adapters."""

from __future__ import annotations

import hashlib
import json
import os
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable, TextIO

SCHEMA_VERSION = "arda.project-adapter.v1"
PROTOCOL_VERSION = "1"
MAX_LINE_BYTES = 64 * 1024

JsonObject = dict[str, Any]
Handler = Callable[[str, JsonObject, "AdapterContext"], Any]


class CancelledError(Exception):
    """Raised by a cooperative handler after cancellation."""


@dataclass
class AdapterContext:
    request_id: str
    recovery_token: str | None
    _cancelled: threading.Event
    _emit: Callable[[JsonObject], None]
    _sequence: int = 0

    @property
    def cancelled(self) -> bool:
        return self._cancelled.is_set()

    def check_cancelled(self) -> None:
        if self.cancelled:
            raise CancelledError("request cancelled")

    def progress(
        self,
        message: str,
        *,
        percent: float | None = None,
        detail: JsonObject | None = None,
    ) -> None:
        self.check_cancelled()
        self._sequence += 1
        frame: JsonObject = {
            "schema_version": SCHEMA_VERSION,
            "id": f"{self.request_id}:progress:{self._sequence}",
            "type": "progress",
            "request_id": self.request_id,
            "sequence": self._sequence,
            "message": message,
        }
        if percent is not None:
            frame["percent"] = percent
        if detail is not None:
            frame["detail"] = detail
        self._emit(frame)


class AdapterServer:
    """One-process reference server with cooperative request cancellation."""

    _FIELDS = {
        "initialize": {
            "schema_version",
            "id",
            "type",
            "protocol_version",
            "project_root",
            "allowed_capabilities",
        },
        "health": {"schema_version", "id", "type"},
        "request": {
            "schema_version",
            "id",
            "type",
            "operation",
            "arguments",
            "timeout_ms",
            "required_capabilities",
            "idempotency_key",
            "recovery_token",
        },
        "cancel": {"schema_version", "id", "type", "request_id"},
    }

    def __init__(
        self,
        capabilities: list[str],
        handler: Handler,
        *,
        name: str = "arda-python-reference",
        version: str = "1.0.0",
        recovery_supported: bool = True,
    ) -> None:
        self._available = tuple(capabilities)
        self._handler = handler
        self._name = name
        self._version = version
        self._recovery_supported = recovery_supported
        self._initialized = False
        self._project_root: Path | None = None
        self._capabilities: tuple[str, ...] = ()
        self._active: dict[str, threading.Event] = {}
        self._active_lock = threading.Lock()
        self._write_lock = threading.Lock()
        self._output: TextIO | None = None
        self._error_sequence = 0
        self._pool = ThreadPoolExecutor(max_workers=1, thread_name_prefix="arda-adapter")

    def serve(self, input_stream: TextIO, output_stream: TextIO) -> None:
        self._output = output_stream
        try:
            for line in input_stream:
                if len(line.encode("utf-8")) > MAX_LINE_BYTES:
                    self._protocol_error("line_too_large", "input line exceeds 65536 bytes")
                    continue
                try:
                    message = json.loads(line)
                except json.JSONDecodeError as exc:
                    self._protocol_error("invalid_json", f"invalid JSON: {exc.msg}")
                    continue
                if not isinstance(message, dict):
                    self._protocol_error("invalid_message", "message must be a JSON object")
                    continue
                self._dispatch(message)
        finally:
            with self._active_lock:
                for cancellation in self._active.values():
                    cancellation.set()
            self._pool.shutdown(wait=True, cancel_futures=False)

    def _dispatch(self, message: JsonObject) -> None:
        identifier = message.get("id")
        kind = message.get("type")
        if message.get("schema_version") != SCHEMA_VERSION:
            self._error(identifier, "unsupported_schema", "unsupported schema_version")
            return
        if not isinstance(identifier, str) or not identifier:
            self._protocol_error("invalid_id", "id must be a non-empty string")
            return
        if kind not in self._FIELDS:
            self._error(identifier, "unknown_message_type", "unknown message type")
            return
        unknown = set(message) - self._FIELDS[kind]
        if unknown:
            self._error(identifier, "unknown_field", f"unknown fields: {sorted(unknown)}")
            return
        if kind == "initialize":
            self._initialize(message)
        elif kind == "health":
            self._health(message)
        elif kind == "request":
            self._request(message)
        else:
            self._cancel(message)

    def _initialize(self, message: JsonObject) -> None:
        identifier = message["id"]
        if self._initialized:
            self._error(identifier, "already_initialized", "adapter is already initialized")
            return
        if message.get("protocol_version") != PROTOCOL_VERSION:
            self._error(identifier, "unsupported_protocol", "only protocol version 1 is supported")
            return
        root = message.get("project_root")
        allowed = message.get("allowed_capabilities")
        if not isinstance(root, str) or not root or not isinstance(allowed, list):
            self._error(identifier, "invalid_initialize", "invalid project root or capabilities")
            return
        if not all(isinstance(item, str) and item for item in allowed):
            self._error(identifier, "invalid_initialize", "capabilities must be non-empty strings")
            return
        self._project_root = Path(root).resolve()
        available = set(self._available)
        self._capabilities = tuple(item for item in allowed if item in available)
        self._initialized = True
        self._emit(
            self._frame(
                f"{identifier}:initialized",
                "initialized",
                request_id=identifier,
                adapter=self._name,
                adapter_version=self._version,
                capabilities=list(self._capabilities),
                recovery_supported=self._recovery_supported,
            )
        )

    def _health(self, message: JsonObject) -> None:
        identifier = message["id"]
        if not self._initialized:
            self._error(identifier, "not_initialized", "initialize must be first")
            return
        self._emit(
            self._frame(
                f"{identifier}:status",
                "health_status",
                request_id=identifier,
                status="ready",
            )
        )

    def _request(self, message: JsonObject) -> None:
        identifier = message["id"]
        if not self._initialized:
            self._error(identifier, "not_initialized", "initialize must be first")
            return
        operation = message.get("operation")
        arguments = message.get("arguments")
        required = message.get("required_capabilities")
        timeout_ms = message.get("timeout_ms")
        if (
            not isinstance(operation, str)
            or not operation
            or not isinstance(arguments, dict)
            or not isinstance(required, list)
            or not all(isinstance(item, str) and item for item in required)
            or not isinstance(timeout_ms, int)
            or isinstance(timeout_ms, bool)
            or timeout_ms <= 0
        ):
            self._error(identifier, "invalid_request", "invalid bounded request")
            return
        denied = next(
            (item for item in [*required, operation] if item not in self._capabilities),
            None,
        )
        if denied is not None:
            self._emit(
                self._frame(
                    f"{identifier}:denied",
                    "denied_capability",
                    request_id=identifier,
                    capability=denied,
                    reason="capability was not approved and advertised",
                )
            )
            return
        with self._active_lock:
            if self._active:
                self._error(identifier, "busy", "v1 permits one active request")
                return
            cancellation = threading.Event()
            self._active[identifier] = cancellation
        self._pool.submit(self._run_request, message, cancellation)

    def _run_request(self, message: JsonObject, cancellation: threading.Event) -> None:
        identifier = message["id"]
        started_at = self._now()
        context = AdapterContext(
            request_id=identifier,
            recovery_token=message.get("recovery_token"),
            _cancelled=cancellation,
            _emit=self._emit,
        )
        status = "succeeded"
        try:
            output = self._handler(message["operation"], message["arguments"], context)
            context.check_cancelled()
        except CancelledError:
            status = "cancelled"
            output = {"cancelled": True}
        except Exception as exc:  # boundary converts adapter failures into protocol data
            status = "failed"
            output = {"error": str(exc)}
        finished_at = self._now()
        canonical = json.dumps(message, sort_keys=True, separators=(",", ":")).encode("utf-8")
        result = self._frame(
            f"{identifier}:result",
            "result",
            request_id=identifier,
            status=status,
            output=output,
            provenance={
                "adapter": self._name,
                "adapter_version": self._version,
                "cwd": str(Path.cwd().resolve()),
                "started_at": started_at,
                "finished_at": finished_at,
                "request_digest": f"sha256:{hashlib.sha256(canonical).hexdigest()}",
            },
        )
        if message.get("recovery_token") is not None:
            result["recovery_token"] = message["recovery_token"]
        self._emit(result)
        with self._active_lock:
            self._active.pop(identifier, None)

    def _cancel(self, message: JsonObject) -> None:
        identifier = message["id"]
        request_id = message.get("request_id")
        if not isinstance(request_id, str) or not request_id:
            self._error(identifier, "invalid_cancel", "request_id must be non-empty")
            return
        with self._active_lock:
            cancellation = self._active.get(request_id)
        if cancellation is None:
            self._error(identifier, "unknown_request", "request is not active")
            return
        cancellation.set()
        self._emit(
            self._frame(
                f"{identifier}:cancelled",
                "cancelled",
                request_id=request_id,
            )
        )

    def _protocol_error(self, code: str, message: str) -> None:
        self._error_sequence += 1
        self._error(f"protocol-error-{self._error_sequence}", code, message)

    def _error(self, request_id: Any, code: str, message: str) -> None:
        correlation = request_id if isinstance(request_id, str) and request_id else "unknown"
        self._emit(
            self._frame(
                f"{correlation}:error",
                "error",
                request_id=correlation,
                code=code,
                message=message,
                retryable=False,
            )
        )

    def _emit(self, frame: JsonObject) -> None:
        if self._output is None:
            raise RuntimeError("server is not serving")
        encoded = json.dumps(frame, sort_keys=True, separators=(",", ":"))
        with self._write_lock:
            self._output.write(encoded + "\n")
            self._output.flush()

    @staticmethod
    def _frame(identifier: str, kind: str, **fields: Any) -> JsonObject:
        return {
            "schema_version": SCHEMA_VERSION,
            "id": identifier,
            "type": kind,
            **fields,
        }

    @staticmethod
    def _now() -> str:
        return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def reference_handler(operation: str, arguments: JsonObject, context: AdapterContext) -> Any:
    """Small deterministic handler used by the offline conformance suite."""
    if operation == "echo":
        return arguments
    if operation == "inspect":
        requested = arguments.get("environment", [])
        if not isinstance(requested, list):
            raise ValueError("environment must be a list")
        return {
            "cwd": str(Path.cwd().resolve()),
            "environment": {
                key: os.environ[key]
                for key in requested
                if isinstance(key, str) and key in os.environ
            },
        }
    if operation == "progress":
        steps = arguments.get("steps", 1)
        if not isinstance(steps, int) or isinstance(steps, bool) or steps < 1 or steps > 100:
            raise ValueError("steps must be an integer from 1 through 100")
        for step in range(1, steps + 1):
            context.progress(f"step {step}", percent=step * 100 / steps)
        return {"steps": steps}
    if operation == "sleep":
        seconds = arguments.get("seconds", 0)
        if not isinstance(seconds, (int, float)) or isinstance(seconds, bool) or seconds < 0:
            raise ValueError("seconds must be non-negative")
        deadline = time.monotonic() + float(seconds)
        while time.monotonic() < deadline:
            context.check_cancelled()
            time.sleep(min(0.01, max(0.0, deadline - time.monotonic())))
        return {"slept": seconds}
    raise ValueError(f"unsupported operation: {operation}")


def main() -> int:
    pid_file = os.environ.get("ARDA_ADAPTER_PID_FILE")
    if pid_file:
        Path(pid_file).write_text(str(os.getpid()), encoding="utf-8")
    server = AdapterServer(
        ["echo", "progress", "sleep", "inspect"],
        reference_handler,
    )
    server.serve(sys.stdin, sys.stdout)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
