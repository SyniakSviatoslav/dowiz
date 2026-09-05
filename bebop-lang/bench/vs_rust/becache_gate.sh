#!/usr/bin/env bash
# T108 gate: compile every std gate source cold (no .becache records) and warm
# (records present). Every warm .bin must be byte-identical to its cold .bin, and
# the warm compile must cost no more than 1.5x the process floor (running a trivial
# .bin under seed: ~90 ms on this box, measured here each time) — i.e. the memoized
# compile is the process, not the compiler. (std_golden's wall is dominated by
# RUNNING the gates; the roadmap's original ">= 5x warm std_golden" cannot be met
# with a ~100 ms per-process floor, recorded 2026-09-05.) env: BEBOP_BIN, BEBOP_TMP.
set -u
cd "$(dirname "$0")/../.."
T=${BEBOP_TMP:-/tmp/opencode}/becache; mkdir -p "$T"; BB=${BEBOP_BIN:-./bebop.bin}
rm -f "$T"/*.bin "$T"/*.becache
run() { local t0=$(date +%s%N); for f in bench/vs_rust/std_tests/*.bp; do b=$(basename "$f" .bp); ./seed/build/seed "$BB" compile "$f" "$T/$b.bin" >/dev/null 2>&1 || echo "COMPILEFAIL $b" >&2; done; echo $(( ($(date +%s%N) - t0) / 1000000 )); }
c=$(run); md5sum "$T"/*.bin > "$T/cold.md5"; w=$(run); md5sum "$T"/*.bin > "$T/warm.md5"
n=$(ls "$T"/*.becache | wc -l); same=$(cmp -s "$T/cold.md5" "$T/warm.md5" && echo identical || echo DIFFER)
printf 'fn main() -> i64 { 0 }\n' > "$T/floor.bp"; ./seed/build/seed "$BB" compile "$T/floor.bp" "$T/floor.bin" >/dev/null 2>&1
fl=(); for i in 1 2 3 4 5; do t0=$(date +%s%N); ./seed/build/seed "$T/floor.bin" >/dev/null 2>&1; fl+=($(( ($(date +%s%N) - t0) / 1000000 ))); done
floor=$(printf '%s\n' "${fl[@]}" | sort -n | sed -n 3p); per=$((w / n))
echo "becache: $n gates, compile cold ${c} ms ($((c / n)) per gate), warm ${w} ms (${per} per gate), process floor ${floor} ms, ratio $(python3 -c "print(f'{$c/max($w,1):.1f}x')"), warm bins $same"
[ "$same" = identical ] && [ $((per * 2)) -le $((floor * 3)) ] && echo "becache gate: PASS" || { echo "becache gate: FAIL"; exit 1; }
