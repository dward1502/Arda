#!/bin/sh
set -eu

payload=$(printf '{"crawl_service_url":"%s","filter":"%s"}' \
  "$ARDA_CRAWL4AI_URL" \
  "$ARDA_EXTERNAL_LANE_FILTER")
exec /usr/bin/curl \
  --fail \
  --silent \
  --show-error \
  --max-time 120 \
  --header 'Content-Type: application/json' \
  --data "$payload" \
  "${ARDA_VARDA_URL}/external_lane/import"
