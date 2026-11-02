#!/bin/sh
# SHEAF S2: build the kitchen tablet's writer (`--features tablet`, src/tablet.rs over
# dowiz_hub::glue::tablet) and PROMOTE it to the canvas board, with the digest of its sources.
#
#   sh crates/dowiz-hub-wasm/tablet.sh            build + copy to public/room/canvas/tablet.wasm + .src
#   sh crates/dowiz-hub-wasm/tablet.sh --digest   print the source digest only
#
# Built apart (`target/tablet/`), so the reader module `bytes.baseline` ratchets never carries
# dowiz-hub. On this box run it through the slot: bash bebop-lang/tools/slot.sh <lane> sh <this>.
# WHY A DIGEST: a promoted binary that is not its source measured nothing (memory
# bebop-binary-must-match-source); `--digest` recomputes it, and the two must be equal.
#
# THE DIGEST IS EVERYTHING THE BYTES DEPEND ON (W-MG-TAB, 2026-11-01). It named six crates by hand and
# the graph had ten: dowiz-core, einfile-crc and einfile-sig-mldsa could change and exit 15 never moved, and
# neither did dowiz-hub's include_bytes! weights, the lockfile or the toolchain. So the crate list now
# comes from Cargo.lock (a package with no `source` is a path crate, directory crates/<name>), which
# means a new path dependency is covered with no edit here. Every file under each src/ counts, not only
# .rs. Out: tests.rs and *_tests.rs (every `mod` of one is under cfg(test), checked 2026-11-01) and
# dowiz-core's KAT vectors (read by tests only). A missing input is a refusal, never a smaller digest.
#
# THE SAME SOURCE IS THE SAME BYTES, IN ANY DIRECTORY. Two inputs moved with the tree. The remap takes
# the box's paths out of the panic strings: this repo and the cargo registry, both, or the module
# ships /root/lanes/... and /root/.cargo/... The wrapper is tools/verify/rustc-stable-id.sh, the one
# the MIT reader uses, not a copy: this crate's path dependencies sit outside its one-crate workspace,
# so cargo hashes their absolute path into `-C metadata` (see that file). Its header says one crate of
# each name; this graph has two `syn`, under serde_derive and zeroize_derive. They are harmless here:
# a proc macro's dependencies never reach the crate that uses it, and were they ever to meet, rustc
# stops on the colliding ids rather than guess.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
OUT="$ROOT/workers/api/public/room/canvas"
WRAP=tools/verify/rustc-stable-id.sh

die() { echo "tablet.sh: $*" >&2; exit 2; }
path_crates() {
  awk '/^\[\[package\]\]/ { if (n != "" && !s) print n; n = ""; s = 0 }
       /^name = /         { gsub(/"/, "", $3); n = $3 }
       /^source = /       { s = 1 }
       END                { if (n != "" && !s) print n }' "$HERE/Cargo.lock"
}
digest() (
  cd "$ROOT"
  crates=$(path_crates); [ -n "$crates" ] || die "no path crate in Cargo.lock"
  for c in $crates; do [ -d "crates/$c/src" ] || die "path crate $c has no crates/$c/src"; done
  list=$( { printf '%s\n' .cargo/config.toml rust-toolchain.toml crates/dowiz-hub-wasm/Cargo.lock crates/dowiz-hub-wasm/tablet.sh "$WRAP"
            for c in $crates; do
              printf '%s\n' "crates/$c/Cargo.toml"
              [ ! -f "crates/$c/build.rs" ] || printf '%s\n' "crates/$c/build.rs"
              find "crates/$c/src" -type f
            done; } | grep -Ev '(/|_)tests\.rs$|/pq/kat/' | LC_ALL=C sort -u )
  for f in $list; do [ -f "$f" ] || die "missing input $f"; done
  printf '%s\n' "$list" | xargs sha256sum | sha256sum | cut -c1-16
)
if [ "${1:-}" = "--digest" ]; then digest; exit 0; fi

CARGO="${CARGO:-$HOME/.cargo/bin/cargo}"
TDIR="${TABLET_TARGET:-$HERE/target/tablet}"
REG="${CARGO_HOME:-$HOME/.cargo}/registry/src"
# The pin is the compiler: a bump must rebuild with the new one, not re-record the old one.
TC=$(sed -n 's/^channel *= *"\([^"]*\)".*/\1/p' "$ROOT/rust-toolchain.toml"); [ -n "$TC" ] || die "no channel in rust-toolchain.toml"
[ -x "$ROOT/$WRAP" ] || die "$WRAP is missing or not executable"
before=$(digest)
cd "$HERE"
# CARGO_ENCODED_RUSTFLAGS would silently win over RUSTFLAGS and a second wrapper would stack on this one.
unset CARGO_ENCODED_RUSTFLAGS RUSTC_WORKSPACE_WRAPPER
CARGO_INCREMENTAL=0 RUSTC_WRAPPER="$ROOT/$WRAP" RUSTFLAGS="--remap-path-prefix=$ROOT=/dowiz --remap-path-prefix=$REG=/cargo/registry/src" \
  "$CARGO" "+$TC" build --offline --locked --release --features tablet --target wasm32-unknown-unknown --target-dir "$TDIR"
WASM="$TDIR/wasm32-unknown-unknown/release/dowiz_hub_wasm.wasm"
[ -s "$WASM" ] || { echo "tablet.sh: no module at $WASM"; exit 1; }
[ "$(digest)" = "$before" ] || { echo "tablet.sh: the sources moved during the build, nothing promoted"; exit 1; }
cp "$WASM" "$OUT/tablet.wasm"
echo "$before" > "$OUT/tablet.wasm.src"
echo "tablet.sh: tablet.wasm $(wc -c < "$OUT/tablet.wasm") B raw, $(gzip -9c "$OUT/tablet.wasm" | wc -c) B gzip, src $(cat "$OUT/tablet.wasm.src")"
