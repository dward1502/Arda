#!/usr/bin/env bash
set -euo pipefail

ROOT="${ARDA_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
QUEUE_REL="data/hades/action_queue.jsonl"
QUEUE_PATH="$ROOT/$QUEUE_REL"

if ! git -C "$ROOT" rev-parse --verify HEAD >/dev/null 2>&1; then
  echo "queue append-only guard: no HEAD commit available; skipping baseline check" >&2
  exit 0
fi

base="$(git -C "$ROOT" show "HEAD:$QUEUE_REL" 2>/dev/null || true)"
current="$(cat "$QUEUE_PATH" 2>/dev/null || true)"

if [[ -z "$base" ]]; then
  echo "queue append-only guard: no HEAD baseline for $QUEUE_REL; skipping" >&2
  exit 0
fi

if [[ "$current" == "$base" ]]; then
  echo "queue append-only guard: ok unchanged"
  exit 0
fi

if [[ "$current" == "$base"$'\n'* ]]; then
  echo "queue append-only guard: ok append-only"
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
