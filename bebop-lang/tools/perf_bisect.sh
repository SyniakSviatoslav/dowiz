#!/usr/bin/env bash
# perf_bisect.sh (E12, D12-A): a `git bisect run` step. Uses the COMMITTED bebop.bin + seed of the
# checked-out commit (no build), measures ONE metric with tools/perf.py and compares it to a
# threshold: exit 0 = good, 1 = bad, 125 = skip (invalid window per E7).
# Usage: tools/perf_bisect.sh <metric> <max-value> [--n 5|--r 11]
#   metrics: selfcompile_wall selfcompile_utime selfcompile_maxrss k1h_ms k2h_ms k3h_ms k4_ms bin_words k4_loopwords ...
#   e.g. git bisect start <bad> <good>; git bisect run bebop-lang/tools/perf_bisect.sh selfcompile_utime 2.0
cd "$(dirname "$0")/.." || exit 125
M=${1:?metric}; MAX=${2:?max value}; shift 2
[ -s bebop.bin ] && [ -x seed/build/seed ] || exit 125
T=${BEBOP_TMP:-/tmp/opencode}/bisect; mkdir -p "$T"
case $M in
  selfcompile_*) out=$(BEBOP_TMP=$T CSV=/dev/null python3 tools/perf.py selfcompile --bin ./bebop.bin "$@" 2>&1) ;;
  k*_ms|k*_loopwords) out=$(BEBOP_TMP=$T python3 tools/perf.py kernels --bin ./bebop.bin "$@" 2>&1) ;;
  bin_words|stub_words) out=$(python3 tools/perf.py size --bin ./bebop.bin 2>&1) ;;
  *) echo "unknown metric $M"; exit 125 ;;
esac
echo "$out" | grep -q '"valid": 0' && { echo "bisect: INVALID window (throttled/busy) -> skip"; exit 125; }
v=$(tail -1 bench/perf.csv | awk -F, -v m="$M" '$4==m{print $5}'); [ -z "$v" ] && v=$(grep -E "^$M " <<<"$out" | awk '{print $2}')
[ -z "$v" ] && v=$(awk -F, -v m="$M" '$4==m{v=$5} END{print v}' bench/perf.csv)
echo "bisect: $M = $v (max $MAX) at $(git rev-parse --short HEAD)"
python3 -c "import sys; sys.exit(0 if float('$v') <= float('$MAX') else 1)"
