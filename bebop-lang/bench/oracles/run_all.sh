#!/usr/bin/env bash
# run_all.sh — third column of the golden gate: oracle == frozen (std_golden.sh).
# Prints "<gate> <oracle> <frozen> OK|SELF-FROZEN|MISMATCH|MISSING" per gate + a summary.
# RUST=1 additionally re-runs spectral_golden/generator (cargo) and diffs golden.txt byte-exact.
# 2026-09-06 (dev-loop parallelism): the oracles run J at a time on the little cores (they
# never touch the compiler, so they must not compete with the std shards on the A78s), and
# a run whose inputs (every oracle file + the frozen list) equal the last GREEN run's is
# REPLAYED from $RUN_ALL_MEMO (FORCE=1 reruns; RUST=1 never memoizes).
cd "$(dirname "$0")/../.." || exit 1
D=bench/oracles
FROZEN=$(grep -E '^gate [A-Za-z0-9_]+ -?[0-9]+ ' bench/vs_rust/std_golden.sh | awk '{print $2, $3}')
MEMO=${RUN_ALL_MEMO:-$HOME/.cache/bebop/run_all}; mkdir -p "$MEMO"
KEY=$({ cat "$D"/*.py; ls "$D"/*.self-frozen 2>/dev/null; echo "$FROZEN"; } | sha256sum | cut -c1-16)
if [ "${FORCE:-0}" != 1 ] && [ "${RUST:-0}" != 1 ] && [ -s "$MEMO/$KEY" ]; then
  sed '$ s/$/ (memo: inputs unchanged since the last GREEN run)/' "$MEMO/$KEY"; exit 0
fi
one() {  # one <gate> <frozen>
  local g=$1 f=$2 o
  if [ -f "$D/$g.py" ]; then
    o=$(timeout 300 python3 "$D/$g.py" 2>/dev/null | tail -1)
    if [ "$o" = "$f" ]; then echo "$g $o $f OK"; else echo "$g ${o:-ERR} $f MISMATCH"; fi
  elif [ -f "$D/$g.self-frozen" ]; then echo "$g - $f SELF-FROZEN"
  else echo "$g - $f MISSING"; fi
}
export -f one; export D
LITTLE=$(awk '/^processor/{p=$3} /CPU part/ && $NF=="0xd05"{print p}' /proc/cpuinfo | paste -sd,)
OUT=$(echo "$FROZEN" | ${LITTLE:+taskset -c $LITTLE} xargs -P "${J:-4}" -n 2 bash -c 'one "$@"' _ | sort)
echo "$OUT"
OK=$(grep -c ' OK$' <<<"$OUT"); SF=$(grep -c ' SELF-FROZEN$' <<<"$OUT"); MM=$(grep -c ' MISMATCH$' <<<"$OUT"); MISS=$(grep -c ' MISSING$' <<<"$OUT")

RUST=""
if [ "${RUST:-0}" = 1 ]; then
  G=bench/vs_rust/spectral_golden; R=${BEBOP_TMP:-/tmp/opencode}/golden.regen.txt
  if (cd "$G/generator" && cargo run --release >"$R" 2>/dev/null) && cmp -s "$R" "$G/golden.txt"
  then RUST=" golden.txt=BYTE-EXACT"
  else RUST=" golden.txt=DIFF(only-in-golden=$(diff "$R" "$G/golden.txt" | grep -c '^>') only-in-generator=$(diff "$R" "$G/golden.txt" | grep -c '^<'))"; MM=$((MM+1)); fi
fi
SUM="SUMMARY ok=$OK self-frozen=$SF mismatch=$MM missing=$MISS$RUST"
echo "$SUM"
[ $((MM+MISS)) -eq 0 ] && [ -z "$RUST" ] && { echo "$OUT"; echo "$SUM"; } > "$MEMO/$KEY"
[ $((MM+MISS)) -eq 0 ]
