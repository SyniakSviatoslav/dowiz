#!/bin/sh
# canvas-frame's proof: the doubling defect put back (memory sea-canvas-doubles-every-frame).
set -u
. "$(dirname "$0")/canvas-prove-lib.inc"
G="$HERE/canvas-frame.sh"
copy; want 0 "clean" "$G" --canvas-dir "$C"
# The Sea's exact shape: no CSS size on the canvas, and the size read back from the element. (Dropping
# only the width is NOT the defect: with a CSS height, the canvas keeps its aspect ratio and the
# element width stays innerWidth -- measured 2026-10-07, S/canvas1-frame-mut.out.)
copy; mutate loader.js "[canvas, 'width', '100vw'], [canvas, 'height', '100vh'], " ""
mutate loader.js 'w = Math.max(1, Math.round(innerWidth));' 'w = Math.max(1, Math.round(canvas.clientWidth || innerWidth));'
want 1 "the backing store sized from the element (grows per resize at dpr 2)" "$G" --canvas-dir "$C"
[ $fail -eq 0 ] && echo "canvas-frame.prove: the gate fires in both directions" || echo "canvas-frame.prove: FAILED"
exit $fail
