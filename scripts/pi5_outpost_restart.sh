#!/usr/bin/env bash
# Explicit-node, allowlisted restart/reboot actions for canonical Pi5 outposts.
set -uo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FLEET_FILE="${PI5_FLEET_FILE:-${ROOT_DIR}/config/fleet.toml}"
SSH_BIN="${PI5_SSH_BIN:-ssh}"
TAILSCALE_BIN="${PI5_TAILSCALE_BIN:-tailscale}"
CURL_BIN="${PI5_CURL_BIN:-curl}"
POLL_INTERVAL="${PI5_POLL_INTERVAL:-2}"
REBOOT_DOWN_ATTEMPTS="${PI5_REBOOT_DOWN_ATTEMPTS:-30}"
REBOOT_UP_ATTEMPTS="${PI5_REBOOT_UP_ATTEMPTS:-90}"
HEALTH_ATTEMPTS="${PI5_HEALTH_ATTEMPTS:-30}"
SSH_OPTS=(-o BatchMode=yes -o ConnectTimeout=6 -o ConnectionAttempts=1)

usage() {
  cat <<'USAGE'
Usage: scripts/pi5_outpost_restart.sh <warden|citadel> <service-group>

Allowlisted service groups:
  warden   scout | inference | node-reboot
  citadel  presence | node-reboot

There is deliberately no all-node or caller-supplied unit/command mode.
USAGE
}

die() {
  printf 'pi5 outpost action: %s\n' "$*" >&2
  exit 1
}

for value in "$REBOOT_DOWN_ATTEMPTS" "$REBOOT_UP_ATTEMPTS" "$HEALTH_ATTEMPTS"; do
  [[ "$value" =~ ^[1-9][0-9]*$ ]] || die "attempt limits must be positive integers"
done
[[ "$POLL_INTERVAL" =~ ^[0-9]+([.][0-9]+)?$ ]] || die "poll interval must be numeric"

load_node() {
  local selector="$1"
  mapfile -t NODE_FIELDS < <(python3 - "$FLEET_FILE" "$selector" <<'PY'
import sys
import tomllib

path, selector = sys.argv[1:]
expected = {
    "warden": {
        "id": "node-pi5-warden",
        "role": "warden_guardhouse",
        "hostname": "warden",
        "ssh_alias": "warden",
        "ssh_user": "numenor",
        "tailscale_name": "warden",
        "restart_scope": "ssh",
        "restart_group": "inference",
        "restart_cmd": "systemctl --user restart llama-server.service",
    },
    "citadel": {
        "id": "node-pi5-citadel-avatar",
        "role": "citadel_avatar_controller",
        "hostname": "raspberrypi",
        "ssh_alias": "citadel",
        "ssh_user": "citadel",
        "tailscale_name": "raspberrypi-1",
        "restart_scope": "ssh",
        "restart_group": "presence",
        "restart_cmd": "systemctl --user restart relic.service citadel-kiosk.service",
    },
}
want = expected[selector]
with open(path, "rb") as handle:
    nodes = tomllib.load(handle).get("nodes", [])
matches = [node for node in nodes if node.get("id") == want["id"]]
if len(matches) != 1:
    raise SystemExit(f"expected one {want['id']} record, found {len(matches)}")
node = matches[0]
for key in (
    "id", "role", "hostname", "ssh_alias", "ssh_user", "tailscale_ip",
    "tailscale_name", "health_url", "restart_scope", "restart_group", "restart_cmd",
):
    if not node.get(key):
        raise SystemExit(f"missing required field: {key}")
for key, value in want.items():
    if node.get(key) != value:
        raise SystemExit(f"{key} mismatch: expected {value!r}, got {node.get(key)!r}")
if selector == "warden" and not node.get("scout_health_url"):
    raise SystemExit("missing required field: scout_health_url")
for key in (
    "id", "hostname", "ssh_alias", "ssh_user", "tailscale_ip", "tailscale_name",
    "health_url", "scout_health_url",
):
    print(str(node.get(key, "")))
PY
  )
  [[ ${#NODE_FIELDS[@]} -eq 8 ]] || die "invalid or incomplete ${selector} fleet record"
  NODE_ID="${NODE_FIELDS[0]}"
  NODE_HOSTNAME="${NODE_FIELDS[1]}"
  NODE_ALIAS="${NODE_FIELDS[2]}"
  NODE_USER="${NODE_FIELDS[3]}"
  NODE_IP="${NODE_FIELDS[4]}"
  NODE_TS_NAME="${NODE_FIELDS[5]}"
  NODE_HEALTH="${NODE_FIELDS[6]}"
  NODE_SCOUT_HEALTH="${NODE_FIELDS[7]}"
}

ssh_node() {
  # Commands passed here are constants selected by the node/group allowlist below.
  # shellcheck disable=SC2029
  timeout 10 "$SSH_BIN" "${SSH_OPTS[@]}" "$NODE_ALIAS" "$1"
}

verify_transport_identity() {
  local output user host arch
  output="$(timeout 8 "$TAILSCALE_BIN" ping --c 1 --timeout 5s "$NODE_IP" 2>&1)" || \
    die "Tailscale gate failed for ${NODE_ALIAS}: ${output}"
  [[ "$output" == *"${NODE_TS_NAME}"* && "$output" == *"${NODE_IP}"* ]] || \
    die "Tailscale identity mismatch for ${NODE_ALIAS}: ${output}"

  # Expansion is intentionally deferred to the fixed remote identity probe.
  # shellcheck disable=SC2016
  output="$(ssh_node 'printf "%s\t%s\t%s\n" "$(id -un)" "$(hostname)" "$(uname -m)"' 2>&1)" || \
    die "SSH identity gate failed for ${NODE_ALIAS}: ${output}"
  IFS=$'\t' read -r user host arch <<<"$output"
  [[ "$user" == "$NODE_USER" && "$host" == "$NODE_HOSTNAME" && "$arch" == "aarch64" ]] || \
    die "SSH identity mismatch: expected ${NODE_USER}@${NODE_HOSTNAME}/aarch64, got ${output}"
  printf 'preflight=pass node=%s alias=%s identity=%s@%s tailscale=%s\n' \
    "$NODE_ID" "$NODE_ALIAS" "$user" "$host" "$NODE_TS_NAME"
}

wait_health() {
  local url="$1" attempt
  for ((attempt = 1; attempt <= HEALTH_ATTEMPTS; attempt++)); do
    if "$CURL_BIN" --fail --silent --show-error --max-time 5 "$url" >/dev/null 2>&1; then
      printf 'health=pass url=%s attempt=%d\n' "$url" "$attempt"
      return 0
    fi
    sleep "$POLL_INTERVAL"
  done
  return 1
}

verify_units() {
  local unit state
  for unit in "$@"; do
    state="$(ssh_node "systemctl --user is-active '${unit}'" 2>&1)" || \
      die "unit check failed: ${unit} state=${state}"
    [[ "$state" == "active" ]] || die "unit is not active: ${unit} state=${state}"
    printf 'unit=pass name=%s state=active\n' "$unit"
  done
}

restart_group() {
  local remote_cmd health_url
  local -a units
  case "${NODE_SELECTOR}:${SERVICE_GROUP}" in
    warden:scout)
      units=(arda-warden-scout.service)
      health_url="$NODE_SCOUT_HEALTH"
      ;;
    warden:inference)
      units=(llama-server.service)
      health_url="$NODE_HEALTH"
      ;;
    citadel:presence)
      units=(relic.service citadel-kiosk.service)
      health_url="$NODE_HEALTH"
      ;;
    *)
      die "service group ${SERVICE_GROUP} is not allowlisted for ${NODE_SELECTOR}"
      ;;
  esac

  remote_cmd="systemctl --user restart"
  local unit
  for unit in "${units[@]}"; do
    remote_cmd+=" '${unit}'"
  done
  printf 'mutation_scope=node:%s group:%s units:%s\n' \
    "$NODE_SELECTOR" "$SERVICE_GROUP" "${units[*]}"
  ssh_node "$remote_cmd" || die "restart command failed for ${NODE_SELECTOR}/${SERVICE_GROUP}"
  verify_units "${units[@]}"
  wait_health "$health_url" || die "health did not recover: ${health_url}"
  printf 'result=pass action=restart node=%s group=%s\n' "$NODE_SELECTOR" "$SERVICE_GROUP"
}

wait_for_ssh_down() {
  local attempt
  for ((attempt = 1; attempt <= REBOOT_DOWN_ATTEMPTS; attempt++)); do
    if ! timeout 3 "$SSH_BIN" "${SSH_OPTS[@]}" "$NODE_ALIAS" true >/dev/null 2>&1; then
      return 0
    fi
    sleep "$POLL_INTERVAL"
  done
  return 1
}

wait_for_ssh_up() {
  local attempt
  for ((attempt = 1; attempt <= REBOOT_UP_ATTEMPTS; attempt++)); do
    if timeout 5 "$SSH_BIN" "${SSH_OPTS[@]}" "$NODE_ALIAS" true >/dev/null 2>&1; then
      return 0
    fi
    sleep "$POLL_INTERVAL"
  done
  return 1
}

reboot_node() {
  local before after
  local -a units health_urls
  ssh_node 'sudo -n true' >/dev/null || die "passwordless sudo preflight failed"
  before="$(ssh_node 'cat /proc/sys/kernel/random/boot_id')" || die "could not read pre-reboot boot ID"
  [[ "$before" =~ ^[0-9a-f-]{36}$ ]] || die "invalid pre-reboot boot ID: ${before}"
  printf 'mutation_scope=node:%s group:node-reboot\nboot_id_before=%s\n' "$NODE_SELECTOR" "$before"

  ssh_node 'sudo -n systemctl reboot' >/dev/null 2>&1 || true
  wait_for_ssh_down || die "node never became unreachable during reboot"
  wait_for_ssh_up || die "node did not return after reboot"
  verify_transport_identity
  after="$(ssh_node 'cat /proc/sys/kernel/random/boot_id')" || die "could not read post-reboot boot ID"
  [[ "$after" =~ ^[0-9a-f-]{36}$ && "$after" != "$before" ]] || \
    die "boot ID did not change: before=${before} after=${after}"

  if [[ "$NODE_SELECTOR" == "warden" ]]; then
    units=(arda-warden-scout.service llama-server.service arda-warden-searxng.service)
    health_urls=("$NODE_SCOUT_HEALTH" "$NODE_HEALTH")
  else
    units=(relic.service citadel-kiosk.service)
    health_urls=("$NODE_HEALTH")
  fi
  verify_units "${units[@]}"
  local url
  for url in "${health_urls[@]}"; do
    wait_health "$url" || die "health did not recover after reboot: ${url}"
  done
  printf 'boot_id_after=%s\nresult=pass action=reboot node=%s group=node-reboot\n' \
    "$after" "$NODE_SELECTOR"
}

NODE_SELECTOR="${1:-}"
SERVICE_GROUP="${2:-}"
case "$NODE_SELECTOR" in
  warden|citadel) ;;
  -h|--help) usage; exit 0 ;;
  *) usage >&2; exit 2 ;;
esac
[[ -n "$SERVICE_GROUP" ]] || { usage >&2; exit 2; }
case "${NODE_SELECTOR}:${SERVICE_GROUP}" in
  warden:scout|warden:inference|citadel:presence|warden:node-reboot|citadel:node-reboot) ;;
  *) die "service group ${SERVICE_GROUP} is not allowlisted for ${NODE_SELECTOR}" ;;
esac

load_node "$NODE_SELECTOR"
verify_transport_identity
if [[ "$SERVICE_GROUP" == "node-reboot" ]]; then
  reboot_node
else
  restart_group
fi
