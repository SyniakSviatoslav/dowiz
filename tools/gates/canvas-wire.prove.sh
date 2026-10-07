#!/bin/sh
# canvas-wire's proof: the gate fires both ways (no browser; seconds).
set -u
. "$(dirname "$0")/canvas-prove-lib.inc"
G="$HERE/canvas-wire.sh"
copy; want 0 "clean" "$G" "$C"
copy; mutate index.html '<body><canvas></canvas></body>' '<body><canvas></canvas><div></div></body>'; want 1 "a stray element in <body>" "$G" "$C"
copy; echo 0000000000000000 > "$C/board.wasm.src"; want 1 "board.wasm built from other sources" "$G" "$C"
copy; head -c 70000 /dev/urandom | base64 | sed 's#^#// #' >> "$C/loader.js"; want 1 "70 KB of incompressible bytes in the loader" "$G" "$C"
copy; sed "s/ 'wasm-unsafe-eval'//" "$REPO/workers/api/public/_headers" > "$SCRATCH/headers"; want 1 "CSP without 'wasm-unsafe-eval'" "$G" "$C" "$SCRATCH/headers"
copy; mutate index.html '</head>' '<style>body{margin:0}</style></head>'; want 1 "an inline <style> the CSP blocks" "$G" "$C"
copy; rm "$C/board.wasm"; want 2 "no module at all (never a pass)" "$G" "$C"
[ $fail -eq 0 ] && echo "canvas-wire.prove: the gate fires in both directions" || echo "canvas-wire.prove: FAILED"
exit $fail
