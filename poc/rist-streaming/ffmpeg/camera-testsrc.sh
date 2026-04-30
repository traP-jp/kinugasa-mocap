#!/usr/bin/env sh
set -eu

: "${RIST_PUSH_URL:=rist://rist-server:9000}"
: "${RIST_VIDEO_CODEC:=libx264}"
: "${RIST_H264_PROFILE:=baseline}"
: "${RIST_H264_LEVEL:=3.1}"
: "${RIST_VIDEO_BITRATE:=3000k}"
: "${RIST_GOP_SIZE:=60}"

printf 'Sending testsrc over RIST with video codec=%s profile=%s level=%s bitrate=%s gop=%s\n' \
  "$RIST_VIDEO_CODEC" "$RIST_H264_PROFILE" "$RIST_H264_LEVEL" "$RIST_VIDEO_BITRATE" "$RIST_GOP_SIZE"

exec ffmpeg -hide_banner -loglevel info \
  -re \
  -f lavfi \
  -i "testsrc2=size=1280x720:rate=30" \
  -f lavfi \
  -i "sine=frequency=1000:sample_rate=48000" \
  -c:v "$RIST_VIDEO_CODEC" \
  -preset veryfast \
  -tune zerolatency \
  -profile:v "$RIST_H264_PROFILE" \
  -level:v "$RIST_H264_LEVEL" \
  -x264-params repeat-headers=1 \
  -pix_fmt yuv420p \
  -g "$RIST_GOP_SIZE" \
  -b:v "$RIST_VIDEO_BITRATE" \
  -c:a aac \
  -b:a 128k \
  -ar 48000 \
  -ac 2 \
  -f mpegts \
  "$RIST_PUSH_URL"
