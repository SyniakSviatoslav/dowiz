#!/usr/bin/env bash
# slot.sh (operator 2026-09-08: "max agents without overloading the box") -- the box-safety
# semaphore for HEAVY jobs (chain.sh, battery.sh, sweeps, fuzz batches). It REPLACES, for heavy
# jobs, the `reap.sh --check 30` + "exit 97, reap, ONE retry" dance: waiting on a slot is free
# (a blocking flock, no CPU, no poll loop), so a lane never burns a retry or dies on a full box.
#
#   Usage:  tools/slot.sh <label> <command ...>
#           tools/slot.sh --status
#
# Measured on this box 2026-09-08: 7 cores in the affinity mask (4,5,6 = A78 big; 0-3 little),
# 7.5 GB RAM, one self-compile of bebop.bp = 2.0 s wall / 22 MB maxrss, a SERIAL=1 chain = ~13
# extra procs. So the limit is not memory and not the old proc cap -- it is CPU contention and
# the battery's fork storm. SLOTS=3 heavy jobs, each CONFINED (taskset on the whole process tree)
# to its own big core + its own little core, leaves core 3 for the light agents (editing,
# analysis, python oracles, llm_route.sh) which never take a slot.
#
#   slot 1 -> cores 4,5,6 (A78, reserved for the main session merge chain via SLOT_ONLY=1)
#   slot 2 -> cores 0,1    slot 3 -> cores 2,3
#
# Inside a slot PROC_CAP defaults to 70: the slot IS the concurrency gate now, so chain.sh's own
# gate must not refuse a legitimate third lane (25 idle + 3*13 = ~64 procs at the peak).
# The MemAvailable floor below is the one hard refusal: it is the only failure mode that can take
# the box down rather than merely slow it.
#
# 2026-09-08 (operator: "зроби так, щоб не отримувати Signal 9"): the box is Android 16 / SDK 36
# (getprop ro.build.version.sdk = 36) under Termux + proot-distro, and the SIGKILLs that end a
# session are NOT the kernel OOM killer -- they are Android's PHANTOM PROCESS KILLER, which caps
# the processes an app forks outside ActivityManager's knowledge at `max_phantom_processes` = 32
# by default and kills the excess with SIGKILL, oldest-heaviest first. Every process in this proot
# is one of those. Measured here: 12 procs idle, a SERIAL=1 chain adds ~15 (peak ~27) and a
# battery's fork storm more, so ONE heavy job already sits at the edge of 32 and two blow past it.
# That is why SLOTS now defaults to 1 (serialise heavy jobs -- waiting on a slot is free) and why
# a slot also waits on a live process-count ceiling below the Android cap. Neither knob can be
# changed from inside: this proot runs as uid 10546, `device_config` is not reachable
# ("Can't find service: device_config") and /proc/sys/vm/* is not readable, so raising
# max_phantom_processes stays an on-device / ADB action for the operator (see docs/BOX.md).
# What IS reachable from inside, and is used here: oom_score_adj can only be RAISED by an
# unprivileged process, which is the direction we want -- a slot raises its own tree to 700 so
# that if the kernel OOM killer does fire it eats the compile, not the claude session at 0.
set -u
LOCKDIR=${SLOT_DIR:-/root/.cache/bebop/slots}; mkdir -p "$LOCKDIR"
N=${SLOTS:-1}   # was 3; see the phantom-process-killer note above -- heavy jobs serialise now
CORES_1=${CORES_1:-4,5,6}; CORES_2=${CORES_2:-0,1}; CORES_3=${CORES_3:-2,3}  # slot 1 = the 3 A78 big cores, reserved for the main session's authoritative merge chain (SLOT_ONLY=1); lanes take 2 and 3
MEM_FLOOR_MB=${MEM_FLOOR_MB:-1000}   # was 600: MemAvailable here already dips to ~3 GB with the session alone
PHANTOM_CAP=${PHANTOM_CAP:-26}       # live `ps -e` ceiling a slot waits under; 26 + one chain's ~15 stays inside Android's 32
PHANTOM_WAIT_S=${PHANTOM_WAIT_S:-300}
SLOT_OOM_ADJ=${SLOT_OOM_ADJ:-700}    # raise-only for an unprivileged process; the claude session stays at 0

if [ "${1:-}" = --status ]; then
  for i in $(seq 1 "$N"); do
    if flock -n "$LOCKDIR/slot$i" true 2>/dev/null; then echo "slot $i free"; else echo "slot $i BUSY $(cat "$LOCKDIR/slot$i.who" 2>/dev/null)"; fi
  done
  echo "procs $(ps -e --no-headers | wc -l)/$PHANTOM_CAP (android phantom cap 32)  memavail $(awk '/MemAvailable/{print int($2/1024)"MB"}' /proc/meminfo)"
  exit 0
fi

LABEL=${1:?usage: tools/slot.sh <label> <command ...>}; shift
[ $# -gt 0 ] || { echo "slot: no command" >&2; exit 2; }

# Wait (do not refuse) under the process ceiling: a slot that blocks costs nothing, a slot that
# starts at 30 procs gets its tree SIGKILLed halfway through and costs the whole run.
pw0=$(date +%s)
while [ "$(ps -e --no-headers | wc -l)" -gt "$PHANTOM_CAP" ]; do
  [ $(( $(date +%s) - pw0 )) -lt "$PHANTOM_WAIT_S" ] || {
    echo "slot: REFUSED -- $(ps -e --no-headers | wc -l) procs still above the ${PHANTOM_CAP} ceiling after ${PHANTOM_WAIT_S}s (Android kills past 32); run tools/reap.sh kill" >&2; exit 95; }
  sleep 5
done

avail=$(awk '/MemAvailable/{print int($2/1024)}' /proc/meminfo)
[ "$avail" -ge "$MEM_FLOOR_MB" ] || { echo "slot: REFUSED -- MemAvailable ${avail}MB < ${MEM_FLOOR_MB}MB floor (box safety)" >&2; exit 96; }

# fair queue: one waiter at a time hunts for a free slot, so waiters never spin against each other
exec 8>"$LOCKDIR/queue"; flock -x 8
# Hand-out ORDER, not reservation (2026-09-08: reserving slot 1 outright inverted priorities --
# a lane took it while the main session's merge chain blocked on it with 2 and 3 idle):
#   SLOT_ONLY=k   wait for exactly slot k (rarely what you want)
#   SLOT_PREFER=k try k first, then the rest -- the main session's merge chain uses SLOT_PREFER=1
#   default       try 2..N first, then 1, so the big-core slot 1 goes to a lane only when the
#                 cheaper slots are taken and the main session is not using it
SLOT=""
FIRST=${SLOT_ONLY:-}
if [ -n "$FIRST" ]; then ORDER=$FIRST
elif [ -n "${SLOT_PREFER:-}" ]; then ORDER="$SLOT_PREFER $(seq 1 "$N" | grep -vx "$SLOT_PREFER" | tr '\n' ' ')"
else ORDER="$(seq 2 "$N" | tr '\n' ' ') 1"; fi
while [ -z "$SLOT" ]; do
  for i in $ORDER; do
    eval "exec 9>\"$LOCKDIR/slot$i\""
    if flock -n 9; then SLOT=$i; break; fi
    exec 9>&-
  done
  [ -n "$SLOT" ] || flock -w 10 -x "$LOCKDIR/slot${FIRST:-$(echo $ORDER | cut -d" " -f1)}" true 2>/dev/null  # blocking wait, no busy loop
done
flock -u 8; exec 8>&-
eval "CORES=\$CORES_$SLOT"
echo "$LABEL pid=$$ started=$(date +%H:%M:%S) cores=$CORES" > "$LOCKDIR/slot$SLOT.who"
echo "slot: acquired $SLOT (cores $CORES) for '$LABEL' at $(date +%H:%M:%S)"
# PROC_CAP was 70 to stop chain.sh refusing a legitimate third lane. There are no three lanes
# any more (SLOTS=1) and 70 is above Android's phantom cap, so the gate was disabled in the
# one direction that matters: 32 is now the real ceiling the platform enforces.
export PROC_CAP=${PROC_CAP:-32} SLOT_ID=$SLOT SLOT_CORES=$CORES
export PIN=${PIN:-taskset -c $CORES}  # chain.sh would otherwise re-pin to 4-6 and escape the slot (sched_setaffinity can always widen)
# Make THIS tree the OOM killer's first choice instead of the session that is driving it.
# Unprivileged processes may only raise oom_score_adj, which is exactly the direction we need;
# children inherit it, so the whole heavy job is covered by this one write.
echo "$SLOT_OOM_ADJ" > /proc/self/oom_score_adj 2>/dev/null || true
t0=$(date +%s)
taskset -c "$CORES" nice -n "${SLOT_NICE:-5}" "$@"; rc=$?
echo "slot: released $SLOT after $(( $(date +%s) - t0 )) s, rc=$rc"
: > "$LOCKDIR/slot$SLOT.who"
exit $rc
