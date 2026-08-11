#!/usr/bin/env bash
# Read-only, independently gated status checks for the two canonical Pi5 outposts.
set -uo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FLEET_FILE="${PI5_FLEET_FILE:-${ROOT_DIR}/config/fleet.toml}"
SSH_BIN="${PI5_SSH_BIN:-ssh}"
TAILSCALE_BIN="${PI5_TAILSCALE_BIN:-tailscale}"
CURL_BIN="${PI5_CURL_BIN:-curl}"
TCP_PROBE_BIN="${PI5_TCP_PROBE_BIN:-}"
SSH_OPTS=(-o BatchMode=yes -o ConnectTimeout=6 -o ConnectionAttempts=1)
FAILURES=0

usage() {
  cat <<'USAGE'
Usage: scripts/pi5_outpost_status.sh <warden|citadel|all>

Runs separate fleet-record, Tailscale, TCP/22, SSH identity, named-unit,
and HTTP health gates. This helper is read-only and never restarts services.
USAGE
}

report() {
  local node="$1" gate="$2" status="$3" detail="$4"
  detail="${detail//$'\n'/; }"
  detail="${detail//$'\t'/ }"
  printf '%s\t%s\t%s\t%s\n' "$node" "$gate" "$status" "$detail"
  if [[ "$status" == "FAIL" ]]; then
    FAILURES=$((FAILURES + 1))
  fi
}

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
    raise SystemExit(f"expected exactly one {want['id']} record, found {len(matches)}")
node = matches[0]
required = (
    "id", "role", "hostname", "ssh_alias", "ssh_user", "tailscale_ip",
    "tailscale_name", "health_url", "restart_scope", "restart_group", "restart_cmd",
)
missing = [key for key in required if not node.get(key)]
if missing:
    raise SystemExit("missing required fields: " + ",".join(missing))
for key, value in want.items():
    if node.get(key) != value:
        raise SystemExit(f"{key} mismatch: expected {value!r}, got {node.get(key)!r}")
if selector == "warden" and not node.get("scout_health_url"):
    raise SystemExit("missing required field: scout_health_url")
if selector == "citadel" and "raspberrypi" not in node.get("ssh_compat_aliases", []):
    raise SystemExit("citadel compatibility alias raspberrypi is missing")
for key in (
    "id", "role", "hostname", "ssh_alias", "ssh_user", "tailscale_ip",
    "tailscale_name", "health_url", "scout_health_url", "restart_scope",
    "restart_group", "restart_cmd",
):
    print(str(node.get(key, "")))
PY
  )
  if [[ ${#NODE_FIELDS[@]} -ne 12 ]]; then
    LOAD_ERROR="${NODE_FIELDS[*]:-unable to parse fleet record}"
    return 1
  fi
  NODE_ID="${NODE_FIELDS[0]}"
  NODE_ROLE="${NODE_FIELDS[1]}"
  NODE_HOSTNAME="${NODE_FIELDS[2]}"
  NODE_ALIAS="${NODE_FIELDS[3]}"
  NODE_USER="${NODE_FIELDS[4]}"
  NODE_IP="${NODE_FIELDS[5]}"
  NODE_TS_NAME="${NODE_FIELDS[6]}"
  NODE_HEALTH="${NODE_FIELDS[7]}"
  NODE_SCOUT_HEALTH="${NODE_FIELDS[8]}"
  NODE_RESTART_GROUP="${NODE_FIELDS[10]}"
}

probe_tcp() {
  local ip="$1"
  if [[ -n "$TCP_PROBE_BIN" ]]; then
    "$TCP_PROBE_BIN" "$ip" 22
  else
    timeout 6 python3 -c \
      'import socket,sys; s=socket.create_connection((sys.argv[1], int(sys.argv[2])), 5); s.close()' \
      "$ip" 22
  fi
}

check_node() {
  local selector="$1" output rc identity_user identity_host identity_arch
  local -a units health_urls

  if ! load_node "$selector"; then
    report "$selector" fleet_record FAIL "$LOAD_ERROR"
    return
  fi
  report "$selector" fleet_record PASS \
    "id=${NODE_ID} role=${NODE_ROLE} alias=${NODE_ALIAS} user=${NODE_USER} ip=${NODE_IP} restart=${NODE_RESTART_GROUP}"

  output="$(timeout 8 "$TAILSCALE_BIN" ping --c 1 --timeout 5s "$NODE_IP" 2>&1)"
  rc=$?
  if [[ $rc -eq 0 && "$output" == *"${NODE_TS_NAME}"* && "$output" == *"${NODE_IP}"* ]]; then
    report "$selector" tailscale PASS "$output"
  else
    report "$selector" tailscale FAIL "rc=${rc} expected=${NODE_TS_NAME}/${NODE_IP} ${output}"
  fi

  output="$(probe_tcp "$NODE_IP" 2>&1)"
  rc=$?
  if [[ $rc -eq 0 ]]; then
    report "$selector" tcp_22 PASS "${NODE_IP}:22 reachable"
  else
    report "$selector" tcp_22 FAIL "rc=${rc} ${NODE_IP}:22 unreachable ${output}"
  fi

  # Expansion is intentionally deferred to the fixed remote identity probe.
  # shellcheck disable=SC2016
  output="$(timeout 10 "$SSH_BIN" "${SSH_OPTS[@]}" "$NODE_ALIAS" \
    'printf "%s\t%s\t%s\n" "$(id -un)" "$(hostname)" "$(uname -m)"' 2>&1)"
  rc=$?
  if [[ $rc -eq 0 ]]; then
    IFS=$'\t' read -r identity_user identity_host identity_arch <<<"$output"
  else
    identity_user=""; identity_host=""; identity_arch=""
  fi
  if [[ $rc -eq 0 && "$identity_user" == "$NODE_USER" && \
        "$identity_host" == "$NODE_HOSTNAME" && "$identity_arch" == "aarch64" ]]; then
    report "$selector" ssh_identity PASS \
      "${identity_user}@${NODE_IP} hostname=${identity_host} arch=${identity_arch} alias=${NODE_ALIAS}"
  else
    report "$selector" ssh_identity FAIL \
      "rc=${rc} expected=${NODE_USER}@${NODE_HOSTNAME}/aarch64 observed=${output}"
  fi

  if [[ "$selector" == "warden" ]]; then
    units=(arda-warden-scout.service llama-server.service arda-warden-searxng.service)
    health_urls=("$NODE_SCOUT_HEALTH" "$NODE_HEALTH")
  else
    units=(relic.service citadel-kiosk.service)
    health_urls=("$NODE_HEALTH")
  fi

  for unit in "${units[@]}"; do
    output="$(timeout 10 "$SSH_BIN" "${SSH_OPTS[@]}" "$NODE_ALIAS" \
      "systemctl --user is-active '${unit}'" 2>&1)"
    rc=$?
    if [[ $rc -eq 0 && "$output" == "active" ]]; then
      report "$selector" "unit:${unit}" PASS active
    else
      report "$selector" "unit:${unit}" FAIL "rc=${rc} state=${output}"
    fi
  done

  for url in "${health_urls[@]}"; do
    output="$("$CURL_BIN" --fail --silent --show-error --max-time 6 "$url" 2>&1)"
    rc=$?
    if [[ $rc -eq 0 && -n "$output" ]]; then
      report "$selector" "health:${url}" PASS "http_ok bytes=${#output}"
    else
      report "$selector" "health:${url}" FAIL "rc=${rc} ${output}"
    fi
  done
}

case "${1:-}" in
  warden|citadel)
    check_node "$1"
    ;;
  all)
    check_node warden
    check_node citadel
    ;;
  -h|--help)
    usage
    exit 0
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac

if (( FAILURES > 0 )); then
  printf 'summary\tstatus\tFAIL\tfailures=%d\n' "$FAILURES"
  exit 1
fi
printf 'summary\tstatus\tPASS\tfailures=0\n'
