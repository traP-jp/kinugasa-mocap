#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/.."

docker compose --profile stream --profile camera up rist-server recording-uploader camera-testsrc
