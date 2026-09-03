#!/usr/bin/env bash
# Install the source-current governed automation runtime atomically.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE_UNIT_DIR="$ROOT_DIR/config/systemd"
USER_UNIT_DIR="${ARDA_SYSTEMD_USER_DIR:-${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user}"
CLI_SOURCE="${ARDA_CLI_SOURCE:-$ROOT_DIR/target/release/arda-cli}"
CLI_DEST="${ARDA_CLI_DEST:-$HOME/.local/bin/arda-cli}"
SKIP_RELOAD="${ARDA_SKIP_SYSTEMD_RELOAD:-false}"
SYSTEMCTL_BIN="${ARDA_SYSTEMCTL_BIN:-systemctl}"
ROLLBACK_ROOT="${ARDA_ROLLBACK_ROOT:-$HOME/.local/state/arda/rollback}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
ROLLBACK_DIR="$ROLLBACK_ROOT/$STAMP-automation-install"
TRANSACTION_ACTIVE=false
TIMER_STATE_CAPTURED=false

declare -A TIMER_UNIT_FILE_STATES=()
declare -A TIMER_ACTIVE_STATES=()
TIMERS=(
  arda-aule-autopilot-read-only.timer
  arda-aule-autopilot.timer
  arda-workbench-queue-executor.timer
)

UNITS=(
  arda-aule-autopilot.service
  arda-aule-autopilot.timer
  arda-aule-autopilot-read-only.service
  arda-aule-autopilot-read-only.timer
)

RETIRED_UNITS=(
  arda-workbench-queue-executor.service
  arda-workbench-queue-executor.timer
)

systemctl_user() {
  if [[ "$SYSTEMCTL_BIN" != "systemctl" ]]; then
    "$SYSTEMCTL_BIN" --user "$@"
    return
  fi
  "$SYSTEMCTL_BIN" --user "$@" 2>/dev/null || \
    "$SYSTEMCTL_BIN" --user --machine="$(id -un)@.host" "$@"
}

atomic_install() {
  local source="$1"
  local destination="$2"
  local mode="$3"
  local temporary
  temporary="$(mktemp "${destination}.new.XXXXXX")"
  install -m"$mode" "$source" "$temporary"
  mv -f "$temporary" "$destination"
}

backup_path() {
  local path="$1"
  local name="$2"
  if [[ -L "$path" ]]; then
    readlink "$path" > "$ROLLBACK_DIR/.symlink-$name"
  elif [[ -f "$path" ]]; then
    install -m0600 "$path" "$ROLLBACK_DIR/$name"
  elif [[ -e "$path" ]]; then
    printf 'refusing to replace unsupported non-regular path: %s\n' "$path" >&2
    return 1
  else
    : > "$ROLLBACK_DIR/.absent-$name"
  fi
}

restore_path() {
  local destination="$1"
  local name="$2"
  local mode="$3"
  if [[ -f "$ROLLBACK_DIR/.symlink-$name" ]]; then
    local target
    target="$(< "$ROLLBACK_DIR/.symlink-$name")"
    rm -f "$destination"
    ln -s "$target" "$destination"
  elif [[ -f "$ROLLBACK_DIR/.absent-$name" ]]; then
    rm -f "$destination"
  else
    atomic_install "$ROLLBACK_DIR/$name" "$destination" "$mode"
  fi
}

capture_timer_states() {
  local timer
  for timer in "${TIMERS[@]}"; do
    TIMER_UNIT_FILE_STATES["$timer"]="$(
      systemctl_user show "$timer" --property=UnitFileState --value
    )"
    TIMER_ACTIVE_STATES["$timer"]="$(
      systemctl_user show "$timer" --property=ActiveState --value
    )"
    printf 'timer_state=%s unit_file=%s active=%s\n' \
      "$timer" "${TIMER_UNIT_FILE_STATES[$timer]}" "${TIMER_ACTIVE_STATES[$timer]}" \
      >> "$ROLLBACK_DIR/manifest"
  done
  TIMER_STATE_CAPTURED=true
}

restore_timer_states() {
  local timer
  [[ "$TIMER_STATE_CAPTURED" == "true" ]] || return 0
  for timer in "${TIMERS[@]}"; do
    case "${TIMER_UNIT_FILE_STATES[$timer]}" in
      enabled) systemctl_user enable "$timer" >/dev/null 2>&1 || true ;;
      enabled-runtime) systemctl_user enable --runtime "$timer" >/dev/null 2>&1 || true ;;
      disabled) systemctl_user disable "$timer" >/dev/null 2>&1 || true ;;
      masked) systemctl_user mask "$timer" >/dev/null 2>&1 || true ;;
      masked-runtime) systemctl_user mask --runtime "$timer" >/dev/null 2>&1 || true ;;
    esac
    if [[ "${TIMER_ACTIVE_STATES[$timer]}" == "active" ]]; then
      systemctl_user start "$timer" >/dev/null 2>&1 || true
    else
      systemctl_user stop "$timer" >/dev/null 2>&1 || true
    fi
  done
}

rollback() {
  restore_path "$CLI_DEST" arda-cli 0755 || true
  for unit in "${UNITS[@]}"; do
    restore_path "$USER_UNIT_DIR/$unit" "$unit" 0644 || true
  done
  for unit in "${RETIRED_UNITS[@]}"; do
    restore_path "$USER_UNIT_DIR/$unit" "$unit" 0644 || true
  done
  if [[ "$SKIP_RELOAD" != "true" ]]; then
    systemctl_user daemon-reload >/dev/null 2>&1 || true
    restore_timer_states
  fi
}

on_exit() {
  local status=$?
  trap - EXIT
  if [[ "$TRANSACTION_ACTIVE" == "true" ]]; then
    rollback
  fi
  exit "$status"
}

if [[ ! -x "$CLI_SOURCE" ]]; then
  printf 'source-current arda-cli is missing or not executable: %s\n' "$CLI_SOURCE" >&2
  exit 1
fi
for unit in "${UNITS[@]}"; do
  if [[ ! -f "$SOURCE_UNIT_DIR/$unit" ]]; then
    printf 'automation unit missing: %s\n' "$SOURCE_UNIT_DIR/$unit" >&2
    exit 1
  fi
done

mkdir -p "$USER_UNIT_DIR" "$(dirname "$CLI_DEST")" "$ROLLBACK_DIR"
backup_path "$CLI_DEST" arda-cli
for unit in "${UNITS[@]}"; do
  backup_path "$USER_UNIT_DIR/$unit" "$unit"
done
for unit in "${RETIRED_UNITS[@]}"; do
  backup_path "$USER_UNIT_DIR/$unit" "$unit"
done
printf 'cli_destination=%s\nunit_directory=%s\n' "$CLI_DEST" "$USER_UNIT_DIR" > "$ROLLBACK_DIR/manifest"
CLI_SOURCE_SHA256="$(sha256sum "$CLI_SOURCE" | cut -d' ' -f1)"
printf 'cli_source_sha256=%s\n' "$CLI_SOURCE_SHA256" >> "$ROLLBACK_DIR/manifest"
if [[ "$SKIP_RELOAD" != "true" ]]; then
  capture_timer_states
fi

trap on_exit EXIT
TRANSACTION_ACTIVE=true
atomic_install "$CLI_SOURCE" "$CLI_DEST" 0755
CLI_INSTALLED_SHA256="$(sha256sum "$CLI_DEST" | cut -d' ' -f1)"
printf 'cli_installed_sha256=%s\n' "$CLI_INSTALLED_SHA256" >> "$ROLLBACK_DIR/manifest"
if [[ "$CLI_INSTALLED_SHA256" != "$CLI_SOURCE_SHA256" ]]; then
  printf 'installed arda-cli checksum mismatch: source=%s installed=%s\n' \
    "$CLI_SOURCE_SHA256" "$CLI_INSTALLED_SHA256" >&2
  exit 1
fi
for unit in "${UNITS[@]}"; do
  atomic_install "$SOURCE_UNIT_DIR/$unit" "$USER_UNIT_DIR/$unit" 0644
done
rm -f "${RETIRED_UNITS[@]/#/$USER_UNIT_DIR/}"

"$ROOT_DIR/scripts/systemd_user_verify.sh" "${UNITS[@]/#/$USER_UNIT_DIR/}"

if [[ "$SKIP_RELOAD" != "true" ]]; then
  systemctl_user daemon-reload
  systemctl_user disable --now arda-aule-autopilot-read-only.timer
  systemctl_user disable --now arda-workbench-queue-executor.timer
  systemctl_user enable --now arda-aule-autopilot.timer
  [[ "$(systemctl_user show arda-aule-autopilot.timer --property=LoadState --value)" == "loaded" ]]
fi

TRANSACTION_ACTIVE=false
trap - EXIT
printf 'arda automation installed: cli=%s unit_dir=%s rollback=%s\n' \
  "$CLI_DEST" "$USER_UNIT_DIR" "$ROLLBACK_DIR"
