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
# THE DIGEST IS EVERYTHING THE BYTES DEPEND ON (W-MG-TAB, 2026-11-01): the crate, its lockfile, this
# script, the toolchain pin, the cargo config, and every file under src/ except the test files. No
# remap and no stable-id wrapper here, unlike crates/bebop-wasm/tablet.sh: this crate has no
# dependencies and is its own workspace root, so cargo passes relative paths, none is hashed, and the
# committed module carries no box path.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
OUT="$ROOT/workers/api/public/room/canvas"

digest() (
  cd "$ROOT"
  list=$( { printf '%s\n' .cargo/config.toml rust-toolchain.toml crates/dowiz-canvas/Cargo.toml crates/dowiz-canvas/Cargo.lock crates/dowiz-canvas/build.sh
            find crates/dowiz-canvas/src -type f; } | grep -Ev '(/|_)tests\.rs$' | LC_ALL=C sort -u )
  for f in $list; do [ -f "$f" ] || { echo "build.sh: missing input $f" >&2; exit 2; }; done
  printf '%s\n' "$list" | xargs sha256sum | sha256sum | cut -c1-16
)
if [ "${1:-}" = "--digest" ]; then digest; exit 0; fi

CARGO="${CARGO:-$HOME/.cargo/bin/cargo}"
# The pin is the compiler: a bump must rebuild with the new one, not re-record the old one.
TC=$(sed -n 's/^channel *= *"\([^"]*\)".*/\1/p' "$ROOT/rust-toolchain.toml"); [ -n "$TC" ] || { echo "build.sh: no channel in rust-toolchain.toml"; exit 2; }
before=$(digest)
cd "$HERE"
"$CARGO" "+$TC" build --locked --release --target wasm32-unknown-unknown
WASM="$HERE/target/wasm32-unknown-unknown/release/dowiz_canvas.wasm"
[ -s "$WASM" ] || { echo "build.sh: no module at $WASM"; exit 1; }
[ "$(digest)" = "$before" ] || { echo "build.sh: the sources moved during the build, nothing promoted"; exit 1; }
cp "$WASM" "$OUT/board.wasm"
echo "$before" > "$OUT/board.wasm.src"
echo "build.sh: board.wasm $(wc -c < "$OUT/board.wasm") B raw, $(gzip -9c "$OUT/board.wasm" | wc -c) B gzip, src $(cat "$OUT/board.wasm.src")"
