#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SANDBOX="$(mktemp -d)"
trap 'rm -rf "$SANDBOX"' EXIT

mkdir -p "$SANDBOX/source" "$SANDBOX/home"
printf '#!/usr/bin/env bash\nexit 0\n' > "$SANDBOX/source/arda-cli"
chmod 0755 "$SANDBOX/source/arda-cli"
mkdir -p "$SANDBOX/home/.config/systemd/user"
cp "$ROOT_DIR/config/systemd/arda.service" \
  "$SANDBOX/home/.config/systemd/user/arda.service"
printf 'legacy service\n' > "$SANDBOX/home/.config/systemd/user/arda-workbench-queue-executor.service"
printf 'legacy timer\n' > "$SANDBOX/home/.config/systemd/user/arda-workbench-queue-executor.timer"

HOME="$SANDBOX/home" \
XDG_CONFIG_HOME="$SANDBOX/home/.config" \
ARDA_CLI_SOURCE="$SANDBOX/source/arda-cli" \
ARDA_SKIP_SYSTEMD_RELOAD=true \
  "$ROOT_DIR/scripts/install_arda_automation_units.sh"

cmp "$SANDBOX/source/arda-cli" "$SANDBOX/home/.local/bin/arda-cli"
ROLLBACK_DIR="$(find "$SANDBOX/home/.local/state/arda/rollback" -mindepth 1 -maxdepth 1 -type d)"
SOURCE_HASH="$(sha256sum "$SANDBOX/source/arda-cli" | cut -d' ' -f1)"
grep -Fx "cli_source_sha256=$SOURCE_HASH" "$ROLLBACK_DIR/manifest"
grep -Fx "cli_installed_sha256=$SOURCE_HASH" "$ROLLBACK_DIR/manifest"
for unit in \
  arda-aule-autopilot.service \
  arda-aule-autopilot.timer \
  arda-aule-autopilot-read-only.service \
  arda-aule-autopilot-read-only.timer; do
  cmp "$ROOT_DIR/config/systemd/$unit" "$SANDBOX/home/.config/systemd/user/$unit"
done
test ! -e "$SANDBOX/home/.config/systemd/user/arda-workbench-queue-executor.service"
test ! -e "$SANDBOX/home/.config/systemd/user/arda-workbench-queue-executor.timer"

# A failed live cutover must restore both bytes and timer state.
ROLLBACK_SANDBOX="$SANDBOX/rollback"
mkdir -p "$ROLLBACK_SANDBOX/home/.config/systemd/user" "$ROLLBACK_SANDBOX/bin" \
  "$ROLLBACK_SANDBOX/state"
printf '#!/usr/bin/env bash\nexit 23\n' > "$ROLLBACK_SANDBOX/home/.local-old-cli"
chmod 0755 "$ROLLBACK_SANDBOX/home/.local-old-cli"
mkdir -p "$ROLLBACK_SANDBOX/home/.local/bin"
cp "$ROLLBACK_SANDBOX/home/.local-old-cli" "$ROLLBACK_SANDBOX/home/.local/bin/arda-cli"
for unit in \
  arda-aule-autopilot.service \
  arda-aule-autopilot.timer \
  arda-aule-autopilot-read-only.service \
  arda-aule-autopilot-read-only.timer \
  arda-workbench-queue-executor.service \
  arda-workbench-queue-executor.timer; do
  printf 'old-%s\n' "$unit" > "$ROLLBACK_SANDBOX/home/.config/systemd/user/$unit"
done
rm "$ROLLBACK_SANDBOX/home/.config/systemd/user/arda-aule-autopilot-read-only.timer"
ln -s /dev/null \
  "$ROLLBACK_SANDBOX/home/.config/systemd/user/arda-aule-autopilot-read-only.timer"
cp "$ROOT_DIR/config/systemd/arda.service" \
  "$ROLLBACK_SANDBOX/home/.config/systemd/user/arda.service"
printf 'masked inactive\n' > "$ROLLBACK_SANDBOX/state/arda-aule-autopilot-read-only.timer"
printf 'disabled inactive\n' > "$ROLLBACK_SANDBOX/state/arda-aule-autopilot.timer"
printf 'disabled inactive\n' > "$ROLLBACK_SANDBOX/state/arda-workbench-queue-executor.timer"
cat > "$ROLLBACK_SANDBOX/bin/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
while [[ "${1:-}" == --* ]]; do shift; done
command="$1"; shift
case "$command" in
  daemon-reload) exit 0 ;;
  show)
    unit="$1"; shift
    if [[ "$unit" == arda-aule-autopilot.timer && "$*" == *LoadState* && ! -e "$ARDA_TEST_STATE/failed" ]]; then
      cmp "$ARDA_TEST_CLI_SOURCE" "$ARDA_TEST_CLI_DEST"
      grep -F '[Unit]' "$ARDA_TEST_UNIT_DIR/arda-aule-autopilot.service" >/dev/null
      test ! -e "$ARDA_TEST_UNIT_DIR/arda-workbench-queue-executor.service"
      test ! -e "$ARDA_TEST_UNIT_DIR/arda-workbench-queue-executor.timer"
      touch "$ARDA_TEST_STATE/mutation-observed"
      touch "$ARDA_TEST_STATE/failed"
      exit 1
    fi
    read -r enabled active < "$ARDA_TEST_STATE/$unit"
    if [[ "$*" == *UnitFileState* ]]; then
      printf '%s\n' "$enabled"
    elif [[ "$*" == *LoadState* ]]; then
      printf 'loaded\n'
    else
      printf '%s\n' "$active"
    fi
    ;;
  enable|disable|mask|start|stop)
    action="$command"
    [[ "${1:-}" == --now ]] && shift
    for unit in "$@"; do
      read -r enabled active < "$ARDA_TEST_STATE/$unit"
      case "$action" in
        enable) enabled=enabled ;;
        disable) enabled=disabled ;;
        mask) enabled=masked ;;
        start) active=active ;;
        stop) active=inactive ;;
      esac
      printf '%s %s\n' "$enabled" "$active" > "$ARDA_TEST_STATE/$unit"
    done
    ;;
  *) exit 2 ;;
esac
EOF
chmod 0755 "$ROLLBACK_SANDBOX/bin/systemctl"

if HOME="$ROLLBACK_SANDBOX/home" \
  XDG_CONFIG_HOME="$ROLLBACK_SANDBOX/home/.config" \
  ARDA_CLI_SOURCE="$SANDBOX/source/arda-cli" \
  ARDA_SYSTEMCTL_BIN="$ROLLBACK_SANDBOX/bin/systemctl" \
  ARDA_TEST_STATE="$ROLLBACK_SANDBOX/state" \
  ARDA_TEST_CLI_SOURCE="$SANDBOX/source/arda-cli" \
  ARDA_TEST_CLI_DEST="$ROLLBACK_SANDBOX/home/.local/bin/arda-cli" \
  ARDA_TEST_UNIT_DIR="$ROLLBACK_SANDBOX/home/.config/systemd/user" \
  "$ROOT_DIR/scripts/install_arda_automation_units.sh"; then
  printf 'expected forced live-cutover failure\n' >&2
  exit 1
fi
test -f "$ROLLBACK_SANDBOX/state/mutation-observed"
cmp "$ROLLBACK_SANDBOX/home/.local-old-cli" "$ROLLBACK_SANDBOX/home/.local/bin/arda-cli"
for unit in \
  arda-aule-autopilot.service \
  arda-aule-autopilot.timer \
  arda-aule-autopilot-read-only.service \
  arda-workbench-queue-executor.service \
  arda-workbench-queue-executor.timer; do
  grep -Fx "old-$unit" "$ROLLBACK_SANDBOX/home/.config/systemd/user/$unit"
done
test -L "$ROLLBACK_SANDBOX/home/.config/systemd/user/arda-aule-autopilot-read-only.timer"
test "$(readlink "$ROLLBACK_SANDBOX/home/.config/systemd/user/arda-aule-autopilot-read-only.timer")" = /dev/null
grep -Fx 'masked inactive' "$ROLLBACK_SANDBOX/state/arda-aule-autopilot-read-only.timer"
grep -Fx 'disabled inactive' "$ROLLBACK_SANDBOX/state/arda-aule-autopilot.timer"
grep -Fx 'disabled inactive' "$ROLLBACK_SANDBOX/state/arda-workbench-queue-executor.timer"

printf 'arda automation installer test: pass\n'
