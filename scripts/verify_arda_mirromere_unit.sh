#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
UNIT="${1:-$ROOT_DIR/config/systemd/arda-mirromere.service}"
python3 - "$UNIT" <<'PY'
from pathlib import Path
import sys
path = Path(sys.argv[1])
text = path.read_text()
required = [
    "ExecStart=%h/.local/lib/arda/mirromere/arda_mirromere",
    "Restart=no",
    "PartOf=graphical-session.target",
]
for needle in required:
    if needle not in text:
        raise SystemExit(f"{path}: missing {needle}")
for forbidden in ("WantedBy=", "Restart=always", "Restart=on-failure", "arda-hud.service"):
    if forbidden in text:
        raise SystemExit(f"{path}: forbidden lifecycle coupling {forbidden}")
print(f"verified Mirromere unit: {path}")
PY
