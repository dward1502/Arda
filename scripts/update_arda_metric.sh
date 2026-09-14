#!/usr/bin/env bash
set -euo pipefail

# Export a Prometheus metric indicating the last successful ARDA projection refresh.
# The metric is written to a textfile exporter compatible file.
METRICS_DIR="/var/home/mythos/Eregion/Arda/metrics"
FILE="$METRICS_DIR/arda_projection_last_success.prom"
mkdir -p "$METRICS_DIR"
# Current unix timestamp
NOW=$(date +%s)
# Write metric in Prometheus text exposition format
cat > "$FILE" <<EOF
# HELP arda_projection_last_success_timestamp Unix timestamp of the last successful ARDA projection export.
# TYPE arda_projection_last_success_timestamp gauge
arda_projection_last_success_timestamp $NOW
EOF

echo "Metric updated: $FILE"
