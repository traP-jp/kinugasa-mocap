#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/.."

room="${ROOM_NAME:-rist-poc}"
identity="${1:-${VIEWER_IDENTITY:-viewer}}"

docker compose run --rm lk token create \
  --api-key "${LIVEKIT_API_KEY:-devkey}" \
  --api-secret "${LIVEKIT_API_SECRET:-devsecretdevsecretdevsecretdevsecret}" \
  --join \
  --room "$room" \
  --identity "$identity" \
  --valid-for 24h
