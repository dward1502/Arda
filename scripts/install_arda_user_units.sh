#!/usr/bin/env bash
# sigil: REPAIR
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE_UNIT_DIR="$ROOT_DIR/config/systemd"
USER_UNIT_DIR="${ARDA_SYSTEMD_USER_DIR:-${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user}"
HUD_INSTALL_PATH="$HOME/.local/lib/arda/hud/arda_hud"
HUD_SOURCE="${ARDA_HUD_NATIVE_SOURCE:-${1:-}}"
SKIP_RELOAD="${ARDA_SKIP_SYSTEMD_RELOAD:-false}"

systemctl_user() {
  systemctl --user "$@" 2>/dev/null || \
    systemctl --user --machine="$(id -un)@.host" "$@"
}

SESSION_DEST="$USER_UNIT_DIR/arda-session.target"
HUD_UNIT_DEST="$USER_UNIT_DIR/arda-hud.service"
MIRROMERE_UNIT_DEST="$USER_UNIT_DIR/arda-mirromere.service"
BACKUP_DIR=""
SESSION_HAD_PREVIOUS=false
HUD_UNIT_HAD_PREVIOUS=false
MIRROMERE_UNIT_HAD_PREVIOUS=false
HUD_BINARY_HAD_PREVIOUS=false
HUD_BINARY_CHANGED=false
TRANSACTION_ACTIVE=false

atomic_install() {
  local source="$1"
  local destination="$2"
  local mode="$3"
  local temporary
  temporary="$(mktemp "${destination}.new.XXXXXX")"
  if ! install -m"$mode" "$source" "$temporary"; then
    rm -f "$temporary"
    return 1
  fi
  if ! mv -f "$temporary" "$destination"; then
    rm -f "$temporary"
    return 1
  fi
}

backup_path() {
  local source="$1"
  local backup="$2"
  if [[ -f "$source" ]]; then
    install -m0600 "$source" "$backup"
    return 0
  fi
  return 1
}

restore_path() {
  local destination="$1"
  local backup="$2"
  local had_previous="$3"
  local mode="$4"
  if [[ "$had_previous" == "true" ]]; then
    atomic_install "$backup" "$destination" "$mode"
  else
    rm -f "$destination"
  fi
}

cleanup_backup() {
  [[ -n "$BACKUP_DIR" ]] || return 0
  rm -f \
    "$BACKUP_DIR/arda-session.target" \
    "$BACKUP_DIR/arda-hud.service" \
    "$BACKUP_DIR/arda-mirromere.service" \
    "$BACKUP_DIR/arda_hud"
  rmdir "$BACKUP_DIR" 2>/dev/null || true
}

rollback() {
  restore_path "$SESSION_DEST" "$BACKUP_DIR/arda-session.target" "$SESSION_HAD_PREVIOUS" 0644 || true
  restore_path "$HUD_UNIT_DEST" "$BACKUP_DIR/arda-hud.service" "$HUD_UNIT_HAD_PREVIOUS" 0644 || true
  restore_path "$MIRROMERE_UNIT_DEST" "$BACKUP_DIR/arda-mirromere.service" "$MIRROMERE_UNIT_HAD_PREVIOUS" 0644 || true
  if [[ "$HUD_BINARY_CHANGED" == "true" ]]; then
    restore_path "$HUD_INSTALL_PATH" "$BACKUP_DIR/arda_hud" "$HUD_BINARY_HAD_PREVIOUS" 0755 || true
  fi
  if [[ "$SKIP_RELOAD" != "true" ]]; then
    systemctl_user daemon-reload >/dev/null 2>&1 || true
  fi
}

on_exit() {
  local status=$?
  trap - EXIT
  if [[ "$TRANSACTION_ACTIVE" == "true" ]]; then
    rollback
  fi
  cleanup_backup
  exit "$status"
}

if [[ -n "$HUD_SOURCE" && ! -f "$HUD_SOURCE" ]] || \
   [[ -n "$HUD_SOURCE" && ! -x "$HUD_SOURCE" ]]; then
  printf 'HUD native source is not executable: %s\n' "$HUD_SOURCE" >&2
  exit 1
fi
if [[ -z "$HUD_SOURCE" && ! -x "$HUD_INSTALL_PATH" ]]; then
  printf 'stable HUD binary missing: %s; provide ARDA_HUD_NATIVE_SOURCE or argv[1]\n' \
    "$HUD_INSTALL_PATH" >&2
  exit 1
fi

mkdir -p "$USER_UNIT_DIR" "$(dirname "$HUD_INSTALL_PATH")"
BACKUP_DIR="$(mktemp -d "${XDG_RUNTIME_DIR:-/tmp}/arda-user-unit-install.XXXXXX")"
trap on_exit EXIT
if backup_path "$SESSION_DEST" "$BACKUP_DIR/arda-session.target"; then
  SESSION_HAD_PREVIOUS=true
fi
if backup_path "$HUD_UNIT_DEST" "$BACKUP_DIR/arda-hud.service"; then
  HUD_UNIT_HAD_PREVIOUS=true
fi
if backup_path "$MIRROMERE_UNIT_DEST" "$BACKUP_DIR/arda-mirromere.service"; then
  MIRROMERE_UNIT_HAD_PREVIOUS=true
fi
if [[ -n "$HUD_SOURCE" ]] && backup_path "$HUD_INSTALL_PATH" "$BACKUP_DIR/arda_hud"; then
  HUD_BINARY_HAD_PREVIOUS=true
fi

TRANSACTION_ACTIVE=true

if [[ -n "$HUD_SOURCE" ]]; then
  HUD_BINARY_CHANGED=true
  atomic_install "$HUD_SOURCE" "$HUD_INSTALL_PATH" 0755
fi

"$ROOT_DIR/scripts/verify_arda_user_units.sh" "$SOURCE_UNIT_DIR"
atomic_install "$SOURCE_UNIT_DIR/arda-session.target" "$SESSION_DEST" 0644
atomic_install "$SOURCE_UNIT_DIR/arda-hud.service" "$HUD_UNIT_DEST" 0644
atomic_install "$SOURCE_UNIT_DIR/arda-mirromere.service" "$MIRROMERE_UNIT_DEST" 0644
"$ROOT_DIR/scripts/verify_arda_user_units.sh" "$USER_UNIT_DIR"

if [[ "$SKIP_RELOAD" != "true" ]]; then
  ENVIRONMENT_NAMES=()
  for name in DISPLAY WAYLAND_DISPLAY XDG_RUNTIME_DIR DBUS_SESSION_BUS_ADDRESS XDG_SESSION_TYPE; do
    if [[ -n "${!name:-}" ]]; then
      ENVIRONMENT_NAMES+=("$name")
    fi
  done
  if ((${#ENVIRONMENT_NAMES[@]} > 0)); then
    systemctl_user import-environment "${ENVIRONMENT_NAMES[@]}"
    if command -v dbus-update-activation-environment >/dev/null 2>&1; then
      dbus-update-activation-environment --systemd "${ENVIRONMENT_NAMES[@]}"
    fi
  fi

  systemctl_user daemon-reload
  [[ "$(systemctl_user show arda-session.target --property=LoadState --value)" == "loaded" ]]
  [[ "$(systemctl_user show arda-hud.service --property=LoadState --value)" == "loaded" ]]
  [[ "$(systemctl_user show arda-mirromere.service --property=LoadState --value)" == "loaded" ]]
fi

TRANSACTION_ACTIVE=false
trap - EXIT
cleanup_backup
printf 'arda user units installed: unit_dir=%s hud=%s\n' "$USER_UNIT_DIR" "$HUD_INSTALL_PATH"
