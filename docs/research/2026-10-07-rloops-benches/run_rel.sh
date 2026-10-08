#!/bin/bash
# release run: each bench separately so a SIGKILL loses one, not all
B=/tmp/claude-0/-root/4384f304-d791-40f9-ae45-2e35de9584c7/scratchpad/rloops/target/release/rbench
O=/tmp/claude-0/-root/4384f304-d791-40f9-ae45-2e35de9584c7/scratchpad/rloops/results
for w in router bucket orders ppr logwalk journal; do
  echo "== $w $(date +%H:%M:%S) mhz=$(cat /sys/devices/system/cpu/cpu4/cpufreq/scaling_cur_freq 2>/dev/null)"
  $B $w 9 > $O/rel-$w.out 2>&1; echo "rc=$?"; cat $O/rel-$w.out
done
