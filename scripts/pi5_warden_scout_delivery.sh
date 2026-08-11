#!/usr/bin/env bash
# Reproducible, fixed-scope delivery for the Warden scout binary.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="aarch64-unknown-linux-gnu"
RUST_TOOLCHAIN="1.94.0"
CROSS_VERSION="0.2.5"
CROSS_IMAGE="ghcr.io/cross-rs/${TARGET}:${CROSS_VERSION}"
PACKAGE="arda-outpost-scout"
BINARY="arda-outpost-scout"
WARDEN_HOST="warden"
WARDEN_UNIT="arda-warden-scout.service"
WARDEN_HEALTH_URL="http://100.110.85.37:8092/health"
WARDEN_SEARCH_URL="http://100.110.85.37:8092/search"
WARDEN_RECALL_URL="http://100.110.85.37:8092/recall"
REMOTE_ROOT=".local/lib/arda/warden"
ARTIFACT_DIR="${ARDA_PI5_ARTIFACT_DIR:-${HOME}/.cache/arda-artifacts/pi5-warden}"
TARGET_DIR="${ARDA_PI5_TARGET_DIR:-${HOME}/.cache/arda-build/pi5-warden-target}"
LOCAL_ARTIFACT="${ARTIFACT_DIR}/${BINARY}"
LOCAL_MANIFEST="${ARTIFACT_DIR}/${BINARY}.manifest.json"
SMOKE_EVIDENCE="${ARTIFACT_DIR}/last-smoke.json"
SSH_OPTS=(-o BatchMode=yes -o ConnectTimeout=8 -o ConnectionAttempts=1)

usage() {
  cat <<'EOF'
Usage: scripts/pi5_warden_scout_delivery.sh COMMAND

Commands:
  build          Cross-build the current committed scout source and emit provenance.
  deploy         Atomically install the built artifact, preserve the prior binary,
                 restart only arda-warden-scout.service, and verify health.
  smoke          Run one bounded source-cited request and persist local receipt evidence.
  reboot-verify  Reboot Warden and prove service recovery plus exactly-once receipt recall.
  rollback       Restore the immediately prior binary and verify service health.
  status         Report the deployed binary checksum, manifest, unit state, and health.

The target host, unit, paths, and HTTP endpoints are fixed. The helper accepts no
remote command, service, host, credential, or password arguments.
EOF
}

die() {
  printf 'pi5 Warden delivery: %s\n' "$*" >&2
  exit 1
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || die "required command not found: $1"
}

ssh_warden() {
  # Arguments are fixed helper-owned commands; no user-supplied remote shell is accepted.
  # shellcheck disable=SC2029
  ssh "${SSH_OPTS[@]}" "$WARDEN_HOST" "$@"
}

wait_for_health() {
  local attempts="${1:-30}"
  local i
  for ((i = 1; i <= attempts; i++)); do
    if curl --fail --silent --show-error --connect-timeout 2 --max-time 5 \
      "$WARDEN_HEALTH_URL" | jq -e \
      '.status == "ok" and .source == "node-pi5-warden" and .authority == "advisory"' \
      >/dev/null; then
      return 0
    fi
    sleep 2
  done
  return 1
}

assert_clean_build_inputs() {
  local status
  status="$(git -C "$ROOT_DIR" status --porcelain -- \
    Cargo.toml Cargo.lock \
    outposts/arda-outpost-protocol \
    outposts/arda-outpost-scout)"
  [[ -z "$status" ]] || die "build inputs are not committed:\n${status}"
}

build_artifact() {
  local cross_actual rust_actual source_revision source_tree cargo_lock_sha
  local artifact_sha artifact_size image_digest built_at

  require_command git
  require_command cross
  require_command podman
  require_command rustup
  require_command sha256sum
  require_command python3
  require_command file

  assert_clean_build_inputs

  cross_actual="$(cross --version 2>/dev/null | sed -n '1p')"
  [[ "$cross_actual" == "cross ${CROSS_VERSION}" ]] || \
    die "cross ${CROSS_VERSION} is required; found: ${cross_actual}"
  rust_actual="$(rustup run "$RUST_TOOLCHAIN" rustc --version)"
  [[ "$rust_actual" == rustc\ 1.94.0\ * ]] || \
    die "Rust ${RUST_TOOLCHAIN} is required; found: ${rust_actual}"

  source_revision="$(git -C "$ROOT_DIR" rev-parse HEAD)"
  source_tree="$(
    git -C "$ROOT_DIR" ls-tree -r HEAD -- \
      Cargo.toml Cargo.lock \
      outposts/arda-outpost-protocol \
      outposts/arda-outpost-scout |
      sha256sum | cut -d' ' -f1
  )"
  cargo_lock_sha="$(sha256sum "$ROOT_DIR/Cargo.lock" | cut -d' ' -f1)"

  mkdir -p "$ARTIFACT_DIR" "$TARGET_DIR"
  export CROSS_CONTAINER_ENGINE=podman
  export CARGO_TARGET_DIR="$TARGET_DIR"
  (
    cd "$ROOT_DIR"
    cross "+${RUST_TOOLCHAIN}" build --locked --release \
      --target "$TARGET" -p "$PACKAGE"
  )

  local built_binary="${TARGET_DIR}/${TARGET}/release/${BINARY}"
  [[ -x "$built_binary" ]] || die "cross build did not produce ${built_binary}"
  file "$built_binary" | grep -q 'ARM aarch64' || die "artifact is not an AArch64 ELF binary"

  install -m 0755 "$built_binary" "${LOCAL_ARTIFACT}.new"
  mv -f "${LOCAL_ARTIFACT}.new" "$LOCAL_ARTIFACT"
  artifact_sha="$(sha256sum "$LOCAL_ARTIFACT" | cut -d' ' -f1)"
  artifact_size="$(stat -c '%s' "$LOCAL_ARTIFACT")"
  image_digest="$(podman image inspect "$CROSS_IMAGE" --format '{{.Digest}}')"
  [[ "$image_digest" == sha256:* ]] || die "unable to resolve cross image digest"
  built_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

  SOURCE_REVISION="$source_revision" \
  SOURCE_TREE_SHA256="$source_tree" \
  CARGO_LOCK_SHA256="$cargo_lock_sha" \
  ARTIFACT_SHA256="$artifact_sha" \
  ARTIFACT_SIZE="$artifact_size" \
  IMAGE_DIGEST="$image_digest" \
  BUILT_AT="$built_at" \
  python3 - "$LOCAL_MANIFEST" <<'PY'
import json
import os
import sys

manifest = {
    "schema_version": "arda.pi5.warden-scout-artifact.v1",
    "node_id": "node-pi5-warden",
    "unit": "arda-warden-scout.service",
    "artifact": {
        "name": "arda-outpost-scout",
        "target": "aarch64-unknown-linux-gnu",
        "sha256": os.environ["ARTIFACT_SHA256"],
        "size_bytes": int(os.environ["ARTIFACT_SIZE"]),
    },
    "source": {
        "git_revision": os.environ["SOURCE_REVISION"],
        "tracked_input_tree_sha256": os.environ["SOURCE_TREE_SHA256"],
        "cargo_lock_sha256": os.environ["CARGO_LOCK_SHA256"],
        "dirty_build_inputs": False,
    },
    "toolchain": {
        "rust": "1.94.0",
        "cross": "0.2.5",
        "container_image": "ghcr.io/cross-rs/aarch64-unknown-linux-gnu:0.2.5",
        "container_digest": os.environ["IMAGE_DIGEST"],
    },
    "built_at_utc": os.environ["BUILT_AT"],
}

path = sys.argv[1]
temporary = f"{path}.new"
with open(temporary, "w", encoding="utf-8") as handle:
    json.dump(manifest, handle, indent=2, sort_keys=True)
    handle.write("\n")
os.replace(temporary, path)
PY

  printf 'artifact=%s\nmanifest=%s\nsha256=%s\nsource_revision=%s\n' \
    "$LOCAL_ARTIFACT" "$LOCAL_MANIFEST" "$artifact_sha" "$source_revision"
}

verify_local_artifact() {
  require_command jq
  require_command sha256sum
  [[ -x "$LOCAL_ARTIFACT" ]] || die "missing artifact; run build first"
  [[ -f "$LOCAL_MANIFEST" ]] || die "missing manifest; run build first"

  local expected actual target
  expected="$(jq -er '.artifact.sha256' "$LOCAL_MANIFEST")"
  target="$(jq -er '.artifact.target' "$LOCAL_MANIFEST")"
  actual="$(sha256sum "$LOCAL_ARTIFACT" | cut -d' ' -f1)"
  [[ "$expected" =~ ^[0-9a-f]{64}$ ]] || die "manifest contains an invalid checksum"
  [[ "$target" == "$TARGET" ]] || die "manifest target is ${target}, expected ${TARGET}"
  [[ "$actual" == "$expected" ]] || die "artifact checksum does not match manifest"
  printf '%s\n' "$expected"
}

deploy_artifact() {
  require_command curl
  require_command jq
  require_command scp
  require_command ssh

  local expected stage
  expected="$(verify_local_artifact)"
  stage="${REMOTE_ROOT}/releases/.staging-${expected:0:16}"

  ssh_warden "set -eu; rm -rf '${stage}'; mkdir -p '${stage}' '${REMOTE_ROOT}/bin' '${REMOTE_ROOT}/releases/previous'"
  scp "${SSH_OPTS[@]}" "$LOCAL_ARTIFACT" "${WARDEN_HOST}:${stage}/${BINARY}"
  scp "${SSH_OPTS[@]}" "$LOCAL_MANIFEST" "${WARDEN_HOST}:${stage}/${BINARY}.manifest.json"

  ssh_warden "set -eu
expected='${expected}'
stage='${stage}'
root='${REMOTE_ROOT}'
current=\"\${root}/bin/${BINARY}\"
manifest=\"\${current}.manifest.json\"
actual=\$(sha256sum \"\${stage}/${BINARY}\" | cut -d' ' -f1)
[ \"\${actual}\" = \"\${expected}\" ]
if [ -x \"\${current}\" ]; then
  previous_sha=\$(sha256sum \"\${current}\" | cut -d' ' -f1)
  install -m 0755 \"\${current}\" \"\${root}/releases/previous/${BINARY}-\${previous_sha}\"
  printf '%s\\n' \"\${previous_sha}\" >\"\${root}/releases/previous/sha256.new\"
  mv -f \"\${root}/releases/previous/sha256.new\" \"\${root}/releases/previous/sha256\"
  if [ -f \"\${manifest}\" ]; then
    install -m 0644 \"\${manifest}\" \"\${root}/releases/previous/${BINARY}-\${previous_sha}.manifest.json\"
  fi
fi
install -m 0755 \"\${stage}/${BINARY}\" \"\${current}.new\"
install -m 0644 \"\${stage}/${BINARY}.manifest.json\" \"\${manifest}.new\"
mv -f \"\${current}.new\" \"\${current}\"
mv -f \"\${manifest}.new\" \"\${manifest}\"
rm -rf \"\${stage}\"
systemctl --user restart '${WARDEN_UNIT}'
systemctl --user is-active --quiet '${WARDEN_UNIT}'"

  if ! wait_for_health 30; then
    printf 'new deployment failed health; restoring the prior binary\n' >&2
    rollback_artifact
    die "deployment failed health and was rolled back"
  fi

  local deployed
  deployed="$(ssh_warden "sha256sum '${REMOTE_ROOT}/bin/${BINARY}' | cut -d' ' -f1")"
  [[ "$deployed" == "$expected" ]] || die "remote checksum changed after deployment"
  printf 'deployed_sha256=%s\nunit=%s\nhealth=ok\n' "$deployed" "$WARDEN_UNIT"
}

smoke_request() {
  require_command curl
  require_command jq
  require_command python3

  local revision query expires request response recall memory_id result_count matching_count
  revision="$(jq -er '.source.git_revision' "$LOCAL_MANIFEST")"
  query="official Rust aarch64 cross compilation guidance ${revision:0:12}"
  expires="$(date -u -d '+10 minutes' +%Y-%m-%dT%H:%M:%SZ)"
  request="$(jq -cn --arg query "$query" --arg expires "$expires" \
    '{query:$query,limit:3,source_policy:"allowlisted_public_web",expires_at:$expires}')"
  response="$(curl --fail --silent --show-error --connect-timeout 3 --max-time 30 \
    -H 'Content-Type: application/json' -d "$request" "$WARDEN_SEARCH_URL")"

  memory_id="$(jq -er '.memory.memory_id | select(type == "string" and length > 0)' <<<"$response")"
  result_count="$(jq -er '[.report.results[] | select(.url | test("^https?://[^/]+"))] | length' <<<"$response")"
  ((result_count >= 1 && result_count <= 3)) || die "smoke request returned no bounded source URLs"

  recall="$(curl --fail --silent --show-error --connect-timeout 3 --max-time 15 \
    -H 'Content-Type: application/json' \
    -d "$(jq -cn --arg query "$query" '{hours:24,query:$query,limit:10}')" \
    "$WARDEN_RECALL_URL")"
  matching_count="$(jq --arg memory_id "$memory_id" \
    '[.records[] | select(.memory_id == $memory_id)] | length' <<<"$recall")"
  [[ "$matching_count" == "1" ]] || die "expected one recalled receipt; found ${matching_count}"

  QUERY="$query" MEMORY_ID="$memory_id" RESULT_COUNT="$result_count" \
  MATCHING_COUNT="$matching_count" RESPONSE="$response" \
  python3 - "$SMOKE_EVIDENCE" <<'PY'
import json
import os
import sys
from datetime import datetime, timezone

response = json.loads(os.environ["RESPONSE"])
evidence = {
    "schema_version": "arda.pi5.warden-scout-smoke.v1",
    "recorded_at_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "query": os.environ["QUERY"],
    "memory_id": os.environ["MEMORY_ID"],
    "result_count": int(os.environ["RESULT_COUNT"]),
    "matching_receipts_before_reboot": int(os.environ["MATCHING_COUNT"]),
    "source_urls": [item["url"] for item in response["report"]["results"]],
}
path = sys.argv[1]
temporary = f"{path}.new"
with open(temporary, "w", encoding="utf-8") as handle:
    json.dump(evidence, handle, indent=2, sort_keys=True)
    handle.write("\n")
os.replace(temporary, path)
PY

  printf 'memory_id=%s\nresult_count=%s\nmatching_receipts=%s\nevidence=%s\n' \
    "$memory_id" "$result_count" "$matching_count" "$SMOKE_EVIDENCE"
  jq -r '.source_urls[] | "source_url=" + .' "$SMOKE_EVIDENCE"
}

reboot_verify() {
  require_command curl
  require_command jq
  require_command ssh
  [[ -f "$SMOKE_EVIDENCE" ]] || die "missing smoke evidence; run smoke first"

  local query memory_id before_count boot_before boot_after unavailable=0 recall after_count
  query="$(jq -er '.query' "$SMOKE_EVIDENCE")"
  memory_id="$(jq -er '.memory_id' "$SMOKE_EVIDENCE")"
  before_count="$(jq -er '.matching_receipts_before_reboot' "$SMOKE_EVIDENCE")"
  [[ "$before_count" == "1" ]] || die "smoke evidence did not record exactly one receipt"

  ssh_warden 'sudo -n true' || die "Warden passwordless reboot authority is unavailable"
  boot_before="$(ssh_warden 'cat /proc/sys/kernel/random/boot_id')"
  ssh_warden 'sudo -n systemctl reboot' || true

  local i
  for ((i = 1; i <= 30; i++)); do
    if ! ssh_warden 'true' >/dev/null 2>&1; then
      unavailable=1
      break
    fi
    sleep 2
  done
  [[ "$unavailable" == "1" ]] || die "Warden never became unreachable during reboot"

  boot_after=""
  for ((i = 1; i <= 90; i++)); do
    if boot_after="$(ssh_warden 'cat /proc/sys/kernel/random/boot_id' 2>/dev/null)"; then
      break
    fi
    sleep 2
  done
  [[ -n "$boot_after" ]] || die "Warden SSH did not recover after reboot"
  [[ "$boot_after" != "$boot_before" ]] || die "boot identity did not change"
  wait_for_health 60 || die "Warden scout health did not recover after reboot"

  recall="$(curl --fail --silent --show-error --connect-timeout 3 --max-time 15 \
    -H 'Content-Type: application/json' \
    -d "$(jq -cn --arg query "$query" '{hours:24,query:$query,limit:10}')" \
    "$WARDEN_RECALL_URL")"
  after_count="$(jq --arg memory_id "$memory_id" \
    '[.records[] | select(.memory_id == $memory_id)] | length' <<<"$recall")"
  [[ "$after_count" == "$before_count" ]] || \
    die "receipt count changed across reboot: before=${before_count} after=${after_count}"

  local updated
  updated="$(mktemp)"
  jq --arg boot_before "$boot_before" --arg boot_after "$boot_after" \
    --argjson after_count "$after_count" \
    '. + {reboot_verified:true, boot_id_before:$boot_before, boot_id_after:$boot_after,
      matching_receipts_after_reboot:$after_count}' \
    "$SMOKE_EVIDENCE" >"$updated"
  mv -f "$updated" "$SMOKE_EVIDENCE"

  printf 'boot_id_before=%s\nboot_id_after=%s\nhealth=ok\nmatching_receipts_after_reboot=%s\n' \
    "$boot_before" "$boot_after" "$after_count"
}

rollback_artifact() {
  require_command curl
  require_command jq
  require_command ssh

  ssh_warden "set -eu
root='${REMOTE_ROOT}'
current=\"\${root}/bin/${BINARY}\"
manifest=\"\${current}.manifest.json\"
previous_sha=\$(cat \"\${root}/releases/previous/sha256\")
case \"\${previous_sha}\" in (*[!0-9a-f]*|'') exit 2;; esac
[ \"\${#previous_sha}\" -eq 64 ]
previous=\"\${root}/releases/previous/${BINARY}-\${previous_sha}\"
[ -x \"\${previous}\" ]
actual=\$(sha256sum \"\${previous}\" | cut -d' ' -f1)
[ \"\${actual}\" = \"\${previous_sha}\" ]
current_sha=\$(sha256sum \"\${current}\" | cut -d' ' -f1)
install -m 0755 \"\${current}\" \"\${root}/releases/rollback-origin-\${current_sha}\"
install -m 0755 \"\${previous}\" \"\${current}.new\"
mv -f \"\${current}.new\" \"\${current}\"
previous_manifest=\"\${previous}.manifest.json\"
if [ -f \"\${previous_manifest}\" ]; then
  install -m 0644 \"\${previous_manifest}\" \"\${manifest}.new\"
  mv -f \"\${manifest}.new\" \"\${manifest}\"
else
  rm -f \"\${manifest}\"
fi
systemctl --user restart '${WARDEN_UNIT}'
systemctl --user is-active --quiet '${WARDEN_UNIT}'"

  wait_for_health 30 || die "rollback binary failed health"
  local deployed preserved_memory_id preserved_file
  deployed="$(ssh_warden "sha256sum '${REMOTE_ROOT}/bin/${BINARY}' | cut -d' ' -f1")"
  preserved_file=""
  if [[ -f "$SMOKE_EVIDENCE" ]]; then
    preserved_memory_id="$(jq -er '.memory_id' "$SMOKE_EVIDENCE")"
    [[ "$preserved_memory_id" =~ ^mem_[0-9a-f]{32}$ ]] || \
      die "smoke evidence contains an invalid memory id"
    preserved_file="$(ssh_warden "find \"\$HOME/.local/share/arda/warden/episodic\" -type f -name '${preserved_memory_id}.jsonl' -print -quit")"
    [[ -n "$preserved_file" ]] || die "rollback lost append-only receipt ${preserved_memory_id}"
  fi
  printf 'rolled_back_sha256=%s\nunit=%s\nhealth=ok\n' "$deployed" "$WARDEN_UNIT"
  if [[ -n "$preserved_file" ]]; then
    printf 'preserved_receipt_file=%s\n' "$preserved_file"
  fi
}

show_status() {
  require_command curl
  require_command jq
  ssh_warden "set -eu
printf 'architecture='; uname -m
printf 'binary_sha256='; sha256sum '${REMOTE_ROOT}/bin/${BINARY}' | cut -d' ' -f1
printf 'unit_active='; systemctl --user is-active '${WARDEN_UNIT}'
printf 'unit_enabled='; systemctl --user is-enabled '${WARDEN_UNIT}'
printf 'boot_id='; cat /proc/sys/kernel/random/boot_id
if [ -f '${REMOTE_ROOT}/bin/${BINARY}.manifest.json' ]; then
  printf '%s\\n' 'manifest:'
  cat '${REMOTE_ROOT}/bin/${BINARY}.manifest.json'
fi"
  printf '%s\n' 'health:'
  curl --fail --silent --show-error --connect-timeout 3 --max-time 8 "$WARDEN_HEALTH_URL" | jq .
}

case "${1:-}" in
  build) build_artifact ;;
  deploy) deploy_artifact ;;
  smoke) smoke_request ;;
  reboot-verify) reboot_verify ;;
  rollback) rollback_artifact ;;
  status) show_status ;;
  -h|--help|help) usage ;;
  *) usage >&2; exit 2 ;;
esac
