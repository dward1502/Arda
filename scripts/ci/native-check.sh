#!/usr/bin/env bash
# Build once: podman build -t localhost/arda-native-check:ubuntu24.04 -f scripts/ci/Containerfile scripts/ci
# Run any validation command without GPU devices, host services or network.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
rust_sysroot=$(rustc --print sysroot)
cargo_cache=${CARGO_HOME:-$HOME/.cargo}
mkdir -p "$root/target/native-check/tmp" "$root/target/native-check/var-tmp" "$root/target/native-check/cargo-home"
if (( $# == 0 )); then
  set -- cargo check --workspace --all-targets --all-features --locked --offline -j 2
fi
exec podman run --rm --init --userns=keep-id --network=none \
  --security-opt label=disable --security-opt unmask=ALL \
  -v "$root:/workspace" \
  -v "$root/target/native-check/tmp:/tmp" \
  -v "$root/target/native-check/var-tmp:/var/tmp" \
  -v "$root/target/native-check/cargo-home:/cargo" \
  -v "$rust_sysroot:/opt/rust:ro" \
  -v "$cargo_cache/registry:/cargo/registry" \
  -v "$cargo_cache/git:/cargo/git" \
  -e PATH=/opt/rust/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
  -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/workspace/target/native-check \
  -e CARGO_BUILD_JOBS=2 -e CARGO_PROFILE_DEV_DEBUG=0 \
  -e CARGO_PROFILE_TEST_DEBUG=0 -e CARGO_INCREMENTAL=0 \
  localhost/arda-native-check:ubuntu24.04 "$@"
