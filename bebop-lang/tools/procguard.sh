#!/bin/bash
# procguard -- the box's safety valve and flight recorder (2026-09-28, after the 11th session death).
#
# Android's phantom-process killer takes the WHOLE proot (every session and lane) once the app
# holds more than 32 processes. slot.sh keeps heavy jobs from STARTING above 26, but nothing
# stopped processes piling up after that: death #11 had `procs 34/26` logged two minutes before
# the kill, and the extra ~20 were polling waiters (`until ...; sleep`, `tail -f | grep`).
# This loop turns "the box dies" into "one waiter dies":
#   >= KILL_AT (30): SIGKILL the newest polling waiters (the loop's bash, sleep, tail -f, watch)
#   >= LAST_AT (31) with no waiter left: SIGKILL the slot holder's job tree (one build fails)
# and it records procs/memory every REC_S seconds (plus a full process list at >= LIST_AT) to
# $LOG, which survives the death, so the next post-mortem reads numbers instead of guessing.
#
# Costs ONE process: counting is a /proc glob, reading is `read`/`mapfile`, the nap is a read on
# a fifo (same technique as slot.sh), the clock is $EPOCHSECONDS. It only forks to kill or log.
# Started by the SessionStart hook in ~/.claude/settings.json; a lock makes a second copy exit.
#   tools/procguard.sh            run (normally via the hook, detached)
#   tools/procguard.sh --status   is it running, and the log's last lines
set -u
DIR=${SLOT_DIR:-/root/.cache/bebop/slots}; mkdir -p "$DIR"
LOG=${PROCGUARD_LOG:-/root/.cache/bebop/procguard.log}
KILL_AT=${KILL_AT:-30}; LAST_AT=${LAST_AT:-31}; LIST_AT=${LIST_AT:-27}; REC_S=${REC_S:-30}; TICK=${TICK:-1}

if [ "${1:-}" = --status ]; then
  if flock -n "$DIR/procguard.lock" true 2>/dev/null; then echo "procguard: NOT running"; else echo "procguard: running (pid $(cat "$DIR/procguard.pid" 2>/dev/null))"; fi
  tail -n "${2:-8}" "$LOG" 2>/dev/null; exit 0
fi

exec 9>"$DIR/procguard.lock"; flock -n 9 || exit 0      # one guard per box
echo $$ > "$DIR/procguard.pid"
[ -p "$DIR/tick" ] || mkfifo "$DIR/tick" 2>/dev/null
exec 7<>"$DIR/tick"
exec </dev/null >/dev/null 2>&1
nap() { read -t "$1" -u 7 _ || :; }
procn() { set -- /proc/[0-9]*; PROCN=$#; }
mem() { MA=0 SF=0; local k v _u; while read -r k v _u; do case $k in MemAvailable:) MA=$((v/1024));; SwapFree:) SF=$((v/1024));; esac; done < /proc/meminfo; }
stamp() { printf -v NOW '%(%Y-%m-%dT%H:%M:%SZ)T' -1; }   # TZ=UTC set below, no fork
export TZ=UTC
# ancestors of this guard, and PID 1-ish plumbing, are never victims
PROTECT=" $$ $PPID "

# argv of pid $1 -> ARGV (array), PP (ppid). No fork.
argv() { ARGV=(); PP=0; mapfile -d '' -t ARGV < "/proc/$1/cmdline" 2>/dev/null || return 1
  local l; while read -r l; do case $l in PPid:*) PP=${l#PPid:}; PP=${PP//[[:space:]]/}; break;; esac; done < "/proc/$1/status" 2>/dev/null; }

# Print the waiters, newest (highest pid) first: "pid what". A waiter is
#  - a `sleep` whose parent shell's command line loops (until/while/for) -> the PARENT is the victim
#  - a `tail -f/-F/--follow`, or a `watch`
waiters() {
  local p c pids=() line
  for p in /proc/[0-9]*; do pids+=("${p#/proc/}"); done
  local i
  for ((i=${#pids[@]}-1; i>=0; i--)); do
    p=${pids[i]}; [[ $PROTECT == *" $p "* ]] && continue
    argv "$p" || continue; c=${ARGV[0]##*/}
    case $c in
      sleep)
        local sp=$PP; argv "$sp" || continue
        line="${ARGV[*]}"
        [[ $line =~ (until|while|for)[^\;]*\; ]] || [[ $line =~ (until|while|for).*sleep ]] || continue
        [[ $PROTECT == *" $sp "* ]] && continue
        echo "$sp loop:${line:0:160}";;
      tail) line="${ARGV[*]}"; [[ $line =~ \ (-[a-zA-Z]*[fF]|--follow) ]] && echo "$p $line";;
      watch) echo "$p ${ARGV[*]:0:6}";;
    esac
  done
}

snapshot() {   # full list into the log, no fork
  local p; for p in /proc/[0-9]*; do argv "${p#/proc/}" || continue; echo "    ${p#/proc/} ppid=$PP ${ARGV[*]:0:8}"; done
}

kill_tree() {  # $1 = root pid; children first (reads /proc/*/stat ppid), then the root
  local root=$1 p st rest
  for p in /proc/[0-9]*; do
    read -r st < "$p/stat" 2>/dev/null || continue
    rest=${st##*) }; set -- $rest
    [ "$2" = "$root" ] && kill_tree "${p#/proc/}"
  done
  kill -9 "$root" 2>/dev/null
}

stamp; mem; procn
echo "$NOW START procguard pid=$$ kill_at=$KILL_AT last_at=$LAST_AT procs=$PROCN memavail=${MA}MB swapfree=${SF}MB" >> "$LOG"
LAST_REC=0; LISTED=0
while :; do
  procn
  if [ "$PROCN" -ge "$KILL_AT" ]; then
    stamp; mem
    { echo "$NOW ALERT procs=$PROCN memavail=${MA}MB swapfree=${SF}MB"; snapshot; } >> "$LOG"
    mapfile -t W < <(waiters)
    for w in "${W[@]}"; do
      kill_tree "${w%% *}"; echo "$NOW   KILLED waiter ${w:0:200}" >> "$LOG"
      procn; [ "$PROCN" -lt "$((KILL_AT-1))" ] && break
    done
    procn
    if [ "$PROCN" -ge "$LAST_AT" ]; then
      for who in "$DIR"/slot*.who; do
        [ -e "$who" ] || continue
        read -r lbl pidf _ < "$who"; sp=${pidf#pid=}
        if [ -n "$sp" ] && [ -d "/proc/$sp" ] && ! flock -n "${who%.who}" true 2>/dev/null; then
          kill_tree "$sp"; echo "$NOW   KILLED slot job '$lbl' pid=$sp (procs still $PROCN >= $LAST_AT)" >> "$LOG"
        fi
      done
    fi
    procn; echo "$NOW   after: procs=$PROCN" >> "$LOG"
    nap 3; continue
  fi
  if [ "$EPOCHSECONDS" -ge "$((LAST_REC+REC_S))" ] || { [ "$PROCN" -ge "$LIST_AT" ] && [ "$LISTED" = 0 ]; }; then
    stamp; mem; LAST_REC=$EPOCHSECONDS
    echo "$NOW procs=$PROCN memavail=${MA}MB swapfree=${SF}MB" >> "$LOG"
    if [ "$PROCN" -ge "$LIST_AT" ] && [ "$LISTED" = 0 ]; then snapshot >> "$LOG"; LISTED=1; fi
    [ "$PROCN" -lt "$LIST_AT" ] && LISTED=0
  fi
  nap "$TICK"
done
