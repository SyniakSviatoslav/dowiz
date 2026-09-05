#!/usr/bin/env bash
# T108 gate: std_golden cold (no .becache records) vs warm (every record present):
# same pass count, warm wall >= 5x faster. env: BEBOP_BIN, BEBOP_TMP.
set -u
cd "$(dirname "$0")/../.."
T=${BEBOP_TMP:-/tmp/opencode}; export BEBOP_TMP=$T BEBOP_BIN=${BEBOP_BIN:-./bebop.bin}
rm -f "$T"/*.becache
t0=$(date +%s%N); cold=$(bash bench/vs_rust/std_golden.sh 2>/dev/null | tail -1); t1=$(date +%s%N)
warm=$(bash bench/vs_rust/std_golden.sh 2>/dev/null | tail -1); t2=$(date +%s%N)
c=$(( (t1 - t0) / 1000000 )); w=$(( (t2 - t1) / 1000000 )); n=$(ls "$T"/*.becache 2>/dev/null | wc -l)
echo "becache: cold ${c} ms [$cold] warm ${w} ms [$warm] records=$n ratio=$(python3 -c "print(f'{$c/max($w,1):.1f}x')")"
[ "$cold" = "$warm" ] && [ $((c)) -ge $((w * 5)) ] && echo "becache gate: PASS" || { echo "becache gate: FAIL"; exit 1; }
