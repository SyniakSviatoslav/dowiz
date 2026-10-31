#!/bin/sh
# SPDX-License-Identifier: MIT OR Apache-2.0
# Build the MIT reader (crates/bebop-wasm-reader) for wasm32, ratchet its bytes, and write the
# single-file Node verifier tools/verify/bebop-verify.mjs with the module embedded as base64.
#
#   tools/verify/build.sh            (heavy: one cargo build -- run it through slot.sh, never nested)
#
# THE RATCHET IS THIS CRATE'S OWN: tools/verify/bytes.baseline, never crates/bebop-wasm's 48501.
# Written when absent; a build above it is refused; a build below it says so and the number is
# lowered by hand in the commit that earns it. BWR_RATCHET=write records a build ABOVE it instead
# of refusing -- for the commit that deliberately grows the reader (W-MG-M5: DOC, sha256, verify)
# and says the new number in its message; never set it in a gate. The path is remapped to /dowiz
# exactly as bebop-wasm/gate.sh does, so the count is the code's and not the checkout's.
#
# THE .mjs IS GENERATED AND COMMITTED: run-all (no --cargo) and a CI job without a wasm32 toolchain
# read it, and "one file, no npm" is the deliverable. This script says UNCHANGED or REWRITTEN so a
# stale committed copy is visible; tools/gates/verify-file.sh reads the committed one.
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
V="$ROOT/tools/verify"
C="$ROOT/crates/bebop-wasm-reader"
CARGO="${CARGO:-$HOME/.cargo/bin/cargo}"
[ -x "$CARGO" ] || CARGO=cargo
TARGET="${BWR_TARGET_DIR:-$C/target}"
WASM="$TARGET/wasm32-unknown-unknown/release/bebop_wasm_reader.wasm"
say() { echo "verify-build: $*"; }

if ! (cd "$C" && CARGO_INCREMENTAL=0 RUSTFLAGS="--remap-path-prefix=$ROOT=/dowiz" "$CARGO" build --release --target wasm32-unknown-unknown --offline --target-dir "$TARGET") > "$V/.build.log" 2>&1; then
  say "RED wasm32 build failed rc=$? -- $(grep -m1 'error' "$V/.build.log")"; exit 1
fi
[ -f "$WASM" ] || { say "RED no module at $WASM"; exit 1; }
bytes=$(wc -c < "$WASM" | tr -d ' ')
if [ -f "$V/bytes.baseline" ]; then
  base=$(cat "$V/bytes.baseline")
  if [ "$bytes" -gt "$base" ]; then
    if [ "${BWR_RATCHET:-}" = write ]; then
      echo "$bytes" > "$V/bytes.baseline"; say "RATCHET RAISED BY HAND $base -> $bytes bytes (BWR_RATCHET=write; name it in the commit)"
    else
      say "REFUSED -- the reader grew $base -> $bytes bytes"; exit 1
    fi
  fi
  if [ "$bytes" -lt "$base" ]; then say "the ratchet may fall: $base -> $bytes (lower tools/verify/bytes.baseline in this commit)"; fi
  say "wasm32: $bytes bytes (baseline $(cat "$V/bytes.baseline"))"
else
  echo "$bytes" > "$V/bytes.baseline"; say "wasm32: $bytes bytes -- baseline written"
fi
sha=$(sha256sum "$WASM" | cut -c1-64)
# one base64 line; the template's two placeholders replaced, nothing else touched. (python, not
# awk -v: at 117 KB of wasm the 157 KB base64 argument silently produced an EMPTY file, 2026-10-31.)
base64 -w0 "$WASM" > "$V/.wasm.b64"
python3 -I - "$V/bebop-verify.src.mjs" "$V/.wasm.b64" "$sha" > "$V/.bebop-verify.new.mjs" <<'PY' || { say "RED could not write the .mjs"; exit 1; }
import sys
t = open(sys.argv[1]).read(); b = open(sys.argv[2]).read().strip()
assert t.count("__WASM_BASE64__") == 1 and t.count("__WASM_SHA256__") == 1, "template placeholders"
sys.stdout.write(t.replace("__WASM_BASE64__", b).replace("__WASM_SHA256__", sys.argv[3]))
PY
rm -f "$V/.wasm.b64"
[ -s "$V/.bebop-verify.new.mjs" ] || { say "RED the generated .mjs is empty"; exit 1; }
if cmp -s "$V/.bebop-verify.new.mjs" "$V/bebop-verify.mjs"; then
  rm -f "$V/.bebop-verify.new.mjs"; say "tools/verify/bebop-verify.mjs UNCHANGED (reader sha256 $sha)"
else
  mv "$V/.bebop-verify.new.mjs" "$V/bebop-verify.mjs"; chmod +x "$V/bebop-verify.mjs"
  say "REWRITTEN tools/verify/bebop-verify.mjs ($(wc -c < "$V/bebop-verify.mjs" | tr -d ' ') bytes; reader sha256 $sha) -- commit it"
fi
# the file must stand alone: no import but node:fs, no placeholder left
if grep -q '__WASM_' "$V/bebop-verify.mjs"; then say "RED a placeholder survived"; exit 1; fi
if [ "$(grep -c '^import ' "$V/bebop-verify.mjs")" != "1" ]; then say "RED more than one import"; exit 1; fi
rm -f "$V/.build.log"
say "GREEN"
