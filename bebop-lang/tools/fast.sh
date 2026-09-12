#!/usr/bin/env bash
# fast.sh -- ROADMAP A20, the fast lane: a strict SUBSET of tools/chain.sh, for an answer in
# ~30 s instead of the 516 s a chain cost on 2026-09-12. It NEVER promotes and never writes
# bebop.bin; the chain remains the only thing that may.
#
# Usage: tools/fast.sh <edited files...>
#   (i)   bebop.bin check on each edited file      -- sub-second, catches syntax
#   (ii)  ONE self-compile of bebop.bp             -- prints the gen2 digest
#   (iii) construct_parity against gen2, FREEZE=0  -- catches a codegen change
#   (iv)  typecheck over the WHOLE corpus          -- the T48 census
#   (v)   check_words                              -- hand-typed em()/st[] literals
#
# WHAT IT DOES NOT DO, stated in the summary line so nobody cites `fast: GREEN` as a battery:
# gen3/gen4 and the fixpoint, std_golden's 117 gates, the oracles, invariants.sh, the ABI
# check, diag, pool_parity, the census freeze, perf, and promotion.
#
# --- one-compile guard (operator 2026-09-12) ---------------------------------------------
# Steps (ii) and (iii) are heavy jobs. tools/slot.sh holds ONE global flock shared by every
# lane worktree, so at most one compilation exists on the box at any instant; re-exec through
# it exactly as chain.sh:19-22 and battery.sh:15-19 do. NO_SLOT=1 opts out (triage only).
set -u
if [ "${BEBOP_SLOT_HELD:-0}" != 1 ] && [ "${NO_SLOT:-0}" != 1 ]; then
  _slot="$(dirname "$0")/slot.sh"
  [ -f "$_slot" ] || _slot=/root/dowiz/bebop-lang/tools/slot.sh
  [ -f "$_slot" ] && exec bash "$_slot" "auto:fast.sh" bash "$0" "$@"
fi
cd "$(dirname "$0")/.." || exit 1

[ $# -gt 0 ] || { echo "usage: tools/fast.sh <edited files...>" >&2; exit 2; }
BEBOP_BIN=${BEBOP_BIN:-./bebop.bin}
BEBOP_TMP=${BEBOP_TMP:-/tmp/opencode}/fast
mkdir -p "$BEBOP_TMP"
before=$(md5sum < "$BEBOP_BIN" | cut -c1-8)

echo "fast: step (i) check..."
t0=$(date +%s%N)
for f in "$@"; do
  [ -f "$f" ] || { echo "fast: RED -- file not found: $f"; exit 1; }
  out=$(./seed/build/seed "$BEBOP_BIN" check "$f" 2>&1); rc=$?
  [ $rc = 0 ] || { echo "fast: RED at (i) check -- $f exits $rc"; echo "$out"; exit $rc; }
done
check_ms=$(( ($(date +%s%N) - t0) / 1000000 ))
echo "fast: (i) check OK (${check_ms} ms)"

echo "fast: step (ii) self-compile..."
t0=$(date +%s%N)
./seed/build/seed "$BEBOP_BIN" compile bebop.bp "$BEBOP_TMP/gen2.bin" >/dev/null 2>&1 || {
  echo "fast: RED at (ii) -- bebop.bp does not self-compile"; exit 1; }
cc_ms=$(( ($(date +%s%N) - t0) / 1000000 ))
gen2=$(md5sum < "$BEBOP_TMP/gen2.bin" | cut -c1-8)
echo "fast: (ii) gen2 $gen2 (${cc_ms} ms)"

# (iii) against gen2 -- the binary the EDIT produces, not the promoted one.
echo "fast: step (iii) construct_parity vs gen2..."
t0=$(date +%s%N)
BEBOP_BIN="$BEBOP_TMP/gen2.bin" FREEZE=0 bash bench/vs_rust/construct_parity.sh > "$BEBOP_TMP/parity.log" 2>&1
prc=$?
parity_ms=$(( ($(date +%s%N) - t0) / 1000000 ))
# The real summary line is `construct parity: pass=N fail=M`. A grep for '^PASS|^FAIL' matches
# NOTHING this script prints, so a fallback on it reports success unconditionally.
parity_line=$(grep -E '^construct parity:' "$BEBOP_TMP/parity.log" | tail -1)
[ -n "$parity_line" ] || parity_line="construct parity: NO SUMMARY LINE (harness changed?)"
if [ $prc != 0 ]; then
  echo "fast: RED at (iii) -- $parity_line"
  grep -E 'MISMATCH|COMPILEFAIL|WORD_BUDGET' "$BEBOP_TMP/parity.log" | head -8
  exit 1
fi
echo "fast: (iii) $parity_line (${parity_ms} ms)"

# (iv) the SAME corpus invariants.sh rung (vii) uses. With no file arguments typecheck.py
# reports `0 findings` over an EMPTY file list -- a step that always passes checks nothing.
echo "fast: step (iv) typecheck..."
t0=$(date +%s%N)
tc=$(python3 tools/typecheck.py bebop.bp bench/vs_rust/std_tests/*.bp bench/vs_rust/kernels/*.bp bench/parity_constructs/*.bp 2>&1 | tail -1)
type_ms=$(( ($(date +%s%N) - t0) / 1000000 ))
if [ "$tc" != "typecheck census: 0 findings" ]; then
  echo "fast: RED at (iv) -- $tc"; exit 1
fi
echo "fast: (iv) $tc (${type_ms} ms)"

echo "fast: step (v) check_words..."
t0=$(date +%s%N)
python3 tools/check_words.py > "$BEBOP_TMP/words.log" 2>&1 || {
  echo "fast: RED at (v) check_words"; tail -5 "$BEBOP_TMP/words.log"; exit 1; }
words_ms=$(( ($(date +%s%N) - t0) / 1000000 ))
echo "fast: (v) $(tail -1 "$BEBOP_TMP/words.log") (${words_ms} ms)"

after=$(md5sum < "$BEBOP_BIN" | cut -c1-8)
[ "$before" = "$after" ] || { echo "fast: RED -- bebop.bin changed $before -> $after; fast.sh must never promote"; exit 1; }
total=$(( check_ms + cc_ms + parity_ms + type_ms + words_ms ))
echo "fast: GREEN in ${total} ms (bebop.bin $after unchanged). SUBSET -- this is NOT a battery:"
echo "fast: it does not run gen3/gen4 or the fixpoint, std_golden's 117 gates, the oracles,"
echo "fast: invariants.sh, check_abi, diag, pool_parity, the census freeze, perf, or promotion."
