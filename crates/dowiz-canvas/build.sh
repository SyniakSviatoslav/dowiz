#!/bin/sh
# Build the canvas board's wasm and PROMOTE it to the surface, with the digest of its sources.
#
#   sh crates/dowiz-canvas/build.sh            build + copy + write board.wasm.src
#   sh crates/dowiz-canvas/build.sh --digest   print the source digest only (the wire gate uses it)
#
# Toolchain: the wasm32 target lives in the rustup toolchain the repo pins (1.96.1), not in the
# distro cargo on PATH (memory wasm32-needs-upstream-toolchain), so cargo is taken explicitly.
# On this box run it through slot.sh: bash bebop-lang/tools/slot.sh <lane> sh crates/dowiz-canvas/build.sh
#
# WHY A DIGEST: a promoted binary that is not its source measured nothing (memory
# bebop-binary-must-match-source). tools/gates/canvas-wire.sh recomputes it and refuses a
# board.wasm whose recorded digest is not the digest of the sources in the tree.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
OUT="$HERE/../../workers/api/public/room/canvas"

digest() {
  ( cd "$HERE" && find Cargo.toml src -type f -name '*.rs' -o -name Cargo.toml -type f | grep -v '/tests\.rs$' | LC_ALL=C sort \
      | while read -r f; do printf '%s\n' "$f"; cat "$f"; done ) | sha256sum | cut -c1-16
}
if [ "${1:-}" = "--digest" ]; then digest; exit 0; fi

CARGO="${CARGO:-$HOME/.cargo/bin/cargo}"
cd "$HERE"
"$CARGO" +1.96.1 build --release --target wasm32-unknown-unknown
WASM="$HERE/target/wasm32-unknown-unknown/release/dowiz_canvas.wasm"
[ -s "$WASM" ] || { echo "build.sh: no module at $WASM"; exit 1; }
cp "$WASM" "$OUT/board.wasm"
digest > "$OUT/board.wasm.src"
echo "build.sh: board.wasm $(wc -c < "$OUT/board.wasm") B raw, $(gzip -9c "$OUT/board.wasm" | wc -c) B gzip, src $(cat "$OUT/board.wasm.src")"
