#!/bin/sh
# One lesson cut -> the published files, FFmpeg only (no voice: operator, 2026-09-24).
#
#   sh tools/learn/assemble.sh DIR LANG [--force]
#
# DIR is a capture.mjs output (lesson.json, LANG/raw.webm, LANG/marks.json). Writes into DIR/LANG/:
#   video.mp4       720x1280, 24 fps, H.264 High, <= 1.4 Mbps, +faststart, no audio, step titles burned in,
#                   one MP4 chapter per step
#   subs_sq.vtt subs_en.vtt subs_uk.vtt   the captions, on this cut's timings (the player overlays any)
#   chapters.json   { n, key, startMs, endMs, title{sq,en,uk} } per step
#   step-N.mp4      each step as a short loop (<= 8 s, <= 400 KB, 540x960)
#   poster.jpg      a frame of step 1;   contact.jpg  one frame per step, tiled
#   .assembled      the inputs' hash: a re-run with nothing changed does nothing
# Every ffmpeg's exit code is checked; a failure stops the script and names the stage.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
DIR=${1:?usage: assemble.sh DIR LANG [--force]}
LANG_=${2:?usage: assemble.sh DIR LANG [--force]}
FORCE=${3:-}
CUT="$DIR/$LANG_"
PLAN="$CUT/plan"
FF="ffmpeg -nostdin -hide_banner -loglevel error -y"

lines=$(node "$HERE/assemble-plan.mjs" "$DIR" "$LANG_" $FORCE) || { echo "assemble: plan failed for $CUT" >&2; exit 1; }
eval "$lines"
if [ "$SKIP" = 1 ]; then echo "assemble: $CUT unchanged (hash ${HASH%${HASH#????????????}}), nothing to do"; exit 0; fi
rm -f "$CUT/.assembled"

run() { stage=$1; shift; "$@" || { echo "assemble: $stage failed (rc=$?) for $CUT" >&2; exit 1; }; }

# 1. the cut: burn-in graph from the plan, chapters from ffmeta
run video $FF -i "$CUT/raw.webm" -i "$PLAN/ffmeta.txt" -filter_complex_script "$PLAN/filter.txt" \
  -map '[v]' -map_metadata 1 -map_chapters 1 -an \
  -c:v libx264 -preset veryfast -profile:v high -crf 23 -maxrate 1400k -bufsize 2800k -r 24 -g 48 \
  -pix_fmt yuv420p -movflags +faststart "$CUT/video.mp4"

# 2. the loops: one per step, re-encoded tighter when one lands over 400 KB
rm -f "$CUT"/step-*.mp4
while IFS="$(printf '\t')" read -r n ss dur; do
  [ -n "$n" ] || continue
  for crf in 28 33 38; do
    run "loop $n" $FF -ss "$ss" -t "$dur" -i "$CUT/video.mp4" -an -vf "scale=540:960:flags=lanczos" \
      -c:v libx264 -preset veryfast -crf "$crf" -maxrate 380k -bufsize 760k -r 24 -pix_fmt yuv420p \
      -movflags +faststart "$CUT/step-$n.mp4"
    [ "$(wc -c < "$CUT/step-$n.mp4")" -le 409600 ] && break
  done
done < "$PLAN/steps.tsv"

# 3. poster and contact sheet
run poster $FF -ss "$POSTER" -i "$CUT/video.mp4" -frames:v 1 -q:v 4 "$CUT/poster.jpg"
rm -f "$PLAN"/sheet-*.png
i=0
for t in $SHEET; do
  i=$((i + 1))
  run "sheet $i" $FF -ss "$t" -i "$CUT/video.mp4" -frames:v 1 -vf "scale=240:-2" "$PLAN/sheet-$(printf %02d $i).png"
done
ROWS=$(( (STEPS + COLS - 1) / COLS ))
run contact $FF -framerate 1 -i "$PLAN/sheet-%02d.png" -vf "tile=${COLS}x${ROWS}:padding=8:margin=8:color=0x16130f" \
  -frames:v 1 -q:v 3 "$CUT/contact.jpg"

echo "$HASH" > "$CUT/.assembled"
echo "assemble: $CUT -> video.mp4 ($(wc -c < "$CUT/video.mp4") bytes, ${DURATION}s, $STEPS steps)"
