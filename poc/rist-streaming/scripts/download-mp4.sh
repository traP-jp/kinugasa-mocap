#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/.."

: "${MINIO_ROOT_USER:=minioadmin}"
: "${MINIO_ROOT_PASSWORD:=minioadmin}"
: "${MINIO_BUCKET:=recordings}"
: "${ROOM_NAME:=rist-poc}"
: "${OUTPUT_DIR:=./out}"
: "${OUTPUT_MP4:=$ROOM_NAME.mp4}"

run_user="$(id -u):$(id -g)"
work_dir="./.work/download-mp4/$ROOM_NAME-$(date +%Y%m%dT%H%M%S)-$$"
mkdir -p "$work_dir" "$OUTPUT_DIR"

docker compose up -d minio minio-init

docker compose run --rm \
  --user "$run_user" \
  -e MINIO_ROOT_USER \
  -e MINIO_ROOT_PASSWORD \
  -e MINIO_BUCKET \
  -e ROOM_NAME \
  -e HOME=/tmp \
  -v "$PWD/$work_dir:/download" \
  --entrypoint /bin/sh \
  minio-init \
  -ec '
    mc alias set local http://minio:9000 "$MINIO_ROOT_USER" "$MINIO_ROOT_PASSWORD" >/dev/null
    mc mirror --overwrite "local/$MINIO_BUCKET/$ROOM_NAME/" /download
  '

if [ ! -f "$work_dir/index.m3u8" ]; then
  echo "index.m3u8 was not found in s3://$MINIO_BUCKET/$ROOM_NAME" >&2
  exit 1
fi

docker compose run --rm --no-deps \
  --user "$run_user" \
  -v "$PWD/$work_dir:/input:ro" \
  -v "$PWD/$OUTPUT_DIR:/output" \
  rist-server \
  ffmpeg -hide_banner -y \
    -allowed_extensions ALL \
    -i /input/index.m3u8 \
    -map 0:v:0? \
    -map 0:a:0? \
    -c copy \
    -movflags +faststart \
    "/output/$OUTPUT_MP4"

echo "$OUTPUT_DIR/$OUTPUT_MP4"
