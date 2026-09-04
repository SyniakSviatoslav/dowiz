#!/usr/bin/env bash
# run_all.sh — third column of the golden gate: oracle == frozen (std_golden.sh).
# Prints "<gate> <oracle> <frozen> OK|SELF-FROZEN|MISMATCH|MISSING" per gate + a summary.
# RUST=1 additionally re-runs spectral_golden/generator (cargo) and diffs golden.txt byte-exact.
cd "$(dirname "$0")/../.." || exit 1
D=bench/oracles
FROZEN=$(grep -E '^gate [A-Za-z0-9_]+ -?[0-9]+ ' bench/vs_rust/std_golden.sh | awk '{print $2, $3}')

OK=0; SF=0; MM=0; MISS=0
while read -r g f; do
  [ -n "$g" ] || continue
  if [ -f "$D/$g.py" ]; then
    o=$(timeout 300 python3 "$D/$g.py" 2>/dev/null | tail -1)
    if [ "$o" = "$f" ]; then echo "$g $o $f OK"; OK=$((OK+1)); else echo "$g ${o:-ERR} $f MISMATCH"; MM=$((MM+1)); fi
  elif [ -f "$D/$g.self-frozen" ]; then
    echo "$g - $f SELF-FROZEN"; SF=$((SF+1))
  else
    echo "$g - $f MISSING"; MISS=$((MISS+1))
  fi
done <<<"$FROZEN"

RUST=""
if [ "${RUST:-0}" = 1 ]; then
  G=bench/vs_rust/spectral_golden; R=${BEBOP_TMP:-/tmp/opencode}/golden.regen.txt
  if (cd "$G/generator" && cargo run --release >"$R" 2>/dev/null) && cmp -s "$R" "$G/golden.txt"
  then RUST=" golden.txt=BYTE-EXACT"
  else RUST=" golden.txt=DIFF(only-in-golden=$(diff "$R" "$G/golden.txt" | grep -c '^>') only-in-generator=$(diff "$R" "$G/golden.txt" | grep -c '^<'))"; MM=$((MM+1)); fi
fi
echo "SUMMARY ok=$OK self-frozen=$SF mismatch=$MM missing=$MISS$RUST"
[ $((MM+MISS)) -eq 0 ]
