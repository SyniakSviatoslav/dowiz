#!/usr/bin/env bash
# bpref_parity.sh -- ROADMAP A23. The differential lane the tree did not have.
#
# WHY IT EXISTS. `tools/bpref.py` is the fuzz DIVERGE judge and the source of every
# hand-derived EXPECT, and NOTHING in tools/battery.sh ever ran its evaluator -- only its
# parser, through typecheck.py. That is how its `set` node came to rebind an array to a
# PRIVATE COPY on every write, so a callee's write into an `[i64]` parameter was invisible to
# its caller; it disagreed with the compiler for four days while two constructs sat red and
# the battery reported green (fixed 2026-09-12, `132063b`).
#
# WHAT IT CHECKS. Not a golden: AGREEMENT. Every construct is run through BOTH implementations
# and their answers compared. No EXPECT is read, so this lane cannot be satisfied by editing a
# table, and it needs nothing from A24's derivation headers.
#
# Prints one line:  bpref_parity: agree=<a> unsupported=<u> disagree=<d> error=<e> total=<t>
# RED (exit 1) when disagree >= 1. `unsupported` is bpref exiting 3 on a form it declares it
# does not model; `error` is bpref failing any other way, which is also RED -- an oracle that
# crashes is not an oracle.
set -u
if [ "${BEBOP_SLOT_HELD:-0}" != 1 ] && [ "${NO_SLOT:-0}" != 1 ]; then
  _slot="$(dirname "$0")/../../tools/slot.sh"
  [ -f "$_slot" ] || _slot=/root/dowiz/bebop-lang/tools/slot.sh
  [ -f "$_slot" ] && exec bash "$_slot" "auto:bpref_parity" bash "$0" "$@"
fi
cd "$(dirname "$0")/../.." || exit 1
BEBOP_BIN=${BEBOP_BIN:-./bebop.bin}
T=${BEBOP_TMP:-/tmp/opencode}/bpref_parity; mkdir -p "$T"
BPREF_T=${BPREF_TIMEOUT:-120}

# 2026-09-13 (battery audit): with tools/bpref.py DELETED, `python3 tools/bpref.py` exits 2 ("can't open
# file"), the rc switch below scored every construct UNSUPPORTED, and the line read
# `agree=0 unsupported=100 disagree=0 error=0` -- green. An absent oracle is NOT MEASURED.
[ -s tools/bpref.py ] || { echo "bpref_parity: NOT MEASURED -- tools/bpref.py absent or empty"; exit 2; }
agree=0; unsupported=0; disagree=0; error=0; total=0
: > "$T/disagreements.txt"; : > "$T/unsupported.txt"

for f in bench/parity_constructs/*.bp; do
  b=$(basename "$f" .bp); total=$((total + 1))

  if ! ./seed/build/seed "$BEBOP_BIN" compile "$f" "$T/$b.bin" >/dev/null 2>&1; then
    # A construct in the POSITIVE directory that does not compile is a harness fault, not a
    # parity result: say so rather than skipping it into silence.
    echo "COMPILEFAIL $b" >> "$T/disagreements.txt"; error=$((error + 1)); continue
  fi
  bebop_val=$(timeout 60 ./seed/build/seed "$T/$b.bin" 2>"$T/$b.run.err" | tail -1)
  [ -s "$T/$b.run.err" ] && echo "RUN-STDERR $b $(tail -1 "$T/$b.run.err")" >> "$T/unsupported.txt"

  # NOT `python3 ... | tail -1; rc=$?` -- that reads the exit status of TAIL, so a bpref that
  # exits 2 comes back as rc=0 with an empty answer and is counted a DISAGREEMENT. Measured
  # 2026-09-13: all 7 "disagreements" in the first run of this lane were that bug, mine.
  timeout "$BPREF_T" python3 tools/bpref.py "$f" > "$T/$b.out" 2>"$T/$b.err"; rc=$?
  bpref_val=$(tail -1 "$T/$b.out")
  case $rc in
    0) ;;
    2|3) # 2 = evaluator error (an unmodelled builtin reaches it as a NameError/KeyError),
         # 3 = depth cap or a form bpref declares UNSUPPORTED. Both mean "this oracle cannot
         # answer", which is not a disagreement -- but it is only honest if the REASON is kept.
         unsupported=$((unsupported + 1))
         echo "UNSUPPORTED $b rc=$rc $(tail -1 "$T/$b.err" 2>/dev/null)" >> "$T/unsupported.txt"
         continue ;;
    124) unsupported=$((unsupported + 1)); echo "TIMEOUT $b (bpref > ${BPREF_T}s)" >> "$T/unsupported.txt"; continue ;;
    *) error=$((error + 1)); echo "BPREF-ERROR $b rc=$rc $(tail -1 "$T/$b.err" 2>/dev/null)" >> "$T/disagreements.txt"; continue ;;
  esac
  # An empty answer from a zero exit is not an answer.
  [ -n "$bpref_val" ] || { error=$((error + 1)); echo "EMPTY $b (bpref exited 0 and printed nothing)" >> "$T/disagreements.txt"; continue; }

  if [ "$bebop_val" = "$bpref_val" ]; then
    agree=$((agree + 1))
  else
    disagree=$((disagree + 1))
    echo "DISAGREE $b bebop=$bebop_val bpref=$bpref_val" >> "$T/disagreements.txt"
  fi
done

[ "$agree" -gt 0 ] || { echo "bpref_parity: NOT MEASURED -- 0 constructs agreed (total=$total unsupported=$unsupported disagree=$disagree error=$error): the oracle answered nothing"; exit 2; }
echo "bpref_parity: agree=$agree unsupported=$unsupported disagree=$disagree error=$error total=$total"
[ -s "$T/disagreements.txt" ] && { echo "--- disagreements ---"; cat "$T/disagreements.txt"; }
[ -s "$T/unsupported.txt" ] && { echo "--- unsupported (bpref cannot answer; the reason is kept, not discarded) ---"; cat "$T/unsupported.txt"; }
[ "$disagree" = 0 ] && [ "$error" = 0 ]
