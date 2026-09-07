#!/usr/bin/env bash
# sigil: verify_arda_hud
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# 1. Verify projection file exists
PROJECTION="$ROOT_DIR/core/state/source_absorption_pipeline.json"
if [[ ! -f "$PROJECTION" ]]; then
  echo "FAIL: projection file missing: $PROJECTION"
  exit 1
fi

# 2. Verify HUD package state is tracked (non-fatal if absent)
PACKAGE_STATE="$ROOT_DIR/data/prometheus/arda_hud_package_last.json"
if [[ -f "$PACKAGE_STATE" ]]; then
  echo "HUD package state present."
else
  echo "WARN: HUD package state missing: $PACKAGE_STATE"
fi

echo "OK: ARDA HUD verification passed."
