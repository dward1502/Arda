#!/usr/bin/env python3
from __future__ import annotations

import sys
from pathlib import Path


def verify(path: Path, installed: bool = False) -> None:
    if not path.is_file():
        raise SystemExit(f"missing desktop entry: {path}")
    fields: dict[str, str] = {}
    for raw in path.read_text(encoding="utf-8").splitlines():
        if "=" in raw and not raw.lstrip().startswith("#"):
            key, value = raw.split("=", 1)
            fields[key] = value
    required = {
        "Type": "Application",
        "Name": "Arda Launcher",
        "Terminal": "false",
        "StartupNotify": "true",
        "Icon": "arda-launcher",
    }
    for key, value in required.items():
        if fields.get(key) != value:
            raise SystemExit(f"desktop entry {key} must equal {value!r}")
    executable = fields.get("Exec", "")
    if installed:
        if executable != str(Path.home() / ".local/lib/arda/arda-launcher"):
            raise SystemExit("installed Exec must use the managed launcher binary")
    elif executable != "@ARDA_LAUNCHER_EXEC@":
        raise SystemExit("template Exec must use the installer substitution token")
    for forbidden in ("Eregion/Arda", "target/", "cargo", "pnpm", "bash", "sh -c"):
        if forbidden in executable:
            raise SystemExit(f"desktop Exec contains forbidden value: {forbidden}")


if __name__ == "__main__":
    if len(sys.argv) not in (2, 3):
        raise SystemExit("usage: verify_arda_launcher_desktop.py PATH [--installed]")
    verify(Path(sys.argv[1]), len(sys.argv) == 3 and sys.argv[2] == "--installed")
    print(f"launcher desktop verification: pass path={sys.argv[1]}")
