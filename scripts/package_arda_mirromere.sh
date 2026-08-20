#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_DIR="${ARDA_MIRROMERE_APP_DIR:-$ROOT_DIR/apps/arda-mirromere}"
RECEIPT="${ARDA_MIRROMERE_PACKAGE_RECEIPT:-$ROOT_DIR/data/prometheus/arda_mirromere_package_last.json}"
if [[ "${ARDA_MIRROMERE_PACKAGE_IN_CONTAINER:-0}" != "1" ]] && command -v distrobox >/dev/null 2>&1; then
  exec distrobox enter lothlorien -- env \
    ARDA_MIRROMERE_PACKAGE_IN_CONTAINER=1 \
    ARDA_MIRROMERE_APP_DIR="$APP_DIR" \
    ARDA_MIRROMERE_PACKAGE_RECEIPT="$RECEIPT" \
    "$ROOT_DIR/scripts/package_arda_mirromere.sh"
fi
cd "$APP_DIR"
pnpm install --frozen-lockfile
pnpm run build
pnpm exec tauri build --bundles appimage
BINARY="$APP_DIR/src-tauri/target/release/arda_mirromere"
[[ -x "$BINARY" ]] || { printf 'missing release binary: %s\n' "$BINARY" >&2; exit 1; }
APPIMAGES=("$APP_DIR"/src-tauri/target/release/bundle/appimage/*.AppImage)
[[ -f "${APPIMAGES[0]}" ]] || { printf 'missing AppImage bundle\n' >&2; exit 1; }
mkdir -p "$(dirname "$RECEIPT")"
sha256="$(sha256sum "$BINARY" | cut -d' ' -f1)"
appimage_sha256="$(sha256sum "${APPIMAGES[0]}" | cut -d' ' -f1)"
python3 - "$RECEIPT" "$BINARY" "$sha256" "${APPIMAGES[0]}" "$appimage_sha256" <<'PY'
import json, os, sys, tempfile
from datetime import datetime, timezone
path, binary, sha256, appimage, appimage_sha256 = sys.argv[1:]
payload = {
  "schema_version": "arda.mirromere-package.v1",
  "observed_at": datetime.now(timezone.utc).isoformat(),
  "binary": binary,
  "sha256": sha256,
  "appimage": appimage,
  "appimage_sha256": appimage_sha256,
}
fd, temporary = tempfile.mkstemp(prefix=".mirromere-package-", dir=os.path.dirname(path))
with os.fdopen(fd, "w") as handle:
    json.dump(payload, handle, indent=2)
    handle.write("\n")
os.replace(temporary, path)
PY
printf 'binary=%s\nappimage=%s\nreceipt=%s\n' "$BINARY" "${APPIMAGES[0]}" "$RECEIPT"
