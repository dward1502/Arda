#!/usr/bin/env bash
# Probe the authoritative Arda Manwe inference router and recover it only after
# consecutive failures. This replaces the retired Annunimas/Charon probe lane.
set -euo pipefail

HOST="${ARDA_MANWE_HTTP_HOST:-127.0.0.1}"
PORT="${ARDA_MANWE_HTTP_PORT:-5110}"
BASE_URL="http://${HOST}:${PORT}"
COMPLETIONS_URL="${BASE_URL}/v1/chat/completions"
PAYLOAD='{"model":"auto","messages":[{"role":"user","content":"Reply with exactly: ok"}],"max_tokens":8}'
STATE_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/arda"
STATE_FILE="${STATE_DIR}/manwe_inference_failures"
FAIL_THRESHOLD="${ARDA_MANWE_INFERENCE_FAIL_THRESHOLD:-2}"
PROBE_TIMEOUT="${ARDA_MANWE_INFERENCE_TIMEOUT:-30}"
READY_TIMEOUT="${ARDA_MANWE_INFERENCE_READY_TIMEOUT:-25}"

mkdir -p "$STATE_DIR"

read_count() { [[ -r "$STATE_FILE" ]] && command cat "$STATE_FILE" 2>/dev/null || printf '0\n'; }
write_count() { printf '%s\n' "$1" >"$STATE_FILE"; }
probe_inference() {
  curl -4 --noproxy '*' --retry 2 --retry-all-errors --retry-connrefused --retry-delay 1 \
    -fsS --connect-timeout 2 --max-time "$PROBE_TIMEOUT" \
    -H 'Content-Type: application/json' -d "$PAYLOAD" "$COMPLETIONS_URL" >/dev/null
}

if probe_inference; then
  write_count 0
  exit 0
fi

count=$(read_count)
count=$((count + 1))
write_count "$count"

if (( count < FAIL_THRESHOLD )); then
  printf 'manwe inference probe: failure %s/%s — not restarting yet\n' "$count" "$FAIL_THRESHOLD" >&2
  exit 0
fi

printf 'manwe inference probe: %s consecutive failures — restarting arda-manwe.service\n' "$count" >&2
systemctl --user restart arda-manwe.service
write_count 0

deadline=$(( $(date +%s) + READY_TIMEOUT ))
until curl -4 --noproxy '*' -fsS --connect-timeout 1 --max-time 3 "${BASE_URL}/health" >/dev/null; do
  if (( $(date +%s) >= deadline )); then
    printf 'manwe inference probe: %s/health was not ready after %ss\n' "$BASE_URL" "$READY_TIMEOUT" >&2
    exit 1
  fi
  sleep 1
done
probe_inference
