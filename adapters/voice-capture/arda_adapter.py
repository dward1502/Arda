#!/usr/bin/env python3
"""Supervised local speech-to-text adapter for Personal Operations."""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Sequence

REQUEST_SCHEMA = "arda.voice-capture.request.v1"
RESPONSE_SCHEMA = "arda.voice-capture.result.v1"


@dataclass(frozen=True)
class AdapterConfig:
    executable: str
    arguments: tuple[str, ...]
    model: str
    audio_root: Path
    allowed_extensions: frozenset[str]
    timeout_seconds: float
    max_audio_bytes: int
    max_output_bytes: int
    default_audio_retention: str
    default_transcript_retention: str


@dataclass(frozen=True)
class ProcessResult:
    returncode: int
    stdout: bytes


Runner = Callable[[Sequence[str], float, int], ProcessResult]


def load_config(path: Path) -> AdapterConfig:
    with path.open("rb") as handle:
        raw = tomllib.load(handle)
    supervision = raw.get("supervision", {})
    retention = raw.get("retention", {})
    return AdapterConfig(
        executable=str(raw["executable"]),
        arguments=tuple(str(value) for value in raw.get("arguments", [])),
        model=str(raw.get("model", "")),
        audio_root=Path(str(raw["audio_root"])).expanduser().resolve(),
        allowed_extensions=frozenset(str(value).lower() for value in raw.get("allowed_extensions", [])),
        timeout_seconds=float(supervision.get("timeout_seconds", 30)),
        max_audio_bytes=int(supervision.get("max_audio_bytes", 25 * 1024 * 1024)),
        max_output_bytes=int(supervision.get("max_output_bytes", 256 * 1024)),
        default_audio_retention=str(retention.get("audio", "ephemeral")),
        default_transcript_retention=str(retention.get("transcript", "ephemeral")),
    )


def supervised_runner(command: Sequence[str], timeout_seconds: float, max_output_bytes: int) -> ProcessResult:
    with tempfile.TemporaryFile() as stdout:
        process = subprocess.Popen(
            list(command),
            stdin=subprocess.DEVNULL,
            stdout=stdout,
            stderr=subprocess.DEVNULL,
            shell=False,
        )
        try:
            returncode = process.wait(timeout=timeout_seconds)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
            raise
        stdout.seek(0)
        output = stdout.read(max_output_bytes + 1)
        return ProcessResult(returncode=returncode, stdout=output)


def _result(
    *,
    status: str,
    audio_reference: str | None,
    audio_retention: str,
    transcript_retention: str,
    transcript: str | None = None,
    error_class: str | None = None,
) -> dict[str, Any]:
    return {
        "schema_version": RESPONSE_SCHEMA,
        "status": status,
        "audio_reference": audio_reference,
        "audio_retention": audio_retention,
        "transcript": transcript,
        "transcript_retention": transcript_retention,
        "editable": status == "transcript_pending_review",
        "review_state": "operator_review_required" if status == "transcript_pending_review" else "recovery_required",
        "external_send_authorized": False,
        "governed_action_authorized": False,
        "error_class": error_class,
    }


def _recoverable(audio_reference: str | None, transcript_retention: str, error_class: str) -> dict[str, Any]:
    return _result(
        status="recoverable_inbox",
        audio_reference=audio_reference,
        audio_retention="preserve_until_recovered",
        transcript_retention=transcript_retention,
        error_class=error_class,
    )


def process_request(request: dict[str, Any], config: AdapterConfig, runner: Runner = supervised_runner) -> dict[str, Any]:
    audio_value = request.get("audio_path")
    requested_retention = request.get("transcript_retention", config.default_transcript_retention)
    transcript_retention = requested_retention if requested_retention in {"ephemeral", "retain"} else config.default_transcript_retention
    audio_reference = str(audio_value) if isinstance(audio_value, str) else None

    if request.get("schema_version") != REQUEST_SCHEMA or not audio_reference:
        return _recoverable(audio_reference, transcript_retention, "invalid_request")

    try:
        audio_path = Path(audio_reference).expanduser().resolve(strict=True)
        audio_path.relative_to(config.audio_root)
        if not audio_path.is_file():
            raise ValueError("not a regular file")
        if audio_path.suffix.lower() not in config.allowed_extensions:
            raise ValueError("unsupported audio extension")
        if audio_path.stat().st_size > config.max_audio_bytes:
            raise ValueError("audio exceeds configured size limit")
    except (OSError, ValueError):
        return _recoverable(audio_reference, transcript_retention, "rejected_audio")

    substitutions = {"audio_path": str(audio_path), "model": config.model}
    try:
        arguments = [argument.format_map(substitutions) for argument in config.arguments]
    except (KeyError, ValueError):
        return _recoverable(audio_reference, transcript_retention, "invalid_configuration")
    command = [config.executable, *arguments]

    try:
        completed = runner(command, config.timeout_seconds, config.max_output_bytes)
    except FileNotFoundError:
        return _recoverable(audio_reference, transcript_retention, "backend_unavailable")
    except subprocess.TimeoutExpired:
        return _recoverable(audio_reference, transcript_retention, "backend_timeout")
    except OSError:
        return _recoverable(audio_reference, transcript_retention, "backend_unavailable")

    if len(completed.stdout) > config.max_output_bytes:
        return _recoverable(audio_reference, transcript_retention, "output_limit_exceeded")
    if completed.returncode != 0:
        return _recoverable(audio_reference, transcript_retention, "backend_failed")

    try:
        transcript = completed.stdout.decode("utf-8").strip()
    except UnicodeDecodeError:
        return _recoverable(audio_reference, transcript_retention, "invalid_backend_output")
    if not transcript:
        return _recoverable(audio_reference, transcript_retention, "empty_transcript")

    return _result(
        status="transcript_pending_review",
        audio_reference=audio_reference,
        audio_retention=config.default_audio_retention,
        transcript_retention=transcript_retention,
        transcript=transcript,
    )


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", required=True, type=Path)
    parser.add_argument("--request", type=Path, help="JSON request file; stdin when omitted")
    args = parser.parse_args(argv)
    try:
        source = args.request.read_text(encoding="utf-8") if args.request else sys.stdin.read()
        request = json.loads(source)
        if not isinstance(request, dict):
            raise ValueError("request must be a JSON object")
        response = process_request(request, load_config(args.config))
    except (OSError, ValueError, json.JSONDecodeError, tomllib.TOMLDecodeError):
        response = _recoverable(None, "ephemeral", "invalid_request_or_configuration")
    json.dump(response, sys.stdout, sort_keys=True)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
