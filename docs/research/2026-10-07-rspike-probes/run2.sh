#!/usr/bin/env bash
# second batch: the fixed k1h/k4/k3h/sum probes + stream calibration; R runs pinned to core 4
set -u
D=$(cd "$(dirname "$0")" && pwd); OUT=${1:-$D/../results/raw2.txt}; R=${R:-5}
: > "$OUT"; echo "# $(date -u +%FT%TZ) core4MHz=$(cat /sys/devices/system/cpu/cpu4/cpufreq/scaling_cur_freq 2>/dev/null)" >> "$OUT"
for p in k1h k4 k3h sum stream; do gcc -O2 -march=armv8.2-a -o "$D/bin/$p" "$D/$p.c" 2>>"$OUT" || echo "COMPILEFAIL $p" >> "$OUT"; done
for p in k1h k4 k3h sum stream; do [ -x "$D/bin/$p" ] || continue
  for r in $(seq 1 "$R"); do echo "## $p run $r MHz=$(cat /sys/devices/system/cpu/cpu4/cpufreq/scaling_cur_freq 2>/dev/null)" >> "$OUT"
    taskset -c 4 "$D/bin/$p" >> "$OUT" 2>&1; rc=$?; [ $rc -eq 0 ] || echo "RC $p $rc" >> "$OUT"; done; done
echo "# done $(date -u +%FT%TZ)" >> "$OUT"
