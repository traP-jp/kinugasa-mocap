#!/usr/bin/env sh
set -eu

: "${CAMERA_DEVICE:=/dev/video0}"
: "${CAMERA_SIZE:=1280x720}"
: "${CAMERA_FPS:=30}"
: "${RIST_PUSH_URL:=rist://rist-server:9000}"
: "${RIST_VIDEO_CODEC:=libx264}"
: "${RIST_H264_PROFILE:=baseline}"
: "${RIST_H264_LEVEL:=3.1}"
: "${RIST_VIDEO_BITRATE:=3000k}"
: "${RIST_GOP_SIZE:=60}"

printf 'Sending V4L2 camera over RIST with video codec=%s profile=%s level=%s bitrate=%s gop=%s\n' \
  "$RIST_VIDEO_CODEC" "$RIST_H264_PROFILE" "$RIST_H264_LEVEL" "$RIST_VIDEO_BITRATE" "$RIST_GOP_SIZE"

exec ffmpeg -hide_banner -loglevel info \
  -f v4l2 \
  -framerate "$CAMERA_FPS" \
  -video_size "$CAMERA_SIZE" \
  -i "$CAMERA_DEVICE" \
  -f lavfi \
  -i "anullsrc=channel_layout=stereo:sample_rate=48000" \
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
  -shortest \
  -f mpegts \
  "$RIST_PUSH_URL"
