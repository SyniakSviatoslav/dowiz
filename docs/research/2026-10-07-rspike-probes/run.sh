#!/usr/bin/env bash
# R-SPIKE driver: build every probe with gcc -O2, run each R times pinned to core 4, print all RES lines.
# Run as:  tools/slot.sh rspike bash run.sh <outfile>
set -u
D=$(cd "$(dirname "$0")" && pwd); OUT=${1:-$D/../results/raw.txt}; R=${R:-5}
mkdir -p "$(dirname "$OUT")" "$D/bin"
: > "$OUT"
echo "# $(date -u +%FT%TZ) gcc=$(gcc -dumpversion) core4MHz=$(cat /sys/devices/system/cpu/cpu4/cpufreq/scaling_cur_freq 2>/dev/null)" >> "$OUT"
for p in k1h k4 k3h k8h fib sum dispatch; do
  gcc -O2 -march=armv8.2-a -o "$D/bin/$p" "$D/$p.c" 2>>"$OUT" || { echo "COMPILEFAIL $p" | tee -a "$OUT"; continue; }
done
# k8h also at -O2 with if-conversion disabled, to see what gcc does by default vs forced branch (objdump check below)
objdump -d "$D/bin/k8h" > "$D/bin/k8h.dis" 2>/dev/null
for fn in k8_branch k8_csel k8_mask; do
  awk -v f="<$fn>:" '$0 ~ f {p=1} p && /^$/ {p=0} p' "$D/bin/k8h.dis" | grep -cE '\bcsel|\bcsinc|\bcsneg|\bcinc' | sed "s/^/OBJDUMP $fn csel_count=/" >> "$OUT"
  awk -v f="<$fn>:" '$0 ~ f {p=1} p && /^$/ {p=0} p' "$D/bin/k8h.dis" | grep -cE '\bb\.(ne|eq|lt|gt|le|ge|hi|ls|cs|cc|mi|pl)|\bcbz|\bcbnz|\btbz|\btbnz' | sed "s/^/OBJDUMP $fn bcond_count=/" >> "$OUT"
done
objdump -d "$D/bin/dispatch" > "$D/bin/dispatch.dis" 2>/dev/null
grep -cE '^\s+[0-9a-f]+:\s+[0-9a-f]+\s+br\s' "$D/bin/dispatch.dis" | sed 's/^/OBJDUMP dispatch br_count=/' >> "$OUT"
for p in k1h k4 k3h k8h fib sum dispatch; do
  [ -x "$D/bin/$p" ] || continue
  for r in $(seq 1 "$R"); do
    echo "## $p run $r MHz=$(cat /sys/devices/system/cpu/cpu4/cpufreq/scaling_cur_freq 2>/dev/null)" >> "$OUT"
    taskset -c 4 "$D/bin/$p" >> "$OUT" 2>&1; rc=$?; [ $rc -eq 0 ] || echo "RC $p $rc" >> "$OUT"
  done
done
echo "# done $(date -u +%FT%TZ)" >> "$OUT"
# perf counters (may be unavailable under proot)
gcc -O2 -o "$D/bin/perfprobe" "$D/perfprobe.c" 2>>"$OUT" && taskset -c 4 "$D/bin/perfprobe" >> "$OUT" 2>&1
# bebop: loop twin (bench630/k1ht.bp, REPS=100) vs closed form (k1h_closed.bp, REPS=20000) + the agreement check
BL=/root/dowiz/bebop-lang; mkdir -p "$D/bin/bp"
( cd "$BL" && for s in k1h_closed k1h_closed_check; do rm -f "$D/bin/bp/$s.bin"*; ./seed/build/seed ./bebop.bin compile "$D/$s.bp" "$D/bin/bp/$s.bin" >> "$OUT" 2>&1 || echo "BPCOMPILEFAIL $s" >> "$OUT"; done
  rm -f "$D/bin/bp/k1ht.bin"*; ./seed/build/seed ./bebop.bin compile bench/vs_rust/bench630/k1ht.bp "$D/bin/bp/k1ht.bin" >> "$OUT" 2>&1 || echo "BPCOMPILEFAIL k1ht" >> "$OUT"
  echo "BPCHECK k1h_closed_check $(taskset -c 4 ./seed/build/seed "$D/bin/bp/k1h_closed_check.bin" 2>&1 | tail -1)" >> "$OUT"
  for r in $(seq 1 "$R"); do
    echo "BP k1ht_loop_total_ms_100reps $(taskset -c 4 ./seed/build/seed "$D/bin/bp/k1ht.bin" 2>&1 | tail -1) MHz=$(cat /sys/devices/system/cpu/cpu4/cpufreq/scaling_cur_freq 2>/dev/null)" >> "$OUT"
    echo "BP k1h_closed_total_ms_20000reps $(taskset -c 4 ./seed/build/seed "$D/bin/bp/k1h_closed.bin" 2>&1 | tail -1) MHz=$(cat /sys/devices/system/cpu/cpu4/cpufreq/scaling_cur_freq 2>/dev/null)" >> "$OUT"
  done )
# python oracle (slow: ~1 min of CPU)
taskset -c 4 python3 -I "$D/oracle.py" >> "$OUT" 2>&1
echo "# all done $(date -u +%FT%TZ)" >> "$OUT"
