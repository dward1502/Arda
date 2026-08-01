#!/usr/bin/env bash
set -euo pipefail

# Arda Queue Hygiene Monitoring Script
# This script monitors active queue hygiene and reports historical evidence drift.

# Configuration
ROOT_DIR="${ARDA_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
HYGIENE_FILE="${ARDA_QUEUE_HYGIENE_FILE:-$ROOT_DIR/core/state/queue_hygiene.json}"
ACTIVE_ALERT_THRESHOLD="${ARDA_QUEUE_ACTIVE_ALERT_THRESHOLD:-0}"
STALE_WARNING_THRESHOLD="${ARDA_QUEUE_STALE_WARNING_THRESHOLD:-10}"

# Monitor queue hygiene state
monitor_hygiene() {
    echo "Monitoring queue hygiene state..."

    if [[ ! -r "$HYGIENE_FILE" ]]; then
        echo "Queue hygiene state is unavailable: $HYGIENE_FILE" >&2
        return 1
    fi

    LATEST_OPEN_TOTAL=$(jq -r '.metrics.latest_open_total // 0' "$HYGIENE_FILE")
    STALE_RAW_QUEUED_ROWS=$(jq -r '.metrics.stale_raw_queued_rows_total // 0' "$HYGIENE_FILE")
    RAW_QUEUED_ROWS=$(jq -r '.metrics.raw_queued_rows_total // 0' "$HYGIENE_FILE")

    if [ "$LATEST_OPEN_TOTAL" -gt "$ACTIVE_ALERT_THRESHOLD" ]; then
        echo "ALERT: Active queue backlog is above threshold: latest_open_total=$LATEST_OPEN_TOTAL threshold=$ACTIVE_ALERT_THRESHOLD"
    else
        echo "Queue hygiene state is normal. Active queue backlog: $LATEST_OPEN_TOTAL"
    fi

    if [ "$STALE_RAW_QUEUED_ROWS" -gt "$STALE_WARNING_THRESHOLD" ]; then
        echo "WARN: Historical stale raw queued rows: $STALE_RAW_QUEUED_ROWS raw_queued_rows_total=$RAW_QUEUED_ROWS"
        echo "WARN: These are append-only evidence rows and are not active backlog when latest_open_total=$LATEST_OPEN_TOTAL."
    else
        echo "Historical stale raw queued rows: $STALE_RAW_QUEUED_ROWS"
    fi
}

# Main execution
monitor_hygiene
