#!/usr/bin/env bash
# sigil: ANKH
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/runtime_build_env.sh"
arda_runtime_build_env "$ROOT_DIR"
APP_DIR="${ARDA_HUD_APP_DIR:-$ROOT_DIR/apps/arda-hud}"
OUTPUT_JSON="${ARDA_HUD_PACKAGE_RECEIPT:-$ROOT_DIR/data/prometheus/arda_hud_package_last.json}"
WORKSPACE_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT_DIR/target}"
PACKAGE_TMP_ROOT=""

mkdir -p "$(dirname "$OUTPUT_JSON")"

TAURI_BUILD="${TAURI_BUILD:-true}"
RUN_TESTS="${RUN_TESTS:-true}"
INSTALL_DEPS_IF_MISSING="${INSTALL_DEPS_IF_MISSING:-true}"
TAURI_BUNDLE="${TAURI_BUNDLE:-false}"
NO_STRIP="${NO_STRIP:-true}"

STATUS="ready"
BUILD_MODE="frontend-only"
TARGET_PATH=""
APPIMAGE_PATH=""
ERROR_MESSAGE=""
BLOCKERS=()
TAURI_ATTEMPTED="false"
SYSTEM_PKG_CONFIG_PATH=""

prepend_pkg_config_path() {
  local dir="$1"
  if [[ -d "$dir" ]]; then
    case ":${PKG_CONFIG_PATH:-}:" in
      *":$dir:"*) ;;
      *)
        if [[ -n "${PKG_CONFIG_PATH:-}" ]]; then
          PKG_CONFIG_PATH="$dir:$PKG_CONFIG_PATH"
        else
          PKG_CONFIG_PATH="$dir"
        fi
        ;;
    esac
  fi
}

seed_system_pkg_config_path() {
  prepend_pkg_config_path "/usr/lib64/pkgconfig"
  prepend_pkg_config_path "/usr/lib/pkgconfig"
  prepend_pkg_config_path "/usr/share/pkgconfig"
  export PKG_CONFIG_PATH
  SYSTEM_PKG_CONFIG_PATH="${PKG_CONFIG_PATH:-}"

  if [[ -x /usr/bin/pkg-config ]]; then
    export PKG_CONFIG="/usr/bin/pkg-config"
  fi
}

write_payload() {
  local blockers_json
  if ((${#BLOCKERS[@]} == 0)); then
    blockers_json='[]'
  else
    blockers_json="$(printf '%s\n' "${BLOCKERS[@]}" | python3 -c 'import json,sys; print(json.dumps([line.rstrip("\n") for line in sys.stdin if line.strip()]))')"
  fi

  python3 - <<'PY' "$OUTPUT_JSON" "$STATUS" "$BUILD_MODE" "$APP_DIR" "$TARGET_PATH" "$APPIMAGE_PATH" "$ERROR_MESSAGE" "$blockers_json"
import json
import os
import sys
from datetime import datetime, timezone

output, status, build_mode, app_dir, target_path, appimage_path, error_message, blockers_json = sys.argv[1:]
payload = {
    "schema_version": "arda.hud.package.v2",
    "generated_at_utc": datetime.now(timezone.utc).isoformat(),
    "authority": "arda_hud_package_script",
    "status": status,
    "build_mode": build_mode,
    "app_dir": os.path.relpath(app_dir, os.path.dirname(os.path.dirname(output))),
    "frontend_dist": os.path.relpath(os.path.join(app_dir, "dist"), os.path.dirname(os.path.dirname(output))),
    "binary_path": os.path.relpath(target_path, os.path.dirname(os.path.dirname(output))) if target_path else None,
    "appimage_path": os.path.relpath(appimage_path, os.path.dirname(os.path.dirname(output))) if appimage_path else None,
    "error_message": error_message or None,
    "blockers": json.loads(blockers_json),
}
with open(output, "w", encoding="utf-8") as handle:
    json.dump(payload, handle, indent=2)
    handle.write("\n")
print(json.dumps(payload, indent=2))
PY
}

trap 'if [[ ! -f "$OUTPUT_JSON" ]]; then STATUS="${STATUS:-failed}"; ERROR_MESSAGE="${ERROR_MESSAGE:-unexpected packaging failure}"; write_payload; fi' EXIT

cd "$APP_DIR"

seed_system_pkg_config_path
if ! PACKAGE_TMP_ROOT="$(mktemp -d "${TMPDIR%/}/arda_hud_package.XXXXXX" 2>/dev/null)"; then
  PACKAGE_TMP_ROOT="$(mktemp -d "/tmp/arda_hud_package.XXXXXX")"
fi
export TMPDIR="$PACKAGE_TMP_ROOT"

if [[ "$INSTALL_DEPS_IF_MISSING" == "true" && ! -d node_modules ]]; then
  pnpm install --frozen-lockfile
fi

pnpm run build >/dev/null

if [[ "$RUN_TESTS" == "true" ]]; then
  pnpm test >/dev/null
fi

have_native_tauri_prereqs() {
  command -v pkg-config >/dev/null 2>&1 || return 1
  pkg-config --exists gio-2.0 gobject-2.0 glib-2.0
}

if [[ "$TAURI_BUILD" == "true" ]]; then
  if ! have_native_tauri_prereqs; then
    STATUS="partial"
    BUILD_MODE="frontend-only"
    BLOCKERS+=("missing_native_tauri_prereqs:glib-2.0,gobject-2.0,gio-2.0")
  else
    TAURI_ATTEMPTED="true"
    export NO_STRIP
    export __NV_DISABLE_EXPLICIT_SYNC="${__NV_DISABLE_EXPLICIT_SYNC:-1}"

    set +e
    if [[ "$TAURI_BUNDLE" == "true" ]]; then
      env PKG_CONFIG="${PKG_CONFIG:-pkg-config}" PKG_CONFIG_PATH="${SYSTEM_PKG_CONFIG_PATH:-/usr/lib64/pkgconfig:/usr/lib/pkgconfig:/usr/share/pkgconfig}" pnpm run tauri build >/tmp/arda_hud_package.log 2>&1
      TAURI_EXIT=$?
      BUILD_MODE="tauri-bundle"
    else
      env PKG_CONFIG="${PKG_CONFIG:-pkg-config}" PKG_CONFIG_PATH="${SYSTEM_PKG_CONFIG_PATH:-/usr/lib64/pkgconfig:/usr/lib/pkgconfig:/usr/share/pkgconfig}" pnpm run tauri build --no-bundle >/tmp/arda_hud_package.log 2>&1
      TAURI_EXIT=$?
      BUILD_MODE="tauri-no-bundle"
    fi
    set -e

    if [[ "$TAURI_EXIT" -ne 0 ]]; then
      STATUS="partial"
      BUILD_MODE="frontend-only"
      ERROR_MESSAGE="$(tail -n 20 /tmp/arda_hud_package.log | tr '\n' ' ' | sed 's/  */ /g' | cut -c1-1200)"
      BLOCKERS+=("tauri_build_failed")
    fi

    if [[ "$TAURI_EXIT" -eq 0 && -x "$WORKSPACE_TARGET_DIR/release/arda_hud" ]]; then
      TARGET_PATH="$WORKSPACE_TARGET_DIR/release/arda_hud"
      STATUS="ready"
    elif [[ "$TAURI_EXIT" -eq 0 && -x "$APP_DIR/src-tauri/target/release/arda_hud" ]]; then
      TARGET_PATH="$APP_DIR/src-tauri/target/release/arda_hud"
      STATUS="ready"
    fi

    shopt -s nullglob
    for appimage in "$WORKSPACE_TARGET_DIR"/release/bundle/appimage/*.AppImage; do
      [[ "$TAURI_EXIT" -eq 0 ]] || break
      APPIMAGE_PATH="$appimage"
      STATUS="ready"
      break
    done
    for appimage in "$APP_DIR"/src-tauri/target/release/bundle/appimage/*.AppImage; do
      [[ "$TAURI_EXIT" -eq 0 ]] || break
      APPIMAGE_PATH="$appimage"
      STATUS="ready"
      break
    done
    shopt -u nullglob

    if [[ "$TAURI_ATTEMPTED" == "true" && "$TAURI_EXIT" -eq 0 && -z "$TARGET_PATH" && -z "$APPIMAGE_PATH" ]]; then
      STATUS="partial"
      BUILD_MODE="frontend-only"
      ERROR_MESSAGE="tauri build exited successfully but no arda_hud binary or AppImage was found in expected output paths"
      BLOCKERS+=("tauri_artifact_missing")
    fi
  fi
fi

write_payload
