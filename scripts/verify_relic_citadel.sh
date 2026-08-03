#!/usr/bin/env bash
# Verify the Arda-owned RELIC bridge against CITADEL presence requirements.
# Read-only: does not mutate services or remote state.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RELIC_ROOT="${ARDA_RELIC_KIOSK_ROOT:-/var/home/mythos/Eregion/relic-kiosk}"
REMOTE_HOST="${ARDA_RELIC_REMOTE_HOST:-citadel}"
REMOTE_ROOT="${ARDA_RELIC_REMOTE_ROOT:-/home/citadel/annunimas_embodied/relic}"
SSH_OPTS=(-o BatchMode=yes -o ConnectTimeout=6 -o ConnectionAttempts=1 -o ServerAliveInterval=10)

echo "1. Local build: presence snapshot preflight"
curl --fail --silent --show-error --max-time 5 \
  "${ARDA_RELIC_PRESENCE_URL:-http://127.0.0.1:7878/v1/presence/snapshot}" > /tmp/arda-presence.preflight.json
python3 -c 'import sys,json
d=json.load(open(sys.argv[1]))
snap=d.get("snapshot", d)
assert snap.get("schema_version")=="arda.runtime-presence.v1", "schema mismatch"
assert isinstance(d.get("snapshot_sequence"), int), "sequence missing"
print("  presence snapshot ok:", "seq=%s nodes=%s" % (d.get("snapshot_sequence"), len(snap.get("nodes", []))))
' /tmp/arda-presence.preflight.json

echo "2. Bridge service: arda-relic-bridge.service"
MACHINE="$(id -un)@.host"
systemctl --user --machine="$MACHINE" --quiet is-active arda-relic-bridge.service
echo "  bridge service: active"

echo "3. Local bridge runtime state"
RUNTIME_STATE="${XDG_RUNTIME_DIR:-/tmp}/arda-relic-bridge/scene.json"
[[ -f "$RUNTIME_STATE" ]] || { echo "  missing local scene state: $RUNTIME_STATE" >&2; exit 1; }
python3 -c 'import sys,json
d=json.load(open(sys.argv[1]))
assert d.get("schema_version")=="arda.relic.scene-adapter.v1", "adapter schema mismatch"
print("  adapter schema ok:", "state=%s forms=%s" % (d.get("scene_state"), len(d.get("forms", []))))
' "$RUNTIME_STATE"

echo "4. Remote sidecar: scene.json schema"
remote_schema="$(ssh "${SSH_OPTS[@]}" "$REMOTE_HOST" \
  "python3 -c 'import json; print(json.load(open(\"${REMOTE_ROOT}/public/scene.json\"))[\"schema_version\"])'")"
[[ "$remote_schema" == "arda.relic.scene-adapter.v1" ]] || {
  printf '  remote scene schema mismatch: %s\n' "$remote_schema" >&2
  exit 1
}
echo "  remote schema ok: $remote_schema"

echo "5. Remote services"
ssh "${SSH_OPTS[@]}" "$REMOTE_HOST" \
  "systemctl --user is-active relic.service citadel-kiosk.service" 2>&1 | sed 's/^/  /'

echo "6. Local sidecar validation"
npm --prefix "$RELIC_ROOT" run validate >/dev/null
echo "  sidecar validate ok"

echo "7. Bridge crate tests"
cargo test -p arda-relic-bridge --all-features -- --test-threads=1 --quiet
echo "  bridge tests ok"

printf '\nrelic_citadel_verification=pass\n'
