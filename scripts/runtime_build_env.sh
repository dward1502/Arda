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

# Invoke the prebuilt arda-cli release binary. Hard-fails with an
# instructive message if the binary is missing — never silently falls back
# to `cargo run`, which historically caused Hermes APIConnectionError
# outages from cold-compile latency. Build the binary once with:
#   source scripts/runtime_build_env.sh && arda_runtime_build_env .
#   cargo build -p arda-aule --bin arda-cli --features full-cli --release
arda_cli() {
  local cli_bin="${ARDA_CLI_BIN:-${CARGO_TARGET_DIR:-}/release/arda-cli}"
  if [[ -z "${CARGO_TARGET_DIR:-}" || ! -x "$cli_bin" ]]; then
    echo "[arda_cli] prebuilt CLI not found at: ${cli_bin}" >&2
    echo "[arda_cli] build it with: cargo build -p arda-aule --bin arda-cli --features full-cli --release" >&2
    echo "[arda_cli] (ensure runtime_build_env.sh is sourced first so CARGO_TARGET_DIR is set)" >&2
    return 127
  fi
  "$cli_bin" "$@"
}