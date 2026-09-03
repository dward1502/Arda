#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
UNIT_PATH="${1:-${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user/arda-metrics-exporter.service}"
BINARY_PATH="${ARDA_METRICS_BINARY_PATH:-$HOME/.local/bin/arda-cli}"
RUNTIME_CHECK="${ARDA_METRICS_RUNTIME_CHECK:-false}"

systemctl_user() {
  systemctl --user "$@" 2>/dev/null || \
    systemctl --user --machine="$(id -un)@.host" "$@"
}

python3 - "$UNIT_PATH" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
if not path.is_file():
    raise SystemExit(f"missing metrics unit: {path}")
text = "\n".join(
    line for line in path.read_text(encoding="utf-8").splitlines()
    if not line.lstrip().startswith(("#", ";"))
)
required = (
    "ExecStart=%h/.local/bin/arda-cli metrics serve",
    "--bind 127.0.0.1",
    "--port 9101",
    "StartLimitIntervalSec=5min",
    "StartLimitBurst=5",
    "Restart=on-failure",
)
for needle in required:
    if needle not in text:
        raise SystemExit(f"metrics unit missing {needle!r}")
for forbidden in ("--bind 0.0.0.0", "ExecStart=/var/home/"):
    if forbidden in text:
        raise SystemExit(f"metrics unit contains forbidden {forbidden!r}")
PY

[[ -x "$BINARY_PATH" ]] || { printf 'metrics binary is not executable: %s\n' "$BINARY_PATH" >&2; exit 1; }
"$BINARY_PATH" metrics serve --help | python3 -c \
  'import sys; text=sys.stdin.read(); assert "[default: 127.0.0.1]" in text, text'
"$BINARY_PATH" metrics snapshot --root "$ROOT_DIR" >/dev/null

if command -v systemd-analyze >/dev/null 2>&1; then
  "$ROOT_DIR/scripts/systemd_user_verify.sh" "$UNIT_PATH"
fi

if [[ "$RUNTIME_CHECK" == "true" ]]; then
  [[ "$(systemctl_user show arda-metrics-exporter.service -p ActiveState --value)" == "active" ]]
  [[ "$(systemctl_user show arda-metrics-exporter.service -p SubState --value)" == "running" ]]
  curl --fail --silent --show-error http://127.0.0.1:9101/health | \
    python3 -c 'import sys; assert sys.stdin.read().strip() == "ok"'
  curl --fail --silent --show-error http://127.0.0.1:9101/metrics | \
    python3 -c 'import sys; text=sys.stdin.read(); assert "annunimas_metrics_exporter_refresh_success" in text'
  ss -ltnH 'sport = :9101' | python3 -c \
    'import sys; rows=sys.stdin.read(); assert "127.0.0.1:9101" in rows and "0.0.0.0:9101" not in rows, rows'
fi

printf 'arda metrics exporter verification: pass unit=%s binary=%s runtime=%s\n' \
  "$UNIT_PATH" "$BINARY_PATH" "$RUNTIME_CHECK"
