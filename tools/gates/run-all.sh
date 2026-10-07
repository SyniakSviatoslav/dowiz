#!/bin/sh
# EVERY GATE, ONE TABLE, EVERY EXIT CODE.
#
#   sh tools/gates/run-all.sh            # the gates that need no compiler (seconds to minutes)
#   sh tools/gates/run-all.sh --cargo    # also vocab.sh and the bebop-wasm four-reader gate
#
# On the dev box, anything that compiles goes through the slot, in the foreground:
#   bash bebop-lang/tools/slot.sh gates sh tools/gates/run-all.sh --cargo
#
# ORDER: each `*.prove.sh` runs BEFORE its gate. A proof shows the gate can fire (it breaks the
# rule in a scratch copy and demands RED, then restores and demands GREEN); a gate whose alarm has
# never been heard measures nothing (docs/code-quality.md, "Mutation proofs").
#
# A ROW IS ITS EXIT CODE. The last line each gate printed is shown beside it, because a gate's
# number is the deliverable and "rc=0" alone has hidden a gate that counted nothing.
# Exit status: 0 when every row is 0; otherwise 1, and the failing rows are marked FAIL.
set -u
cd "$(dirname "$0")/../.." || exit 2
CARGO=0
[ "${1:-}" = "--cargo" ] && CARGO=1
OUT="${TMPDIR:-/tmp}/dowiz-gates.$$"
mkdir -p "$OUT"
trap 'rm -rf "$OUT"' EXIT
fail=0
n=0

run() { # run <name> <command...>
  name=$1; shift
  n=$((n + 1))
  "$@" > "$OUT/$n.log" 2>&1
  rc=$?
  # The gate's own verdict line ("<name>: ...") when it printed one, else its last line.
  last=$(grep "^$name: " "$OUT/$n.log" | tail -1)
  [ -n "$last" ] || last=$(grep -v '^[[:space:]]*$' "$OUT/$n.log" | tail -1)
  last=$(printf '%s' "$last" | cut -c1-110)
  mark=ok; [ "$rc" -eq 0 ] || { mark=FAIL; fail=1; }
  printf '%-4s %-26s rc=%-3s %s\n' "$mark" "$name" "$rc" "$last"
  if [ "$rc" -ne 0 ]; then sed 's/^/       | /' "$OUT/$n.log" | tail -15; fi
}
skip() { printf '%-4s %-26s %-6s %s\n' skip "$1" "" "$2"; }

printf '%-4s %-26s %-6s %s\n' "" "gate" "exit" "last line"

# Proofs first, then every shell gate. vocab.sh compiles tools/gen-vocab.
for p in tools/gates/*.prove.sh; do run "$(basename "$p" .sh)" sh "$p"; done
for g in tools/gates/*.sh; do
  case "$g" in
    *.prove.sh|*/run-all.sh) continue ;;
    # Needs the network, a browser and a DEPLOYED build: tools/deploy/deploy.sh runs it after the upload.
    # Run here it tested the OLD live build before the new one existed, and nested a browser inside the gates' slot (rc=137).
    */flows.sh) skip flows "live browser gate; run by tools/deploy/deploy.sh after the upload"; continue ;;
    */vocab.sh)
      if [ $CARGO = 1 ]; then run vocab sh "$g"; else skip vocab "compiles Rust; pass --cargo"; fi
      continue ;;
    # Two instrumented builds and both test suites (~15 min cold on the dev box, ~3 min warm):
    # its own CI job (`coverage` in ci.yml), never a row here. Run it by hand with
    #   bash bebop-lang/tools/slot.sh cov sh tools/gates/coverage.sh
    */coverage.sh)
      skip coverage "instrumented builds; its own CI job"
      continue ;;
    # The canvas gates (W-CANVAS1, Wave CV7) run HERE: canvas-wire.sh needs nothing, and
    # canvas-{dom,frame,ctxloss}.sh start ONE local headless Chromium each against a 127.0.0.1
    # server (no network, no deployed build) -- the flows.sh problem above does not apply.
    # A missing playwright answers exit 2 (never a pass). Their proofs ran in the loop above.
  esac
  run "$(basename "$g" .sh)" sh "$g"
done

run unreached python3 tools/gates/unreached.py
run design-gate python3 scripts/design_gate.py
# W-VERIFY 2026-09-30: browser senders vs the Worker's body structs, and icon names vs lib/icons.css.
# body-fields parses the JS with TypeScript; TS_PATH overrides where it is found.
run body-fields node tools/gates/body-fields.mjs
run icons node tools/gates/icons.mjs
# The live audits' own alarms, against stubs (the audits themselves need a venue and a token).
for p in e2e/gates/*.prove.mjs; do run "$(basename "$p" .mjs)" node "$p"; done

if [ $CARGO = 1 ]; then
  run bebop-wasm sh crates/bebop-wasm/gate.sh
else
  skip bebop-wasm "compiles Rust (wasm32); pass --cargo"
fi

echo
if [ $fail = 0 ]; then echo "run-all: $n gate(s), all exit 0"; else echo "run-all: FAILURES above ($n run)"; fi
exit $fail
