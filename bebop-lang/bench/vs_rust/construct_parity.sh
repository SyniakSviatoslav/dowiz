#!/usr/bin/env bash
# M7 construct parity gate: compile with bebop.bin (no C compiler),
# compare word-for-byte against frozen .bin artifacts, and verify
# execution values against frozen expected values.
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
PASS=0; FAIL=0

for f in "$DIR"/*.bp; do
  b=$(basename "$f" .bp)
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
  IVAL=$(timeout 30 ./seed/build/seed "${BEBOP_TMP:-/tmp/opencode}/${b}_test.bin" | tail -1)
  case "$b" in
    c01_lit) EXPECT=1000000065571;;
    c02_arith) EXPECT=34;;
    c03_precedence) EXPECT=7;;
    c04_cmp) EXPECT=310;;
    c05_if) EXPECT=111;;
    c06_let) EXPECT=7;;
    c07_while) EXPECT=45;;
    c08_call) EXPECT=6;;
    c09_recursion) EXPECT=720;;
    c10_struct) EXPECT=11;;
    c11_enum) EXPECT=5;;
    c12_match) EXPECT=6;;
    c13_array) EXPECT=119;;
    c14_string) EXPECT=8;;
    c15_bitwise) EXPECT=27;;
    c16_compound) EXPECT=3;;
    c17_neg) EXPECT=-103;;
    c18_bigconst) EXPECT=-8392076198348418983;;
    c19_multi) EXPECT=115;;
    c20_deep) EXPECT=43;;
    c21_param13) EXPECT=91;;
    c22_matchbind) EXPECT=7;;
    c23_spillcall) EXPECT=110;;
    c24_ifspill) EXPECT=99;;
    c25_matchtail) EXPECT=42;;
    c26_selfrec) EXPECT=60943;;
    c27_zeroarg) EXPECT=7;;
    c30_unary) EXPECT=16351;;
    c31_nested_lit) EXPECT=1222;;
    c32_asr) EXPECT=96138;;
    c33_loopalloc) EXPECT=24999750000;;
    c34_loopescape) EXPECT=74;;
    c35_return) EXPECT=15041;;
    c36_break) EXPECT=4950014;;
    c40_struct) EXPECT=6420822;;
    c41_clz) EXPECT=64631045;;
    c42_crc32) EXPECT=1001269;;
    c43_arena_persist) EXPECT=16048003;;
    c44_use24) EXPECT=131;;
    c45_crc32x) EXPECT=1001978;;
    c46_andor) EXPECT=111100;;
    c47_usenest) EXPECT=51071;;
    c50_cas) EXPECT=7136;;
    c53_param9) EXPECT=73;;
    *) EXPECT="";;
  esac
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
  case "$b" in
    c28_plusplus) EXPECT=COMPILEFAIL:96;;
    c29_emptybody) EXPECT=COMPILEFAIL:97;;
    c37_arenafull) EXPECT=RUNFAIL:80;;
    c38_frameheap) EXPECT=RUNFAIL:81;;
    c48_stackovf) EXPECT=RUNFAIL:82;;
    c52_undef) EXPECT=RUNFAIL:87;;
    c51_casbad) EXPECT=COMPILEFAIL:88;;
    c39_fnmatch) EXPECT=COMPILEFAIL:99;;
    *) EXPECT="";;
  esac
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

echo "construct parity: pass=$PASS fail=$FAIL"
[ "$FAIL" = 0 ]