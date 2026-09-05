#!/usr/bin/env bash
# chain.sh (2026-09-06, session-10 speed-up): the three-generation self-hosting chain plus
# the battery, pipelined. gen2 = <bin0> compiling <src>; then IN PARALLEL: gen3 -> gen4
# (the fixpoint test is gen3 == gen4) and the battery against gen2. When the change is
# NOT a codegen change gen2 == gen3 == gen4 and the battery result stands as soon as the
# md5s agree; pass --codegen to run the battery against gen4 instead (gen2 differs then).
# Usage: tools/chain.sh <src.bp> <out-dir> [--codegen] [BIN0=./bebop.bin]
cd "$(dirname "$0")/.." || exit 1
SRC=${1:?src.bp}; OUT=${2:?out dir}; shift 2; CG=0; [ "${1:-}" = --codegen ] && { CG=1; shift; }
BIN0=$(realpath -m "${1:-./bebop.bin}"); mkdir -p "$OUT"
[ -s "$BIN0" ] || { echo "GUARD: $BIN0 missing or empty (L12)"; exit 1; }
PIN=${PIN:-taskset -c 4-6}  # the 3 A78 cores; PIN="" to unpin
gen() { $PIN ./seed/build/seed "$1" compile "$SRC" "$2" >/dev/null 2>&1; local rc=$?; [ $rc = 0 ] && [ -s "$2" ] || { echo "gen $2 FAILED rc=$rc"; exit 1; }; }
t0=$(date +%s); gen "$BIN0" "$OUT/gen2.bin"; echo "gen2 $(md5sum < "$OUT/gen2.bin" | cut -c1-8) $(( $(date +%s) - t0 )) s"
( gen "$OUT/gen2.bin" "$OUT/gen3.bin"; echo "gen3 $(md5sum < "$OUT/gen3.bin" | cut -c1-8)"; gen "$OUT/gen3.bin" "$OUT/gen4.bin"; echo "gen4 $(md5sum < "$OUT/gen4.bin" | cut -c1-8)" ) > "$OUT/chain.log" 2>&1 &
if [ $CG = 0 ]; then bash tools/battery.sh "$OUT/gen2.bin" "$OUT/bat" > "$OUT/battery.log" 2>&1; fi
wait; cat "$OUT/chain.log"
if [ $CG = 1 ]; then bash tools/battery.sh "$OUT/gen4.bin" "$OUT/bat" > "$OUT/battery.log" 2>&1; fi
cat "$OUT/battery.log"
m3=$(md5sum < "$OUT/gen3.bin" 2>/dev/null | cut -c1-8); m4=$(md5sum < "$OUT/gen4.bin" 2>/dev/null | cut -c1-8); m2=$(md5sum < "$OUT/gen2.bin" | cut -c1-8)
if [ -n "$m4" ] && [ "$m3" = "$m4" ]; then
  if [ $CG = 0 ] && [ "$m2" != "$m3" ]; then echo "chain: gen3 == gen4 $m4 but gen2 differs ($m2) -- codegen changed: rerun with --codegen"; exit 1; fi
  echo "chain: fixpoint gen3 == gen4 $m4 ($(( $(date +%s) - t0 )) s total)"; grep -q 'battery: GREEN' "$OUT/battery.log"
else echo "chain: NO FIXPOINT (gen3 $m3, gen4 $m4)"; exit 1; fi
