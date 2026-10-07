#!/usr/bin/env bash
# find which k1h variant segfaults, then time the survivors R times
set -u
D=$(cd "$(dirname "$0")" && pwd); OUT=${1:-$D/../results/raw3.txt}; R=${R:-5}
: > "$OUT"; echo "# $(date -u +%FT%TZ)" >> "$OUT"
gcc -O2 -g -march=armv8.2-a -o "$D/bin/k1h" "$D/k1h.c" 2>>"$OUT"
for w in 0 1 2 3 4; do taskset -c 4 "$D/bin/k1h" $w >> "$OUT" 2>&1; echo "VARIANT $w rc=$?" >> "$OUT"; done
for r in $(seq 1 "$R"); do for w in 0 1 2 3 4; do echo "## k1h w=$w run $r MHz=$(cat /sys/devices/system/cpu/cpu4/cpufreq/scaling_cur_freq 2>/dev/null)" >> "$OUT"; taskset -c 4 "$D/bin/k1h" $w >> "$OUT" 2>&1 || echo "RC k1h_w$w $?" >> "$OUT"; done; done
echo "# done $(date -u +%FT%TZ)" >> "$OUT"
