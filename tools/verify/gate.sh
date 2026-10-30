#!/bin/sh
# SPDX-License-Identifier: MIT OR Apache-2.0
# THE CLEAN-MACHINE GATE for bebop-verify (W-MG-MR, R-MAINGOAL-A §7 M5 kill-test, -B §4):
# a Node with NO repo and a Python with NO repo each read a fixture from ONE file and answer the
# line conformance.sh's five readers answer, on all 21 of its fixtures (and the log fixture).
#
# "No repo" is enforced, not assumed: the two verifier files and the fixtures are COPIED to an
# empty directory outside the checkout, the gate cd's there, and the verifiers run with that as
# the only path they see. A verifier that imported anything beside node:fs / the stdlib, or read
# any file but its argument, would fail there.
#
# --prove: the cell conformance.sh --prove names (bit 0 of the first value cell of kv3max's newest
# delta record) is flipped and BOTH verifiers must move from their clean answer to status 10.
# --no-build: skip tools/verify/build.sh (the wasm32 build and the .mjs regeneration).
#
# Usage: tools/verify/gate.sh [--prove] [--no-build]   (heavy with the build: under slot.sh)
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
V="$ROOT/tools/verify"
FIX="$ROOT/crates/bebop-wasm/fixtures"
PROVE=0; BUILD=1
for a in "$@"; do
  case "$a" in
    --prove) PROVE=1 ;;
    --no-build) BUILD=0 ;;
    *) echo "verify-gate: unknown argument $a" >&2; exit 2 ;;
  esac
done
say() { echo "verify-gate: $*"; }
fail=0
if [ "$BUILD" = 1 ]; then sh "$V/build.sh" || { say "RED build"; exit 1; }; fi
[ -f "$V/bebop-verify.mjs" ] || { say "RED tools/verify/bebop-verify.mjs is absent (run build.sh)"; exit 1; }

# the same list conformance.sh reads (W-ES-P10), plus the one log fixture
FIXTURES="kv kv2 kv3 ptaway kv4stub kv3max kv3c torn ptbad empty kv0 kv4small kv4deep kv4del kv4zero kv4badn kv4badv kv4badf kv4badd kv4stale kv4old"
CLEAN="${VERIFY_SCRATCH:-${TMPDIR:-/tmp}/bebop-verify-clean.$$}"
rm -rf "$CLEAN"; mkdir -p "$CLEAN"
trap 'rm -rf "$CLEAN"' EXIT
cp "$V/bebop-verify.mjs" "$V/bebop-verify.py" "$CLEAN/"
for f in $FIXTURES proj4.log; do cp "$FIX/$f.store" "$CLEAN/"; done
say "clean dir $CLEAN holds $(ls "$CLEAN" | wc -l | tr -d ' ') files and no repo"

want() { awk -F= "/^$2=/{print \$2}" "$FIX/$1.expected"; }
# one verifier over one copied image, FROM the clean dir, HOME and PATH stripped to the interpreter
node_read() { (cd "$CLEAN" && env -i PATH="$(dirname "$(command -v node)")" node ./bebop-verify.mjs "./$1" 2>/dev/null); }
py_read() { (cd "$CLEAN" && env -i PATH="$(dirname "$(command -v python3)")" python3 -I ./bebop-verify.py "./$1" 2>/dev/null); }

agree=0
for f in $FIXTURES; do
  st=$(want "$f" status)
  if [ -n "$st" ]; then exp="status=$st"; else exp="status=0 n=$(want "$f" n) root=$(want "$f" root)"; fi
  n=$(node_read "$f.store"); nrc=$?
  p=$(py_read "$f.store"); prc=$?
  bad=""
  if [ -n "$st" ]; then
    case "$n" in "kv status=$st "*) ;; *) bad="$bad node" ;; esac
    case "$p" in "kv status=$st "*) ;; *) bad="$bad python" ;; esac
    [ "$nrc" = "$st" ] || bad="$bad node-rc=$nrc"
    [ "$prc" = "$st" ] || bad="$bad python-rc=$prc"
  else
    [ "$n" = "kv $exp" ] && [ "$nrc" = 0 ] || bad="$bad node(rc=$nrc)"
    [ "$p" = "kv $exp" ] && [ "$prc" = 0 ] || bad="$bad python(rc=$prc)"
  fi
  if [ -n "$bad" ]; then say "$f: want '$exp' -- node '$n' python '$p' DISAGREE:$bad"; fail=1
  else say "$f: want '$exp' -- node and python agree"; agree=$((agree + 1)); fi
done
# the log fixture: the log fold `proj.expected` records for the same chain (DG5)
exp="log status=0 n=$(want proj n) root=$(want proj root)"
n=$(node_read proj4.log.store); p=$(py_read proj4.log.store)
if [ "$n" = "$exp" ] && [ "$p" = "$exp" ]; then say "proj4.log: want '$exp' -- node and python agree"
else say "proj4.log: want '$exp' -- node '$n' python '$p' DISAGREE"; fail=1; fi

if [ "$PROVE" = 1 ]; then
  python3 -I - "$FIX/kv3max.store" "$CLEAN/flip.store" <<'PY' || { say "RED --prove: could not name the cell"; exit 1; }
import struct, sys
b = bytearray(open(sys.argv[1], "rb").read())
c = lambda i: struct.unpack_from("<q", b, 8 * i)[0]
sb = max((at for at in (0, 512) if c(at) == 3554557610294396226), key=lambda at: c(at + 2))
pt = c(sb + 3); root = c(pt + 18); rec = root + c(root + 2 + 6)
cell = rec + 2 + 4 + (c(rec + 4) + 7) // 8
b[8 * cell] ^= 1
open(sys.argv[2], "wb").write(b)
print("verify-gate --prove: kv3max.store newest record %d -> flipped bit 0 of cell %d" % (rec, cell))
PY
  for r in node python; do
    if [ "$r" = node ]; then a=$(node_read kv3max.store); b=$(node_read flip.store); brc=$?; else a=$(py_read kv3max.store); b=$(py_read flip.store); brc=$?; fi
    say "prove $r: clean '$a' -> flipped '$b' rc=$brc"
    case "$b" in "kv status=10 "*) [ "$a" != "$b" ] || { say "RED --prove: $r did not move"; fail=1; } ;; *) say "RED --prove: $r did not land on status 10"; fail=1 ;; esac
  done
fi

bytes=$(cat "$V/bytes.baseline" 2>/dev/null || echo "?")
if [ "$fail" -ne 0 ]; then say "RED"; exit 1; fi
say "GREEN (node and python, from a clean dir, agree with conformance on $agree of 21 fixtures + the log; reader wasm $bytes bytes)"
exit 0
