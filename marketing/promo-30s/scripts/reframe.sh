#!/bin/bash
# 16:9 and 1:1 from the 9:16 master: the stage is bone (#f2f1ec), so the tall frame sits centred on bone.
# usage: scripts/reframe.sh out/promo-en-1080x1920.mp4
set -e
in="$1"; base="${in%-1080x1920.mp4}"
ffmpeg -v error -y -i "$in" -vf "scale=-2:1080,pad=1920:1080:(ow-iw)/2:0:0xf2f1ec" -c:v libx264 -crf 18 -preset medium -c:a copy "${base}-1920x1080.mp4"
ffmpeg -v error -y -i "$in" -vf "scale=-2:1080,pad=1080:1080:(ow-iw)/2:0:0xf2f1ec" -c:v libx264 -crf 18 -preset medium -c:a copy "${base}-1080x1080.mp4"
echo "${base}-1920x1080.mp4 ${base}-1080x1080.mp4"
