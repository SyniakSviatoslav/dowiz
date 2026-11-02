#!/bin/sh
# SPDX-License-Identifier: MIT OR Apache-2.0
# reader-repro (W-MG-REPRO, 2026-11-01): THE PIN IS WHAT THE SOURCE BUILDS. A file that reads itself
# is worth what its pinned reader is worth, and a stranger can only check the pin by rebuilding it.
# a7507f4c pinned 3f098ec8 and a control build of that source gave cb81bdf3: nothing compared them,
# because tools/gates/atk3.sh holds the pins equal to EACH OTHER, not to the source.
#
#   sh tools/gates/reader-repro.sh         (two wasm32 builds: on the box under slot.sh, never nested)
#
# Copies what Cargo.lock names into two directories of different name and length, runs each copy's
# own tools/verify/build.sh, and requires one sha, equal to WASM_SHA256 (ein-verify.mjs), PINNED
# (read.mjs) and the sha of the embedded base64. The repo is only read. Whether build.sh's ratchet
# accepted the module is reported, not judged: that is verify-gate's question.
#
# Exit: 0 GREEN, 1 RED, 2 REFUSED (no pinned toolchain or wasm32 std), 3 NO VERDICT (a build died, or
# made no module). A gate that cannot measure never passes (memory: gates-that-count-skips-as-passes).
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
C="$ROOT/crates/einfile-reader"
unset BWR_TARGET_DIR   # one shared target dir would make "two directories" one build
say() { echo "reader-repro: $*"; }
red() { say "RED $*"; exit 1; }
want=$(sed -n 's/^channel = "\(.*\)"$/\1/p' "$ROOT/rust-toolchain.toml")
RUSTC="${RUSTC:-$HOME/.cargo/bin/rustc}"; [ -x "$RUSTC" ] || RUSTC=rustc
got=$(cd "$C" && "$RUSTC" --version 2>&1 | cut -d' ' -f2)
[ -n "$want" ] && [ "$got" = "$want" ] || { say "REFUSED rustc here is '$got', rust-toolchain.toml pins '$want' (rustup, not the distro rustc)"; exit 2; }
lib=$(cd "$C" && "$RUSTC" --print target-libdir --target wasm32-unknown-unknown 2>/dev/null)
[ -d "$lib" ] || { say "REFUSED rustc $got has no wasm32-unknown-unknown std"; exit 2; }

S="$(mktemp -d "${TMPDIR:-/tmp}/reader-repro.XXXXXX")" || exit 2
trap 'rm -rf "$S"' EXIT
crates=$(sed -n 's/^name = "\(.*\)"$/\1/p' "$C/Cargo.lock")
checkout() { # checkout <dir>: exactly what build.sh reads, found from the lockfile and never listed
  mkdir -p "$1/crates" "$1/tools/gates" "$1/tools/verify" "$1/examples" || exit 2
  for c in $crates; do
    tar -C "$ROOT/crates" --exclude=target -cf - "$c" | tar -C "$1/crates" -xf - || exit 2
  done
  cp -a "$ROOT/tools/verify/." "$1/tools/verify/" && cp "$ROOT/tools/gates/atk3_pin.py" "$1/tools/gates/" \
    && cp "$ROOT/examples/read.mjs" "$1/examples/" && cp "$ROOT/rust-toolchain.toml" "$1/" \
    && cp -a "$ROOT/.cargo" "$1/" || exit 2
}
build() { # build <dir>: sets $sha and $bytes, or stops with NO VERDICT
  checkout "$1"
  sh "$1/tools/verify/build.sh" > "$S/log" 2>&1; brc=$?
  w="$1/crates/einfile-reader/target/wasm32-unknown-unknown/release/einfile_reader.wasm"
  if [ ! -s "$w" ]; then
    grep -q 'signal: 9' "$1/tools/verify/.build.log" 2>/dev/null && say "the compiler was SIGKILLed, by the box and not by the source"
    say "NO VERDICT no module in $(basename "$1"): rc=$brc -- $(tail -1 "$S/log")"; exit 3
  fi
  sha=$(sha256sum "$w" | cut -c1-64); bytes=$(wc -c < "$w" | tr -d ' ')
  say "$(basename "$1"): $sha $bytes B (build.sh rc=$brc: $(grep '^verify-build:' "$S/log" | tail -1 | cut -c15-90))"
}
build "$S/a"; sha1=$sha
build "$S/a-second-checkout-under-a-longer-name"
[ "$sha1" = "$sha" ] || red "the same source built $sha1 and $sha: the bytes depend on the directory (is rustc-stable-id.sh still build.sh's RUSTC_WRAPPER?)"

pin=$(sed -n 's/^const PINNED = "\([0-9a-f]\{64\}\)";$/\1/p' "$ROOT/examples/read.mjs")
sum=$(sed -n 's/^const WASM_SHA256 = "\([0-9a-f]\{64\}\)";$/\1/p' "$ROOT/tools/verify/ein-verify.mjs")
emb=$(sed -n 's/^const WASM_B64 = "\(.*\)";$/\1/p' "$ROOT/tools/verify/ein-verify.mjs" | base64 -d 2>/dev/null | sha256sum | cut -c1-64)
fix="rebuild with tools/verify/build.sh and commit tools/verify/ein-verify.mjs and examples/read.mjs"
[ "$sum" = "$sha" ] || red "WASM_SHA256 is ${sum:-missing}, the source builds $sha: $fix"
[ "$pin" = "$sha" ] || red "read.mjs PINNED is ${pin:-missing}, the source builds $sha: $fix"
[ "$emb" = "$sha" ] || red "the embedded base64 is $emb, the source builds $sha: $fix"
say "GREEN the source builds $sha, $bytes B, in two directories, equal to WASM_SHA256, PINNED and the embedded base64 (rustc $got, host $(uname -m))"
