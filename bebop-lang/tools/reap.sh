#!/usr/bin/env bash
# reap.sh (L21, operator 2026-09-06): after every task list -- and with `kill` remove -- the
# leftovers of dead sessions: work processes (bash/python3/seed/xargs running bench/ or
# tools/ scripts, or a seed run) whose parent is 1 = orphaned from a dead Claude shell, plus
# any process whose children are all zombies (an xargs/bash that stopped reaping). Zombies
# themselves cannot be killed; their orphaned parent can. Never touches claude, node, proot,
# sshd, runsv*, svlogd, boxguard, the fuzzd loop (its own proot; stop = `sv down fuzzd`).
# Usage: tools/reap.sh [kill]   self-test: `REAP_PS=tools/reap.fixture tools/reap.sh` lists 29014 + 29083 (the 2026-09-06 orphan)
# --check N (item 1, retro D13): print the total process count and exit non-zero above N --
# the process-count gate the runners (chain.sh/battery.sh/fuzz.sh/fuzz_batch.py) call first.
psl() { if [ -n "${REAP_PS:-}" ]; then cat "$REAP_PS"; else ps -eo pid,ppid,stat,etime,args --no-headers; fi; }
if [ "${1:-}" = --check ]; then
  N=${2:?tools/reap.sh --check N}; n=$(psl | wc -l)
  echo "reap: $n procs (cap $N)"
  [ "$n" -le "$N" ]
  exit $?
fi
PROTECT='claude|node|proot|sshd|runsv|svlogd|boxguard|fuzzd\.sh run|reap\.sh'
list=$(psl | awk -v prot="$PROTECT" '
  $3 ~ /^Z/ { z[$2]++; next }
  { live[$2]++; line[$1]=$0 }
  $2==1 && $5 ~ /^(bash|sh|python3|xargs|\.?\/?seed)/ && $0 ~ /bench\/|tools\/|seed\/build\/seed/ && $0 !~ prot { out[$1]=1 }
  END { for (p in z) if (!(p in live) && (p in line) && line[p] !~ prot) out[p]=1; for (p in out) print line[p] }' | sort -n)
n=$(psl | wc -l)
[ -z "$list" ] && { echo "reap: clean ($n procs)"; exit 0; }
echo "$list"
if [ "${1:-}" = kill ]; then
  echo "$list" | awk '{print $1}' | xargs -r kill 2>/dev/null; sleep 1
  echo "$list" | awk '{print $1}' | xargs -r kill -9 2>/dev/null
  echo "reap: killed $(echo "$list" | wc -l), now $(ps -e --no-headers | wc -l) procs"
else echo "reap: $(echo "$list" | wc -l) leftover(s) of $n procs -- run tools/reap.sh kill"; fi
