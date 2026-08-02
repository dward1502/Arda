#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT
BIN_DIR="$TMP_DIR/bin"
LOG_FILE="$TMP_DIR/ssh.log"
STATE_FILE="$TMP_DIR/state"
mkdir -p "$BIN_DIR"
: >"$LOG_FILE"
printf 'before\n' >"$STATE_FILE"

cat >"$TMP_DIR/fleet.toml" <<'TOML'
[[nodes]]
id = "node-pi5-warden"
role = "warden_guardhouse"
hostname = "warden"
ssh_alias = "warden"
tailscale_ip = "100.110.85.37"
tailscale_name = "warden"
ssh_user = "numenor"
health_url = "http://100.110.85.37:1234/health"
scout_health_url = "http://100.110.85.37:8092/health"
restart_scope = "ssh"
restart_group = "inference"
restart_cmd = "systemctl --user restart llama-server.service"

[[nodes]]
id = "node-pi5-citadel-avatar"
role = "citadel_avatar_controller"
hostname = "raspberrypi"
ssh_alias = "citadel"
ssh_compat_aliases = ["raspberrypi"]
tailscale_ip = "100.119.130.127"
tailscale_name = "raspberrypi-1"
ssh_user = "citadel"
health_url = "http://100.119.130.127:8091/"
restart_scope = "ssh"
restart_group = "presence"
restart_cmd = "systemctl --user restart relic.service citadel-kiosk.service"
TOML

cat >"$BIN_DIR/fake-tailscale" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
ip="${*: -1}"
if [[ "${PI5_FIXTURE_MODE:-}" == "unreachable" && "$ip" == "100.110.85.37" ]]; then
  echo "fixture: warden unreachable" >&2
  exit 1
fi
case "$ip" in
  100.110.85.37) echo "pong from warden (100.110.85.37) via fixture in 1ms" ;;
  100.119.130.127) echo "pong from raspberrypi-1 (100.119.130.127) via fixture in 1ms" ;;
  *) exit 1 ;;
esac
SH

cat >"$BIN_DIR/fake-tcp" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${PI5_FIXTURE_MODE:-}" == "unreachable" && "$1" == "100.110.85.37" ]]; then
  exit 1
fi
exit 0
SH

cat >"$BIN_DIR/fake-curl" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '{"status":"ok"}\n'
SH

cat >"$BIN_DIR/fake-ssh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
while [[ "${1:-}" == -* ]]; do
  case "$1" in
    -o) shift 2 ;;
    *) shift ;;
  esac
done
alias_name="${1:?missing alias}"
shift
command="${1:-true}"
printf '%s|%s\n' "$alias_name" "$command" >>"$PI5_FIXTURE_LOG"

if [[ "${PI5_FIXTURE_MODE:-}" == "unreachable" && "$alias_name" == "warden" ]]; then
  exit 255
fi

if [[ "$command" == *'printf "%s\t%s\t%s\n"'* ]]; then
  if [[ "$alias_name" == "warden" ]]; then
    printf 'numenor\twarden\taarch64\n'
  else
    printf 'citadel\traspberrypi\taarch64\n'
  fi
  exit 0
fi
if [[ "$command" == "sudo -n true" ]]; then
  exit 0
fi
if [[ "$command" == "cat /proc/sys/kernel/random/boot_id" ]]; then
  if [[ "$(<"$PI5_FIXTURE_STATE")" == "before" ]]; then
    echo "11111111-1111-1111-1111-111111111111"
  else
    echo "22222222-2222-2222-2222-222222222222"
  fi
  exit 0
fi
if [[ "$command" == "sudo -n systemctl reboot" ]]; then
  printf 'down\n' >"$PI5_FIXTURE_STATE"
  exit 255
fi
if [[ "$command" == "true" && "$(<"$PI5_FIXTURE_STATE")" == "down" ]]; then
  printf 'after\n' >"$PI5_FIXTURE_STATE"
  exit 255
fi
if [[ "$command" == systemctl\ --user\ is-active* ]]; then
  echo active
  exit 0
fi
if [[ "$command" == systemctl\ --user\ restart* ]]; then
  exit 0
fi
if [[ "$command" == "true" ]]; then
  exit 0
fi
printf 'fixture: unsupported remote command: %s\n' "$command" >&2
exit 2
SH

chmod 0755 "$BIN_DIR"/*
export PI5_FLEET_FILE="$TMP_DIR/fleet.toml"
export PI5_SSH_BIN="$BIN_DIR/fake-ssh"
export PI5_TAILSCALE_BIN="$BIN_DIR/fake-tailscale"
export PI5_TCP_PROBE_BIN="$BIN_DIR/fake-tcp"
export PI5_CURL_BIN="$BIN_DIR/fake-curl"
export PI5_FIXTURE_LOG="$LOG_FILE"
export PI5_FIXTURE_STATE="$STATE_FILE"
export PI5_POLL_INTERVAL=0
export PI5_REBOOT_DOWN_ATTEMPTS=2
export PI5_REBOOT_UP_ATTEMPTS=2
export PI5_HEALTH_ATTEMPTS=2

# An unreachable Warden must not prevent independent CITADEL status checks.
export PI5_FIXTURE_MODE=unreachable
if "$ROOT_DIR/scripts/pi5_outpost_status.sh" all >"$TMP_DIR/status.out"; then
  echo "expected mixed status fixture to fail" >&2
  exit 1
fi
grep -F $'warden\ttailscale\tFAIL' "$TMP_DIR/status.out" >/dev/null
grep -F $'citadel\tssh_identity\tPASS' "$TMP_DIR/status.out" >/dev/null
grep -F $'citadel\thealth:http://100.119.130.127:8091/\tPASS' "$TMP_DIR/status.out" >/dev/null

# An unreachable target must fail before any mutation and never touch CITADEL.
: >"$LOG_FILE"
if "$ROOT_DIR/scripts/pi5_outpost_restart.sh" warden scout >"$TMP_DIR/unreachable.out" 2>&1; then
  echo "expected unreachable restart fixture to fail" >&2
  exit 1
fi
if grep -F 'systemctl --user restart' "$LOG_FILE" >/dev/null; then
  echo "unreachable fixture issued a restart" >&2
  exit 1
fi
if grep -F 'citadel|' "$LOG_FILE" >/dev/null; then
  echo "unreachable Warden fixture contacted CITADEL" >&2
  exit 1
fi

# There is no all-fleet mutation mode.
: >"$LOG_FILE"
if "$ROOT_DIR/scripts/pi5_outpost_restart.sh" all scout >"$TMP_DIR/all.out" 2>&1; then
  echo "expected all-node restart to be rejected" >&2
  exit 1
fi
[[ ! -s "$LOG_FILE" ]]

# A CITADEL presence restart is exact and does not contact Warden.
export PI5_FIXTURE_MODE=healthy
: >"$LOG_FILE"
"$ROOT_DIR/scripts/pi5_outpost_restart.sh" citadel presence >"$TMP_DIR/presence.out"
grep -F "citadel|systemctl --user restart 'relic.service' 'citadel-kiosk.service'" "$LOG_FILE" >/dev/null
if grep -F 'warden|' "$LOG_FILE" >/dev/null; then
  echo "CITADEL restart contacted Warden" >&2
  exit 1
fi
grep -F 'result=pass action=restart node=citadel group=presence' "$TMP_DIR/presence.out" >/dev/null

# Reboot recovery is finite, proves a boot-ID change, and touches only Warden.
export PI5_FIXTURE_MODE=reboot
printf 'before\n' >"$STATE_FILE"
: >"$LOG_FILE"
"$ROOT_DIR/scripts/pi5_outpost_restart.sh" warden node-reboot >"$TMP_DIR/reboot.out"
grep -F 'boot_id_before=11111111-1111-1111-1111-111111111111' "$TMP_DIR/reboot.out" >/dev/null
grep -F 'boot_id_after=22222222-2222-2222-2222-222222222222' "$TMP_DIR/reboot.out" >/dev/null
grep -F 'result=pass action=reboot node=warden group=node-reboot' "$TMP_DIR/reboot.out" >/dev/null
[[ "$(grep -Fc 'warden|sudo -n systemctl reboot' "$LOG_FILE")" -eq 1 ]]
if grep -F 'citadel|' "$LOG_FILE" >/dev/null; then
  echo "Warden reboot contacted CITADEL" >&2
  exit 1
fi

printf 'pi5 outpost helper fixtures passed\n'
