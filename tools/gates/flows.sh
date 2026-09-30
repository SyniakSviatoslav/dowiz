#!/usr/bin/env bash
# flows.sh -- the browser gate before (and after) a deploy: the customer, owner
# and courier flows on qa-durres, then the cleanup (e2e/flows/run.mjs).
#
#   bash tools/gates/flows.sh              # prints ONE line, exits non-zero on any FAIL
#   FLOWS_LOCAL=<public dir> bash tools/gates/flows.sh   # UI .js/.css from a local tree
#
# WHY: the worst dowiz defects were found by a browser while every suite was
# green (memory dowiz-live-bugs-only-a-browser-found); W-VERIFY found four more
# the same way on 2026-09-30.
#
# ONE CHROMIUM AT A TIME, THROUGH THE SLOT, IN THE FOREGROUND: the box dies
# above 32 processes (DOWIZ-COMMON-RULES rule 1). slot.sh blocks until the slot
# is free; a refused slot (95/96/97) is reported, never retried here.
#
# Output: `flows: F1 ok F2 ok F3 ok F4 ok (NN s)` and rc 0, or
# `flows: FAIL F<n> :: <reason>` and rc 1 (3: chromium could not launch; the
# slot's own refusal codes pass through). The full log's path is printed.
set -u
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
SLOT=${SLOT_SH:-$ROOT/bebop-lang/tools/slot.sh}
[ -f "$SLOT" ] || SLOT=/root/dowiz/bebop-lang/tools/slot.sh
LOGDIR=${FLOWS_LOGDIR:-${TMPDIR:-/tmp}/dowiz-flows}
mkdir -p "$LOGDIR"
LOG="$LOGDIR/gate-$(date +%Y%m%d-%H%M%S).log"

bash "$SLOT" flows node "$ROOT/e2e/flows/run.mjs" > "$LOG" 2>&1
rc=$?
last=$(grep '^flows: ' "$LOG" | tail -1)
case $rc in
  95|96|97|137)
    echo "flows: FAIL could not launch: slot.sh refused or was killed (rc=$rc), log $LOG"; exit "$rc" ;;
esac
if [ -z "$last" ]; then
  echo "flows: FAIL no verdict line from run.mjs (rc=$rc) -- $(grep -v '^\s*$' "$LOG" | tail -1 | cut -c1-200), log $LOG"
  [ "$rc" -eq 0 ] && rc=1
  exit "$rc"
fi
echo "$last"
echo "flows: log $LOG"
# A zero exit with a FAIL line (or the reverse) is a broken instrument, not a pass.
if [ "$rc" -eq 0 ] && ! printf '%s' "$last" | grep -Eq '^flows: F1 ok F2 ok F3 ok F4 ok \([0-9]+ s\)$'; then
  echo "flows: FAIL run.mjs exited 0 but its verdict is not all-ok"; exit 1
fi
exit "$rc"
