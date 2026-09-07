#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
QUEUE_FILE="$ROOT_DIR/core/projects/tasks/queue.jsonl"

OWNER="prometheus"; PRIORITY="high"; ORIGIN="cron"
SCOPE="arda_hud"; GLYPH="↝"; STATUS="completed"; RESULT=""
TITLE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --owner) OWNER="$2"; shift 2 ;;
    --priority) PRIORITY="$2"; shift 2 ;;
    --origin) ORIGIN="$2"; shift 2 ;;
    --scope) SCOPE="$2"; shift 2 ;;
    --glyph) GLYPH="$2"; shift 2 ;;
    --status) STATUS="$2"; shift 2 ;;
    --result) RESULT="$2"; shift 2 ;;
    *) TITLE="$1"; shift ;;
  esac
done

[[ -z "$TITLE" ]] && { echo "Usage: task-pivot <TITLE> [OPTIONS]" >&2; exit 1; }

mkdir -p "$(dirname "$QUEUE_FILE")"
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
ID="task-pivot-$(date +%s)-$(openssl rand -hex 4 2>/dev/null || echo $$)"

python3 <<PYEOF >> "$QUEUE_FILE"
import json, sys
record = {
    "id": "$ID",
    "source_record_id": "$ID",
    "contract": "arda.operator_project_task.v1",
    "title": "$TITLE",
    "detail": "$RESULT",
    "owner": "$OWNER",
    "priority": "$PRIORITY",
    "status": "$STATUS",
    "origin": "$ORIGIN",
    "scope": "$SCOPE",
    "glyph": "$GLYPH",
    "queued_at_utc": "$TIMESTAMP",
    "completed_at_utc": "$TIMESTAMP" if "$STATUS" == "completed" else None,
    "result": "$RESULT"
}
print(json.dumps(record))
PYEOF

echo "Task pivot appended: $TITLE -> $QUEUE_FILE"
echo "ID: $ID | Status: $STATUS | Owner: $OWNER | Priority: $PRIORITY"
