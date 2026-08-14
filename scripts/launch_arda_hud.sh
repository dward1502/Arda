#!/usr/bin/env bash
# sigil: ANKH
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/runtime_build_env.sh"
arda_runtime_build_env "$ROOT_DIR"
APP_DIR="${ARDA_HUD_APP_DIR:-$ROOT_DIR/apps/arda-hud}"
PACKAGE_STATE="$ROOT_DIR/data/prometheus/arda_hud_package_last.json"
WORKSPACE_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT_DIR/target}"
LATEST_PATH=""
LATEST_MTIME=0

find_available_preview_port() {
  local host="$1"
  local port="$2"
  while python3 - "$host" "$port" <<'PY'
import socket
import sys

host = sys.argv[1]
port = int(sys.argv[2])
s = socket.socket()
s.settimeout(0.2)
try:
    s.connect((host, port))
except OSError:
    sys.exit(1)
finally:
    s.close()
sys.exit(0)
PY
  do
    port=$((port + 1))
  done

  printf '%s\n' "$port"
}

pick_if_newer() {
  local candidate="$1"
  if [[ -x "$candidate" ]]; then
    local mtime
    mtime=$(stat -c %Y "$candidate" 2>/dev/null || printf '0')
    if [[ "$mtime" -gt "$LATEST_MTIME" ]]; then
      LATEST_MTIME="$mtime"
      LATEST_PATH="$candidate"
    fi
  fi
}

pick_if_newer "$WORKSPACE_TARGET_DIR/release/arda_hud"
pick_if_newer "$APP_DIR/src-tauri/target/release/arda_hud"

shopt -s nullglob
for appimage in "$WORKSPACE_TARGET_DIR"/release/bundle/appimage/*.AppImage; do
  pick_if_newer "$appimage"
done
for appimage in "$APP_DIR"/src-tauri/target/release/bundle/appimage/*.AppImage; do
  pick_if_newer "$appimage"
done
shopt -u nullglob

pick_if_newer "/usr/bin/arda_hud"

if [[ -z "$LATEST_PATH" ]]; then
  if [[ -d "$APP_DIR/dist" ]]; then
    export ARDA_HUD_PREVIEW_HOST="${ARDA_HUD_PREVIEW_HOST:-127.0.0.1}"
    export ARDA_HUD_PREVIEW_PORT="${ARDA_HUD_PREVIEW_PORT:-4173}"
    export ARDA_HUD_PREVIEW_PORT="$(find_available_preview_port "$ARDA_HUD_PREVIEW_HOST" "$ARDA_HUD_PREVIEW_PORT")"
    cd "$APP_DIR"
    exec pnpm run preview -- --host "$ARDA_HUD_PREVIEW_HOST" --port "$ARDA_HUD_PREVIEW_PORT"
  fi
  echo "ARDA HUD binary not found in project or /usr/bin"
  if [[ -f "$PACKAGE_STATE" ]]; then
    echo "Last package state:"
    cat "$PACKAGE_STATE"
  fi
  exit 1
fi

export __NV_DISABLE_EXPLICIT_SYNC="${__NV_DISABLE_EXPLICIT_SYNC:-1}"
# Prefer the compositor-native WebKitGTK path when the host session exposes a
# usable Wayland socket. The NVIDIA X11 GBM path has repeatedly produced a
# native shell with a black webview at fullscreen display resolutions. An
# explicit operator-provided GDK_BACKEND still wins.
if [[ -z "${GDK_BACKEND:-}" && "${XDG_SESSION_TYPE:-}" == "wayland" && -n "${WAYLAND_DISPLAY:-}" ]]; then
  WAYLAND_SOCKET="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/${WAYLAND_DISPLAY}"
  if [[ -S "$WAYLAND_SOCKET" ]]; then
    export GDK_BACKEND=wayland
  fi
fi
# GTK sound-event modules are optional and frequently unavailable in
# containerized/distrobox shells; leaving GTK_MODULES set can print a
# scary but non-fatal "failed to load module canberra-gtk-module" warning.
unset GTK_MODULES
export NO_PROXY="${NO_PROXY:-localhost,127.0.0.1,::1}"
export no_proxy="${no_proxy:-localhost,127.0.0.1,::1}"

exec "$LATEST_PATH"
