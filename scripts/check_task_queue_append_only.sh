#!/usr/bin/env bash
set -euo pipefail

ROOT="${ARDA_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
QUEUE_CONFIG="${ARDA_PROJECT_TASK_QUEUE_PATH:-core/projects/tasks/queue.jsonl}"
if [[ "$QUEUE_CONFIG" = /* ]]; then
  QUEUE_PATH="$QUEUE_CONFIG"
  if [[ "$QUEUE_PATH" == "$ROOT/"* ]]; then
    QUEUE_REL="${QUEUE_PATH#"$ROOT/"}"
  else
    echo "queue append-only guard: external queue has no repository HEAD baseline: $QUEUE_PATH" >&2
    exit 0
  fi
else
  QUEUE_REL="$QUEUE_CONFIG"
  QUEUE_PATH="$ROOT/$QUEUE_REL"
fi

if ! git -C "$ROOT" rev-parse --verify HEAD >/dev/null 2>&1; then
  echo "queue append-only guard: no HEAD commit available; skipping baseline check" >&2
  exit 0
fi

base_ref="HEAD"
base=""
base_found=false
if git -C "$ROOT" cat-file -e "$base_ref:$QUEUE_REL" 2>/dev/null; then
  base="$(git -C "$ROOT" show "$base_ref:$QUEUE_REL")"
  base_found=true
fi
if [[ -f "$QUEUE_PATH" ]]; then
  current="$(<"$QUEUE_PATH")"
else
  current=""
fi

if [[ "$base_found" != true ]]; then
  while IFS= read -r candidate; do
    if git -C "$ROOT" cat-file -e "$candidate:$QUEUE_REL" 2>/dev/null; then
      base="$(git -C "$ROOT" show "$candidate:$QUEUE_REL")"
      base_ref="$candidate"
      base_found=true
      break
    fi
  done < <(git -C "$ROOT" rev-list --all -- "$QUEUE_REL")
fi

if [[ "$base_found" != true ]]; then
  echo "queue append-only guard: no historical baseline for $QUEUE_REL; skipping" >&2
  exit 0
fi

if [[ "$current" == "$base" ]]; then
  echo "queue append-only guard: ok unchanged $QUEUE_REL"
  exit 0
fi

if [[ "$current" == "$base"$'\n'* ]]; then
  echo "queue append-only guard: ok append-only $QUEUE_REL"
  exit 0
fi

if [[ "${ARDA_ALLOW_HADES_QUEUE_COMPACTION:-}" == "1" ]]; then
  rollback_receipt_glob="$ROOT/audit/hades-queue-compaction-runs/rollback/queue-compaction-apply-"*.json
  compaction_receipt_glob="$ROOT/audit/hades-queue-compaction-runs/"*_queue_compaction_receipt.json
  if compgen -G "$rollback_receipt_glob" >/dev/null || compgen -G "$compaction_receipt_glob" >/dev/null; then
    echo "queue append-only guard: bypassed for HADES compaction receipt"
    exit 0
  fi
fi

base_lines="$(printf '%s\n' "$base" | sed '/^[[:space:]]*$/d' | wc -l)"
current_lines="$(printf '%s\n' "$current" | sed '/^[[:space:]]*$/d' | wc -l)"

cat >&2 <<EOF
queue append-only guard: blocked non-append edit to $QUEUE_REL

baseline_ref:            $base_ref
baseline_nonempty_lines: $base_lines
current_nonempty_lines:  $current_lines

This queue is an append-only evidence ledger. Do not rewrite, compact, or
delete rows directly. Close work by appending same-id terminal rows via
task-pivot or another queue API.

Approved compaction requires a HADES compaction receipt and an explicit
ARDA_ALLOW_HADES_QUEUE_COMPACTION=1 override. The current arda-cli does not
expose the retired queue-compaction command surface.

EOF
exit 1
