#!/bin/sh
# CANVAS DOM (Wave CV7) -- see tools/gates/canvas.mjs for what is measured and the threshold.
#   sh tools/gates/canvas-dom.sh [--canvas-dir DIR] [--out FILE]
# Exit 0 pass, 1 a threshold failed, 2 the harness could not run (never counted as a pass).
# Run on this box through slot.sh (one Chromium): bash bebop-lang/tools/slot.sh <lane> sh tools/gates/canvas-dom.sh
HERE=$(cd "$(dirname "$0")" && pwd)
exec node "$HERE/canvas.mjs" dom "$@"
