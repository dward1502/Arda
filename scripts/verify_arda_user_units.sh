#!/usr/bin/env bash
# sigil: REPAIR
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
UNIT_DIR="${1:-$ROOT_DIR/config/systemd}"
TARGET_UNIT="$UNIT_DIR/arda-session.target"
HUD_UNIT="$UNIT_DIR/arda-hud.service"

python3 - "$TARGET_UNIT" "$HUD_UNIT" <<'PY'
from pathlib import Path
import sys

target_path = Path(sys.argv[1])
hud_path = Path(sys.argv[2])
for path in (target_path, hud_path):
    if not path.is_file():
        raise SystemExit(f"missing unit: {path}")

def unit_text(path: Path) -> str:
    return "\n".join(
        line
        for line in path.read_text(encoding="utf-8").splitlines()
        if not line.lstrip().startswith(("#", ";"))
    )

target = unit_text(target_path)
hud = unit_text(hud_path)

def require(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise SystemExit(f"{label}: missing {needle!r}")

def forbid(text: str, needle: str, label: str) -> None:
    if needle in text:
        raise SystemExit(f"{label}: forbidden {needle!r}")

require(target, "Wants=arda.service hermes-gateway.service", "session target")
require(target, "After=network-online.target arda.service hermes-gateway.service", "session target")
forbid(target, "arda-hud.service", "session target")

require(hud, "Wants=arda-session.target", "HUD service")
require(hud, "After=graphical-session.target arda-session.target", "HUD service")
require(hud, "PartOf=graphical-session.target", "HUD service")
require(hud, "ExecStart=%h/.local/lib/arda/hud/arda_hud", "HUD service")
for forbidden in (
    "target/release",
    "/Eregion/Arda",
    "launch_arda_hud.sh",
    "pnpm",
    "preview",
    "http://",
    "https://",
):
    forbid(hud, forbidden, "HUD service")

if "[Install]" in hud:
    raise SystemExit("HUD service must remain static for health-gated explicit start")
PY

if command -v systemd-analyze >/dev/null 2>&1; then
  systemd-analyze --user verify "$TARGET_UNIT" "$HUD_UNIT"
fi

printf 'arda user unit verification: pass unit_dir=%s\n' "$UNIT_DIR"
