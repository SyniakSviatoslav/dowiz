#!/usr/bin/env bash
# run_all2.sh — D11-E second independent oracles vs the first-generation oracles.
# For each gate: last line of bench/oracles2/<gate>.py vs last line of the OLD oracle
# (bench/oracles/<gate>.py, executed as a black box; crc -> crc32.py, nnidx ->
# bench/tq_sqlite/oracle.py).  Prints "<gate> MATCH|MISMATCH <new> <old>".
# MISMATCH = a finding (fold underspecified in prose or a real disagreement), not a failure.
cd "$(dirname "$0")/../.." || exit 1
NEW=bench/oracles2; OLD=bench/oracles
old_of() { case "$1" in crc) echo "$OLD/crc32.py";; nnidx) echo "bench/tq_sqlite/oracle.py";; *) echo "$OLD/$1.py";; esac; }
m=0; mm=0
for g in csr bt store tq mvcc stm sha256 crc sort rng money ordfsm nnidx; do
  n=$(timeout 60 python3 "$NEW/$g.py" 2>/dev/null | tail -1)
  o=$(timeout 300 python3 "$(old_of "$g")" 2>/dev/null | tail -1)
  if [ -n "$n" ] && [ "$n" = "$o" ]; then echo "$g MATCH $n $o"; m=$((m+1)); else echo "$g MISMATCH ${n:-ERR} ${o:-ERR}"; mm=$((mm+1)); fi
  if [ "$g" = nnidx ]; then  # the old oracle's last line is the BRUTE-FORCE nearest fold; the gate uses the 3x3 window fold
    t=$(timeout 60 python3 "$NEW/$g.py" 2>/dev/null | awk '/^true_nearest_fold/{print $2}')
    if [ -n "$t" ] && [ "$t" = "$o" ]; then echo "nnidx(true-nearest) MATCH $t $o"; m=$((m+1)); else echo "nnidx(true-nearest) MISMATCH ${t:-ERR} ${o:-ERR}"; mm=$((mm+1)); fi
  fi
done
echo "SUMMARY match=$m mismatch=$mm"
