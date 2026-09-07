#!/usr/bin/env bash
set -euo pipefail

FILE="data/athena/external_source_lane_ledger.jsonl"
if [[ ! -f "$FILE" ]]; then
  echo "ERROR: Ledger file not found: $FILE"
  exit 1
fi

echo "Validating Athena ledger $FILE..."
BAD=0
TOTAL=0
while IFS= read -r line; do
  TOTAL=$((TOTAL+1))
  # Validate JSON syntax
  if ! echo "$line" | jq . > /dev/null 2>&1; then
    echo "INVALID JSON: $line"
    BAD=$((BAD+1))
    continue
  fi
  # Check required keys (Arda schema uses id/kind/status, legacy uses timestamp/event/source_id)
  HAS_ID=$(echo "$line" | jq -e 'has("id")' 2>/dev/null || true)
  HAS_KIND=$(echo "$line" | jq -e 'has("kind")' 2>/dev/null || true)
  HAS_STATUS=$(echo "$line" | jq -e 'has("status")' 2>/dev/null || true)
  if [[ "$HAS_ID" != "true" || "$HAS_KIND" != "true" || "$HAS_STATUS" != "true" ]]; then
    # Check legacy keys
    HAS_TS=$(echo "$line" | jq -e 'has("timestamp")' 2>/dev/null || true)
    HAS_EV=$(echo "$line" | jq -e 'has("event")' 2>/dev/null || true)
    HAS_SRC=$(echo "$line" | jq -e 'has("source_id")' 2>/dev/null || true)
    if [[ "$HAS_TS" != "true" || "$HAS_EV" != "true" || "$HAS_SRC" != "true" ]]; then
      echo "MISSING required keys in: $line"
      BAD=$((BAD+1))
    fi
  fi
done < "$FILE"

if [[ $BAD -eq 0 ]]; then
  echo "All $TOTAL entries valid."
  exit 0
else
  echo "$BAD issues found in $FILE (out of $TOTAL)."
  exit 1
fi
