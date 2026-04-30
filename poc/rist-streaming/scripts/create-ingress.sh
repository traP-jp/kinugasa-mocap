#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/.."

docker compose up -d redis livekit ingress

output="$(docker compose run --rm lk ingress create config/ingress-rtmp.json)"
printf '%s\n' "$output"

rtmp_url="$(printf '%s\n' "$output" | sed -nE 's/.*(rtmp:\/\/[^[:space:]]+).*/\1/p' | head -n 1 || true)"
stream_key="$(printf '%s\n' "$output" | sed -nE 's/.*stream[_ -]?key[^[:alnum:]]+([[:alnum:]_-]+).*/\1/Ip' | head -n 1 || true)"

if [ -n "$rtmp_url" ] && echo "$rtmp_url" | grep -q '/live/'; then
  publish_url="$rtmp_url"
elif [ -n "$rtmp_url" ] && [ -n "$stream_key" ]; then
  publish_url="${rtmp_url%/}/$stream_key"
else
  cat <<'EOF'

Could not auto-detect the RTMP publish URL from lk output.
Copy the RTMP URL and stream key from the output above, then set:

  LIVEKIT_RTMP_URL=rtmp://ingress:1935/live/<stream-key>

EOF
  exit 0
fi

cat <<EOF

Set this before starting the RIST bridge:

  export LIVEKIT_RTMP_URL=$publish_url

Or add this line to poc/rist-streaming/.env:

  LIVEKIT_RTMP_URL=$publish_url

EOF
