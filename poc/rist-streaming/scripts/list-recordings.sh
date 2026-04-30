#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/.."

docker compose up -d minio minio-init
docker compose run --rm --entrypoint /bin/sh minio-init -ec '
mc alias set local http://minio:9000 "$MINIO_ROOT_USER" "$MINIO_ROOT_PASSWORD" >/dev/null
mc ls --recursive "local/$MINIO_BUCKET"
'
