#!/usr/bin/env bash
# M7 construct parity gate: compile with bebop.bin (no C compiler),
# compare word-for-byte against frozen .bin artifacts, and verify
# execution values against frozen expected values.
# EXPECT headers are read from construct file comments (// EXPECT <value> from:)
ulimit -s 65536 2>/dev/null || true  # eval recursion: 113+ fn self-compile needs >8MB stack
set -u
mkdir -p "${BEBOP_TMP:-/tmp/opencode}"
BEBOPC=./seed/build/seed
BEBOP_BIN=${BEBOP_BIN:-./bebop.bin}
# FREEZE=1: after the value check passes, copy the candidate .bin over the frozen
# one and print the word delta (T96: every codegen step re-freezes with an
# asserted per-construct delta). Word mismatches are then reported, not fatal.
FREEZE=${FREEZE:-0}
GUARD="GUARD: bebop.bin is missing or empty (silent-artifact class, journal 1788288248)"
[ -s "${BEBOP_BIN:-bebop.bin}" ] || { echo "$GUARD"; exit 1; }

DIR=${1:-bench/parity_constructs}
FROZEN=bench/parity_constructs/frozen
PASS=0; FAIL=0; NO_EXPECT=0

# Helper: extract EXPECT from file header (// EXPECT <value> from:)
extract_expect() {
  local file="$1"
  grep -E '// EXPECT [^ ]+ from:' "$file" | head -1 | sed 's|.*// EXPECT \([^ ]*\).*|\1|'
}

for f in "$DIR"/*.bp; do
  [ -e "$f" ] || continue  # unexpanded glob (empty/missing $DIR): fall through to the NOT MEASURED guard below
  b=$(basename "$f" .bp)

  # Extract EXPECT from file header
  EXPECT=$(extract_expect "$f")
  if [ -z "$EXPECT" ]; then
    echo "NO_EXPECT $b (missing // EXPECT header)"; NO_EXPECT=$((NO_EXPECT+1)); FAIL=$((FAIL+1)); continue
  fi

  ./seed/build/seed ${BEBOP_BIN:-bebop.bin} compile "$f" "${BEBOP_TMP:-/tmp/opencode}/${b}_test.bin" 2>/dev/null || {
    echo "COMPILEFAIL $b"; FAIL=$((FAIL+1)); continue
  }
  # Word-for-byte comparison against frozen artifact
  if ! cmp -s "${BEBOP_TMP:-/tmp/opencode}/${b}_test.bin" "$FROZEN/${b}.bin"; then
    if [ "$FREEZE" = 1 ]; then
      OLDW=0; [ -f "$FROZEN/${b}.bin" ] && OLDW=$(( $(stat -c %s "$FROZEN/${b}.bin") / 4 ))
      NEWW=$(( $(stat -c %s "${BEBOP_TMP:-/tmp/opencode}/${b}_test.bin") / 4 ))
      echo "WORD_DELTA $b $OLDW -> $NEWW words (0 = new construct)"
      # D11-F: growth needs a committed budget line `<construct> <newwords> <reason>`
      if [ "$OLDW" != 0 ] && [ "$NEWW" -gt "$OLDW" ] && ! grep -q "^$b $NEWW " bench/parity_constructs/word_budget.txt; then
        echo "WORD_BUDGET_MISSING $b ($OLDW -> $NEWW): add \"$b $NEWW <reason>\" to bench/parity_constructs/word_budget.txt"; FAIL=$((FAIL+1)); continue
      fi
    else
      echo "WORD_MISMATCH $b"; FAIL=$((FAIL+1)); continue
    fi
  fi
  # Execution value check
  # A construct that TRAPS and still prints the right number used to MATCH: only the last line of
  # stdout was read, and neither the exit code nor stderr was looked at. Measured 2026-09-13 with
  # a construct that called sys_clone with non-thread flags and a garbage stack -- it printed its
  # correct 36 and left `trap 82: SIGSEGV` in stderr, and this gate called it a pass.
  timeout 30 ./seed/build/seed "${BEBOP_TMP:-/tmp/opencode}/${b}_test.bin" \
      > "${BEBOP_TMP:-/tmp/opencode}/${b}.out" 2> "${BEBOP_TMP:-/tmp/opencode}/${b}.err"; RRC=$?
  IVAL=$(tail -1 "${BEBOP_TMP:-/tmp/opencode}/${b}.out")
  if [ "$RRC" != 0 ]; then
    echo "RUNFAIL $b (exit $RRC, stdout tail '$IVAL'): $(tail -1 "${BEBOP_TMP:-/tmp/opencode}/${b}.err" 2>/dev/null)"; FAIL=$((FAIL+1)); continue
  fi
  if [ -s "${BEBOP_TMP:-/tmp/opencode}/${b}.err" ]; then
    echo "RUNNOISE $b (exit 0 but wrote to stderr): $(tail -1 "${BEBOP_TMP:-/tmp/opencode}/${b}.err")"; FAIL=$((FAIL+1)); continue
  fi

  [ "$FREEZE" = 1 ] && [ "$IVAL" = "$EXPECT" ] && cp "${BEBOP_TMP:-/tmp/opencode}/${b}_test.bin" "$FROZEN/${b}.bin"
  if [ "$IVAL" = "$EXPECT" ]; then
    echo "MATCH $b (value $IVAL)"
    PASS=$((PASS+1))
  else
    echo "VALUE_MISMATCH $b (got $IVAL, want $EXPECT)"
    FAIL=$((FAIL+1))
  fi
done

# Negative gates (T42 2026-09-04): bench/parity_constructs/neg/*.bp must be
# REJECTED at compile time with a specific exit code and produce no .bin.
# They live outside the positive dir so invariants.sh (which fresh-compiles
# every positive construct) never sees them.
for f in "${DIR%/}/neg"/*.bp; do
  [ -e "$f" ] || continue
  b=$(basename "$f" .bp)

  # Extract EXPECT from file header
  EXPECT=$(extract_expect "$f")
  if [ -z "$EXPECT" ]; then
    echo "NO_EXPECT $b (missing // EXPECT header)"; NO_EXPECT=$((NO_EXPECT+1)); FAIL=$((FAIL+1)); continue
  fi

  out="${BEBOP_TMP:-/tmp/opencode}/${b}_test.bin"
  rm -f "$out"
  # RUNFAIL:<code> (T118): the program must COMPILE and then exit with <code> at run time
  if [ "${EXPECT%%:*}" = RUNFAIL ]; then
    want=${EXPECT#RUNFAIL:}
    ./seed/build/seed ${BEBOP_BIN:-bebop.bin} compile "$f" "$out" >/dev/null 2>&1 || { echo "TRAP_MISMATCH $b (compile failed, want run exit $want)"; FAIL=$((FAIL+1)); continue; }
    timeout 30 ./seed/build/seed "$out" >/dev/null 2>&1; rc=$?
    if [ "$rc" = "$want" ]; then echo "MATCH $b (run exit $rc)"; PASS=$((PASS+1)); else echo "TRAP_MISMATCH $b (run exit $rc, want $want)"; FAIL=$((FAIL+1)); fi
    continue
  fi
  want=${EXPECT#COMPILEFAIL:}
  ./seed/build/seed ${BEBOP_BIN:-bebop.bin} compile "$f" "$out" >/dev/null 2>&1; rc=$?
  if [ -n "$want" ] && [ "$rc" = "$want" ] && [ ! -e "$out" ]; then
    echo "MATCH $b (compile exit $rc, no .bin)"; PASS=$((PASS+1))
  else
    echo "TRAP_MISMATCH $b (compile exit $rc, want ${want:-?}$([ -e "$out" ] && echo ', .bin produced'))"; FAIL=$((FAIL+1))
  fi
done

# 2026-09-13 (battery audit): zero constructs measured is NOT MEASURED, not `pass=0 fail=0`.
if [ $((PASS + FAIL)) = 0 ]; then echo "construct parity: NOT MEASURED -- 0 constructs ran under $DIR"; exit 2; fi
echo "construct parity: pass=$PASS fail=$FAIL no_expect=$NO_EXPECT"
[ "$FAIL" = 0 ]
