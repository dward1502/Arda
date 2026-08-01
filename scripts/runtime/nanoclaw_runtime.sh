#!/usr/bin/env bash
# sigil: ANKH
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

ACTION="${1:-status}"

load_env() {
  if [[ -f config/.env ]]; then
    set -a
    # shellcheck disable=SC1091
    source config/.env
    set +a
  fi
}

load_env

PROJECT_ROOT="${ARDA_NANOCLAW_ROOT:-}"
EDGE_TARGET="${ARDA_NANOCLAW_EDGE_TARGET:-node-pi5-warden}"
EDGE_TRANSPORT="${ARDA_NANOCLAW_EDGE_TRANSPORT:-tailscale}"
CONTROL_MODE="${ARDA_NANOCLAW_CONTROL_MODE:-headless}"
EDGE_TARGETS_PATH="${ARDA_EDGE_TARGETS_PATH:-core/edge/targets.toml}"
NODE_BIN="${ARDA_NANOCLAW_NODE_BIN:-node}"
ENTRYPOINT="$PROJECT_ROOT/dist/index.js"
PID_FILE="$PROJECT_ROOT/nanoclaw.pid"
LOG_FILE="$PROJECT_ROOT/logs/nanoclaw.log"
ERR_FILE="$PROJECT_ROOT/logs/nanoclaw.error.log"
AUTH_DIR="$PROJECT_ROOT/store/auth"
DB_PATH="$PROJECT_ROOT/store/messages.db"

json_bool() {
  if [[ "$1" == "true" ]]; then
    printf 'true'
  else
    printf 'false'
  fi
}

project_exists="false"
binary_present="false"
container_runtime="none"
container_runtime_ready="false"
tailscale_ready="false"
edge_target_visible="false"
auth_ready="false"
db_present="false"
running="false"
pid_json="null"
root_configured="false"
legacy_root="false"
edge_match_tokens=("$EDGE_TARGET")

if [[ -n "$PROJECT_ROOT" ]]; then
  root_configured="true"
fi
if [[ "$PROJECT_ROOT" == /var/home/dward/Numenor_Prime/* ]]; then
  legacy_root="true"
fi
[[ -n "$PROJECT_ROOT" && -d "$PROJECT_ROOT" ]] && project_exists="true"
command -v nanoclaw >/dev/null 2>&1 && binary_present="true"
if command -v podman >/dev/null 2>&1; then
  container_runtime="podman"
  if podman info >/dev/null 2>&1; then
    container_runtime_ready="true"
  fi
elif command -v docker >/dev/null 2>&1; then
  container_runtime="docker"
  if docker info >/dev/null 2>&1; then
    container_runtime_ready="true"
  fi
fi
if [[ -f "$EDGE_TARGETS_PATH" ]]; then
  while IFS= read -r token; do
    [[ -n "$token" ]] && edge_match_tokens+=("$token")
  done < <(
    awk -v target="$EDGE_TARGET" '
      /^\[\[node\]\]/ { in_node=0; id=""; hostname=""; ip="" }
      $1=="id" && $3 ~ /./ {
        gsub(/"/, "", $3)
        id=$3
        if (id == target) in_node=1
      }
      in_node && $1=="hostname" {
        gsub(/"/, "", $3)
        hostname=$3
      }
      in_node && $1=="tailscale_ip" {
        gsub(/"/, "", $3)
        ip=$3
      }
      /^\[\[node\]\]/ && NR>1 && in_node==1 {
        if (hostname != "") print hostname
        if (ip != "") print ip
        in_node=0
      }
      END {
        if (in_node==1) {
          if (hostname != "") print hostname
          if (ip != "") print ip
        }
      }
    ' "$EDGE_TARGETS_PATH" | sed '/^$/d'
  )
fi
if tailscale status --json >/tmp/arda-nanoclaw-tailscale.json 2>/dev/null; then
  tailscale_ready="true"
  for token in "${edge_match_tokens[@]}"; do
    if [[ -n "$token" ]] && grep -q "\"$token\"" /tmp/arda-nanoclaw-tailscale.json 2>/dev/null; then
      edge_target_visible="true"
      break
    fi
  done
fi
rm -f /tmp/arda-nanoclaw-tailscale.json
if [[ -d "$AUTH_DIR" ]] && find "$AUTH_DIR" -maxdepth 1 -type f | grep -q .; then
  auth_ready="true"
fi
[[ -f "$DB_PATH" ]] && db_present="true"

running_pid() {
  if [[ -f "$PID_FILE" ]]; then
    local pid
    pid="$(cat "$PID_FILE" 2>/dev/null || true)"
    if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
      printf '%s' "$pid"
      return 0
    fi
  fi
  local pid
  pid="$(pgrep -f "$ENTRYPOINT" | head -n 1 || true)"
  if [[ -n "$pid" ]]; then
    printf '%s' "$pid" > "$PID_FILE"
    printf '%s' "$pid"
    return 0
  fi
  return 1
}

if pid="$(running_pid 2>/dev/null)"; then
  running="true"
  pid_json="$pid"
fi

current_status() {
  if [[ "$root_configured" != "true" ]]; then
    printf 'not_configured'
  elif [[ "$legacy_root" == "true" && "$project_exists" != "true" ]]; then
    printf 'legacy_path_configured'
  elif [[ "$running" == "true" ]]; then
    printf 'running'
  elif [[ "$project_exists" != "true" || "$binary_present" != "true" ]]; then
    printf 'missing_install'
  elif [[ "$container_runtime_ready" != "true" ]]; then
    printf 'runtime_blocked'
  elif [[ "$CONTROL_MODE" == "whatsapp" && "$auth_ready" != "true" ]]; then
    printf 'auth_required'
  elif [[ "$CONTROL_MODE" != "whatsapp" && "$tailscale_ready" != "true" && "$EDGE_TRANSPORT" == "tailscale" ]]; then
    printf 'runtime_blocked'
  elif [[ "$CONTROL_MODE" != "whatsapp" && "$EDGE_TRANSPORT" == "tailscale" && "$edge_target_visible" != "true" ]]; then
    printf 'contract_ready'
  elif [[ "$auth_ready" == "true" || "$CONTROL_MODE" != "whatsapp" ]]; then
    printf 'contract_ready'
  else
    printf 'runtime_blocked'
  fi
}

emit_status() {
  local status
  status="$(current_status)"
  local runtime_ready="false"
  if [[ "$status" == "running" || "$status" == "contract_ready" ]]; then
    runtime_ready="true"
  fi
  printf '{'
  printf '"status":"%s",' "$status"
  printf '"pid":%s,' "$pid_json"
  printf '"project_root":"%s",' "$PROJECT_ROOT"
  printf '"entrypoint":"%s",' "$ENTRYPOINT"
  printf '"pid_file":"%s",' "$PID_FILE"
  printf '"log_path":"%s",' "$LOG_FILE"
  printf '"error_log_path":"%s",' "$ERR_FILE"
  printf '"auth_dir":"%s",' "$AUTH_DIR"
  printf '"db_path":"%s",' "$DB_PATH"
  printf '"root_configured":%s,' "$(json_bool "$root_configured")"
  printf '"legacy_root":%s,' "$(json_bool "$legacy_root")"
  printf '"container_runtime":"%s",' "$container_runtime"
  printf '"container_runtime_ready":%s,' "$(json_bool "$container_runtime_ready")"
  printf '"tailscale_ready":%s,' "$(json_bool "$tailscale_ready")"
  printf '"edge_target":"%s",' "$EDGE_TARGET"
  printf '"edge_transport":"%s",' "$EDGE_TRANSPORT"
  printf '"control_mode":"%s",' "$CONTROL_MODE"
  printf '"edge_target_visible":%s,' "$(json_bool "$edge_target_visible")"
  printf '"auth_ready":%s,' "$(json_bool "$auth_ready")"
  printf '"db_present":%s,' "$(json_bool "$db_present")"
  printf '"runtime_ready":%s,' "$(json_bool "$runtime_ready")"
  printf '"binary_present":%s' "$(json_bool "$binary_present")"
  printf '}\n'
}

case "$ACTION" in
  status|test)
    emit_status
    ;;
  start)
    if [[ "$(current_status)" == "running" ]]; then
      emit_status
      exit 0
    fi
    if [[ "$project_exists" != "true" || "$binary_present" != "true" ]]; then
      emit_status
      exit 1
    fi
    if [[ "$container_runtime_ready" != "true" ]]; then
      emit_status
      exit 1
    fi
    if [[ "$CONTROL_MODE" == "whatsapp" && "$auth_ready" != "true" ]]; then
      emit_status
      exit 1
    fi
    mkdir -p "$(dirname "$LOG_FILE")"
    : > "$LOG_FILE"
    : > "$ERR_FILE"
    (
      cd "$PROJECT_ROOT"
      setsid "$NODE_BIN" "$ENTRYPOINT" </dev/null >>"$LOG_FILE" 2>>"$ERR_FILE" &
      echo $! > "$PID_FILE"
    )
    sleep 2
    emit_status
    ;;
  stop)
    if pid="$(running_pid 2>/dev/null)"; then
      kill "$pid" 2>/dev/null || true
      rm -f "$PID_FILE"
    fi
    running="false"
    pid_json="null"
    emit_status
    ;;
  *)
    echo "usage: $0 {start|stop|status|test}" >&2
    exit 2
    ;;
esac
