#!/usr/bin/env sh
set -eu

: "${LIVEKIT_RTMP_URL:?LIVEKIT_RTMP_URL is required}"
: "${RECORD_DIR:=/recordings}"
: "${RIST_LISTEN_URL:=rist://@0.0.0.0:9000}"
: "${SEGMENT_TIME:=10}"

if echo "$LIVEKIT_RTMP_URL" | grep -q 'REPLACE_WITH_STREAM_KEY'; then
  echo "LIVEKIT_RTMP_URL still contains REPLACE_WITH_STREAM_KEY." >&2
  echo "Create a LiveKit RTMP ingress and set LIVEKIT_RTMP_URL first." >&2
  exit 64
fi

mkdir -p "$RECORD_DIR"

control_fifo=/tmp/rist-server-ffmpeg-control
rm -f "$control_fifo"
mkfifo "$control_fifo"
exec 3<> "$control_fifo"

ffmpeg_pid=

shutdown() {
  if [ -n "$ffmpeg_pid" ] && kill -0 "$ffmpeg_pid" 2>/dev/null; then
    echo "Finalizing ffmpeg output before shutdown ..." >&2
    printf 'q\n' > "$control_fifo" || true
    wait "$ffmpeg_pid" || true
  fi
}

trap shutdown INT TERM

ffmpeg -hide_banner -loglevel info \
  -fflags +genpts \
  -i "$RIST_LISTEN_URL" \
  -map 0:v:0? \
  -map 0:a:0? \
  -c:v libx264 \
  -preset veryfast \
  -tune zerolatency \
  -flags +global_header \
  -x264-params repeat-headers=1 \
  -pix_fmt yuv420p \
  -r 30 \
  -g 60 \
  -b:v 3000k \
  -c:a aac \
  -b:a 128k \
  -ar 48000 \
  -ac 2 \
  -f tee \
  "[f=flv:onfail=ignore]${LIVEKIT_RTMP_URL}|[f=hls:hls_time=${SEGMENT_TIME}:hls_list_size=0:hls_flags=temp_file:strftime=1:hls_segment_filename=${RECORD_DIR}/%Y%m%dT%H%M%S.ts]${RECORD_DIR}/index.m3u8" \
  < "$control_fifo" &

ffmpeg_pid=$!
wait "$ffmpeg_pid"
