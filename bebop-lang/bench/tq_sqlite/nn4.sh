#!/usr/bin/env bash
# T106 gate (D1(b), replaces T98): nn4.bp shards the bucketed nearest scan over
# W=3 clone'd workers, each pinned with sys_setaffinity (T72) to one usable A78
# (cpus 4-6 on this box; cpu 7 refuses taskset), vs the same scan on one core.
# nn4 prints ok*1e12 + seq_ms*1e6 + par_ms; ok=1 means the parallel fold ==
# the sequential fold. env: BEBOP_BIN, BEBOP_TMP, R (runs, default 5). Prints
# the median row and appends it to RESULT.md.
set -u
cd "$(dirname "$0")/../.."
ulimit -s 65536 2>/dev/null
BEBOP_BIN=${BEBOP_BIN:-./bebop.bin}; R=${R:-5}
T=${BEBOP_TMP:-/tmp/opencode}/tq_sqlite; mkdir -p "$T"
[ -s "$BEBOP_BIN" ] || { echo "GUARD: BEBOP_BIN=$BEBOP_BIN missing or empty (L12)"; exit 1; }
./seed/build/seed "$BEBOP_BIN" compile bench/tq_sqlite/nn4.bp "$T/nn4.bin" >/dev/null 2>&1 || { echo "COMPILEFAIL nn4"; exit 1; }
seq=(); par=(); ok=1
for i in $(seq "$R"); do
  v=$(taskset -c 4-6 ./seed/build/seed "$T/nn4.bin" | tail -1)
  [ $((v / 1000000000000)) = 1 ] || ok=0
  seq+=($(( (v / 1000000) % 1000000 ))); par+=($(( v % 1000000 )))
done
med() { printf '%s\n' "$@" | sort -n | awk '{a[NR]=$1} END{print a[int((NR+1)/2)]}'; }
s=$(med "${seq[@]}"); p=$(med "${par[@]}")
row="| nn4.bp bucketed scan, 1 A78 vs 3 A78 (sys_setaffinity, R=$R) | seq $s ms / par $p ms | $(python3 -c "print(f'{$s/$p:.2f}x')") | folds equal: $ok |"
echo "$row"
[ "$ok" = 1 ] || { echo "T106 FAIL: parallel fold != sequential fold"; exit 1; }
printf '\n%s\n' "$row" >> bench/tq_sqlite/RESULT.md
