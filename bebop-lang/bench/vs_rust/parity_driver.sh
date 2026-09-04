#!/usr/bin/env bash
# M7 parity driver: compile with bebop.bin, run with seed,
# compare output against frozen expected values (no interp).
ulimit -s 65536 2>/dev/null || true  # eval recursion: 113+ fn self-compile needs >8MB stack
set -u
mkdir -p "${BEBOP_TMP:-/tmp/opencode}"
SEED=./seed/build/seed
BEBOP_BIN=./bebop.bin
GUARD="GUARD: bebop.bin is missing or empty (silent-artifact class, journal 1788288248)"
[ -s "${BEBOP_BIN:-bebop.bin}" ] || { echo "$GUARD"; exit 1; }

DIR=${1:-bench/vs_rust/kernels}
FROZEN=bench/vs_rust/kernels/frozen
PASS=0; FAIL=0; SKIP=0

for f in "$DIR"/*.bp; do
  b=$(basename "$f" .bp)
  if ! grep -qE "^fn main\(" "$f"; then
    SKIP=$((SKIP+1)); continue
  fi
  ./seed/build/seed ${BEBOP_BIN:-bebop.bin} compile "$f" "${BEBOP_TMP:-/tmp/opencode}/${b}_test.bin" 2>/dev/null || {
    echo "COMPILEFAIL $b"; FAIL=$((FAIL+1)); continue
  }
  IVAL=$(timeout 30 ./seed/build/seed "${BEBOP_TMP:-/tmp/opencode}/${b}_test.bin" | tail -1)
  case "$b" in
    hv_stdlib) EXPECT=1;;
    io_probe) EXPECT=110741101;;
    k1) EXPECT=500000500000;;
    k2) EXPECT=75025;;
    k3) EXPECT=67725000;;
    k4) EXPECT=-7260594028850897471;;
    k5) EXPECT=759186635;;
    k6) EXPECT=236;;
    k7) EXPECT=3939697352;;
    k7neon) EXPECT=3939697352;;
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
    *) EXPECT="";;
  esac
  if [ "$IVAL" = "$EXPECT" ]; then
    echo "MATCH $b (value $IVAL)"
    PASS=$((PASS+1))
  else
    echo "MISMATCH $b (got $IVAL, want $EXPECT)"
    FAIL=$((FAIL+1))
  fi
done

echo "parity: pass=$PASS fail=$FAIL skip=$SKIP"
[ "$FAIL" = 0 ]