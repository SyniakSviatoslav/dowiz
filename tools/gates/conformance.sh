#!/bin/sh
# SPDX-License-Identifier: MIT OR Apache-2.0
# CONFORMANCE 5 x 5 (W-ES-P4, R-EMBED-STORE §4.4 T-ES-30/32, §5 row P4). Every KV fixture of
# crates/bebop-wasm/fixtures, read by FIVE readers, each answer mapped to ONE status vocabulary
# (docs/STATUS-CODES.md) and held against the fixture's `.expected`:
#
#   native    crates/bebop-wasm/fixtures/conform (`bebop-conform read`): bebop-image's
#             pick_checked_in + bebop-kv's Kv::load_checked -- the LIBRARY, not bebop-wasm's copy
#   wasm32    crates/bebop-wasm built for wasm32 (the ratchet module, bytes.baseline), under node
#   python    crates/bebop-wasm/oracle.py, from the bytes alone
#   bebop.bin selfhost/std/kv.bp compiled by bebop.bin (AArch64 only: elsewhere NOT MEASURED, said
#             so, never counted as agreement); exit codes mapped by docs/STATUS-CODES.md
#   ffi       crates/bebop-ffi's C ABI through bindings/python/bebop_store.py (Verify::Reachable)
#
# value fixtures   : status=0 and n= and root= must all match
# refusal fixtures : status= must match on every reader (n/root are not compared)
# A reader that cannot run is NAMED and counts as a disagreement. Green = every measured reader
# agrees on every fixture, and at least four readers were measured.
#
# --prove: ONE NAMED CELL of a copy of kv3max.store -- bit 0 of the first value cell of the NEWEST
# delta record (op 31, item/00 = "last") -- is flipped; every reader's answer must MOVE from its
# answer on the clean image AND land on a NAMED answer: status 10, or (bebop.bin only, the anchor
# fallback of FORMAT §7) exactly the `torn` fixture. A reader whose answer cannot move measures nothing.
#
# Usage: tools/gates/conformance.sh [--prove] [--no-build]
# Heavy (three cargo builds): run it FOREGROUND under bebop-lang/tools/slot.sh; never nest slot.sh.
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
W="$ROOT/crates/bebop-wasm"
FIX="$W/fixtures"
CARGO="${CARGO:-$HOME/.cargo/bin/cargo}"
[ -x "$CARGO" ] || CARGO=cargo
PROVE=0; BUILD=1
for a in "$@"; do
  case "$a" in
    --prove) PROVE=1 ;;
    --no-build) BUILD=0 ;;
    *) echo "conformance: unknown argument $a" >&2; exit 2 ;;
  esac
done
# W-ES-P8D: the KV v4 fixtures (crates/bebop-wasm/fixtures/conform/src/kv4.rs) on all five readers;
# kv4stub is now a v4 claim that does not hold (status 2), no longer an unknown version (12).
FIXTURES="kv kv2 kv3 ptaway kv4stub kv3max kv3c torn ptbad empty kv0 kv4small kv4deep kv4del kv4zero kv4badn kv4badv kv4badf kv4badd"
READERS="native wasm32 python bebop.bin ffi"
SCRATCH="${CONFORMANCE_SCRATCH:-${TMPDIR:-/tmp}/conformance.$$}"
mkdir -p "$SCRATCH"
trap 'rm -rf "$SCRATCH"' EXIT
say() { echo "conformance: $*"; }
fail=0

CONF="$W/target/conform/release/bebop-conform"
WASM="$W/target/wasm32-unknown-unknown/release/bebop_wasm.wasm"
LIB="$ROOT/crates/bebop-ffi/target/release/libbebop_ffi.so"
if [ "$BUILD" = 1 ]; then
  (cd "$FIX/conform" && "$CARGO" build --release --offline --target-dir "$W/target/conform") > "$SCRATCH/b1.txt" 2>&1 \
    || { say "RED native build failed -- $(grep -m1 '^error' "$SCRATCH/b1.txt")"; exit 1; }
  (cd "$ROOT/crates/bebop-ffi" && "$CARGO" build --release --offline) > "$SCRATCH/b2.txt" 2>&1 \
    || { say "RED ffi build failed -- $(grep -m1 '^error' "$SCRATCH/b2.txt")"; exit 1; }
  # the same remap gate.sh uses, so the byte count is the code's and not the checkout path's
  (cd "$W" && RUSTFLAGS="--remap-path-prefix=$ROOT=/dowiz" "$CARGO" build --release --target wasm32-unknown-unknown --offline) > "$SCRATCH/b3.txt" 2>&1 \
    || { say "RED wasm32 build failed -- $(grep -m1 'error' "$SCRATCH/b3.txt")"; exit 1; }
fi
if [ -f "$WASM" ]; then
  bytes=$(wc -c < "$WASM" | tr -d ' '); base=$(cat "$W/bytes.baseline")
  if [ "$bytes" -gt "$base" ]; then say "wasm32: REFUSED -- the module grew $base -> $bytes"; fail=1; else say "wasm32: $bytes bytes (baseline $base)"; fi
fi

# bebop.bin: kv.bp compiled once
SEED="$ROOT/bebop-lang/seed/build/seed"
BIN="$ROOT/bebop-lang/bebop.bin"
KVBIN=""
if [ "$(uname -m)" = "aarch64" ] && [ -x "$SEED" ] && [ -f "$BIN" ]; then
  if (cd "$ROOT/bebop-lang" && ./seed/build/seed ./bebop.bin compile selfhost/std/kv.bp "$SCRATCH/kv.bin") > "$SCRATCH/kvc.txt" 2>&1 && [ -s "$SCRATCH/kv.bin" ]; then
    KVBIN="$SCRATCH/kv.bin"
  else
    say "bebop.bin: kv.bp did not compile -- $(tail -1 "$SCRATCH/kvc.txt")"; fail=1
  fi
else
  say "bebop.bin: NOT MEASURED (uname -m = $(uname -m)) -- a skip is not an agreement"
fi

# one reader over one image -> `status=<s> n=<n> root=<r>` (or a line naming why it could not)
ffi_read() {
  BEBOP_FFI_LIB="$LIB" python3 -I - "$ROOT" "$1" <<'PY'
import os, sys
sys.path.insert(0, os.path.join(sys.argv[1], "bindings", "python"))
import bebop_store as bs
data = open(sys.argv[2], "rb").read()
try:
    with bs.Db.from_bytes(data, verify=bs.REACHABLE) as db, db.read() as r:
        print("status=0 n=%d root=%d" % (len(r), r.fold_root()))
except bs.Refusal as e:
    print("status=%d n=0 root=0" % e.code)
PY
}
# bebop exit codes -> STATUS-CODES.md: 77/79 -> 1, 76 -> 2, 78 -> 12; kv.bp's -3 (a v3 or v4 claim that
# does not hold) -> 2, -10 (a v4 object's crc, kv4.bp) -> 10. Any other exit is printed as itself and disagrees.
bebop_read() {
  d="$SCRATCH/bb.$2"; mkdir -p "$d"; cp "$1" "$d/kv.store"
  n=$(cd "$d" && "$SEED" "$KVBIN" n 2>/dev/null); nrc=$?
  cp "$1" "$d/kv.store"
  h=$(cd "$d" && "$SEED" "$KVBIN" h 2>/dev/null); hrc=$?
  if [ "$nrc" != "$hrc" ]; then echo "status=rc$nrc/rc$hrc n=$n root=$h"; return; fi
  case "$nrc" in
    0) case "$n" in
         -3) echo "status=2 n=0 root=0" ;;
         -10) echo "status=10 n=0 root=0" ;;
         *) echo "status=0 n=$n root=$h" ;;
       esac ;;
    77|79) echo "status=1 n=0 root=0" ;;
    76) echo "status=2 n=0 root=0" ;;
    78) echo "status=12 n=0 root=0" ;;
    *) echo "status=rc$nrc n=$n root=$h" ;;
  esac
}
read_one() { # reader image tag
  case "$1" in
    native) [ -x "$CONF" ] && "$CONF" read "$2" 2>&1 | sed 's/^kv //' || echo "ABSENT" ;;
    wasm32) [ -f "$WASM" ] && node "$W/harness.mjs" "$WASM" "$2" 2>&1 | sed 's/^kv //' || echo "ABSENT" ;;
    python) python3 -I "$W/oracle.py" "$2" 2>&1 | sed 's/^kv //' ;;
    bebop.bin) if [ -n "$KVBIN" ]; then bebop_read "$2" "$3"; else echo "NOT-MEASURED"; fi ;;
    ffi) [ -f "$LIB" ] && ffi_read "$2" 2>&1 || echo "ABSENT" ;;
  esac
}
want() { awk -F= "/^$2=/{print \$2}" "$FIX/$1.expected"; }

minran=5
printf '%-8s' "fixture" > "$SCRATCH/matrix"
for r in $READERS; do printf ' | %-44s' "$r" >> "$SCRATCH/matrix"; done
echo >> "$SCRATCH/matrix"
for f in $FIXTURES; do
  st=$(want "$f" status)
  if [ -n "$st" ]; then exp="status=$st"; else exp="status=0 n=$(want "$f" n) root=$(want "$f" root)"; fi
  # W-ES-P8D: a corrupted object inside the LAST commit's range is answered by the ANCHOR first (FORMAT
  # §7: bebop.bin falls back to generation k-1, the C ABI refuses LAST_COMMIT 41), never by the tree
  # rule the fixture exists to test -- measured 2026-10-30 on kv4badf/kv4badd before the writer
  # re-anchored them (fixtures/conform/src/kv4.rs `reanchor`). No alternate answer is accepted here.
  ran=0; bad=""; nm=""
  printf '%-8s' "$f" >> "$SCRATCH/matrix"
  for r in $READERS; do
    got=$(read_one "$r" "$FIX/$f.store" "$f")
    printf ' | %-44s' "$got" >> "$SCRATCH/matrix"
    case "$got" in
      NOT-MEASURED) nm="$nm $r" ;;
      *) if [ -n "$st" ]; then
           case "$got" in "status=$st "*) ran=$((ran + 1)) ;; *) bad="$bad $r" ;; esac
         else
           [ "$got" = "$exp" ] && ran=$((ran + 1)) || bad="$bad $r"
         fi ;;
    esac
  done
  echo >> "$SCRATCH/matrix"
  if [ -n "$bad" ]; then
    say "$f: want '$exp' -- $ran agree; DISAGREE:$bad${nm:+; NOT MEASURED:$nm}"; fail=1
  else
    say "$f: want '$exp' -- $ran of 5 agree${nm:+; NOT MEASURED:$nm}"
  fi
  [ "$ran" -lt "$minran" ] && minran=$ran
done
cat "$SCRATCH/matrix"

if [ "$PROVE" = 1 ]; then
  python3 -I - "$FIX/kv3max.store" "$SCRATCH/flip.store" <<'PY' || { say "RED --prove: could not name the cell"; exit 1; }
import struct, sys
b = bytearray(open(sys.argv[1], "rb").read())
c = lambda i: struct.unpack_from("<q", b, 8 * i)[0]
sb = max((at for at in (0, 512) if c(at) == 3554557610294396226), key=lambda at: c(at + 2))
pt = c(sb + 3); root = c(pt + 18); rec = root + c(root + 2 + 6)
cell = rec + 2 + 4 + (c(rec + 4) + 7) // 8
b[8 * cell] ^= 1
open(sys.argv[2], "wb").write(b)
print("conformance --prove: kv3max.store live sb %d PartTab %d root %d newest record %d -> flipped bit 0 of cell %d" % (sb, pt, root, rec, cell))
PY
  moved=0; still=""; wrong=""
  for r in $READERS; do
    a=$(read_one "$r" "$FIX/kv3max.store" "pc")
    b=$(read_one "$r" "$SCRATCH/flip.store" "pf")
    say "prove $r: clean '$a' -> flipped '$b'"
    case "$a" in NOT-MEASURED) continue ;; esac
    if [ "$a" != "$b" ]; then moved=$((moved + 1)); else still="$still $r"; fi
    # W-PRELUDE (2026-10-29): "moved" alone let a reader move to ANY answer. Now the answer is
    # named: status 10 (the newest record fails its crc, what native/wasm32/python/ffi say), or --
    # for bebop.bin only -- exactly the `torn` fixture's generation k-1, because st_open's anchor
    # check (FORMAT §7, DG5, gate std_golden `sanchor`) stamps a torn last commit and falls back.
    # That fallback is the one KNOWN divergence left (Rust anchor recovery is FORMAT §7 phase P2);
    # a fallback past a bad PartTab is no longer one (store.bp trap 77, fixture `ptbad`).
    tk="status=0 n=$(want torn n) root=$(want torn root)"
    case "$b" in
      "status=10 "*) ;;
      "$tk") [ "$r" = bebop.bin ] || wrong="$wrong $r" ;;
      *) wrong="$wrong $r" ;;
    esac
  done
  if [ -n "$still" ]; then say "RED --prove: the flipped cell did not move:$still"; fail=1
  elif [ -n "$wrong" ]; then say "RED --prove: moved, but not to status 10 (or, for bebop.bin, the anchor fallback = torn):$wrong"; fail=1
  else say "prove: the named cell moved the answer of $moved measured readers (status 10; bebop.bin anchor fallback = torn)"; fi
fi

if [ "$fail" -ne 0 ]; then say "RED"; exit 1; fi
if [ "$minran" -lt 4 ]; then say "RED -- fewer than four readers agreed on some fixture"; exit 1; fi
say "GREEN ($minran+ readers agree on each of: $FIXTURES)"
exit 0
