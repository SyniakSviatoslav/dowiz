#!/bin/sh
# SHEAF S2: build the kitchen tablet's writer (`--features tablet`, src/tablet.rs over
# dowiz_hub::glue::tablet) and PROMOTE it to the canvas board, with the digest of its sources.
#
#   sh crates/bebop-wasm/tablet.sh            build + copy to public/room/canvas/tablet.wasm + .src
#   sh crates/bebop-wasm/tablet.sh --digest   print the source digest only
#
# Built apart (`target/tablet/`), so the reader module `bytes.baseline` ratchets never carries
# dowiz-hub. On this box run it through the slot: bash bebop-lang/tools/slot.sh <lane> sh <this>.
# WHY A DIGEST: a promoted binary that is not its source measured nothing (memory
# bebop-binary-must-match-source); `--digest` recomputes it, and the two must be equal.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
HUB="$HERE/../dowiz-hub"
OUT="$HERE/../../workers/api/public/room/canvas"

digest() {
  ( cd "$HERE/.." && find bebop-wasm/Cargo.toml bebop-wasm/src dowiz-hub/Cargo.toml dowiz-hub/src bebop-store/src bebop-image/src bebop-kv/src bebop-log/src bebop-io/src -type f \( -name '*.rs' -o -name Cargo.toml \) \
      | grep -v '/tests\.rs$' | LC_ALL=C sort | while read -r f; do printf '%s\n' "$f"; cat "$f"; done ) | sha256sum | cut -c1-16
}
if [ "${1:-}" = "--digest" ]; then digest; exit 0; fi

CARGO="${CARGO:-$HOME/.cargo/bin/cargo}"
TDIR="${TABLET_TARGET:-$HERE/target/tablet}"
cd "$HERE"
"$CARGO" +1.96.1 build --offline --release --features tablet --target wasm32-unknown-unknown --target-dir "$TDIR"
WASM="$TDIR/wasm32-unknown-unknown/release/bebop_wasm.wasm"
[ -s "$WASM" ] || { echo "tablet.sh: no module at $WASM"; exit 1; }
cp "$WASM" "$OUT/tablet.wasm"
digest > "$OUT/tablet.wasm.src"
echo "tablet.sh: tablet.wasm $(wc -c < "$OUT/tablet.wasm") B raw, $(gzip -9c "$OUT/tablet.wasm" | wc -c) B gzip, src $(cat "$OUT/tablet.wasm.src")"
