#!/usr/bin/env bash
# Verify user units without clobbering the live user-manager socket.
set -euo pipefail

runtime_dir="$(mktemp -d "${TMPDIR:-/tmp}/arda-systemd-verify.XXXXXX")"
cleanup() {
  python3 -c 'import shutil,sys; shutil.rmtree(sys.argv[1], ignore_errors=True)' "$runtime_dir"
}
trap cleanup EXIT
chmod 0700 "$runtime_dir"

env \
  XDG_RUNTIME_DIR="$runtime_dir" \
  DBUS_SESSION_BUS_ADDRESS="unix:path=$runtime_dir/bus" \
  systemd-analyze --user verify "$@"
