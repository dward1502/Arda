#!/usr/bin/python3
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

args = sys.argv[1:]
transcript_path = Path(os.environ["ARDA_GOLDEN_TRANSCRIPT"])
if args[:2] == ["sessions", "export"]:
    print(transcript_path.read_text(encoding="utf-8"), flush=True)
    raise SystemExit(0)

attempt_marker = Path(os.environ["ARDA_GOLDEN_ATTEMPT"])
if not attempt_marker.exists():
    attempt_marker.write_text("transient failure recorded\n", encoding="utf-8")
    print("injected transient adapter failure", file=sys.stderr, flush=True)
    raise SystemExit(75)

root = Path.cwd()
source = root / "src" / "lib.rs"
before = source.read_text(encoding="utf-8")
after = before.replace('"hello"', '"hello, Arda"')
if after != before:
    source.write_text(after, encoding="utf-8")
    mutation_count = Path(os.environ["ARDA_GOLDEN_MUTATION_COUNT"])
    current = int(mutation_count.read_text(encoding="utf-8")) if mutation_count.exists() else 0
    mutation_count.write_text(str(current + 1), encoding="utf-8")

test = subprocess.run(
    ["cargo", "test", "--quiet"],
    cwd=root,
    capture_output=True,
    check=False,
)
terminal_content = json.dumps({
    "output": (test.stdout + test.stderr).decode("utf-8"),
    "exit_code": test.returncode,
    "error": None,
}, separators=(",", ":"))
file_content = json.dumps({
    "path": "src/lib.rs",
    "changed": after != before,
}, separators=(",", ":"))
session = {
    "id": "golden-rust-vendor-session",
    "source": "tool",
    "model": "fixture-model",
    "billing_provider": "fixture-provider",
    "estimated_cost_usd": 0.002,
    "actual_cost_usd": 0.001,
    "input_tokens": 160,
    "output_tokens": 48,
    "api_call_count": 1,
    "messages": [
        {
            "role": "assistant",
            "content": None,
            "tool_calls": [
                {
                    "id": "call-mutate",
                    "type": "function",
                    "function": {
                        "name": "file",
                        "arguments": json.dumps({"path": "src/lib.rs", "operation": "bounded_replace"}),
                    },
                },
                {
                    "id": "call-test",
                    "type": "function",
                    "function": {
                        "name": "terminal",
                        "arguments": json.dumps({"command": "cargo test --quiet"}),
                    },
                },
            ],
        },
        {"role": "tool", "tool_call_id": "call-mutate", "tool_name": "file", "content": file_content},
        {"role": "tool", "tool_call_id": "call-test", "tool_name": "terminal", "content": terminal_content},
    ],
}
transcript_path.write_text(json.dumps(session), encoding="utf-8")
digest = "sha256:" + hashlib.sha256(source.read_bytes()).hexdigest()
result = {
    "schema_version": "arda.hermes-job-result.v1",
    "status": "succeeded" if test.returncode == 0 else "failed",
    "summary": "Applied the approved Rust greeting mutation and ran its declared test.",
    "tool_evidence": [
        {"tool_call_id": "call-mutate"},
        {"tool_call_id": "call-test"},
    ],
    "test_evidence": [{"check_id": "test", "tool_call_id": "call-test"}],
    "artifacts": [{"path": "src/lib.rs", "digest": digest}],
}
print("session_id: golden-rust-vendor-session")
print(json.dumps(result), flush=True)
raise SystemExit(test.returncode)
