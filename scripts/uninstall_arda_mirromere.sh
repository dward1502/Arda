#!/usr/bin/env bash
set -euo pipefail
UNIT_DEST="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user/arda-mirromere.service"
systemctl_user() {
  if systemctl --user show-environment >/dev/null 2>&1; then
    systemctl --user "$@"
  else
    systemctl --user --machine="${USER}@.host" "$@"
  fi
}
systemctl_user stop arda-mirromere.service 2>/dev/null || true
rm -f "$UNIT_DEST" "$HOME/.local/lib/arda/mirromere/arda_mirromere"
rmdir "$HOME/.local/lib/arda/mirromere" 2>/dev/null || true
systemctl_user daemon-reload
if [[ "${1:-}" == "--purge-state" ]]; then
  rm -f "${XDG_CONFIG_HOME:-$HOME/.config}/arda/mirromere-display.json"
fi
printf 'uninstalled Mirromere; display selection preserved unless --purge-state was supplied\n'
