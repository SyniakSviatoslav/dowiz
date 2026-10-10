#!/usr/bin/env bash
# cf_parity.sh (W-CFVAL 2026-10-09): R-CF (compiler/affine.bp) must not change what a program MEANS.
# Every bench/cf_parity/*.bp is compiled twice by ONE compiler, R-CF on and BEBOP_NO_CF=1, and checked
# against its own header line `// CF-EXPECT rc=<compile rc> out=<stdout|-> touched=<0|1>`:
#   * both compiles exit <rc>;  when rc=0 both binaries run, print <out> and exit 0;
#   * touched=1: the two binaries DIFFER (R-CF rewrote a loop -- proves the switch and the pass both work);
#     touched=0: they are byte-identical (R-CF must leave that loop alone).
# Named regressions: cf_fieldname (F1: a field name counted as a binding -> rc 84 under R-CF),
# cf_letin / cf_letin_tail / cf_letin_rt (F2: a `while` inside `( let ..; .. )` was folded into a valid program).
# The positives cover the fold (cf_fold, cf_cycle, cf_fieldfold) and the runtime helper (cf_rt, cf_rtne).
# Usage: BEBOP_BIN=<compiler> bash bench/vs_rust/cf_parity.sh     exit 0 green, 1 red, 2 measured nothing
if [ "${BEBOP_SLOT_HELD:-0}" != 1 ] && [ "${NO_SLOT:-0}" != 1 ]; then
  _slot="$(dirname "$0")/../../tools/slot.sh"
  [ -f "$_slot" ] || _slot=/root/dowiz/bebop-lang/tools/slot.sh
  [ -f "$_slot" ] && exec bash "$_slot" "auto:$(basename "$0")" bash "$0" "$@"
fi
cd "$(dirname "$0")/../.." || exit 2
ulimit -s 65536 2>/dev/null || true
set -u
BIN=${BEBOP_BIN:-./bebop.bin}
[ -s "$BIN" ] || { echo "GUARD: compiler $BIN missing or empty"; exit 2; }
T=${BEBOP_TMP:-/tmp/opencode}/cf_parity; mkdir -p "$T"
pass=0; fail=0; n=0
for f in bench/cf_parity/*.bp; do
  [ -f "$f" ] || continue
  n=$((n + 1)); b=$(basename "$f" .bp)
  spec=$(grep -o 'CF-EXPECT rc=[^ ]* out=[^ ]* touched=[01]' "$f" | head -1)
  [ -n "$spec" ] || { echo "FAIL $b: no CF-EXPECT line"; fail=$((fail + 1)); continue; }
  wrc=$(echo "$spec" | sed 's/.*rc=\([^ ]*\).*/\1/'); wout=$(echo "$spec" | sed 's/.*out=\([^ ]*\).*/\1/')
  wt=$(echo "$spec" | sed 's/.*touched=\([01]\).*/\1/')
  rm -f "$T/$b".cf.bin* "$T/$b".no.bin*
  ./seed/build/seed "$BIN" compile "$f" "$T/$b.cf.bin" > "$T/$b.cf.err" 2>&1; rc=$?
  BEBOP_NO_CF=1 ./seed/build/seed "$BIN" compile "$f" "$T/$b.no.bin" > "$T/$b.no.err" 2>&1; rn=$?
  why=""
  [ "$rc" = "$wrc" ] || why="$why compile rc $rc with R-CF (want $wrc: $(head -c 120 "$T/$b.cf.err" | tr '\n' ' '));"
  [ "$rn" = "$wrc" ] || why="$why compile rc $rn with BEBOP_NO_CF=1 (want $wrc);"
  if [ -z "$why" ] && [ "$wrc" = 0 ]; then
    oc=$(timeout 30 ./seed/build/seed "$T/$b.cf.bin" 2>&1 < /dev/null); xc=$?
    on=$(timeout 30 ./seed/build/seed "$T/$b.no.bin" 2>&1 < /dev/null); xn=$?
    [ "$xc/$oc" = "0/$wout" ] || why="$why R-CF binary exit $xc printed '$oc' (want 0 '$wout');"
    [ "$xn/$on" = "0/$wout" ] || why="$why NO_CF binary exit $xn printed '$on' (want 0 '$wout');"
    if cmp -s "$T/$b.cf.bin" "$T/$b.no.bin"; then t=0; else t=1; fi
    [ "$t" = "$wt" ] || why="$why touched=$t (want $wt: $([ "$wt" = 1 ] && echo 'R-CF did not rewrite the loop, or BEBOP_NO_CF=1 did not turn it off' || echo 'R-CF rewrote a loop it must leave alone'));"
  fi
  if [ -z "$why" ]; then pass=$((pass + 1)); echo "PASS $b"; else fail=$((fail + 1)); echo "FAIL $b:$why"; fi
done
[ "$n" -gt 0 ] || { echo "cf_parity: REFUSED -- no bench/cf_parity/*.bp (measured nothing)"; exit 2; }
echo "cf_parity: $pass/$n pass, $fail fail"
[ "$fail" = 0 ] && [ "$pass" = "$n" ]
