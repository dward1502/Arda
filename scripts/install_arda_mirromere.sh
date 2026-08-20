#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE="${ARDA_MIRROMERE_BINARY:-$ROOT_DIR/apps/arda-mirromere/src-tauri/target/release/arda_mirromere}"
DEST="$HOME/.local/lib/arda/mirromere/arda_mirromere"
UNIT_DEST="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user/arda-mirromere.service"
systemctl_user() {
  if systemctl --user show-environment >/dev/null 2>&1; then
    systemctl --user "$@"
  else
    systemctl --user --machine="${USER}@.host" "$@"
  fi
}
"$ROOT_DIR/scripts/verify_arda_mirromere_unit.sh"
[[ -x "$SOURCE" ]] || { printf 'missing packaged Mirromere binary: %s\n' "$SOURCE" >&2; exit 1; }
install -d "$(dirname "$DEST")" "$(dirname "$UNIT_DEST")"
install -m 0755 "$SOURCE" "$DEST"
install -m 0644 "$ROOT_DIR/config/systemd/arda-mirromere.service" "$UNIT_DEST"
systemctl_user daemon-reload
systemctl_user show arda-mirromere.service --property=LoadState --value | grep -qx loaded
printf 'installed explicit-launch Mirromere runtime; service remains stopped\n'
