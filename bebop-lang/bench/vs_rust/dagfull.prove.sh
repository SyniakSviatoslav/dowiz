#!/usr/bin/env bash
# dagfull.prove.sh -- the mutation proof of bench/vs_rust/dagfull.sh (docs/design/SPEC-BEBOP-DAG-RUNTIME-
# 2026-09-28.md §8.3 steps 1-2, G-1 step 0). A gate that cannot fail proves nothing: each step breaks one
# thing and requires the gate to SAY so. Mutations are made in a SCRATCH COPY of the compiler tree, never in
# the checkout, so there is nothing to restore (rule 4 of the lane rules holds by construction).
#   step 0 (G-1): --sweep with the frozen dir absent -> a NAMED refusal, exit 2 (never a pass).
#   step 1 (M-1): a compiler whose hit test is skipped (dag_src_eq always 1, the key reduced to the span
#                 length, so equal-length fns collide) -> `dagfull compile bebop.bp` must go RED.
#   step 2 (K-4): the candidate binary with ONE byte patched (in the entry stub's trap text, so it still
#                 compiles) used for the warm run -> `dag: hits 0/<n>`, and its output still equals cold.
# Prints `dagfull.prove: <k>/3 steps fired` and exits 0 only when all three fired.
# env: BEBOP_BIN (candidate), BEBOP_TMP. Run through tools/slot.sh.
set -u
cd "$(dirname "$0")/../.." || exit 1
BB=$(realpath -m "${BEBOP_BIN:-./bebop.bin}"); SEED=$(realpath ./seed/build/seed)
T=${BEBOP_TMP:-/tmp/opencode}/dagprove; rm -rf "$T"; mkdir -p "$T"
fired=0

# step 0
BEBOP_BIN=$BB BEBOP_TMP=$T bash bench/vs_rust/dagfull.sh --sweep "$T/no-such-frozen-dir" > "$T/s0.txt" 2>&1; rc=$?
if [ "$rc" = 2 ] && grep -q "REFUSED -- frozen manifest" "$T/s0.txt"; then echo "step 0 FIRED: $(tail -1 "$T/s0.txt")"; fired=$((fired + 1))
else echo "step 0 NOT FIRED (rc $rc): $(tail -1 "$T/s0.txt")"; fi

# step 1
M=$T/mut; mkdir -p "$M/selfhost"; cp bebop.bp "$M/"; cp -r compiler "$M/"; cp -r selfhost/prelude "$M/selfhost/"
python3 - "$M" <<'PY' || { echo "step 1: mutation target not found"; exit 1; }
import sys
p = sys.argv[1] + '/selfhost/prelude/dagc.bp'; s = open(p).read()
a = 'fn dag_src_eq(dm: [i64], e: i64, sc: [i64], off: i64, len: i64) -> i64 {\n'
b = 'fn dag_key(sc: [i64], off: i64, len: i64) -> i64 {\n  (crc32x(sc, off, (len + 7) / 8) << 32) | (len & 4294967295)\n}'
assert s.count(a) == 1 and s.count(b) == 1
s = s.replace(a, a + '  while len >= 0 { return 1; 0 };\n')
s = s.replace(b, 'fn dag_key(sc: [i64], off: i64, len: i64) -> i64 {\n  len & 4294967295\n}')
open(p, 'w').write(s)
PY
(cd "$M" && "$SEED" "$BB" compile bebop.bp "$T/mut.bin" > /dev/null 2>&1) || { echo "step 1: the mutated compiler did not build"; exit 1; }
BEBOP_BIN=$T/mut.bin BEBOP_TMP=$T bash bench/vs_rust/dagfull.sh compile bebop.bp > "$T/s1.txt" 2>&1; rc=$?
if [ "$rc" != 0 ] && grep -q "warm != cold" "$T/s1.txt"; then echo "step 1 FIRED: $(grep 'warm != cold' "$T/s1.txt" | cut -c1-120) | $(tail -1 "$T/s1.txt")"; fired=$((fired + 1))
else echo "step 1 NOT FIRED (rc $rc): $(tail -2 "$T/s1.txt" | tr '\n' ' ')"; fi

# step 2
P=$T/k4; mkdir -p "$P"
"$SEED" "$BB" compile bebop.bp "$P/c.bin" > /dev/null 2>&1 || { echo "step 2: cold compile failed"; exit 1; }
python3 - "$BB" "$P/patched.bin" <<'PY'
import sys
b = bytearray(open(sys.argv[1], 'rb').read())
b[len(b) - 12] ^= 1          # a byte of the entry stub's trap text, 12 bytes before the 8-byte footer
open(sys.argv[2], 'wb').write(bytes(b))
PY
cp "$P/c.bin.dag" "$P/w.bin.dag"
DAG_HITS=1 "$SEED" "$P/patched.bin" compile bebop.bp "$P/w.bin" > /dev/null 2> "$P/w.err"; rc=$?
hits=$(sed -n 's/^dag: hits \([0-9]*\)\/\([0-9]*\).*/\1 \2/p' "$P/w.err")
if [ "$rc" = 0 ] && [ "${hits%% *}" = 0 ] && [ -n "${hits#* }" ] && cmp -s "$P/w.bin" "$P/c.bin"; then
  echo "step 2 FIRED: patched compiler -> dag: hits ${hits%% *}/${hits#* } (K-4), output == cold"; fired=$((fired + 1))
else echo "step 2 NOT FIRED (rc $rc, hits '${hits}')"; fi

echo "dagfull.prove: $fired/3 steps fired"
[ "$fired" = 3 ]
