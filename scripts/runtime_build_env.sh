#!/usr/bin/env bash
# sigil: ANKH

arda_runtime_build_env() {
  local root_dir="${1:-$(pwd)}"
  local fallback_build_root="${HOME:-$root_dir}/.cache/arda-build"
  local env_file
  for env_file in "$root_dir/config/.env" "$root_dir/config/runtime.env"; do
    if [[ -f "$env_file" ]]; then
      set -a
      # shellcheck disable=SC1090
      source "$env_file"
      set +a
    fi
  done
  local build_root="${ARDA_BUILD_CACHE_ROOT:-${ARDA_RUNTIME_BUILD_ROOT:-}}"
  if [[ -z "$build_root" ]]; then
    build_root="$fallback_build_root"
  fi
  export ARDA_BUILD_CACHE_ROOT="$build_root"
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ARDA_BUILD_CACHE_ROOT/target}"
  export TMPDIR="${TMPDIR:-$ARDA_BUILD_CACHE_ROOT/tmp}"
  mkdir -p "$CARGO_TARGET_DIR" "$TMPDIR"
  if [[ -d "$root_dir/target" ]]; then
    mkdir -p "$root_dir/target"
  fi
}

# Invoke the Arda CLI binary. Uses the prebuilt arda-aule CLI with full-cli features.
# Build the binary with:
#   source scripts/runtime_build_env.sh && arda_runtime_build_env .
#   cargo build -p arda-aule --bin arda-cli --features full-cli --release
ardacli() {
  local cli_bin="${ARDA_CLI_BIN:-${CARGO_TARGET_DIR:-}/release/ardacli}"
  if [[ -z "${CARGO_TARGET_DIR:-}" || ! -x "$cli_bin" ]]; then
    echo "[ardacli] prebuilt CLI not found at: ${cli_bin}" >&2
    echo "[ardacli] build it with: cargo build -p arda-aule --bin arda-cli --features full-cli --release" >&2
    echo "[ardacli] (ensure runtime_build_env.sh is sourced first so CARGO_TARGET_DIR is set)" >&2
    return 127
  fi
  "$cli_bin" "$@"
}
