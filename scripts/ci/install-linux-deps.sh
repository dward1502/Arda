#!/usr/bin/env bash
# Run as root on Ubuntu 24.04, including inside the native-check container.
set -euo pipefail
apt-get update
DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
  build-essential bubblewrap ca-certificates curl file git nodejs pkg-config protobuf-compiler python3 \
  libgtk-3-dev libwebkit2gtk-4.1-dev libssl-dev libxdo-dev \
  libayatana-appindicator3-dev librsvg2-dev xvfb xauth
pkg-config --modversion gtk+-3.0 gdk-3.0 webkit2gtk-4.1
