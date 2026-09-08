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
    # ROADMAP A6 (2026-09-08): was a NEGATIVE gate (neg/c38_frameheap.bp,
    # EXPECT=RUNFAIL:81). A6 deletes the frame heap and with it exit 81, so the
    # 20 KiB of single-activation aggregates this program allocates now RUN on the
    # arena cursor. Re-derived with `python3 tools/bpref.py`, not assumed: 2559.
    c38_frameheap) EXPECT=2559;;
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
    c70_csel) EXPECT=-162834;;
    c71_csel_impure) EXPECT=-164832;;
    c55_vswindow) EXPECT=312;;
    c56_nest) EXPECT=240;;
    c57_flags) EXPECT=13;;
    c58_callmix) EXPECT=241;;
    c59_evict) EXPECT=25;;
    c60_nestctor) EXPECT=1;;
    c61_arrcall) EXPECT=3;;
    c66_fncap) EXPECT=1519;;
    c72_hoist) EXPECT=5504683299252448320;;
    c84_run) EXPECT=1035;;
    c86_selfassign) EXPECT=103;;
    c89_heaptrap) EXPECT=33;;
    c90_symalias) EXPECT=0;;
    c88_arrflags) EXPECT=1;;
    c87_ifselfassign) EXPECT=3;;
    c91_letlive) EXPECT=12;;
    c78_scan) EXPECT=-6715473280576199194;;
    c94_fsync) EXPECT=0;;
    # A14b (2026-09-08): the regression guard for the SYM-entry span fix -- a pending `SYM v`
    # operand below an `if` with impure arms, a write to `v` inside an arm (vs_settle_sym_alias
    # materialises the pre-arm entry from INSIDE that arm) and a `let` binder in the same arm
    # under window pressure. Exited 89 on 42ce19e5, runs 17 (bpref) once vs_span_to_slots also
    # demotes kind-3 SYM entries. Drop either ingredient and it compiled clean before the fix.
    c95_symspan) EXPECT=17;;
    c110_fence) EXPECT=0;;
    c111_kernelfn) EXPECT=315;;
    # A2b (2026-09-08): the regression guard for lifting A2's nested-`while` hoist ban --
    # an outer loop that hoists 1000003 at its own depth 0 AND nests a `while` that uses the
    # same literal. The nested loop releases the enclosing pairs at its entry (so its own
    # cs-mask-must-be-0 assertion still holds) and the outer pair is re-materialised after the
    # inner loop's backward branch, so `m` reads correctly on BOTH sides of the nested loop.
    c73_hoistnest) EXPECT=24000282;;
    # A2b step 2a: vs_try_madd folds `a * b + c` backwards into the multiply's own word by
    # rewriting its Ra field, when the multiply is still the last word emitted and the addend
    # needs no materialisation. Four folding shapes plus one that must DECLINE (a CONST addend,
    # whose movz would land after the word being rewritten). EXPECT from tools/bpref.py.
    c74_madd) EXPECT=82837312;;
    # A2b step 2b: vs_try_and_imm emits `and Xd,Xn,#C` for C = 2^k - 1 and declines every other
    # mask (6 = 110b, and 0) back to the old materialise-then-and path. EXPECT from tools/bpref.py.
    c75_andimm) EXPECT=1477639;;
    # A2b step 2c: vs_try_ubfx collapses `(e >> lsb) & (2^width - 1)` into the shift's own
    # UBFM word, and declines when the extract would run off the top (60+8), when the shift is
    # an asr (SBFM) and when it is a left shift (imms != 63). EXPECT from tools/bpref.py.
    c76_ubfx) EXPECT=2271612;;
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
    c48_stackovf) EXPECT=RUNFAIL:82;;
    c52_undef) EXPECT=RUNFAIL:87;;
    c51_casbad) EXPECT=COMPILEFAIL:88;;
    c39_fnmatch) EXPECT=COMPILEFAIL:99;;
    c85_param15) EXPECT=COMPILEFAIL:100;;
    c93_unbound) EXPECT=COMPILEFAIL:101;;
    c112_kernelsys) EXPECT=COMPILEFAIL:102;;
    # A14b (2026-09-08): the shrunk seed-100744 repro. It used to exit 89 in the register
    # allocator (a pre-arm SYM entry relocated inside one if-arm, then a colliding let binder);
    # A14b removed that, so the compiler now reaches the parser and reports the program's REAL
    # defect -- `main` has no tail expression. Re-derived, not assumed: `python3 tools/bpref.py
    # bench/parity_constructs/neg/c92_letlive2.bp` says "fn main: body has no tail expression
    # (bebop.bin exits 97)". The positive regression guard for the fix is c95_symspan.
    c92_letlive2) EXPECT=COMPILEFAIL:97;;
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