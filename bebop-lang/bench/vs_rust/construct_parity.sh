#!/usr/bin/env bash
# M7 construct parity gate: compile with bebop.bin (no C compiler),
# compare word-for-byte against frozen .bin artifacts, and verify
# execution values against frozen expected values.
ulimit -s 65536 2>/dev/null || true  # eval recursion: 113+ fn self-compile needs >8MB stack
set -u
mkdir -p "${BEBOP_TMP:-/tmp/opencode}"
BEBOPC=./seed/build/seed
BEBOP_BIN=./bebop.bin
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
    echo "WORD_MISMATCH $b"; FAIL=$((FAIL+1)); continue
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
    *) EXPECT="";;
  esac
  if [ "$IVAL" = "$EXPECT" ]; then
    echo "MATCH $b (value $IVAL)"
    PASS=$((PASS+1))
  else
    echo "VALUE_MISMATCH $b (got $IVAL, want $EXPECT)"
    FAIL=$((FAIL+1))
  fi
done

echo "construct parity: pass=$PASS fail=$FAIL"
[ "$FAIL" = 0 ]