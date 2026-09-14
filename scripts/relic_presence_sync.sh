#!/usr/bin/env bash
# Arda-owned, read-only presence sync to the external RELIC sidecar.
# The CITADEL host never receives the harness port; only sanitized scene state
# is copied over the already verified SSH identity.
set -u

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEFAULT_SYNC_BIN="${HOME}/.local/bin/arda-relic-presence-sync"
if [[ ! -x "$DEFAULT_SYNC_BIN" ]]; then
  DEFAULT_SYNC_BIN="${ARDA_ROOT:-$ROOT_DIR}/target/release/arda-relic-presence-sync"
fi
SYNC_BIN="${ARDA_RELIC_SYNC_BIN:-$DEFAULT_SYNC_BIN}"
PRESENCE_URL="${ARDA_RELIC_PRESENCE_URL:-http://127.0.0.1:7878/v1/presence/snapshot}"
REMOTE_HOST="${ARDA_RELIC_REMOTE_HOST:-citadel}"
REMOTE_ROOT="${ARDA_RELIC_REMOTE_ROOT:-annunimas_embodied/relic}"
REMOTE_STATE="${REMOTE_ROOT}/public/scene.json"
INTERVAL_SECONDS="${ARDA_RELIC_SYNC_INTERVAL_SECONDS:-3}"
RUNTIME_ROOT="${XDG_RUNTIME_DIR:-/tmp}/arda-relic-bridge"
LOCAL_STATE="${RUNTIME_ROOT}/scene.json"
DELIVERY_ACK_PATH="${ARDA_RELIC_DELIVERY_ACK_PATH:-${ARDA_ROOT:-$ROOT_DIR}/data/relic/delivery_ack.json}"
SSH_OPTS=(-o BatchMode=yes -o ConnectTimeout=6 -o ConnectionAttempts=1 -o ServerAliveInterval=10)

mkdir -p "$RUNTIME_ROOT"

while true; do
  if "$SYNC_BIN" "$PRESENCE_URL" "$LOCAL_STATE"; then
    temporary="${REMOTE_STATE}.new"
    if scp "${SSH_OPTS[@]}" "$LOCAL_STATE" "${REMOTE_HOST}:${temporary}" >/dev/null 2>&1 && \
       ssh "${SSH_OPTS[@]}" "$REMOTE_HOST" \
         "install -m 0644 '${temporary}' '${REMOTE_STATE}' && rm -f '${temporary}'" >/dev/null 2>&1; then
      python3 - "$DELIVERY_ACK_PATH" "$REMOTE_HOST" "$REMOTE_STATE" "$LOCAL_STATE" <<'PY'
import datetime
import hashlib
import json
import os
from pathlib import Path
import sys

path = Path(sys.argv[1])
local_state = Path(sys.argv[4])
payload = {
    "schema_version": "arda.relic.delivery-ack.v1",
    "delivery_state": "confirmed_remote_install",
    "acknowledgement_method": "ssh_command_success",
    "remote_host": sys.argv[2],
    "remote_state_path": sys.argv[3],
    "scene_sha256": hashlib.sha256(local_state.read_bytes()).hexdigest(),
    "acknowledged_at_utc": datetime.datetime.now(datetime.timezone.utc).isoformat().replace("+00:00", "Z"),
}
path.parent.mkdir(parents=True, exist_ok=True)
temporary = path.with_suffix(f".tmp.{os.getpid()}")
temporary.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
os.replace(temporary, path)
PY
    else
      printf 'relic presence sync: CITADEL copy failed; no delivery acknowledgement recorded\n' >&2
    fi
  fi
  sleep "$INTERVAL_SECONDS"
done
