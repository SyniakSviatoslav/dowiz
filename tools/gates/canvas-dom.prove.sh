#!/bin/sh
# canvas-dom's proof (headless Chromium, ~3 browser starts per case).
set -u
. "$(dirname "$0")/canvas-prove-lib.inc"
G="$HERE/canvas-dom.sh"
copy; want 0 "clean" "$G" --canvas-dir "$C"
copy; mutate index.html '<body><canvas></canvas></body>' '<body><canvas></canvas><div></div></body>'; want 1 "a stray element at rest" "$G" --canvas-dir "$C"
copy; mutate loader.js 'function blurField() { dropInput();' 'function blurField() {'; want 1 "the transient input outlives the focus" "$G" --canvas-dir "$C"
[ $fail -eq 0 ] && echo "canvas-dom.prove: the gate fires in both directions" || echo "canvas-dom.prove: FAILED"
exit $fail
