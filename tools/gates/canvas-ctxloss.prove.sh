#!/bin/sh
# canvas-ctxloss's proof: a restore that does not redraw is caught.
set -u
. "$(dirname "$0")/canvas-prove-lib.inc"
G="$HERE/canvas-ctxloss.sh"
copy; want 0 "clean" "$G" --canvas-dir "$C"
copy; mutate loader.js "size(); draw(lastNow); });" "});"; want 1 "contextrestored does not redraw" "$G" --canvas-dir "$C"
[ $fail -eq 0 ] && echo "canvas-ctxloss.prove: the gate fires in both directions" || echo "canvas-ctxloss.prove: FAILED"
exit $fail
