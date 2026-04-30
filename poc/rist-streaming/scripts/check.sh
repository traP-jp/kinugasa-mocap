#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/.."

docker compose config >/dev/null
docker compose build rist-server >/dev/null
docker compose run --rm --no-deps rist-server /usr/local/bin/check-ffmpeg.sh
