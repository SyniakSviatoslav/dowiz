#!/bin/sh
# canvas-frame's proof: the doubling defect put back (memory sea-canvas-doubles-every-frame).
set -u
. "$(dirname "$0")/canvas-prove-lib.inc"
G="$HERE/canvas-frame.sh"
node --test "$HERE/canvas-calib.test.mjs" > "$SCRATCH/calib" 2>&1 || { tail -20 "$SCRATCH/calib"; echo "prove: canvas-calib.test.mjs FAILED"; fail=1; }
copy; want 0 "clean" "$G" --canvas-dir "$C"
# A first frame 400 ms slower: a FIXED busy-wait just before the first draw. It was "as long again
# as the page has taken so far" (W-RGQUICK) -- RED here (665 ms vs mark 454) but a PASS twice on CI
# (runs 38068929237, 38071024351: board 60 ms with and without it, mark 300 then 110), so the
# doubling read nothing there. 400 ms is past the mark on both machines: CI ~460 vs ~110, this box
# ~730 vs ~454. If CI still passes this case, the mutated loader is not what the browser ran.
copy; mutate loader.js '  size();
  draw();' '  size();
  { const s = performance.now(); while (performance.now() - s < 400); }
  draw();'
want 1 "a first frame 400 ms slower" "$G" --canvas-dir "$C"
# The Sea's exact shape: no CSS size on the canvas, and the size read back from the element. (Dropping
# only the width is NOT the defect: with a CSS height, the canvas keeps its aspect ratio and the
# element width stays innerWidth -- measured 2026-10-07, S/canvas1-frame-mut.out.)
copy; mutate loader.js "[canvas, 'width', '100vw'], [canvas, 'height', '100vh'], " ""
mutate loader.js 'w = Math.max(1, Math.round(innerWidth));' 'w = Math.max(1, Math.round(canvas.clientWidth || innerWidth));'
want 1 "the backing store sized from the element (grows per resize at dpr 2)" "$G" --canvas-dir "$C"
[ $fail -eq 0 ] && echo "canvas-frame.prove: the gate fires in both directions" || echo "canvas-frame.prove: FAILED"
exit $fail
