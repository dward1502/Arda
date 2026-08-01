#!/usr/bin/python3
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

from arda_project_adapter.server import AdapterContext, AdapterServer


def handler(operation: str, arguments: dict, context: AdapterContext) -> dict:
    if operation != "mutate_and_test":
        raise ValueError(f"unsupported operation: {operation}")
    if arguments != {"before": "hello", "after": "hello, Arda"}:
        raise ValueError("golden mutation arguments did not match the approved plan")

    root = Path.cwd().resolve()
    source = root / "src" / "greeting.py"
    test_file = root / "tests" / "test_greeting.py"
    context.progress("applying approved bounded mutation", percent=25)
    changed = False
    for path in (source, test_file):
        before = path.read_text(encoding="utf-8")
        after = before.replace('"hello"', '"hello, Arda"')
        if after != before:
            path.write_text(after, encoding="utf-8")
            changed = True
    if changed:
        count_path = Path(os.environ["ARDA_GOLDEN_MUTATION_COUNT"])
        count = int(count_path.read_text(encoding="utf-8")) if count_path.exists() else 0
        count_path.write_text(str(count + 1), encoding="utf-8")

    context.progress("running declared unittest check", percent=75)
    check = subprocess.run(
        ["python3", "-m", "unittest", "discover", "-s", "tests", "-v"],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )
    if check.returncode != 0:
        raise RuntimeError(check.stdout + check.stderr)
    context.progress("mutation verified", percent=100)
    return {
        "mutation": {
            "files": ["src/greeting.py", "tests/test_greeting.py"],
            "observable_count": 1,
            "source_digest": "sha256:" + hashlib.sha256(source.read_bytes()).hexdigest(),
        },
        "test": {
            "command": "python3 -m unittest discover -s tests -v",
            "exit_code": check.returncode,
            "output_digest": "sha256:" + hashlib.sha256((check.stdout + check.stderr).encode()).hexdigest(),
        },
        "route": {"adapter": "python-reference", "provider": None, "model": None},
        "cost_usd": 0.0,
    }


server = AdapterServer(
    ["mutate_and_test"],
    handler,
    name="arda-python-golden",
    version="1.0.0",
)
server.serve(sys.stdin, sys.stdout)
