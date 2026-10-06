#!/usr/bin/env bash
# Build once: podman build -t localhost/arda-native-check:ubuntu24.04 -f scripts/ci/Containerfile scripts/ci
# Run any validation command without GPU devices, host services or network.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
rust_sysroot=$(rustc --print sysroot)
cargo_cache=${CARGO_HOME:-$HOME/.cargo}
if (( $# == 0 )); then
  set -- cargo check --workspace --all-targets --all-features --locked --offline -j 2
fi
exec podman run --rm --network=none --security-opt label=disable \
  -v "$root:/workspace" \
  -v "$rust_sysroot:/opt/rust:ro" \
  -v "$cargo_cache/registry:/cargo/registry" \
  -v "$cargo_cache/git:/cargo/git" \
  -e PATH=/opt/rust/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
  -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/workspace/target/native-check \
  -e CARGO_BUILD_JOBS=2 -e CARGO_PROFILE_DEV_DEBUG=0 \
  -e CARGO_PROFILE_TEST_DEBUG=0 -e CARGO_INCREMENTAL=0 \
  localhost/arda-native-check:ubuntu24.04 "$@"
