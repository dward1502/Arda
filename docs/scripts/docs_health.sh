#!/usr/bin/env bash
set -u
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DOCS_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
FAIL=0

while IFS= read -r pattern; do
  count=$(rg -n "$pattern" "$DOCS_DIR/plans" -S --glob '*.md' | wc -l || true)
  if [ "$count" -ne 0 ]; then
    echo "FAIL stale_refs pattern='$pattern' count=$count"
    FAIL=1
  else
    echo "OK stale_refs pattern='$pattern'"
  fi
done < <(cat <<'EOF'
core/projects/Plans
core/projects/tasks/queue.jsonl
core/state/manwe_router.json
core/state/operations_flow.json
core/state/warden_guardhouse.json
human/plans/
EOF
)

required_plans=(
  "docs/plans/AIPKG.md"
  "docs/plans/ATHENA.md"
  "docs/plans/CHARON.md"
  "docs/plans/EMBODIED_INTERFACE.md"
  "docs/plans/FEDERATED_COMMS.md"
  "docs/plans/HADES.md"
  "docs/plans/HERMES.md"
  "docs/plans/MNEMOSYNE.md"
  "docs/plans/OPENFANG.md"
  "docs/plans/PLATFORM_OS.md"
  "docs/plans/PROMETHEUS.md"
  "docs/plans/hud-incremental-build.md"
  "docs/plans/substrate-build-plan.md"
  "docs/plans/VAIRE_IMPLEMENTATION_PLAN.md"
)

for plan in "${required_plans[@]}"; do
  if [ -f "/var/home/mythos/Eregion/Arda/$plan" ]; then
    echo "OK plan_exists path='$plan'"
  else
    echo "FAIL plan_missing path='$plan'"
    FAIL=1
  fi
done

exit $FAIL
