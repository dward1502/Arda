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

if [ ! -d "$DOCS_DIR/plans" ]; then
  echo "FAIL active_plan_inventory path='$DOCS_DIR/plans' reason='missing_directory'"
  FAIL=1
else
  mapfile -t active_plans < <(rg --files "$DOCS_DIR/plans" -g '*.md' | sort)
  echo "OK active_plan_inventory count=${#active_plans[@]}"
  for plan in "${active_plans[@]}"; do
    if [ ! -s "$plan" ]; then
      echo "FAIL active_plan path='$plan' reason='empty_file'"
      FAIL=1
    fi
  done
fi

exit $FAIL
