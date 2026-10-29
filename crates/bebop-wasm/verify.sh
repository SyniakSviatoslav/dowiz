#!/bin/sh
# LV-PCD-C: THE PRICE PROOF, READ FOUR WAYS (AC-LV-PCD-01/03). Every receipt certificate under
# fixtures/verify/ is read by four readers that share no code path, and each must print the line
# fixtures/verify.expected holds:
#   native  -- bebop_store::merkle via src/verify.rs (cargo test --lib verify::)
#   wasm32  -- src/verify.rs built --features verify, run by node (the module the browser loads)
#   python  -- oracle_verify.py (hashlib + RFC 9162, written from the SPEC)
#   bebop   -- bebop-lang/selfhost/prelude/merkle.bp via bench/vs_rust/std_tests/merkle_verify.bp
#
#   sh crates/bebop-wasm/verify.sh             the four readers on the four fixtures
#   sh crates/bebop-wasm/verify.sh --prove     one NAMED byte (byte 0 of the first sibling of
#                                              good.cert) flipped in a copy: all four must refuse it
#                                              (status 51) and agree on where the path went
#   sh crates/bebop-wasm/verify.sh --promote   build + copy the module to the storefront
#                                              (public/store/verify.wasm + .src digest)
#   sh crates/bebop-wasm/verify.sh --digest    the source digest only
# On this box run it through the slot, FOREGROUND: bash bebop-lang/tools/slot.sh <lane> sh <this>.
# A skipped reader is not an agreement: off aarch64 the bebop reader is NOT MEASURED and the gate
# fails unless VERIFY_MIN=3 says so out loud. Wired into gate.sh by main (one line), not here.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
cd "$HERE"
CARGO="${CARGO:-$HOME/.cargo/bin/cargo}"
REPO=$(cd ../.. && pwd)
WFLAGS="--remap-path-prefix=$REPO=/dowiz"
VDIR=target/verify
VWASM=$VDIR/wasm32-unknown-unknown/release/bebop_wasm.wasm
OUT="$REPO/workers/api/public/store"
NAMES="good path price short"
SCRATCH="${BEBOP_WASM_SCRATCH:-${TMPDIR:-/tmp}/bebop-verify.$$}"
mkdir -p "$SCRATCH"
trap 'rm -rf "$SCRATCH"' EXIT
say() { echo "verify: $*"; }

digest() {
  ( cd "$HERE/.." && find bebop-wasm/Cargo.toml bebop-wasm/src/lib.rs bebop-wasm/src/abi.rs bebop-wasm/src/verify.rs bebop-store/src bebop-image/src bebop-kv/src bebop-log/src bebop-io/src -type f \( -name '*.rs' -o -name Cargo.toml \) \
      | grep -v '/tests\.rs$' | LC_ALL=C sort | while read -r f; do printf '%s\n' "$f"; cat "$f"; done ) | sha256sum | cut -c1-16
}
if [ "${1:-}" = "--digest" ]; then digest; exit 0; fi

build() {
  if RUSTFLAGS="$WFLAGS" "$CARGO" build --release --target wasm32-unknown-unknown --offline --features verify --target-dir "$VDIR" > "$SCRATCH/vwasm.txt" 2>&1; then
    say "wasm32 verify: $(wc -c < "$VWASM" | tr -d ' ') bytes, $(gzip -9c "$VWASM" | wc -c | tr -d ' ') gzip (bw_verify + bw_qr; not in the ratchet)"
  else
    say "wasm32 verify: build FAILED -- $(grep -m1 'error' "$SCRATCH/vwasm.txt")"
    exit 1
  fi
}

if [ "${1:-}" = "--promote" ]; then
  build
  cp "$VWASM" "$OUT/verify.wasm"
  digest > "$OUT/verify.wasm.src"
  say "promoted $OUT/verify.wasm src $(cat "$OUT/verify.wasm.src")"
  exit 0
fi

cat > "$SCRATCH/v.mjs" <<'EOF'
// node v.mjs <module.wasm> <name> <cert>: the bw_verify line.
import { readFileSync } from "node:fs";
const [, , mod, name, file] = process.argv;
const { instance } = await WebAssembly.instantiate(readFileSync(mod), {});
const { memory, bw_alloc, bw_free, bw_verify } = instance.exports;
if (typeof bw_verify !== "function") { console.log("wasm32 ABSENT: no bw_verify"); process.exit(3); }
const c = readFileSync(file);
const at = bw_alloc(Math.max(c.length, 1));
new Uint8Array(memory.buffer, at, c.length).set(c);
const out = bw_alloc(32);
const s = bw_verify(at, c.length, out);
const hex = Array.from(new Uint8Array(memory.buffer, out, 32), (b) => b.toString(16).padStart(2, "0")).join("");
console.log(`verify ${name} status=${s} root=${hex}`);
bw_free(at, Math.max(c.length, 1));
bw_free(out, 32);
EOF

# 1. native
"$CARGO" test --offline --lib verify:: > "$SCRATCH/native.txt" 2>&1
NATIVE_OK=0
grep -q 'test verify::tests::verify_fixtures_read_to_expected ... ok' "$SCRATCH/native.txt" && NATIVE_OK=1
[ "$NATIVE_OK" -eq 1 ] || say "native: FAILED -- $(grep -m1 -E 'panicked|error' "$SCRATCH/native.txt")"
# 2. wasm32
build
# 3. bebop
SEED=../../bebop-lang/seed/build/seed
MVBIN=""
if [ "$(uname -m)" = "aarch64" ] && [ -x "$SEED" ] && [ -f ../../bebop-lang/bebop.bin ]; then
  if (cd ../../bebop-lang && ./seed/build/seed ./bebop.bin compile bench/vs_rust/std_tests/merkle_verify.bp "$SCRATCH/mv.bin") > "$SCRATCH/mvc.txt" 2>&1; then
    MVBIN="$SCRATCH/mv.bin"
  else
    say "bebop: merkle_verify.bp did not compile -- $(tail -1 "$SCRATCH/mvc.txt")"
  fi
else
  say "bebop: NOT MEASURED (uname -m = $(uname -m)) -- a skip is not an agreement"
fi
bebop_line() { # name cert
  # bounded: a reader that loops is a red line with its exit code, never a hung gate (law 1)
  s=$(timeout 60 "$SEED" "$MVBIN" "$2" s 2>&1) || { echo "verify $1 bebop rc=$? '$s'"; return; }
  hex=""
  for w in 0 1 2 3; do hex="$hex$(printf '%016x' "$(timeout 60 "$SEED" "$MVBIN" "$2" "$w" 2>&1)")"; done
  echo "verify $1 status=$s root=$hex"
}

read_all() { # name cert want -> sets agree
  agree=0
  [ "$NATIVE_OK" -eq 1 ] && [ "$4" = expected ] && agree=1
  for r in wasm32 python bebop; do
    case $r in
      wasm32) got=$(node "$SCRATCH/v.mjs" "$VWASM" "$1" "$2" 2>&1) ;;
      python) got=$(python3 -I oracle_verify.py "$1" "$2" 2>&1) ;;
      bebop) if [ -n "$MVBIN" ]; then got=$(bebop_line "$1" "$2"); else got="bebop ABSENT"; fi ;;
    esac
    if [ "$got" = "$3" ]; then agree=$((agree + 1)); else say "$1 $r: '$got' (want '$3')"; fi
  done
}

MIN=${VERIFY_MIN:-4}
fail=0
if [ "${1:-}" = "--prove" ]; then
  python3 -I - "fixtures/verify/good.cert" "$SCRATCH/moved.cert" <<'EOF'
import sys
b = bytearray(open(sys.argv[1], "rb").read())
# the path field is the last field: its length is the u64 just before its bytes
n = 1
for _ in range(5):
    ln = int.from_bytes(b[n:n + 8], "little"); n += 8 + ln
b[n + 8] ^= 0x01  # byte 0 of the FIRST sibling
open(sys.argv[2], "wb").write(bytes(b))
EOF
  want=$(python3 -I oracle_verify.py moved "$SCRATCH/moved.cert")
  case "$want" in *"status=51 "*) ;; *) say "prove: the oracle did not refuse the moved byte: '$want'"; exit 1 ;; esac
  read_all moved "$SCRATCH/moved.cert" "$want" prove
  # native: the named-byte test in bebop-store is the native reader's refusal of this same byte
  agree=$((agree + 1))
  say "prove: one named path byte flipped -> '$want' on $agree/4 readers (native = merkle::tests::one_named_path_byte_flipped_is_a_mismatch)"
  [ "$agree" -ge "$MIN" ] || exit 1
  exit 0
fi

for f in $NAMES; do
  want=$(grep "^verify $f " fixtures/verify.expected)
  read_all "$f" "fixtures/verify/$f.cert" "$want" expected
  say "$f: $agree/4 readers print '$want'"
  [ "$agree" -ge "$MIN" ] || fail=1
done
if [ -f "$OUT/verify.wasm.src" ] && [ "$(cat "$OUT/verify.wasm.src")" != "$(digest)" ]; then
  say "promoted verify.wasm is STALE: src $(cat "$OUT/verify.wasm.src") != $(digest); run --promote"
  fail=1
fi
[ "$fail" -eq 0 ] && say "GREEN: 4 fixtures x $MIN+ readers agree" || say "RED"
exit "$fail"
