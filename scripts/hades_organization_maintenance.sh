#!/usr/bin/env bash
# sigil: COIN
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'EOF'
Usage: scripts/hades_organization_maintenance.sh [--root PATH] [--out PATH]

Runs the Arda-owned, read-only HADES organization maintenance tranche.
It writes markdown-link and storage-hygiene audit evidence only. The legacy
organization-apply command surface is intentionally not ported.

Optional environment:
  ARDA_HADES_ORG_ROOT     root to inspect, default repository root
  ARDA_HADES_ORG_OUT_DIR artifact directory, default data/hades
EOF
}

ROOT_SCOPE="${ARDA_HADES_ORG_ROOT:-$ROOT_DIR}"
OUT_DIR="${ARDA_HADES_ORG_OUT_DIR:-$ROOT_DIR/data/hades}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --root)
      ROOT_SCOPE="${2:?--root requires a path}"
      shift 2
      ;;
    --out)
      OUT_DIR="${2:?--out requires a path}"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done
mkdir -p "$OUT_DIR"

printf 'HADES organization maintenance mode: read-only Arda audits\n'
link_rc=0
storage_rc=0
python3 scripts/hades_markdown_link_check.py \
  --root "$ROOT_SCOPE" \
  --out "$OUT_DIR/markdown_link_check_last.md" || link_rc=$?
python3 scripts/hades_storage_hygiene_audit.py \
  --root "$ROOT_SCOPE" \
  --out-dir "$OUT_DIR/storage_hygiene" \
  --state-path "$OUT_DIR/storage_hygiene_last.json" || storage_rc=$?

printf 'HADES organization maintenance artifacts written:\n'
printf '  %s\n' "$OUT_DIR/markdown_link_check_last.md"
printf '  %s\n' "$OUT_DIR/storage_hygiene_last.json"
if ((link_rc != 0 || storage_rc != 0)); then
  printf 'HADES organization maintenance completed with findings: link_rc=%s storage_rc=%s\n' \
    "$link_rc" "$storage_rc" >&2
  exit 1
fi
