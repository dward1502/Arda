#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE_UNIT="$ROOT_DIR/config/systemd/arda-metrics-exporter.service"
SOURCE_BINARY="${ARDA_METRICS_SOURCE_BINARY:-}"
DEST_BINARY="${ARDA_METRICS_BINARY_PATH:-$HOME/.local/bin/arda-cli}"
DEST_UNIT="${ARDA_SYSTEMD_USER_DIR:-${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user}/arda-metrics-exporter.service"

systemctl_user() {
  systemctl --user "$@" 2>/dev/null || \
    systemctl --user --machine="$(id -un)@.host" "$@"
}

if [[ -z "$SOURCE_BINARY" ]]; then
  cargo build --release -p arda-aule --features http --bin arda-cli --manifest-path "$ROOT_DIR/Cargo.toml"
  SOURCE_BINARY="$ROOT_DIR/target/release/arda-cli"
fi
[[ -x "$SOURCE_BINARY" ]] || { printf 'metrics source binary is not executable: %s\n' "$SOURCE_BINARY" >&2; exit 1; }

mkdir -p "$(dirname "$DEST_BINARY")" "$(dirname "$DEST_UNIT")"
backup_dir="$(mktemp -d "${XDG_RUNTIME_DIR:-/tmp}/arda-metrics-install.XXXXXX")"
cleanup() { rm -rf "$backup_dir"; }
trap cleanup EXIT

binary_existed=false
unit_existed=false
if [[ -f "$DEST_BINARY" ]]; then
  install -m0755 "$DEST_BINARY" "$backup_dir/arda-cli"
  binary_existed=true
fi
if [[ -f "$DEST_UNIT" ]]; then
  install -m0644 "$DEST_UNIT" "$backup_dir/arda-metrics-exporter.service"
  unit_existed=true
fi

rollback() {
  if [[ "$binary_existed" == "true" ]]; then
    install -m0755 "$backup_dir/arda-cli" "$DEST_BINARY"
  else
    rm -f "$DEST_BINARY"
  fi
  if [[ "$unit_existed" == "true" ]]; then
    install -m0644 "$backup_dir/arda-metrics-exporter.service" "$DEST_UNIT"
  else
    rm -f "$DEST_UNIT"
  fi
  systemctl_user daemon-reload >/dev/null 2>&1 || true
}
trap 'status=$?; if (( status != 0 )); then rollback; fi; cleanup; exit $status' EXIT

install -m0755 "$SOURCE_BINARY" "$DEST_BINARY.new"
mv -f "$DEST_BINARY.new" "$DEST_BINARY"
install -m0644 "$SOURCE_UNIT" "$DEST_UNIT.new"
mv -f "$DEST_UNIT.new" "$DEST_UNIT"

"$ROOT_DIR/scripts/verify_arda_metrics_exporter.sh" "$DEST_UNIT"
systemctl_user daemon-reload
systemctl_user enable --now arda-metrics-exporter.service
ARDA_METRICS_RUNTIME_CHECK=true "$ROOT_DIR/scripts/verify_arda_metrics_exporter.sh" "$DEST_UNIT"

trap cleanup EXIT
printf 'arda metrics exporter installed: binary=%s unit=%s\n' "$DEST_BINARY" "$DEST_UNIT"
