#!/usr/bin/env sh
set -eu

ffmpeg -hide_banner -protocols | grep -q ' rist$'
ffmpeg -hide_banner -muxers | grep -q ' flv '
ffmpeg -hide_banner -muxers | grep -q ' hls '
ffmpeg -hide_banner -encoders | grep -q ' libx264'

echo "ffmpeg has rist, flv, hls, and libx264 support"
