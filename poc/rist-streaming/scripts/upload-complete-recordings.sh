#!/usr/bin/env sh
set -eu

: "${MINIO_ROOT_USER:=minioadmin}"
: "${MINIO_ROOT_PASSWORD:=minioadmin}"
: "${MINIO_BUCKET:=recordings}"
: "${ROOM_NAME:=rist-poc}"
: "${UPLOAD_SCAN_INTERVAL:=2}"
: "${UPLOAD_SHUTDOWN_GRACE:=30}"
: "${RECORD_DIR:=/recordings}"

state_dir=/tmp/recording-uploader
mkdir -p "$state_dir"

mc alias set local http://minio:9000 "$MINIO_ROOT_USER" "$MINIO_ROOT_PASSWORD"
mc mb --ignore-existing "local/$MINIO_BUCKET"

upload_stable_file() {
  file=$1
  base=$(basename "$file")
  current_size=$(wc -c < "$file" | tr -d ' ')
  previous_file="$state_dir/$base.previous-size"
  uploaded_file="$state_dir/$base.uploaded-size"
  previous_size=
  uploaded_size=

  [ -f "$previous_file" ] && previous_size=$(cat "$previous_file")
  [ -f "$uploaded_file" ] && uploaded_size=$(cat "$uploaded_file")

  if [ "$current_size" = "$previous_size" ] && [ "$current_size" != "$uploaded_size" ]; then
    mc cp "$file" "local/$MINIO_BUCKET/$ROOM_NAME/$base"
    echo "$current_size" > "$uploaded_file"
  fi

  echo "$current_size" > "$previous_file"
}

upload_playlist() {
  playlist="$RECORD_DIR/index.m3u8"
  [ -f "$playlist" ] || return 0
  mc cp "$playlist" "local/$MINIO_BUCKET/$ROOM_NAME/index.m3u8"
}

upload_once() {
  for file in "$RECORD_DIR"/*.ts; do
    [ -f "$file" ] || continue
    upload_stable_file "$file"
  done

  upload_playlist
}

shutdown() {
  echo "Final upload pass before shutdown ..." >&2
  remaining=$UPLOAD_SHUTDOWN_GRACE
  while [ "$remaining" -gt 0 ]; do
    upload_once
    sleep 1
    remaining=$((remaining - 1))
  done
  exit 0
}

trap shutdown INT TERM

while :; do
  upload_once

  sleep "$UPLOAD_SCAN_INTERVAL"
done
