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
copy; mutate board.js 'const WASM =' "import './table.js'; const WASM ="; want 1 "table.js imported eagerly (pulled into the first frame)" "$G" "$C"
copy; echo "export { open } from './table.js';" >> "$C/feed.js"; want 1 "table.js re-exported by a first-frame module" "$G" "$C"
copy; head -c 12000 /dev/urandom | base64 | sed 's#^#// #' >> "$C/table.js"; want 1 "the lazy table.js over its own budget" "$G" "$C"
copy; cp "$C/table.js" "$C/sheet2.js"; head -c 12000 /dev/urandom | base64 | sed 's#^#// #' >> "$C/sheet2.js"; mutate board.js "import('./table.js')" "import('./sheet2.js')"; want 1 "a new import() with no row is charged to the first frame" "$G" "$C"
copy; want 2 "no baseline file (never a pass)" "$G" "$C" "$REPO/workers/api/public/_headers" "$SCRATCH/none.baseline"
copy; rm "$C/board.wasm"; want 2 "no module at all (never a pass)" "$G" "$C"
[ $fail -eq 0 ] && echo "canvas-wire.prove: the gate fires in both directions" || echo "canvas-wire.prove: FAILED"
exit $fail
