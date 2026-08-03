#!/usr/bin/env bash
# Deploy the Arda-owned RELIC bridge + sidecar integration only with explicit --apply.
# Defaults to --dry-run to never mutate local services or CITADEL without approval.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RELIC_ROOT="${ARDA_RELIC_KIOSK_ROOT:-/var/home/mythos/Eregion/relic-kiosk}"
REMOTE_HOST="${ARDA_RELIC_REMOTE_HOST:-citadel}"
REMOTE_ROOT="${ARDA_RELIC_REMOTE_ROOT:-/home/citadel/annunimas_embodied/relic}"
SSH_OPTS=(-o BatchMode=yes -o ConnectTimeout=6 -o ConnectionAttempts=1 -o ServerAliveInterval=10)
APPLY=false

usage() {
  printf 'Usage: %s [--dry-run|--apply]\n' "$0"
}

case "${1:---dry-run}" in
  --dry-run) ;;
  --apply) APPLY=true ;;
  -h|--help) usage; exit 0 ;;
  *) usage >&2; exit 2 ;;
esac

# --- preflight: validate sidecar + build bridge binary ---
echo "preflight: sidecar validation"
npm --prefix "$RELIC_ROOT" run validate

echo "preflight: build bridge"
cargo build --release -p arda-relic-bridge --bin arda-relic-presence-sync

printf 'validated_sidecar=%s\n' "$RELIC_ROOT"
printf 'bridge_binary=%s\n' "${ROOT_DIR}/target/release/arda-relic-presence-sync"
printf 'target=%s:%s\n' "$REMOTE_HOST" "$REMOTE_ROOT"

# --- preflight: live presence endpoint ---
preflight="$(mktemp)"
trap 'rm -f "$preflight"' EXIT
curl --fail --silent --show-error --max-time 5 \
  "${ARDA_RELIC_PRESENCE_URL:-http://127.0.0.1:7878/v1/presence/snapshot}" > "$preflight"
python3 - "$preflight" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as handle:
    body = json.load(handle)
snapshot = body.get("snapshot", body)
if snapshot.get("schema_version") != "arda.runtime-presence.v1":
    raise SystemExit("presence preflight schema mismatch")
if not isinstance(body.get("snapshot_sequence"), int):
    raise SystemExit("presence preflight sequence missing")
print("presence preflight ok: seq=%s nodes=%s" % (body.get("snapshot_sequence"), len(snapshot.get("nodes", []))))
PY

if [[ "$APPLY" != true ]]; then
  printf 'dry_run=true; no local service or CITADEL mutation performed\n'
  exit 0
fi

# --- preservation before mutation ---
echo "preservation: saving copies of remote sidecar files before overwrite"
ssh "${SSH_OPTS[@]}" "$REMOTE_HOST" \
  "mkdir -p '$REMOTE_ROOT/.relic-backup' && \
   cp -an '$REMOTE_ROOT/index.html' '$REMOTE_ROOT/styles.css' '$REMOTE_ROOT/src/relic.js' '$REMOTE_ROOT/src/relicSceneState.js' '$REMOTE_ROOT/.relic-backup/' 2>/dev/null || true;
   echo 'remote backup ok'"

# --- install local bridge service ---
echo "deploy: local bridge service"
install -Dm0755 "${ROOT_DIR}/target/release/arda-relic-presence-sync" ~/.local/bin/arda-relic-presence-sync
install -Dm0644 "${ROOT_DIR}/config/systemd/arda-relic-bridge.service" \
  "${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user/arda-relic-bridge.service"
systemctl_user() {
  systemctl --user "$@" 2>/dev/null || \
    systemctl --user --machine="$(id -un)@.host" "$@"
}
systemctl_user daemon-reload
systemctl_user enable --now arda-relic-bridge.service

# --- stage remote sidecar atomically ---
echo "deploy: remote sidecar (atomic)"
stage="${REMOTE_ROOT}/.relic-stage"
ssh "${SSH_OPTS[@]}" "$REMOTE_HOST" "rm -rf '$stage' && mkdir -p '$stage/src' '$stage/public'"
scp "${SSH_OPTS[@]}" \
  "$RELIC_ROOT/index.html" \
  "$RELIC_ROOT/styles.css" \
  "$RELIC_ROOT/src/relic.js" \
  "$RELIC_ROOT/src/relicSceneState.js" \
  "$REMOTE_HOST:$stage/"
ssh "${SSH_OPTS[@]}" "$REMOTE_HOST" \
  "cp '$stage/index.html' '$stage/styles.css' '$REMOTE_ROOT/' && cp '$stage/relic.js' '$stage/relicSceneState.js' '$REMOTE_ROOT/src/' && rm -rf '$stage'"

# --- notify kiosk to reload scene ---
echo "deploy: kiosk refresh"
ssh "${SSH_OPTS[@]}" "$REMOTE_HOST" \
  "killall -HUP node 2>/dev/null; systemctl --user restart citadel-kiosk.service 2>/dev/null || true" \
  | sed 's/^/  /'

printf 'local_bridge=active\nremote_sidecar=installed\nkiosk_reloaded=attempted\n'
