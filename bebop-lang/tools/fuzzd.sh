#!/usr/bin/env bash
# fuzzd.sh (2026-09-06, operator: continuous fuzzing as a background regression shield):
# an endless loop of bench/fuzz/fuzz.sh batches on the LITTLE cores (0xd05, never the A78s
# the chains use), nice 10, one journal line per batch, repros kept under the state dir,
# an ALERT file on any DIVERGE/CRASH/COMPILEFAIL (tools/hooks/pre-push refuses to push while
# it exists). Every batch snapshots the current ./bebop.bin (fuzz.sh copies it), so a
# promotion is picked up at the next batch.
# Usage: tools/fuzzd.sh run [N per batch, default 2000] | start [N] | status | stop | clear
# env: FUZZD (state dir, default ~/.cache/bebop/fuzzd), J (shards, default = little cores)
# Deployed as the Termux runit service `fuzzd` ($PREFIX/var/service/fuzzd/run = proot-distro
# login ubuntu -- tools/fuzzd.sh run): it lives in its own proot, so it survives every Claude
# session (2026-09-05: a batch left in a dying session's proot became a detached tracee
# spinning at 100 % -- boxguard now kills those). Pause/resume: `sv down|up fuzzd`.
cd "$(dirname "$0")/.." || exit 1
D=${FUZZD:-$HOME/.cache/bebop/fuzzd}; mkdir -p "$D/repros"
LITTLE=$(awk '/^processor/{p=$3} /CPU part/ && $NF=="0xd05"{print p}' /proc/cpuinfo | paste -sd,)
alive() { [ -f "$D/pid" ] && kill -0 "$(cat "$D/pid")" 2>/dev/null; }
case "${1:-status}" in
  run)   # foreground loop: what the Termux-side runit service `fuzzd` executes in its OWN proot
    alive && { echo "fuzzd already running (pid $(cat "$D/pid"))"; exit 0; }
    N=${2:-2000}; [ -f "$D/next" ] || echo 100000 > "$D/next"; echo $$ > "$D/pid"
    trap 'rm -f "$D/pid"; exit 0' TERM INT
    while :; do
      START=$(cat "$D/next"); T=$D/run.$START; mkdir -p "$T"
      BEBOP_TMP=$T REPROS=$D/repros J=${J:-$(tr ',' '\n' <<<"$LITTLE" | wc -l)} \
        nice -n 10 ${LITTLE:+taskset -c $LITTLE} bash bench/fuzz/fuzz.sh "$N" "$START" > "$T/log" 2>&1
      SUM=$(grep '^fuzz:' "$T/log" | tail -1); echo "$(date +%s) $SUM" >> "$D/log"
      echo "$(date +%s) H:fuzzd batch $N from $START (background shield, little cores) | DID:tools/fuzzd.sh | GOT:${SUM#fuzz: } | VERDICT:$(grep -q ' DIVERGE=0 COMPILEFAIL=0 CRASH=0 ' <<<"$SUM" && echo confirmed || echo "ALERT (repros in $D/repros)")" >> docs/exp.journal
      grep -q ' DIVERGE=0 COMPILEFAIL=0 CRASH=0 ' <<<"$SUM" || { echo "$(date +%s) $SUM"; grep -E '^(DIVERGE|CRASH|COMPILEFAIL) ' "$T/log"; } >> "$D/ALERT"
      echo $((START + N)) > "$D/next"; rm -rf "$T"
    done;;
  start)  # detached from THIS shell: dies (or spins) with the session's proot -- prefer the service
    alive && { echo "fuzzd already running (pid $(cat "$D/pid"))"; exit 0; }
    nohup "$0" run "${2:-2000}" > "$D/daemon.log" 2>&1 &
    disown; sleep 1; echo "fuzzd started (pid $(cat "$D/pid" 2>/dev/null), cores ${LITTLE:-all}, next seed $(cat "$D/next"), state $D)";;
  status)
    alive && echo "fuzzd RUNNING (pid $(cat "$D/pid"))" || echo "fuzzd STOPPED"
    echo "next seed: $(cat "$D/next" 2>/dev/null); batches: $(wc -l < "$D/log" 2>/dev/null || echo 0); seeds done: $(awk '{for(i=1;i<=NF;i++) if($i ~ /^N=/){split($i,a,"="); s+=a[2]}} END{print s+0}' "$D/log" 2>/dev/null)"
    cur=$(ls -d "$D"/run.* 2>/dev/null | head -1); [ -n "$cur" ] && echo "current batch: $(basename "$cur") $(cat "$cur"/fuzz.*/results.* 2>/dev/null | wc -l) seeds so far"
    tail -n 3 "$D/log" 2>/dev/null | cut -c1-200
    [ -f "$D/ALERT" ] && { echo "ALERT:"; cat "$D/ALERT"; } || echo "no ALERT";;
  stop) alive && { kill -- -"$(cat "$D/pid")" 2>/dev/null || kill "$(cat "$D/pid")"; pkill -P "$(cat "$D/pid")" 2>/dev/null; rm -f "$D/pid"; echo "fuzzd stopped"; } || echo "fuzzd not running";;
  clear) rm -f "$D/ALERT"; echo "ALERT cleared (repros stay in $D/repros)";;
  *) echo "usage: tools/fuzzd.sh run [N] | start [N] | status | stop | clear"; exit 64;;
esac
