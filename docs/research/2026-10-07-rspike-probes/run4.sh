#!/usr/bin/env bash
set -u
D=$(cd "$(dirname "$0")" && pwd); OUT=${1:-$D/../results/raw4.txt}; R=${R:-5}
: > "$OUT"; echo "# $(date -u +%FT%TZ)" >> "$OUT"
gcc -O2 -march=armv8.2-a -o "$D/bin/k1h" "$D/k1h.c" 2>>"$OUT"
for r in $(seq 1 "$R"); do echo "## k1h w=4 run $r MHz=$(cat /sys/devices/system/cpu/cpu4/cpufreq/scaling_cur_freq 2>/dev/null)" >> "$OUT"; taskset -c 4 "$D/bin/k1h" 4 >> "$OUT" 2>&1 || echo "RC k1h_w4 $?" >> "$OUT"; done
echo "# done $(date -u +%FT%TZ)" >> "$OUT"
